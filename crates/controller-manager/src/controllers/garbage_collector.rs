// Garbage Collector - Manages cascade deletion and orphaning of dependent resources
//
// Implements:
// - Owner reference tracking
// - Cascade deletion (foreground and background)
// - Orphan deletion
// - Finalizer handling for deletion protection

use rusternetes_common::types::{DeletionPropagation, ObjectMeta};
use rusternetes_storage::{memory::MemoryStorage, Storage};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info};

/// Garbage collector controller
#[allow(dead_code)]
pub struct GarbageCollector {
    storage: Arc<MemoryStorage>,
    /// How often to run GC scan
    scan_interval: Duration,
}

impl GarbageCollector {
    pub fn new(storage: Arc<MemoryStorage>) -> Self {
        Self {
            storage,
            scan_interval: Duration::from_secs(30), // Run every 30 seconds
        }
    }

    /// Start the garbage collector
    pub async fn run(&self) {
        info!("Starting Garbage Collector");
        loop {
            if let Err(e) = self.scan_and_collect().await {
                error!("Garbage collection scan failed: {}", e);
            }
            sleep(self.scan_interval).await;
        }
    }

    /// Scan all resources and collect orphans
    pub async fn scan_and_collect(&self) -> rusternetes_common::Result<()> {
        debug!("Running garbage collection scan");

        // Get all resources from storage
        let all_resources = self.get_all_resources().await?;

        // Build owner-dependent relationship map
        let (owner_map, dependent_map) = self.build_relationship_maps(&all_resources);

        // Find orphaned resources (resources whose owners no longer exist)
        let orphans = self.find_orphans(&all_resources, &owner_map);

        if !orphans.is_empty() {
            info!("Found {} orphaned resources", orphans.len());
            for orphan in orphans {
                if let Err(e) = self.delete_orphan(&orphan).await {
                    error!("Failed to delete orphan {:?}: {}", orphan, e);
                }
            }
        }

        // Process resources with deletion timestamp
        let being_deleted: Vec<_> = all_resources
            .iter()
            .filter(|r| r.metadata.is_being_deleted())
            .collect();

        for resource in being_deleted {
            if let Err(e) = self.process_deletion(resource, &dependent_map).await {
                error!("Failed to process deletion for {:?}: {}", resource.key, e);
            }
        }

        debug!("Garbage collection scan complete");
        Ok(())
    }

    /// Get all resources from storage
    async fn get_all_resources(&self) -> rusternetes_common::Result<Vec<ResourceInfo>> {
        let mut resources = Vec::new();

        // List all resources across all namespaces and resource types
        // In a real implementation, this would be more sophisticated
        // For now, we'll scan known resource types
        let resource_types = vec![
            ("pods", true),
            ("services", true),
            ("deployments", true),
            ("replicasets", true),
            ("statefulsets", true),
            ("daemonsets", true),
            ("jobs", true),
            ("cronjobs", true),
            ("configmaps", true),
            ("secrets", true),
            ("persistentvolumeclaims", true),
            ("persistentvolumes", false), // cluster-scoped
        ];

        for (resource_type, namespaced) in resource_types {
            if namespaced {
                // For namespaced resources, we need to list across all namespaces
                // This is simplified - in reality we'd list all namespaces first
                let prefix = format!("/{}/", resource_type);
                if let Ok(items) = self.list_resources_with_metadata(&prefix, resource_type).await {
                    resources.extend(items);
                }
            } else {
                // For cluster-scoped resources
                let prefix = format!("/{}/", resource_type);
                if let Ok(items) = self.list_resources_with_metadata(&prefix, resource_type).await {
                    resources.extend(items);
                }
            }
        }

        Ok(resources)
    }

    /// List resources with metadata
    async fn list_resources_with_metadata(
        &self,
        prefix: &str,
        resource_type: &str,
    ) -> rusternetes_common::Result<Vec<ResourceInfo>> {
        let values: Vec<Value> = self.storage.list(prefix).await?;
        let mut resources = Vec::new();

        for value in values {
            if let Ok(metadata) = self.extract_metadata(&value) {
                resources.push(ResourceInfo {
                    key: format!("{}/{}/{}",
                        prefix,
                        metadata.namespace.as_deref().unwrap_or(""),
                        metadata.name
                    ),
                    metadata,
                    resource_type: resource_type.to_string(),
                    value,
                });
            }
        }

        Ok(resources)
    }

