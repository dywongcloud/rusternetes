use rusternetes_common::{
    resources::{Deployment, Pod, PodStatus},
    types::{ObjectMeta, Phase},
};
use rusternetes_storage::{build_key, build_prefix, etcd::EtcdStorage, Storage};
use std::{sync::Arc, time::Duration};
use tracing::{debug, error, info};

/// DeploymentController reconciles Deployment resources
pub struct DeploymentController {
    storage: Arc<EtcdStorage>,
    interval: Duration,
}

impl DeploymentController {
    pub fn new(storage: Arc<EtcdStorage>, interval_secs: u64) -> Self {
        Self {
            storage,
            interval: Duration::from_secs(interval_secs),
        }
    }

    pub async fn run(&self) -> rusternetes_common::Result<()> {
        info!(
            "Deployment controller started, syncing every {:?}",
            self.interval
        );

        let mut interval = tokio::time::interval(self.interval);

        loop {
            interval.tick().await;
            if let Err(e) = self.reconcile_all().await {
                error!("Error reconciling deployments: {}", e);
            }
        }
    }

    async fn reconcile_all(&self) -> rusternetes_common::Result<()> {
        debug!("Reconciling all deployments");

        // Get all deployments
        let prefix = build_prefix("deployments", None);
        let deployments: Vec<Deployment> = self.storage.list(&prefix).await?;

        for deployment in deployments {
            if let Err(e) = self.reconcile_deployment(&deployment).await {
                error!(
                    "Error reconciling deployment {}: {}",
                    deployment.metadata.name, e
                );
            }
        }

        Ok(())
    }

    async fn reconcile_deployment(
        &self,
        deployment: &Deployment,
    ) -> rusternetes_common::Result<()> {
        let namespace = deployment
            .metadata
            .namespace
            .as_deref()
            .unwrap_or("default");

        debug!(
            "Reconciling deployment: {}/{}",
            namespace, deployment.metadata.name
        );

        // Get all pods for this deployment
        let pods_prefix = build_prefix("pods", Some(namespace));
        let all_pods: Vec<Pod> = self.storage.list(&pods_prefix).await?;

        // Filter pods that match this deployment's selector
        let deployment_pods: Vec<Pod> = all_pods
            .into_iter()
            .filter(|p| self.matches_selector(p, deployment))
            .collect();

        let current_replicas = deployment_pods.len() as i32;
        let desired_replicas = deployment.spec.replicas;

        info!(
            "Deployment {}/{}: current={}, desired={}",
            namespace, deployment.metadata.name, current_replicas, desired_replicas
        );

        if current_replicas < desired_replicas {
            // Need to create more pods
            let to_create = desired_replicas - current_replicas;
            for i in 0..to_create {
                self.create_pod(deployment, i).await?;
            }
        } else if current_replicas > desired_replicas {
            // Need to delete excess pods
            let to_delete = current_replicas - desired_replicas;
            for pod in deployment_pods.iter().take(to_delete as usize) {
                self.delete_pod(&pod.metadata.name, namespace).await?;
            }
        }

        Ok(())
    }

    fn matches_selector(&self, pod: &Pod, deployment: &Deployment) -> bool {
        if let Some(match_labels) = &deployment.spec.selector.match_labels {
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

    async fn create_pod(
        &self,
        deployment: &Deployment,
        _index: i32,
    ) -> rusternetes_common::Result<()> {
        let namespace = deployment
            .metadata
            .namespace
            .as_deref()
            .unwrap_or("default");

        let pod_name = format!("{}-{}", deployment.metadata.name, uuid::Uuid::new_v4());

        let mut metadata = ObjectMeta::new(&pod_name);
        metadata.namespace = Some(namespace.to_string());
        metadata.labels = deployment.spec.template.metadata.as_ref().and_then(|m| m.labels.clone());

        let pod = Pod {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "Pod".to_string(),
                api_version: "v1".to_string(),
            },
            metadata,
            spec: deployment.spec.template.spec.clone(),
            status: Some(PodStatus {
                phase: Phase::Pending,
                message: None,
                reason: None,
                host_ip: None,
                pod_ip: None,
                container_statuses: None,
            }),
        };

        let key = build_key("pods", Some(namespace), &pod_name);
        self.storage.create(&key, &pod).await?;

        info!(
            "Created pod {}/{} for deployment {}",
            namespace, pod_name, deployment.metadata.name
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
