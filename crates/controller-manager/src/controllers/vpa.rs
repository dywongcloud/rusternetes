use rusternetes_common::resources::{VerticalPodAutoscaler, VerticalPodAutoscalerStatus};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

#[allow(dead_code)]
pub struct VerticalPodAutoscalerController {
    vpas: Arc<RwLock<HashMap<String, VerticalPodAutoscaler>>>,
}

impl VerticalPodAutoscalerController {
    pub fn new() -> Self {
        Self {
            vpas: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run(self: Arc<Self>) {
        info!("Starting VerticalPodAutoscaler controller");

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;

            if let Err(e) = self.reconcile().await {
                error!("Error reconciling VPAs: {}", e);
            }
        }
    }

    async fn reconcile(&self) -> Result<(), Box<dyn std::error::Error>> {
        let vpas = self.vpas.read().await;

        for (key, vpa) in vpas.iter() {
            if let Err(e) = self.reconcile_vpa(vpa).await {
                warn!("Failed to reconcile VPA {}: {}", key, e);
            }
        }

        Ok(())
    }

    async fn reconcile_vpa(&self, vpa: &VerticalPodAutoscaler) -> Result<(), Box<dyn std::error::Error>> {
        info!("Reconciling VPA: {}", vpa.metadata.name);

        // In a real implementation, this would:
        // 1. Collect historical resource usage data from pods
        // 2. Run recommendation algorithm (using ML or statistical models)
        // 3. Generate resource recommendations for containers
        // 4. Apply recommendations based on update policy (Off, Initial, Recreate, Auto)
        // 5. Update VPA status with recommendations

        let target_ref = &vpa.spec.target_ref;
        info!("VPA {} targets {}/{}",
            vpa.metadata.name,
            target_ref.kind,
            target_ref.name
        );

        if let Some(update_policy) = &vpa.spec.update_policy {
            if let Some(mode) = &update_policy.update_mode {
                info!("  Update mode: {}", mode);
            }
        }

        if let Some(resource_policy) = &vpa.spec.resource_policy {
            if let Some(container_policies) = &resource_policy.container_policies {
                info!("  Container policies: {}", container_policies.len());
                for policy in container_policies {
                    if let Some(container_name) = &policy.container_name {
                        info!("    Container: {}", container_name);
                    }
                    if let Some(min_allowed) = &policy.min_allowed {
                        info!("      Min allowed: {:?}", min_allowed);
                    }
                    if let Some(max_allowed) = &policy.max_allowed {
                        info!("      Max allowed: {:?}", max_allowed);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn create_vpa(&self, vpa: VerticalPodAutoscaler) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}/{}",
            vpa.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
            vpa.metadata.name
        );

        info!("Creating VPA: {}", key);

        let mut vpas = self.vpas.write().await;
        vpas.insert(key, vpa);

        Ok(())
    }

    pub async fn delete_vpa(&self, namespace: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}/{}", namespace, name);

        info!("Deleting VPA: {}", key);

        let mut vpas = self.vpas.write().await;
        vpas.remove(&key);

        Ok(())
    }

    pub async fn get_vpa(&self, namespace: &str, name: &str) -> Option<VerticalPodAutoscaler> {
        let key = format!("{}/{}", namespace, name);
        let vpas = self.vpas.read().await;
        vpas.get(&key).cloned()
    }

    pub async fn list_vpas(&self, namespace: Option<&str>) -> Vec<VerticalPodAutoscaler> {
        let vpas = self.vpas.read().await;

        vpas.values()
            .filter(|vpa| {
                namespace.is_none() ||
                vpa.metadata.namespace.as_ref().map(|ns| ns.as_str()) == namespace
            })
            .cloned()
            .collect()
    }

    pub async fn update_vpa_status(
        &self,
        namespace: &str,
        name: &str,
        status: VerticalPodAutoscalerStatus,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}/{}", namespace, name);

        let mut vpas = self.vpas.write().await;
        if let Some(vpa) = vpas.get_mut(&key) {
            vpa.status = Some(status);
            info!("Updated VPA status: {}", key);
            Ok(())
        } else {
            Err(format!("VPA not found: {}", key).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::resources::{
        VerticalPodAutoscalerSpec, CrossVersionObjectReference,
        PodUpdatePolicy, PodResourcePolicy, ContainerResourcePolicy,
    };

    #[tokio::test]
    async fn test_create_and_get_vpa() {
        let controller = Arc::new(VerticalPodAutoscalerController::new());

        let spec = VerticalPodAutoscalerSpec {
            target_ref: CrossVersionObjectReference {
                kind: "Deployment".to_string(),
                name: "web-app".to_string(),
                api_version: Some("apps/v1".to_string()),
            },
            update_policy: Some(PodUpdatePolicy {
                update_mode: Some("Auto".to_string()),
            }),
            resource_policy: None,
            recommenders: None,
        };

        let vpa = VerticalPodAutoscaler::new("test-vpa", "default", spec);

        controller.create_vpa(vpa.clone()).await.unwrap();

        let retrieved = controller.get_vpa("default", "test-vpa").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().metadata.name, "test-vpa");
    }

    #[tokio::test]
    async fn test_delete_vpa() {
        let controller = Arc::new(VerticalPodAutoscalerController::new());

        let spec = VerticalPodAutoscalerSpec {
            target_ref: CrossVersionObjectReference {
                kind: "StatefulSet".to_string(),
                name: "db".to_string(),
                api_version: Some("apps/v1".to_string()),
            },
            update_policy: None,
            resource_policy: None,
            recommenders: None,
        };

        let vpa = VerticalPodAutoscaler::new("test-vpa", "default", spec);

        controller.create_vpa(vpa).await.unwrap();
        controller.delete_vpa("default", "test-vpa").await.unwrap();

        let retrieved = controller.get_vpa("default", "test-vpa").await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_vpa_with_resource_policy() {
        let controller = Arc::new(VerticalPodAutoscalerController::new());

        let mut min_allowed = HashMap::new();
        min_allowed.insert("cpu".to_string(), "100m".to_string());
        min_allowed.insert("memory".to_string(), "128Mi".to_string());

        let mut max_allowed = HashMap::new();
        max_allowed.insert("cpu".to_string(), "2".to_string());
        max_allowed.insert("memory".to_string(), "2Gi".to_string());

        let spec = VerticalPodAutoscalerSpec {
            target_ref: CrossVersionObjectReference {
                kind: "Deployment".to_string(),
                name: "api".to_string(),
                api_version: Some("apps/v1".to_string()),
            },
            update_policy: Some(PodUpdatePolicy {
                update_mode: Some("Auto".to_string()),
            }),
            resource_policy: Some(PodResourcePolicy {
                container_policies: Some(vec![ContainerResourcePolicy {
                    container_name: Some("api-container".to_string()),
                    mode: Some("Auto".to_string()),
                    min_allowed: Some(min_allowed),
                    max_allowed: Some(max_allowed),
                    controlled_resources: Some(vec!["cpu".to_string(), "memory".to_string()]),
                }]),
            }),
            recommenders: None,
        };

        let vpa = VerticalPodAutoscaler::new("api-vpa", "default", spec);

        controller.create_vpa(vpa).await.unwrap();

        let retrieved = controller.get_vpa("default", "api-vpa").await;
        assert!(retrieved.is_some());

        let vpa = retrieved.unwrap();
        assert!(vpa.spec.resource_policy.is_some());

        let resource_policy = vpa.spec.resource_policy.unwrap();
        assert!(resource_policy.container_policies.is_some());

        let container_policies = resource_policy.container_policies.unwrap();
        assert_eq!(container_policies.len(), 1);
        assert_eq!(container_policies[0].container_name.as_ref().unwrap(), "api-container");
    }
}
