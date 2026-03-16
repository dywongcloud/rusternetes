use rusternetes_common::{
    resources::{Pod, PodStatus, ReplicaSet, ReplicaSetStatus},
    types::{ObjectMeta, Phase},
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::{sync::Arc, time::Duration};
use tracing::{debug, error, info};

/// ReplicaSetController reconciles ReplicaSet resources
/// A ReplicaSet ensures that a specified number of pod replicas are running at any given time
pub struct ReplicaSetController<S: Storage> {
    storage: Arc<S>,
    interval: Duration,
}

impl<S: Storage> ReplicaSetController<S> {
    pub fn new(storage: Arc<S>, interval_secs: u64) -> Self {
        Self {
            storage,
            interval: Duration::from_secs(interval_secs),
        }
    }

    pub async fn run(&self) -> rusternetes_common::Result<()> {
        info!(
            "ReplicaSet controller started, syncing every {:?}",
            self.interval
        );

        let mut interval = tokio::time::interval(self.interval);

        loop {
            interval.tick().await;
            if let Err(e) = self.reconcile_all().await {
                error!("Error reconciling replicasets: {}", e);
            }
        }
    }

    pub async fn reconcile_all(&self) -> rusternetes_common::Result<()> {
        debug!("Reconciling all replicasets");

        // Get all replicasets
        let prefix = build_prefix("replicasets", None);
        let replicasets: Vec<ReplicaSet> = self.storage.list(&prefix).await?;

        for replicaset in replicasets {
            if let Err(e) = self.reconcile_replicaset(&replicaset).await {
                error!(
                    "Error reconciling replicaset {}: {}",
                    replicaset.metadata.name, e
                );
            }
        }

        Ok(())
    }

    async fn reconcile_replicaset(
        &self,
        replicaset: &ReplicaSet,
    ) -> rusternetes_common::Result<()> {
        let namespace = replicaset
            .metadata
            .namespace
            .as_deref()
            .unwrap_or("default");

        debug!(
            "Reconciling replicaset: {}/{}",
            namespace, replicaset.metadata.name
        );

        // Get all pods for this replicaset
        let pods_prefix = build_prefix("pods", Some(namespace));
        let all_pods: Vec<Pod> = self.storage.list(&pods_prefix).await?;

        // Filter pods that match this replicaset's selector
        let replicaset_pods: Vec<Pod> = all_pods
            .into_iter()
            .filter(|p| {
                let matches = self.matches_selector(p, replicaset);
                debug!(
                    "Pod {} matches selector: {} (labels: {:?})",
                    p.metadata.name, matches, p.metadata.labels
                );
                matches
            })
            .collect();

        // Count ready and available pods
        let ready_count = replicaset_pods
            .iter()
            .filter(|p| self.is_pod_ready(p))
            .count() as i32;

        let available_count = replicaset_pods
            .iter()
            .filter(|p| self.is_pod_available(p, replicaset))
            .count() as i32;

        let current_replicas = replicaset_pods.len() as i32;
        let desired_replicas = replicaset.spec.replicas;

        info!(
            "ReplicaSet {}/{}: current={}, ready={}, available={}, desired={}",
            namespace,
            replicaset.metadata.name,
            current_replicas,
            ready_count,
            available_count,
            desired_replicas
        );

        // Reconcile pod count
        if current_replicas < desired_replicas {
            // Need to create more pods
            let to_create = desired_replicas - current_replicas;
            info!(
                "Creating {} pods for replicaset {}/{}",
                to_create, namespace, replicaset.metadata.name
            );
            for _ in 0..to_create {
                self.create_pod(replicaset).await?;
            }
        } else if current_replicas > desired_replicas {
            // Need to delete excess pods
            let to_delete = current_replicas - desired_replicas;
            info!(
                "Deleting {} excess pods for replicaset {}/{}",
                to_delete, namespace, replicaset.metadata.name
            );
            for pod in replicaset_pods.iter().take(to_delete as usize) {
                self.delete_pod(&pod.metadata.name, namespace).await?;
            }
        }

        // Re-fetch and recount pods after create/delete operations to get accurate status
        let pods_prefix = build_prefix("pods", Some(namespace));
        let all_pods_after: Vec<Pod> = self.storage.list(&pods_prefix).await?;

        let replicaset_pods_after: Vec<Pod> = all_pods_after
            .into_iter()
            .filter(|p| self.matches_selector(p, replicaset))
            .collect();

        let final_ready_count = replicaset_pods_after
            .iter()
            .filter(|p| self.is_pod_ready(p))
            .count() as i32;

        let final_available_count = replicaset_pods_after
            .iter()
            .filter(|p| self.is_pod_available(p, replicaset))
            .count() as i32;

        let final_current_replicas = replicaset_pods_after.len() as i32;

        // Update status with accurate counts
        self.update_status(
            replicaset,
            final_current_replicas,
            final_ready_count,
            final_available_count,
        )
        .await?;

        Ok(())
    }

    fn matches_selector(&self, pod: &Pod, replicaset: &ReplicaSet) -> bool {
        if let Some(match_labels) = &replicaset.spec.selector.match_labels {
            if let Some(pod_labels) = &pod.metadata.labels {
                for (key, value) in match_labels {
                    if pod_labels.get(key) != Some(value) {
                        return false;
                    }
                }
                return true;
            }
        }
        false
    }

    fn is_pod_ready(&self, pod: &Pod) -> bool {
        if let Some(status) = &pod.status {
            status.phase == Some(Phase::Running)
        } else {
            false
        }
    }

    fn is_pod_available(&self, pod: &Pod, replicaset: &ReplicaSet) -> bool {
        if !self.is_pod_ready(pod) {
            return false;
        }

        // Check if pod has been ready for minReadySeconds
        let min_ready_seconds = replicaset.spec.min_ready_seconds.unwrap_or(0);
        if min_ready_seconds > 0 {
            // Get pod creation time as a proxy for when it became ready
            // In a full implementation, we'd check the Ready condition's lastTransitionTime
            if let Some(creation_time) = pod.metadata.creation_timestamp {
                let now = chrono::Utc::now();
                let elapsed = now.signed_duration_since(creation_time);

                // Pod is available if it's been ready for at least minReadySeconds
                return elapsed.num_seconds() >= min_ready_seconds as i64;
            }
            // If no timestamp, can't determine availability
            false
        } else {
            // If minReadySeconds is 0, pod is available as soon as it's ready
            true
        }
    }

    async fn update_status(
        &self,
        replicaset: &ReplicaSet,
        replicas: i32,
        ready_replicas: i32,
        available_replicas: i32,
    ) -> rusternetes_common::Result<()> {
        let namespace = replicaset
            .metadata
            .namespace
            .as_deref()
            .unwrap_or("default");

        let status = ReplicaSetStatus {
            replicas,
            ready_replicas,
            available_replicas,
            fully_labeled_replicas: Some(replicas), // All pods matching selector are fully labeled
            observed_generation: None,              // TODO: Track generation properly
            conditions: None, // TODO: Add conditions for ReplicaSetReplicaFailure, etc.
        };

        let mut updated_rs = replicaset.clone();
        updated_rs.status = Some(status);

        let key = build_key("replicasets", Some(namespace), &replicaset.metadata.name);
        self.storage.update(&key, &updated_rs).await?;

        debug!(
            "Updated status for replicaset {}/{}: replicas={}, ready={}, available={}",
            namespace, replicaset.metadata.name, replicas, ready_replicas, available_replicas
        );

        Ok(())
    }

    async fn create_pod(&self, replicaset: &ReplicaSet) -> rusternetes_common::Result<()> {
        let namespace = replicaset
            .metadata
            .namespace
            .as_deref()
            .unwrap_or("default");

        let pod_name = format!("{}-{}", replicaset.metadata.name, uuid::Uuid::new_v4());

        let mut metadata = ObjectMeta::new(&pod_name);
        metadata.namespace = Some(namespace.to_string());
        metadata.labels = replicaset
            .spec
            .template
            .metadata
            .as_ref()
            .and_then(|m| m.labels.clone());

        // Set owner reference so pods are garbage collected when ReplicaSet is deleted
        metadata.owner_references = Some(vec![rusternetes_common::types::OwnerReference {
            api_version: "apps/v1".to_string(),
            kind: "ReplicaSet".to_string(),
            name: replicaset.metadata.name.clone(),
            uid: replicaset.metadata.uid.clone(),
            controller: Some(true),
            block_owner_deletion: Some(true),
        }]);

        let pod = Pod {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "Pod".to_string(),
                api_version: "v1".to_string(),
            },
            metadata,
            spec: Some(replicaset.spec.template.spec.clone()),
            status: Some(PodStatus {
                phase: Some(Phase::Pending),
                message: None,
                reason: None,
                host_ip: None,
                pod_ip: None,
                container_statuses: None,
                init_container_statuses: None,
                ephemeral_container_statuses: None,
            }),
        };

        let key = build_key("pods", Some(namespace), &pod_name);
        self.storage.create(&key, &pod).await?;

        info!(
            "Created pod {}/{} for replicaset {}",
            namespace, pod_name, replicaset.metadata.name
        );

        Ok(())
    }

    async fn delete_pod(&self, name: &str, namespace: &str) -> rusternetes_common::Result<()> {
        let key = build_key("pods", Some(namespace), name);
        self.storage.delete(&key).await?;

        info!("Deleted pod {}/{}", namespace, name);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::types::LabelSelector;
    use std::collections::HashMap;

    #[test]
    fn test_matches_selector() {
        // Basic selector matching test
        let mut labels = HashMap::new();
        labels.insert("app".to_string(), "test".to_string());

        let selector = LabelSelector {
            match_labels: Some(labels.clone()),
            match_expressions: None,
        };

        let pod_with_labels = Pod {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "Pod".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: ObjectMeta::new("test-pod")
                .with_namespace("default")
                .with_labels(labels),
            spec: None,
            status: None,
        };

        // TODO: Add integration tests with actual storage
        // For now, just verify basic struct creation
        assert_eq!(pod_with_labels.metadata.name, "test-pod");
    }
}
