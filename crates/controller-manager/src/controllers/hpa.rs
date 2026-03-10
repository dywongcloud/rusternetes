use rusternetes_common::resources::{HorizontalPodAutoscaler, HorizontalPodAutoscalerStatus, MetricSpec};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

pub struct HorizontalPodAutoscalerController {
    hpas: Arc<RwLock<HashMap<String, HorizontalPodAutoscaler>>>,
}

impl HorizontalPodAutoscalerController {
    pub fn new() -> Self {
        Self {
            hpas: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run(self: Arc<Self>) {
        info!("Starting HorizontalPodAutoscaler controller");

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(15));
        loop {
            interval.tick().await;

            if let Err(e) = self.reconcile().await {
                error!("Error reconciling HPAs: {}", e);
            }
        }
    }

    async fn reconcile(&self) -> Result<(), Box<dyn std::error::Error>> {
        let hpas = self.hpas.read().await;

        for (key, hpa) in hpas.iter() {
            if let Err(e) = self.reconcile_hpa(hpa).await {
                warn!("Failed to reconcile HPA {}: {}", key, e);
            }
        }

        Ok(())
    }

    async fn reconcile_hpa(&self, hpa: &HorizontalPodAutoscaler) -> Result<(), Box<dyn std::error::Error>> {
        info!("Reconciling HPA: {}", hpa.metadata.name);

        // In a real implementation, this would:
        // 1. Fetch metrics from metrics server
        // 2. Calculate desired replica count based on metrics and target
        // 3. Scale the target deployment/replicaset/statefulset
        // 4. Update HPA status

        let target_ref = &hpa.spec.scale_target_ref;
        info!("HPA {} targets {}/{} - min: {:?}, max: {}",
            hpa.metadata.name,
            target_ref.kind,
            target_ref.name,
            hpa.spec.min_replicas,
            hpa.spec.max_replicas
        );

        if let Some(metrics) = &hpa.spec.metrics {
            for metric in metrics {
                self.log_metric(metric);
            }
        }

        Ok(())
    }

    fn log_metric(&self, metric: &MetricSpec) {
        match metric.metric_type.as_str() {
            "Resource" => {
                if let Some(resource) = &metric.resource {
                    info!("  Resource metric: {} - target: {:?}",
                        resource.name,
                        resource.target.average_utilization
                    );
                }
            }
            "Pods" => {
                if let Some(pods) = &metric.pods {
                    info!("  Pods metric: {} - target: {:?}",
                        pods.metric.name,
                        pods.target.average_value
                    );
                }
            }
            "Object" => {
                if let Some(object) = &metric.object {
                    info!("  Object metric: {} - target: {:?}",
                        object.metric.name,
                        object.target.value
                    );
                }
            }
            "External" => {
                if let Some(external) = &metric.external {
                    info!("  External metric: {} - target: {:?}",
                        external.metric.name,
                        external.target.average_value
                    );
                }
            }
            _ => {
                warn!("  Unknown metric type: {}", metric.metric_type);
            }
        }
    }

    pub async fn create_hpa(&self, hpa: HorizontalPodAutoscaler) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}/{}",
            hpa.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
            hpa.metadata.name
        );

        info!("Creating HPA: {}", key);

        let mut hpas = self.hpas.write().await;
        hpas.insert(key, hpa);

        Ok(())
    }

    pub async fn delete_hpa(&self, namespace: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}/{}", namespace, name);

        info!("Deleting HPA: {}", key);

        let mut hpas = self.hpas.write().await;
        hpas.remove(&key);

        Ok(())
    }

    pub async fn get_hpa(&self, namespace: &str, name: &str) -> Option<HorizontalPodAutoscaler> {
        let key = format!("{}/{}", namespace, name);
        let hpas = self.hpas.read().await;
        hpas.get(&key).cloned()
    }

    pub async fn list_hpas(&self, namespace: Option<&str>) -> Vec<HorizontalPodAutoscaler> {
        let hpas = self.hpas.read().await;

        hpas.values()
            .filter(|hpa| {
                namespace.is_none() ||
                hpa.metadata.namespace.as_ref().map(|ns| ns.as_str()) == namespace
            })
            .cloned()
            .collect()
    }

    pub async fn update_hpa_status(
        &self,
        namespace: &str,
        name: &str,
        status: HorizontalPodAutoscalerStatus,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}/{}", namespace, name);

        let mut hpas = self.hpas.write().await;
        if let Some(hpa) = hpas.get_mut(&key) {
            hpa.status = Some(status);
            info!("Updated HPA status: {}", key);
            Ok(())
        } else {
            Err(format!("HPA not found: {}", key).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::resources::{
        HorizontalPodAutoscalerSpec, CrossVersionObjectReference,
        MetricSpec, ResourceMetricSource, MetricTarget,
    };
    use common::types::ObjectMeta;

    #[tokio::test]
    async fn test_create_and_get_hpa() {
        let controller = Arc::new(HorizontalPodAutoscalerController::new());

        let spec = HorizontalPodAutoscalerSpec {
            scale_target_ref: CrossVersionObjectReference {
                kind: "Deployment".to_string(),
                name: "web-app".to_string(),
                api_version: Some("apps/v1".to_string()),
            },
            min_replicas: Some(2),
            max_replicas: 10,
            metrics: Some(vec![MetricSpec {
                metric_type: "Resource".to_string(),
                resource: Some(ResourceMetricSource {
                    name: "cpu".to_string(),
                    target: MetricTarget {
                        target_type: "Utilization".to_string(),
                        value: None,
                        average_value: None,
                        average_utilization: Some(80),
                    },
                }),
                pods: None,
                object: None,
                external: None,
                container_resource: None,
            }]),
            behavior: None,
        };

        let hpa = HorizontalPodAutoscaler::new("test-hpa", "default", spec);

        controller.create_hpa(hpa.clone()).await.unwrap();

        let retrieved = controller.get_hpa("default", "test-hpa").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().metadata.name, "test-hpa");
    }

    #[tokio::test]
    async fn test_delete_hpa() {
        let controller = Arc::new(HorizontalPodAutoscalerController::new());

        let spec = HorizontalPodAutoscalerSpec {
            scale_target_ref: CrossVersionObjectReference {
                kind: "Deployment".to_string(),
                name: "api".to_string(),
                api_version: Some("apps/v1".to_string()),
            },
            min_replicas: Some(1),
            max_replicas: 5,
            metrics: None,
            behavior: None,
        };

        let hpa = HorizontalPodAutoscaler::new("test-hpa", "default", spec);

        controller.create_hpa(hpa).await.unwrap();
        controller.delete_hpa("default", "test-hpa").await.unwrap();

        let retrieved = controller.get_hpa("default", "test-hpa").await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_list_hpas() {
        let controller = Arc::new(HorizontalPodAutoscalerController::new());

        for i in 1..=3 {
            let spec = HorizontalPodAutoscalerSpec {
                scale_target_ref: CrossVersionObjectReference {
                    kind: "Deployment".to_string(),
                    name: format!("app-{}", i),
                    api_version: Some("apps/v1".to_string()),
                },
                min_replicas: Some(1),
                max_replicas: 5,
                metrics: None,
                behavior: None,
            };

            let hpa = HorizontalPodAutoscaler::new(format!("hpa-{}", i), "default", spec);
            controller.create_hpa(hpa).await.unwrap();
        }

        let hpas = controller.list_hpas(Some("default")).await;
        assert_eq!(hpas.len(), 3);
    }
}
