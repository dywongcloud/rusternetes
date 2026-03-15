use rusternetes_common::{
    resources::{ReplicationController, Pod, PodStatus},
    types::{ObjectMeta, Phase},
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::{sync::Arc, time::Duration};
use tracing::{debug, error, info};

/// ReplicationControllerController reconciles ReplicationController resources
pub struct ReplicationControllerController<S: Storage> {
    storage: Arc<S>,
    interval: Duration,
}

impl<S: Storage> ReplicationControllerController<S> {
    pub fn new(storage: Arc<S>, interval_secs: u64) -> Self {
        Self {
            storage,
            interval: Duration::from_secs(interval_secs),
        }
    }

    pub async fn run(&self) -> rusternetes_common::Result<()> {
        info!(
            "ReplicationController controller started, syncing every {:?}",
            self.interval
        );

        let mut interval = tokio::time::interval(self.interval);

        loop {
            interval.tick().await;
            if let Err(e) = self.reconcile_all().await {
                error!("Error reconciling replicationcontrollers: {}", e);
            }
        }
    }

    pub async fn reconcile_all(&self) -> rusternetes_common::Result<()> {
        debug!("Reconciling all replicationcontrollers");

        // Get all replicationcontrollers
        let prefix = build_prefix("replicationcontrollers", None);
        let rcs: Vec<ReplicationController> = self.storage.list(&prefix).await?;

        for rc in rcs {
            if let Err(e) = self.reconcile_rc(&rc).await {
                error!(
                    "Error reconciling replicationcontroller {}: {}",
                    rc.metadata.name, e
                );
            }
        }

        Ok(())
    }

    async fn reconcile_rc(
        &self,
        rc: &ReplicationController,
    ) -> rusternetes_common::Result<()> {
        let namespace = rc
            .metadata
            .namespace
            .as_deref()
            .unwrap_or("default");

        debug!(
            "Reconciling replicationcontroller: {}/{}",
            namespace, rc.metadata.name
        );

        // Get all pods for this replicationcontroller
        let pods_prefix = build_prefix("pods", Some(namespace));
        info!("Querying pods with prefix: {}", pods_prefix);
        let all_pods: Vec<Pod> = self.storage.list(&pods_prefix).await?;
        info!("Found {} total pods in namespace {}", all_pods.len(), namespace);

        // Filter pods that match this replicationcontroller's selector
        let rc_pods: Vec<Pod> = all_pods
            .into_iter()
            .filter(|p| {
                let matches = self.matches_selector(p, rc);
                info!(
                    "Pod {} matches selector: {} (labels: {:?})",
                    p.metadata.name, matches, p.metadata.labels
                );
                matches
            })
            .collect();

        let current_replicas = rc_pods.len() as i32;
        let desired_replicas = rc.spec.replicas.unwrap_or(1);

        info!(
            "ReplicationController {}/{}: current={}, desired={} (matched {} pods)",
            namespace, rc.metadata.name, current_replicas, desired_replicas, rc_pods.len()
        );

        if current_replicas < desired_replicas {
            // Need to create more pods
            let to_create = desired_replicas - current_replicas;
            for i in 0..to_create {
                self.create_pod(rc, i).await?;
            }
        } else if current_replicas > desired_replicas {
            // Need to delete excess pods
            let to_delete = current_replicas - desired_replicas;
            for pod in rc_pods.iter().take(to_delete as usize) {
                self.delete_pod(&pod.metadata.name, namespace).await?;
            }
        }

        // Re-fetch and recount pods after create/delete operations to get accurate status
        let pods_prefix = build_prefix("pods", Some(namespace));
        let all_pods_after: Vec<Pod> = self.storage.list(&pods_prefix).await?;

        let rc_pods_after: Vec<Pod> = all_pods_after
            .into_iter()
            .filter(|p| self.matches_selector(p, rc))
            .collect();

        let final_current_replicas = rc_pods_after.len() as i32;
        let final_ready_replicas = rc_pods_after.len() as i32; // All matched pods

        // Update status with accurate counts
        self.update_status(rc, final_current_replicas, final_ready_replicas).await?;

        Ok(())
    }

    fn matches_selector(&self, pod: &Pod, rc: &ReplicationController) -> bool {
        // ReplicationController uses simple label matching (not label selectors like Deployment)
        if let Some(selector) = &rc.spec.selector {
            if let Some(pod_labels) = &pod.metadata.labels {
                for (key, value) in selector {
                    if pod_labels.get(key) != Some(value) {
                        return false;
                    }
                }
                return true;
            }
        }
        false
    }

    async fn create_pod(
        &self,
        rc: &ReplicationController,
        _index: i32,
    ) -> rusternetes_common::Result<()> {
        let namespace = rc
            .metadata
            .namespace
            .as_deref()
            .unwrap_or("default");

        let pod_name = format!("{}-{}", rc.metadata.name, uuid::Uuid::new_v4());

        let mut metadata = ObjectMeta::new(&pod_name);
        metadata.namespace = Some(namespace.to_string());
        metadata.labels = rc.spec.template.metadata.as_ref().and_then(|m| m.labels.clone());

        let pod = Pod {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "Pod".to_string(),
                api_version: "v1".to_string(),
            },
            metadata,
            spec: Some(rc.spec.template.spec.clone()),
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
            "Created pod {}/{} for replicationcontroller {}",
            namespace, pod_name, rc.metadata.name
        );

        Ok(())
    }

    async fn delete_pod(&self, name: &str, namespace: &str) -> rusternetes_common::Result<()> {
        let key = build_key("pods", Some(namespace), name);
        self.storage.delete(&key).await?;

        info!("Deleted pod {}/{}", namespace, name);

        Ok(())
    }

    async fn update_status(
        &self,
        rc: &ReplicationController,
        current_replicas: i32,
        ready_replicas: i32,
    ) -> rusternetes_common::Result<()> {
        let namespace = rc.metadata.namespace.as_deref().unwrap_or("default");
        let key = build_key("replicationcontrollers", Some(namespace), &rc.metadata.name);

        let mut updated_rc = rc.clone();
        updated_rc.status = Some(rusternetes_common::resources::ReplicationControllerStatus {
            replicas: current_replicas,
            fully_labeled_replicas: Some(current_replicas),
            ready_replicas: Some(ready_replicas),
            available_replicas: Some(ready_replicas),
            observed_generation: None, // TODO: Add generation tracking to ObjectMeta
            conditions: None,
        });

        self.storage.update(&key, &updated_rc).await?;

        debug!(
            "Updated status for replicationcontroller {}/{}",
            namespace, rc.metadata.name
        );

        Ok(())
    }
}