    /// Extract metadata from a resource
    fn extract_metadata(&self, value: &Value) -> rusternetes_common::Result<ObjectMeta> {
        let metadata = value
            .get("metadata")
            .ok_or_else(|| rusternetes_common::Error::InvalidResource(
                "Missing metadata".to_string()
            ))?;

        serde_json::from_value(metadata.clone())
            .map_err(|e| rusternetes_common::Error::Internal(e.to_string()))
    }

    /// Build owner-dependent relationship maps
    fn build_relationship_maps(
        &self,
        resources: &[ResourceInfo],
    ) -> (HashMap<String, Vec<String>>, HashMap<String, Vec<String>>) {
        let mut owner_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut dependent_map: HashMap<String, Vec<String>> = HashMap::new();

        for resource in resources {
            let resource_uid = &resource.metadata.uid;

            // Track what this resource owns
            if let Some(owner_refs) = &resource.metadata.owner_references {
                for owner_ref in owner_refs {
                    // Map: owner UID -> list of dependent UIDs
                    owner_map
                        .entry(owner_ref.uid.clone())
                        .or_insert_with(Vec::new)
                        .push(resource_uid.clone());

                    // Map: dependent UID -> list of owner UIDs
                    dependent_map
                        .entry(resource_uid.clone())
                        .or_insert_with(Vec::new)
                        .push(owner_ref.uid.clone());
                }
            }
        }

        (owner_map, dependent_map)
    }

    /// Find orphaned resources
    fn find_orphans(
        &self,
        resources: &[ResourceInfo],
        _owner_map: &HashMap<String, Vec<String>>,
    ) -> Vec<ResourceInfo> {
        let existing_uids: HashSet<_> = resources.iter().map(|r| r.metadata.uid.as_str()).collect();
        let mut orphans = Vec::new();

        for resource in resources {
            if let Some(owner_refs) = &resource.metadata.owner_references {
                // Check if any owner references point to non-existent owners
                let has_missing_owner = owner_refs.iter().any(|owner_ref| {
                    !existing_uids.contains(owner_ref.uid.as_str())
                });

                if has_missing_owner {
                    orphans.push(resource.clone());
                }
            }
        }

        orphans
    }

    /// Delete an orphaned resource
    async fn delete_orphan(&self, orphan: &ResourceInfo) -> rusternetes_common::Result<()> {
        info!(
            "Deleting orphaned resource: {} ({})",
            orphan.key, orphan.resource_type
        );
        self.storage.delete(&orphan.key).await
    }

    /// Process deletion for a resource with deletion timestamp
    async fn process_deletion(
        &self,
        resource: &ResourceInfo,
        dependent_map: &HashMap<String, Vec<String>>,
    ) -> rusternetes_common::Result<()> {
        // If resource has finalizers, we can't delete it yet
        if resource.metadata.has_finalizers() {
            debug!(
                "Resource {} has finalizers, skipping deletion",
                resource.key
            );
            return Ok(());
        }

        // Check deletion propagation policy from finalizers
        let propagation_policy = self.determine_propagation_policy(&resource.metadata);

        match propagation_policy {
            DeletionPropagation::Foreground => {
                // In foreground deletion, we must delete all dependents first
                self.delete_dependents_foreground(resource, dependent_map)
                    .await?;
            }
            DeletionPropagation::Background => {
                // In background deletion, we delete the owner and let GC clean up dependents
                // This is handled by the orphan detection in the next scan
            }
            DeletionPropagation::Orphan => {
                // In orphan mode, we remove owner references from dependents
                self.orphan_dependents(resource, dependent_map).await?;
            }
        }

        // If no more finalizers and all dependents handled, delete the resource
        if !resource.metadata.has_finalizers() {
            info!("Deleting resource: {}", resource.key);
            self.storage.delete(&resource.key).await?;
        }

        Ok(())
    }

