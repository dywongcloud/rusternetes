use anyhow::Result;
use rusternetes_common::resources::{
    PersistentVolumeClaim, PersistentVolumeClaimSpec, Pod, PodStatus, StatefulSet,
    StatefulSetStatus,
};
use rusternetes_common::types::{ObjectMeta, OwnerReference, Phase, TypeMeta};
use rusternetes_storage::{build_key, Storage};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info};

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
            time::sleep(Duration::from_secs(1)).await;
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
            let is_ordered_ready = statefulset
                .spec
                .pod_management_policy
                .as_ref()
                .map(|p| p == "OrderedReady")
                .unwrap_or(true);

            // Scale up: create pods in order
            for i in current_replicas..desired_replicas {
                // For OrderedReady policy, check that the previous pod is Ready before
                // creating the next one. If it's not ready, halt scaling.
                if is_ordered_ready && i > 0 {
                    let prev_pod_name = format!("{}-{}", name, i - 1);
                    let prev_pod_key =
                        build_key("pods", Some(namespace), &prev_pod_name);
                    match self.storage.get::<Pod>(&prev_pod_key).await {
                        Ok(prev_pod) => {
                            let is_ready = prev_pod
                                .status
                                .as_ref()
                                .and_then(|s| s.conditions.as_ref())
                                .map(|conditions| {
                                    conditions.iter().any(|c| {
                                        c.condition_type == "Ready"
                                            && c.status == "True"
                                    })
                                })
                                .unwrap_or(false);

                            if !is_ready {
                                debug!(
                                    "StatefulSet {}: pod {} not ready, halting scale-up",
                                    name, prev_pod_name
                                );
                                break;
                            }
                        }
                        Err(_) => {
                            // Previous pod doesn't exist yet
                            debug!(
                                "StatefulSet {}: pod {} not found, halting scale-up",
                                name, prev_pod_name
                            );
                            break;
                        }
                    }
                }

                // Ensure PVCs exist for this ordinal before creating the pod
                self.ensure_pvcs_for_ordinal(statefulset, i, namespace)
                    .await?;
                self.create_pod(statefulset, i, namespace).await?;
                info!("Created pod {}-{}", name, i);
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

        // Rolling update: if replica count matches but pods have old revision, delete one at a time
        // The controller will recreate them with the new template on the next reconcile.
        if current_replicas == desired_replicas && desired_replicas > 0 {
            let update_revision = Self::compute_revision(&statefulset.spec.template);
            // Check pods in reverse order for rolling update
            for pod in statefulset_pods.iter().rev() {
                let pod_revision = pod.metadata.labels.as_ref()
                    .and_then(|l| l.get("controller-revision-hash"))
                    .map(|s| s.as_str())
                    .unwrap_or("");
                if pod_revision != update_revision && !pod_revision.is_empty() {
                    // Check if the pod is Ready before deleting (OrderedReady)
                    let is_ready = pod.status.as_ref()
                        .and_then(|s| s.conditions.as_ref())
                        .map(|c| c.iter().any(|cond| cond.condition_type == "Ready" && cond.status == "True"))
                        .unwrap_or(false);
                    if is_ready {
                        let pod_key = format!("/registry/pods/{}/{}", namespace, pod.metadata.name);
                        self.storage.delete(&pod_key).await?;
                        info!("Rolling update: deleted pod {} (old revision {})", pod.metadata.name, pod_revision);
                        break; // Delete one at a time
                    }
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
                    .and_then(|s| s.conditions.as_ref())
                    .map(|conditions| {
                        conditions.iter().any(|c| c.condition_type == "Ready" && c.status == "True")
                    })
                    .unwrap_or(false)
            })
            .count() as i32;

        // Generate a revision hash from the current pod template spec
        let update_revision = Self::compute_revision(&statefulset.spec.template);

        // The current_revision is the revision that existing pods are running.
        // During a rolling update, this differs from update_revision.
        // Preserve the existing current_revision if set, otherwise derive from pods.
        let current_revision = statefulset.status
            .as_ref()
            .and_then(|s| s.current_revision.clone())
            .or_else(|| {
                // No current_revision in status — derive from actual pod labels
                statefulset_pods_after.iter()
                    .find_map(|pod| pod.metadata.labels.as_ref()
                        .and_then(|l| l.get("controller-revision-hash"))
                        .cloned())
            })
            .unwrap_or_else(|| update_revision.clone());

        // Count how many pods match the update revision (have the matching controller-revision-hash label)
        let updated_count = statefulset_pods_after.iter().filter(|pod| {
            pod.metadata.labels.as_ref()
                .and_then(|l| l.get("controller-revision-hash"))
                .map(|h| h == &update_revision)
                .unwrap_or(false)
        }).count() as i32;

        // Update status with accurate counts
        statefulset.status = Some(StatefulSetStatus {
            replicas: final_current_replicas,
            ready_replicas: Some(final_ready_pods),
            current_replicas: Some(final_current_replicas),
            updated_replicas: Some(updated_count),
            available_replicas: Some(final_ready_pods),
            collision_count: None,
            observed_generation: statefulset.metadata.generation,
            current_revision: Some(if updated_count >= desired_replicas { update_revision.clone() } else { current_revision }),
            update_revision: Some(update_revision),
            conditions: None,
        });

        // Save updated status
        let key = format!("/registry/statefulsets/{}/{}", namespace, name);
        self.storage.update(&key, statefulset).await?;

        // Ensure a ControllerRevision exists for the current template revision
        let revision = Self::compute_revision(&statefulset.spec.template);
        let cr_name = format!("{}-{}", name, &revision[..std::cmp::min(10, revision.len())]);
        let cr_key = format!("/registry/controllerrevisions/{}/{}", namespace, cr_name);
        if self.storage.get::<serde_json::Value>(&cr_key).await.is_err() {
            // Create the ControllerRevision
            let template_data = serde_json::to_value(&statefulset.spec.template).unwrap_or_default();
            let cr = serde_json::json!({
                "apiVersion": "apps/v1",
                "kind": "ControllerRevision",
                "metadata": {
                    "name": cr_name,
                    "namespace": namespace,
                    "uid": uuid::Uuid::new_v4().to_string(),
                    "creationTimestamp": chrono::Utc::now().to_rfc3339(),
                    "labels": {
                        "controller.kubernetes.io/hash": revision,
                        "app": name
                    },
                    "ownerReferences": [{
                        "apiVersion": "apps/v1",
                        "kind": "StatefulSet",
                        "name": name,
                        "uid": statefulset.metadata.uid,
                        "controller": true,
                        "blockOwnerDeletion": true
                    }]
                },
                "data": template_data,
                "revision": 1
            });
            if let Err(e) = self.storage.create(&cr_key, &cr).await {
                debug!("ControllerRevision {} already exists or failed: {}", cr_name, e);
            } else {
                info!("Created ControllerRevision {} for StatefulSet {}/{}", cr_name, namespace, name);
            }
        }

        Ok(())
    }

    async fn ensure_pvcs_for_ordinal(
        &self,
        statefulset: &StatefulSet,
        ordinal: i32,
        namespace: &str,
    ) -> Result<()> {
        if let Some(ref templates) = statefulset.spec.volume_claim_templates {
            for template in templates {
                let pvc_name = format!(
                    "{}-{}-{}",
                    template.metadata.name, statefulset.metadata.name, ordinal
                );
                let key = build_key("persistentvolumeclaims", Some(namespace), &pvc_name);

                // Check if PVC already exists
                if self
                    .storage
                    .get::<PersistentVolumeClaim>(&key)
                    .await
                    .is_ok()
                {
                    continue; // PVC already exists
                }

                // Create PVC from template
                let mut pvc_metadata =
                    ObjectMeta::new(&pvc_name).with_namespace(namespace.to_string());

                // Copy labels and annotations from template metadata
                if let Some(ref tmpl_labels) = template.metadata.labels {
                    pvc_metadata.labels = Some(tmpl_labels.clone());
                }
                if let Some(ref tmpl_annotations) = template.metadata.annotations {
                    pvc_metadata.annotations = Some(tmpl_annotations.clone());
                }

                // Set owner reference to the StatefulSet
                pvc_metadata.owner_references = Some(vec![OwnerReference {
                    api_version: "apps/v1".to_string(),
                    kind: "StatefulSet".to_string(),
                    name: statefulset.metadata.name.clone(),
                    uid: statefulset.metadata.uid.clone(),
                    controller: Some(true),
                    block_owner_deletion: Some(true),
                }]);

                let pvc = PersistentVolumeClaim {
                    type_meta: TypeMeta {
                        kind: "PersistentVolumeClaim".to_string(),
                        api_version: "v1".to_string(),
                    },
                    metadata: pvc_metadata,
                    spec: template.spec.clone(),
                    status: None,
                };

                self.storage.create(&key, &pvc).await?;
                info!(
                    "Created PVC {} for StatefulSet {}/{}",
                    pvc_name, namespace, statefulset.metadata.name
                );
            }
        }
        Ok(())
    }

    /// Compute a revision string from the pod template spec.
    /// This produces a deterministic hash like "controller-revision-hash-<hash>".
    fn compute_revision(template: &rusternetes_common::resources::PodTemplateSpec) -> String {
        let serialized = serde_json::to_string(&template.spec).unwrap_or_default();
        let mut hasher = DefaultHasher::new();
        serialized.hash(&mut hasher);
        let hash = hasher.finish();
        // Format as a 10-char hex string, similar to Kubernetes controller revision hashes
        format!("{:010x}", hash)
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
        // Set the controller-revision-hash label so tests can verify pod revision
        let revision = Self::compute_revision(&statefulset.spec.template);
        labels.insert(
            "controller-revision-hash".to_string(),
            revision,
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
