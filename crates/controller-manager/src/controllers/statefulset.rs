use anyhow::Result;
use rusternetes_common::resources::{Pod, PodStatus, StatefulSet, StatefulSetStatus};
use rusternetes_common::types::{OwnerReference, Phase};
use rusternetes_storage::Storage;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info};

pub struct StatefulSetController<S: Storage> {
    storage: Arc<S>,
}

impl<S: Storage> StatefulSetController<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting StatefulSetController");

        loop {
            if let Err(e) = self.reconcile_all().await {
                error!("Error in StatefulSet reconciliation loop: {}", e);
            }
            time::sleep(Duration::from_secs(5)).await;
        }
    }

    pub async fn reconcile_all(&self) -> Result<()> {
        let statefulsets: Vec<StatefulSet> = self.storage.list("/registry/statefulsets/").await?;

        for mut statefulset in statefulsets {
            if let Err(e) = self.reconcile(&mut statefulset).await {
                error!(
                    "Failed to reconcile StatefulSet {}: {}",
                    statefulset.metadata.name, e
                );
            }
        }

        Ok(())
    }

    async fn reconcile(&self, statefulset: &mut StatefulSet) -> Result<()> {
        let name = &statefulset.metadata.name;
        let namespace = statefulset.metadata.namespace.as_ref().unwrap();

        // Skip reconciliation for StatefulSets being deleted — GC handles pod cleanup
        if statefulset.metadata.is_being_deleted() {
            return Ok(());
        }

        info!("Reconciling StatefulSet {}/{}", namespace, name);

        let desired_replicas = statefulset.spec.replicas.unwrap_or(1);

        // Get current pods for this StatefulSet
        let pod_prefix = format!("/registry/pods/{}/", namespace);
        let all_pods: Vec<Pod> = self.storage.list(&pod_prefix).await?;

        // Filter pods that belong to this StatefulSet via ownerReferences (authoritative)
        // Fall back to label matching for backwards compatibility
        let statefulset_uid = &statefulset.metadata.uid;
        let mut statefulset_pods: Vec<Pod> = all_pods
            .into_iter()
            .filter(|pod| {
                let owned_by_ref = pod
                    .metadata
                    .owner_references
                    .as_ref()
                    .map(|refs| refs.iter().any(|r| &r.uid == statefulset_uid))
                    .unwrap_or(false);
                let owned_by_label = pod
                    .metadata
                    .labels
                    .as_ref()
                    .and_then(|labels| labels.get("app"))
                    .map(|app| app == name)
                    .unwrap_or(false)
                    && pod
                        .metadata
                        .labels
                        .as_ref()
                        .and_then(|labels| labels.get("statefulset.kubernetes.io/pod-name"))
                        .is_some();
                owned_by_ref || owned_by_label
            })
            .collect();

        // Sort pods by ordinal index
        statefulset_pods.sort_by_key(|pod| {
            pod.metadata
                .name
                .rsplit_once('-')
                .and_then(|(_, idx)| idx.parse::<i32>().ok())
                .unwrap_or(0)
        });

        let current_replicas = statefulset_pods.len() as i32;

        info!(
            "StatefulSet {}/{}: desired={}, current={}",
            namespace, name, desired_replicas, current_replicas
        );

        // Scale up or down
        if current_replicas < desired_replicas {
            // Scale up: create pods in order
            for i in current_replicas..desired_replicas {
                self.create_pod(statefulset, i, namespace).await?;
                info!("Created pod {}-{}", name, i);

                // Wait for pod to be ready before creating next one (ordered deployment)
                if statefulset
                    .spec
                    .pod_management_policy
                    .as_ref()
                    .map(|p| p == "OrderedReady")
                    .unwrap_or(true)
                {
                    time::sleep(Duration::from_secs(2)).await;
                }
            }
        } else if current_replicas > desired_replicas {
            // Scale down: delete pods in reverse order
            for i in (desired_replicas..current_replicas).rev() {
                let pod_name = format!("{}-{}", name, i);
                let pod_key = format!("/registry/pods/{}/{}", namespace, pod_name);
                self.storage.delete(&pod_key).await?;
                info!("Deleted pod {}", pod_name);

                // Wait between deletions for OrderedReady policy
                if statefulset
                    .spec
                    .pod_management_policy
                    .as_ref()
                    .map(|p| p == "OrderedReady")
                    .unwrap_or(true)
                {
                    time::sleep(Duration::from_secs(2)).await;
                }
            }
        }

        // Re-fetch and recount pods after create/delete operations to get accurate status
        let pod_prefix = format!("/registry/pods/{}/", namespace);
        let all_pods_after: Vec<Pod> = self.storage.list(&pod_prefix).await?;

        let statefulset_pods_after: Vec<Pod> = all_pods_after
            .into_iter()
            .filter(|pod| {
                let owned_by_ref = pod
                    .metadata
                    .owner_references
                    .as_ref()
                    .map(|refs| refs.iter().any(|r| &r.uid == statefulset_uid))
                    .unwrap_or(false);
                let owned_by_label = pod
                    .metadata
                    .labels
                    .as_ref()
                    .and_then(|labels| labels.get("statefulset.kubernetes.io/pod-name"))
                    .is_some()
                    && pod
                        .metadata
                        .labels
                        .as_ref()
                        .and_then(|labels| labels.get("app"))
                        .map(|app| app == name)
                        .unwrap_or(false);
                owned_by_ref || owned_by_label
            })
            .collect();

        let final_current_replicas = statefulset_pods_after.len() as i32;
        let final_ready_pods = statefulset_pods_after
            .iter()
            .filter(|pod| {
                pod.status
                    .as_ref()
                    .map(|s| s.phase == Some(Phase::Running))
                    .unwrap_or(false)
            })
            .count() as i32;

        // Update status with accurate counts
        statefulset.status = Some(StatefulSetStatus {
            replicas: final_current_replicas.min(desired_replicas),
            ready_replicas: Some(final_ready_pods),
            current_replicas: Some(final_current_replicas),
            updated_replicas: Some(final_current_replicas.min(desired_replicas)),
            available_replicas: None,
            collision_count: None,
            observed_generation: None,
            current_revision: None,
            update_revision: None,
            conditions: None,
        });

        // Save updated status
        let key = format!("/registry/statefulsets/{}/{}", namespace, name);
        self.storage.update(&key, statefulset).await?;

        Ok(())
    }

    async fn create_pod(
        &self,
        statefulset: &StatefulSet,
        ordinal: i32,
        namespace: &str,
    ) -> Result<()> {
        let statefulset_name = &statefulset.metadata.name;
        let pod_name = format!("{}-{}", statefulset_name, ordinal);

        // Create pod from template
        let template = &statefulset.spec.template;
        let mut labels = template
            .metadata
            .as_ref()
            .and_then(|m| m.labels.clone())
            .unwrap_or_default();
        labels.insert("app".to_string(), statefulset_name.clone());
        labels.insert(
            "statefulset.kubernetes.io/pod-name".to_string(),
            pod_name.clone(),
        );

        let mut metadata = rusternetes_common::types::ObjectMeta::new(pod_name.clone())
            .with_namespace(namespace.to_string())
            .with_labels(labels)
            .with_owner_reference(OwnerReference {
                api_version: "apps/v1".to_string(),
                kind: "StatefulSet".to_string(),
                name: statefulset_name.clone(),
                uid: statefulset.metadata.uid.clone(),
                controller: Some(true),
                block_owner_deletion: Some(true),
            });

        if let Some(template_meta) = &template.metadata {
            if let Some(ref annotations) = template_meta.annotations {
                metadata.annotations = Some(annotations.clone());
            }
        }

        let pod = Pod {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "Pod".to_string(),
                api_version: "v1".to_string(),
            },
            metadata,
            spec: Some(template.spec.clone()),
            status: Some(PodStatus {
                phase: Some(Phase::Pending),
                message: None,
                reason: None,
                pod_ip: None,
                host_ip: None,
                conditions: None,
                container_statuses: None,
                init_container_statuses: None,
                ephemeral_container_statuses: None,
                resize: None,
                resource_claim_statuses: None,
                observed_generation: None,
                host_i_ps: None,
                pod_i_ps: None,
                nominated_node_name: None,
                qos_class: None,
                start_time: None,
            }),
        };

        let key = format!("/registry/pods/{}/{}", namespace, pod_name);
        self.storage.create(&key, &pod).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_pod_name_generation() {
        let statefulset_name = "web";
        let ordinal = 2;
        let pod_name = format!("{}-{}", statefulset_name, ordinal);
        assert_eq!(pod_name, "web-2");
    }

    #[test]
    fn test_pod_ordinal_parsing() {
        let pod_name = "web-5";
        let ordinal: i32 = pod_name
            .rsplit_once('-')
            .and_then(|(_, idx)| idx.parse().ok())
            .unwrap();
        assert_eq!(ordinal, 5);
    }
}