    /// Determine deletion propagation policy from metadata
    fn determine_propagation_policy(&self, metadata: &ObjectMeta) -> DeletionPropagation {
        if let Some(finalizers) = &metadata.finalizers {
            if finalizers.contains(&"foregroundDeletion".to_string()) {
                return DeletionPropagation::Foreground;
            }
            if finalizers.contains(&"orphan".to_string()) {
                return DeletionPropagation::Orphan;
            }
        }
        // Default to background deletion
        DeletionPropagation::Background
    }

    /// Delete dependents in foreground mode
    async fn delete_dependents_foreground(
        &self,
        _resource: &ResourceInfo,
        _dependent_map: &HashMap<String, Vec<String>>,
    ) -> rusternetes_common::Result<()> {
        // TODO: Implement foreground deletion
        // This would involve finding all dependents and deleting them first
        Ok(())
    }

    /// Orphan dependents by removing owner references
    async fn orphan_dependents(
        &self,
        _resource: &ResourceInfo,
        _dependent_map: &HashMap<String, Vec<String>>,
    ) -> rusternetes_common::Result<()> {
        // TODO: Implement orphaning
        // This would involve finding all dependents and removing owner references
        Ok(())
    }
}

/// Information about a resource for GC purposes
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct ResourceInfo {
    /// Storage key for the resource
    key: String,
    /// Resource metadata
    metadata: ObjectMeta,
    /// Resource type (e.g., "pods", "deployments")
    resource_type: String,
    /// Full resource value
    value: Value,
}

/// Cascade deletion options
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteOptions {
    /// Deletion propagation policy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub propagation_policy: Option<DeletionPropagation>,

    /// Grace period seconds before deletion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period_seconds: Option<i64>,

    /// Preconditions for deletion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preconditions: Option<Preconditions>,

    /// Whether to orphan dependents (deprecated, use propagation_policy)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orphan_dependents: Option<bool>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preconditions {
    /// UID must match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,

    /// ResourceVersion must match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_version: Option<String>,
}

impl Default for DeleteOptions {
    fn default() -> Self {
        Self {
            propagation_policy: Some(DeletionPropagation::Background),
            grace_period_seconds: Some(30),
            preconditions: None,
            orphan_dependents: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::types::OwnerReference;

    #[test]
    fn test_deletion_propagation_policy() {
        let gc = GarbageCollector::new(Arc::new(rusternetes_storage::memory::MemoryStorage::new()));

        // Test foreground deletion
        let mut metadata = ObjectMeta::new("test");
        metadata.finalizers = Some(vec!["foregroundDeletion".to_string()]);
        assert_eq!(
            gc.determine_propagation_policy(&metadata),
            DeletionPropagation::Foreground
        );

        // Test orphan deletion
        metadata.finalizers = Some(vec!["orphan".to_string()]);
        assert_eq!(
            gc.determine_propagation_policy(&metadata),
            DeletionPropagation::Orphan
        );

        // Test default (background)
        metadata.finalizers = None;
        assert_eq!(
            gc.determine_propagation_policy(&metadata),
            DeletionPropagation::Background
        );
    }

    #[test]
    fn test_owner_reference_creation() {
        let owner_ref = OwnerReference::new("v1", "Pod", "my-pod", "abc-123")
            .with_controller(true)
            .with_block_owner_deletion(true);

        assert_eq!(owner_ref.kind, "Pod");
        assert_eq!(owner_ref.controller, Some(true));
        assert_eq!(owner_ref.block_owner_deletion, Some(true));
    }

    #[test]
    fn test_metadata_finalizer_helpers() {
        let mut metadata = ObjectMeta::new("test");

        // Test add_finalizer
        metadata.add_finalizer("my-finalizer".to_string());
        assert!(metadata.has_finalizers());
        assert_eq!(metadata.finalizers.as_ref().unwrap().len(), 1);

        // Test idempotent add
        metadata.add_finalizer("my-finalizer".to_string());
        assert_eq!(metadata.finalizers.as_ref().unwrap().len(), 1);

        // Test remove_finalizer
        metadata.remove_finalizer("my-finalizer");
        assert!(!metadata.has_finalizers());
    }
}
