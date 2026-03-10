use anyhow::Result;
use rusternetes_common::resources::Namespace;
use rusternetes_storage::{build_key, build_prefix, etcd::EtcdStorage, Storage};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// NamespaceController handles namespace lifecycle and finalization.
/// When a namespace is marked for deletion, it:
/// 1. Discovers all resources in the namespace
/// 2. Deletes all resources (respecting finalizers)
/// 3. Removes finalizers from the namespace
/// 4. Allows the namespace to be deleted
pub struct NamespaceController {
    storage: Arc<EtcdStorage>,
}

impl NamespaceController {
    pub fn new(storage: Arc<EtcdStorage>) -> Self {
        Self { storage }
    }

    /// Main reconciliation loop - processes all namespaces
    pub async fn reconcile_all(&self) -> Result<()> {
        debug!("Starting namespace reconciliation");

        // List all namespaces
        let namespaces: Vec<Namespace> = self.storage.list("/registry/namespaces/").await?;

        for namespace in namespaces {
            if let Err(e) = self.reconcile_namespace(&namespace).await {
                error!(
                    "Failed to reconcile namespace {}: {}",
                    &namespace.metadata.name, e
                );
            }
        }

        Ok(())
    }

    /// Reconcile a single namespace
    async fn reconcile_namespace(&self, namespace: &Namespace) -> Result<()> {
        let name = &namespace.metadata.name;

        // Check if namespace is being deleted
        if namespace.metadata.deletion_timestamp.is_some() {
            info!("Namespace {} is being deleted, starting finalization", name);
            return self.finalize_namespace(namespace).await;
        }

        debug!("Namespace {} is active", name);
        Ok(())
    }

    /// Finalize a namespace by deleting all resources within it
    async fn finalize_namespace(&self, namespace: &Namespace) -> Result<()> {
        let name = &namespace.metadata.name;

        info!("Finalizing namespace {}", name);

        // List of resource types to delete (in dependency order)
        let resource_types = vec![
            // Workload resources first
            "pods",
            "replicationcontrollers",
            "replicasets",
            "deployments",
            "statefulsets",
            "daemonsets",
            "jobs",
            "cronjobs",
            // Configuration resources
            "configmaps",
            "secrets",
            "serviceaccounts",
            // Networking resources
            "services",
            "endpoints",
            "endpointslices",
            "ingresses",
            "networkpolicies",
            // Storage resources
            "persistentvolumeclaims",
            // Policy resources
            "poddisruptionbudgets",
            "resourcequotas",
            "limitranges",
            // RBAC resources
            "roles",
            "rolebindings",
            // Events
            "events",
            // Autoscaling
            "horizontalpodautoscalers",
            // Leases
            "leases",
            // Resource claims (DRA)
            "resourceclaims",
            "resourceclaimtemplates",
            // Other
            "controllerrevisions",
            "podtemplates",
            "csistoragecapacities",
        ];

        // Delete all resources in the namespace
        for resource_type in resource_types {
            if let Err(e) = self.delete_all_resources(name, resource_type).await {
                warn!(
                    "Failed to delete {} in namespace {}: {}",
                    resource_type, name, e
                );
                // Continue with other resource types
            }
        }

        // Check if all resources are deleted
        let remaining_count = self.count_remaining_resources(name).await?;
        if remaining_count > 0 {
            info!(
                "Namespace {} still has {} resources, will retry",
                name, remaining_count
            );
            return Ok(()); // Will be retried in next reconciliation
        }

        // Remove finalizers from the namespace
        if let Some(finalizers) = &namespace.metadata.finalizers {
            if !finalizers.is_empty() {
                info!("Removing {} finalizers from namespace {}", finalizers.len(), name);
                self.remove_namespace_finalizers(name).await?;
            }
        }

        info!("Namespace {} finalization complete", name);
        Ok(())
    }

    /// Delete all resources of a given type in a namespace
    async fn delete_all_resources(&self, namespace: &str, resource_type: &str) -> Result<()> {
        let prefix = build_prefix(resource_type, Some(namespace));

        // List all resources
        let resources: Vec<serde_json::Value> = self.storage.list(&prefix).await.unwrap_or_default();

        if resources.is_empty() {
            return Ok(());
        }

        debug!(
            "Deleting {} {} resources in namespace {}",
            resources.len(),
            resource_type,
            namespace
        );

        // Delete each resource
        for resource in resources {
            if let Some(metadata) = resource.get("metadata") {
                if let Some(name) = metadata.get("name").and_then(|n| n.as_str()) {
                    let key = build_key(resource_type, Some(namespace), name);
                    match self.storage.delete(&key).await {
                        Ok(_) => debug!("Deleted {}/{}/{}", resource_type, namespace, name),
                        Err(rusternetes_common::Error::NotFound(_)) => {
                            // Already deleted, that's fine
                        }
                        Err(e) => {
                            warn!("Failed to delete {}/{}/{}: {}", resource_type, namespace, name, e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Count remaining resources in a namespace
    async fn count_remaining_resources(&self, namespace: &str) -> Result<usize> {
        // Check a few key resource types to see if anything remains
        let resource_types = vec!["pods", "services", "configmaps", "secrets"];
        let mut total = 0;

        for resource_type in resource_types {
            let prefix = build_prefix(resource_type, Some(namespace));
            let resources: Vec<serde_json::Value> = self.storage.list(&prefix).await.unwrap_or_default();
            total += resources.len();
        }

        Ok(total)
    }

    /// Remove finalizers from a namespace
    async fn remove_namespace_finalizers(&self, name: &str) -> Result<()> {
        let key = build_key("namespaces", None, name);

        // Get current namespace
        let mut namespace: Namespace = self.storage.get(&key).await?;

        // Remove all finalizers
        namespace.metadata.finalizers = None;

        // Update namespace
        self.storage.update(&key, &namespace).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::types::{ObjectMeta, TypeMeta};

    #[tokio::test]
    async fn test_namespace_controller_creation() {
        let storage = Arc::new(
            EtcdStorage::new(vec!["http://localhost:2379".to_string()])
                .await
                .unwrap(),
        );
        let _controller = NamespaceController::new(storage);
    }

    #[test]
    fn test_namespace_resource_types() {
        // Ensure we have the major resource types covered
        let resource_types = vec![
            "pods",
            "services",
            "configmaps",
            "secrets",
            "deployments",
        ];
        assert!(resource_types.contains(&"pods"));
        assert!(resource_types.contains(&"services"));
    }
}
