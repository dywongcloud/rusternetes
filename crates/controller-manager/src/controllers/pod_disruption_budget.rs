use rusternetes_common::resources::PodDisruptionBudget;
use rusternetes_common::types::LabelSelector;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

#[allow(dead_code)]
pub struct PodDisruptionBudgetController {
    pdbs: Arc<RwLock<HashMap<String, PodDisruptionBudget>>>,
}

impl PodDisruptionBudgetController {
    pub fn new() -> Self {
        Self {
            pdbs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run(self: Arc<Self>) {
        info!("Starting PodDisruptionBudget controller");

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;

            if let Err(e) = self.reconcile().await {
                error!("Error reconciling PodDisruptionBudgets: {}", e);
            }
        }
    }

    async fn reconcile(&self) -> Result<(), Box<dyn std::error::Error>> {
        let pdbs = self.pdbs.read().await;

        for (key, pdb) in pdbs.iter() {
            if let Err(e) = self.reconcile_pdb(pdb).await {
                warn!("Failed to reconcile PDB {}: {}", key, e);
            }
        }

        Ok(())
    }

    async fn reconcile_pdb(&self, pdb: &PodDisruptionBudget) -> Result<(), Box<dyn std::error::Error>> {
        info!("Reconciling PodDisruptionBudget: {}", pdb.metadata.name);

        // In a real implementation, this would:
        // 1. Find all pods matching the selector
        // 2. Count healthy vs unhealthy pods
        // 3. Calculate disruptions_allowed based on min_available or max_unavailable
        // 4. Update the PDB status

        // For now, this is a placeholder that logs the reconciliation
        info!("PDB {} spec: min_available={:?}, max_unavailable={:?}",
            pdb.metadata.name,
            pdb.spec.min_available.as_ref().map(|v| format!("{:?}", v)),
            pdb.spec.max_unavailable.as_ref().map(|v| format!("{:?}", v))
        );

        Ok(())
    }

    pub async fn create_pdb(&self, pdb: PodDisruptionBudget) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}/{}",
            pdb.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
            pdb.metadata.name
        );

        info!("Creating PodDisruptionBudget: {}", key);

        let mut pdbs = self.pdbs.write().await;
        pdbs.insert(key, pdb);

        Ok(())
    }

    pub async fn delete_pdb(&self, namespace: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}/{}", namespace, name);

        info!("Deleting PodDisruptionBudget: {}", key);

        let mut pdbs = self.pdbs.write().await;
        pdbs.remove(&key);

        Ok(())
    }

    pub async fn get_pdb(&self, namespace: &str, name: &str) -> Option<PodDisruptionBudget> {
        let key = format!("{}/{}", namespace, name);
        let pdbs = self.pdbs.read().await;
        pdbs.get(&key).cloned()
    }

    pub async fn list_pdbs(&self, namespace: Option<&str>) -> Vec<PodDisruptionBudget> {
        let pdbs = self.pdbs.read().await;

        pdbs.values()
            .filter(|pdb| {
                namespace.is_none() ||
                pdb.metadata.namespace.as_ref().map(|ns| ns.as_str()) == namespace
            })
            .cloned()
            .collect()
    }

    /// Check if a pod eviction is allowed based on PDBs
    pub async fn is_eviction_allowed(&self, namespace: &str, pod_labels: &HashMap<String, String>) -> bool {
        let pdbs = self.list_pdbs(Some(namespace)).await;

        for pdb in pdbs {
            if self.pod_matches_selector(pod_labels, &pdb.spec.selector) {
                // Check if eviction would violate the PDB
                if let Some(status) = &pdb.status {
                    if status.disruptions_allowed <= 0 {
                        info!("Eviction blocked by PDB: {}", pdb.metadata.name);
                        return false;
                    }
                }
            }
        }

        true
    }

    fn pod_matches_selector(&self, pod_labels: &HashMap<String, String>, selector: &LabelSelector) -> bool {
        if let Some(match_labels) = &selector.match_labels {
            for (key, value) in match_labels {
                if pod_labels.get(key) != Some(value) {
                    return false;
                }
            }
        }

        // TODO: Implement match_expressions logic

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::resources::{IntOrString, PodDisruptionBudgetSpec};

    #[tokio::test]
    async fn test_create_and_get_pdb() {
        let controller = Arc::new(PodDisruptionBudgetController::new());

        let spec = PodDisruptionBudgetSpec {
            min_available: Some(IntOrString::Int(2)),
            max_unavailable: None,
            selector: LabelSelector {
                match_labels: Some(HashMap::from([
                    ("app".to_string(), "web".to_string()),
                ])),
                match_expressions: None,
            },
            unhealthy_pod_eviction_policy: None,
        };

        let pdb = PodDisruptionBudget::new("test-pdb", "default", spec);

        controller.create_pdb(pdb.clone()).await.unwrap();

        let retrieved = controller.get_pdb("default", "test-pdb").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().metadata.name, "test-pdb");
    }

    #[tokio::test]
    async fn test_delete_pdb() {
        let controller = Arc::new(PodDisruptionBudgetController::new());

        let spec = PodDisruptionBudgetSpec {
            min_available: Some(IntOrString::Int(1)),
            max_unavailable: None,
            selector: LabelSelector {
                match_labels: Some(HashMap::from([
                    ("app".to_string(), "api".to_string()),
                ])),
                match_expressions: None,
            },
            unhealthy_pod_eviction_policy: None,
        };

        let pdb = PodDisruptionBudget::new("test-pdb", "default", spec);

        controller.create_pdb(pdb).await.unwrap();
        controller.delete_pdb("default", "test-pdb").await.unwrap();

        let retrieved = controller.get_pdb("default", "test-pdb").await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_list_pdbs() {
        let controller = Arc::new(PodDisruptionBudgetController::new());

        for i in 1..=3 {
            let spec = PodDisruptionBudgetSpec {
                min_available: Some(IntOrString::Int(1)),
                max_unavailable: None,
                selector: LabelSelector {
                    match_labels: Some(HashMap::from([
                        ("app".to_string(), format!("app{}", i)),
                    ])),
                    match_expressions: None,
                },
                unhealthy_pod_eviction_policy: None,
            };

            let pdb = PodDisruptionBudget::new(format!("pdb-{}", i), "default", spec);
            controller.create_pdb(pdb).await.unwrap();
        }

        let pdbs = controller.list_pdbs(Some("default")).await;
        assert_eq!(pdbs.len(), 3);
    }
}
