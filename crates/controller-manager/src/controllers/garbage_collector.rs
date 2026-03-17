// Garbage Collector - Manages cascade deletion and orphaning of dependent resources
//
// Implements:
// - Owner reference tracking
// - Cascade deletion (foreground and background)
// - Orphan deletion
// - Finalizer handling for deletion protection

use rusternetes_common::types::{DeletionPropagation, ObjectMeta};
use rusternetes_storage::Storage;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

/// Garbage collector controller
#[allow(dead_code)]
pub struct GarbageCollector<S: Storage> {
    storage: Arc<S>,
    /// How often to run GC scan
    scan_interval: Duration,
    /// Maximum number of concurrent delete operations
    max_concurrent_deletes: usize,
    /// Batch size for deletion operations
    delete_batch_size: usize,
    /// Maximum retry attempts for failed deletions
    max_retries: u32,
}

impl<S: Storage + 'static> GarbageCollector<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self {
            storage,
            scan_interval: Duration::from_secs(30), // Run every 30 seconds
            max_concurrent_deletes: 50,             // Limit concurrent operations
            delete_batch_size: 100,                 // Process up to 100 deletions per batch
            max_retries: 3,                         // Retry failed deletions up to 3 times
        }
    }

    /// Create a new garbage collector with custom settings
    pub fn with_config(
        storage: Arc<S>,
        scan_interval_secs: u64,
        max_concurrent_deletes: usize,
        delete_batch_size: usize,
    ) -> Self {
        Self {
            storage,
            scan_interval: Duration::from_secs(scan_interval_secs),
            max_concurrent_deletes,
            delete_batch_size,
            max_retries: 3,
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

        // Detect cycles in dependency graph (non-blocking, just warn)
        if let Err(cycle_info) = self.detect_cycles(&dependent_map, &all_resources) {
            warn!(
                "Detected dependency cycle in resource graph: {}",
                cycle_info
            );
        }

        // Find orphaned resources (resources whose owners no longer exist)
        let orphans = self.find_orphans(&all_resources, &owner_map);

        if !orphans.is_empty() {
            info!("Found {} orphaned resources", orphans.len());

            // Process orphans in batches with retry logic
            let mut deleted_count = 0;
            let mut failed_count = 0;

            for batch in orphans.chunks(self.delete_batch_size) {
                let batch_results = self.delete_batch_with_retry(batch).await;

                for result in batch_results {
                    match result {
                        Ok(_) => deleted_count += 1,
                        Err(e) => {
                            failed_count += 1;
                            error!("Failed to delete orphan after retries: {}", e);
                        }
                    }
                }
            }

            info!(
                "GC orphan deletion complete: {} deleted, {} failed",
                deleted_count, failed_count
            );
        }

        // Handle namespace deletion - delete all resources in deleted namespaces
        let deleted_namespaces: Vec<_> = all_resources
            .iter()
            .filter(|r| r.resource_type == "namespaces" && r.metadata.is_being_deleted())
            .collect();

        for namespace in deleted_namespaces {
            if let Err(e) = self
                .cascade_delete_namespace(namespace, &all_resources)
                .await
            {
                error!(
                    "Failed to cascade delete namespace {}: {}",
                    namespace.metadata.name, e
                );
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
            ("namespaces", false), // cluster-scoped
            ("pods", true),
            ("services", true),
            ("endpoints", true),
            ("endpointslices", true),
            ("ingresses", true),
            ("networkpolicies", true),
            ("verticalpodautoscalers", true),
            ("volumesnapshots", true),
            ("volumesnapshotcontents", false), // cluster-scoped
            ("resourceclaims", true),
            ("certificatesigningrequests", false), // cluster-scoped
            ("customresourcedefinitions", false),  // cluster-scoped
            ("deployments", true),
            ("replicasets", true),
            ("statefulsets", true),
            ("daemonsets", true),
            ("jobs", true),
            ("cronjobs", true),
            ("configmaps", true),
            ("secrets", true),
            ("serviceaccounts", true),
            ("persistentvolumeclaims", true),
            ("persistentvolumes", false),   // cluster-scoped
            ("clusterroles", false),        // cluster-scoped
            ("clusterrolebindings", false), // cluster-scoped
        ];

        for (resource_type, namespaced) in resource_types {
            if namespaced {
                // For namespaced resources, we need to list across all namespaces
                // This is simplified - in reality we'd list all namespaces first
                let prefix = format!("/registry/{}/", resource_type);
                if let Ok(items) = self
                    .list_resources_with_metadata(&prefix, resource_type)
                    .await
                {
                    resources.extend(items);
                }
            } else {
                // For cluster-scoped resources
                let prefix = format!("/registry/{}/", resource_type);
                if let Ok(items) = self
                    .list_resources_with_metadata(&prefix, resource_type)
                    .await
                {
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
                // Reconstruct the storage key using the same format as build_key
                let key = match &metadata.namespace {
                    Some(ns) => format!("/registry/{}/{}/{}", resource_type, ns, metadata.name),
                    None => format!("/registry/{}/{}", resource_type, metadata.name),
                };

                resources.push(ResourceInfo {
                    key,
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
        let metadata = value.get("metadata").ok_or_else(|| {
            rusternetes_common::Error::InvalidResource("Missing metadata".to_string())
        })?;

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
                let has_missing_owner = owner_refs
                    .iter()
                    .any(|owner_ref| !existing_uids.contains(owner_ref.uid.as_str()));

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
        resource: &ResourceInfo,
        dependent_map: &HashMap<String, Vec<String>>,
    ) -> rusternetes_common::Result<()> {
        let resource_uid = &resource.metadata.uid;

        // Get all UIDs of dependents for this resource
        if let Some(dependent_uids) = dependent_map.get(resource_uid) {
            if dependent_uids.is_empty() {
                debug!("No dependents to delete for resource {}", resource.key);
                return Ok(());
            }

            info!(
                "Foreground deletion: deleting {} dependents of {}",
                dependent_uids.len(),
                resource.key
            );

            // Find all resources and filter to our dependents
            let all_resources = self.get_all_resources().await?;
            let dependents: Vec<_> = all_resources
                .iter()
                .filter(|r| dependent_uids.contains(&r.metadata.uid))
                .collect();

            // Delete each dependent
            for dependent in dependents {
                // Check if dependent has controller owner reference blocking deletion
                if let Some(owner_refs) = &dependent.metadata.owner_references {
                    for owner_ref in owner_refs {
                        if owner_ref.uid == *resource_uid
                            && owner_ref.block_owner_deletion.unwrap_or(false)
                        {
                            info!(
                                "Deleting dependent {} (blocked owner deletion)",
                                dependent.key
                            );
                            // Recursively delete dependents if they have any
                            let (_, dependent_map) = self.build_relationship_maps(&all_resources);
                            Box::pin(self.delete_dependents_foreground(dependent, &dependent_map))
                                .await?;

                            // Delete the dependent
                            if let Err(e) = self.storage.delete(&dependent.key).await {
                                error!("Failed to delete dependent {}: {}", dependent.key, e);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Orphan dependents by removing owner references
    async fn orphan_dependents(
        &self,
        resource: &ResourceInfo,
        dependent_map: &HashMap<String, Vec<String>>,
    ) -> rusternetes_common::Result<()> {
        let resource_uid = &resource.metadata.uid;

        // Get all UIDs of dependents for this resource
        if let Some(dependent_uids) = dependent_map.get(resource_uid) {
            if dependent_uids.is_empty() {
                debug!("No dependents to orphan for resource {}", resource.key);
                return Ok(());
            }

            info!(
                "Orphan deletion: removing owner references from {} dependents of {}",
                dependent_uids.len(),
                resource.key
            );

            // Find all resources and filter to our dependents
            let all_resources = self.get_all_resources().await?;
            let dependents: Vec<_> = all_resources
                .iter()
                .filter(|r| dependent_uids.contains(&r.metadata.uid))
                .collect();

            // Remove owner references from each dependent
            for dependent in dependents {
                info!("Orphaning dependent {}", dependent.key);

                // Parse the dependent's full object
                let mut dependent_value = dependent.value.clone();

                // Remove the owner reference to this resource
                if let Some(metadata) = dependent_value.get_mut("metadata") {
                    if let Some(owner_refs) = metadata.get_mut("ownerReferences") {
                        if let Some(owner_refs_array) = owner_refs.as_array_mut() {
                            // Filter out the owner reference matching this resource
                            owner_refs_array.retain(|owner_ref| {
                                owner_ref
                                    .get("uid")
                                    .and_then(|uid| uid.as_str())
                                    .map(|uid| uid != resource_uid)
                                    .unwrap_or(true)
                            });

                            // If no more owner references, remove the field
                            if owner_refs_array.is_empty() {
                                if let Some(metadata_obj) = metadata.as_object_mut() {
                                    metadata_obj.remove("ownerReferences");
                                }
                            }
                        }
                    }
                }

                // Update the dependent in storage
                if let Err(e) = self
                    .storage
                    .update_raw(&dependent.key, &dependent_value)
                    .await
                {
                    error!("Failed to orphan dependent {}: {}", dependent.key, e);
                }
            }
        }

        Ok(())
    }

    /// Cascade delete all resources in a namespace
    async fn cascade_delete_namespace(
        &self,
        namespace: &ResourceInfo,
        all_resources: &[ResourceInfo],
    ) -> rusternetes_common::Result<()> {
        let namespace_name = &namespace.metadata.name;
        info!("Cascading delete for namespace: {}", namespace_name);

        // Find all resources in this namespace
        let resources_in_namespace: Vec<_> = all_resources
            .iter()
            .filter(|r| {
                r.metadata.namespace.as_deref() == Some(namespace_name)
                    && r.resource_type != "namespaces"
            })
            .collect();

        // Delete all resources in the namespace
        for resource in resources_in_namespace {
            info!(
                "Deleting {} {} in namespace {}",
                resource.resource_type, resource.metadata.name, namespace_name
            );
            if let Err(e) = self.storage.delete(&resource.key).await {
                error!(
                    "Failed to delete {} {}: {}",
                    resource.resource_type, resource.metadata.name, e
                );
            }
        }

        // If no resources left in namespace and no finalizers, delete the namespace
        if !namespace.metadata.has_finalizers() {
            info!("Deleting namespace: {}", namespace_name);
            self.storage.delete(&namespace.key).await?;
        }

        Ok(())
    }

    /// Detect cycles in the dependency graph
    /// Returns Ok(()) if no cycles, Err with cycle info if found
    fn detect_cycles(
        &self,
        dependent_map: &HashMap<String, Vec<String>>,
        resources: &[ResourceInfo],
    ) -> Result<(), String> {
        // Build UID to resource name map for better error messages
        let uid_to_name: HashMap<_, _> = resources
            .iter()
            .map(|r| (r.metadata.uid.clone(), r.metadata.name.clone()))
            .collect();

        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        // DFS cycle detection
        for resource in resources {
            if !visited.contains(&resource.metadata.uid) {
                if let Some(cycle_path) = self.detect_cycle_dfs(
                    &resource.metadata.uid,
                    dependent_map,
                    &mut visited,
                    &mut rec_stack,
                    &mut Vec::new(),
                    &uid_to_name,
                ) {
                    return Err(format!("Cycle detected in ownership chain: {}", cycle_path));
                }
            }
        }

        Ok(())
    }

    /// DFS helper for cycle detection
    fn detect_cycle_dfs(
        &self,
        uid: &str,
        dependent_map: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        uid_to_name: &HashMap<String, String>,
    ) -> Option<String> {
        visited.insert(uid.to_string());
        rec_stack.insert(uid.to_string());
        path.push(uid.to_string());

        // Check all owners of this resource (dependents -> owners)
        if let Some(owners) = dependent_map.get(uid) {
            for owner_uid in owners {
                if !visited.contains(owner_uid) {
                    if let Some(cycle) = self.detect_cycle_dfs(
                        owner_uid,
                        dependent_map,
                        visited,
                        rec_stack,
                        path,
                        uid_to_name,
                    ) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(owner_uid) {
                    // Cycle detected! Build human-readable path
                    let cycle_start_idx = path.iter().position(|u| u == owner_uid).unwrap();
                    let cycle_path: Vec<_> = path[cycle_start_idx..]
                        .iter()
                        .map(|uid| {
                            uid_to_name
                                .get(uid)
                                .map(|name| name.as_str())
                                .unwrap_or("unknown")
                        })
                        .collect();
                    return Some(format!("{} -> {}", cycle_path.join(" -> "), cycle_path[0]));
                }
            }
        }

        path.pop();
        rec_stack.remove(uid);
        None
    }

    /// Delete a batch of orphans with retry logic
    async fn delete_batch_with_retry(&self, orphans: &[ResourceInfo]) -> Vec<Result<(), String>> {
        use futures::future::join_all;

        // Limit concurrency
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.max_concurrent_deletes));
        let mut tasks = Vec::new();

        for orphan in orphans {
            let sem = Arc::clone(&semaphore);
            let storage = Arc::clone(&self.storage);
            let orphan_clone = orphan.clone();
            let max_retries = self.max_retries;

            let task = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();

                // Retry with exponential backoff
                let mut attempt = 0;
                let mut last_error = None;

                while attempt < max_retries {
                    match storage.delete(&orphan_clone.key).await {
                        Ok(_) => {
                            info!(
                                "Successfully deleted orphan {} (attempt {})",
                                orphan_clone.key,
                                attempt + 1
                            );
                            return Ok(());
                        }
                        Err(e) => {
                            attempt += 1;
                            last_error = Some(e.to_string());

                            if attempt < max_retries {
                                // Exponential backoff: 100ms, 200ms, 400ms, ...
                                let backoff_ms = 100 * (1 << attempt);
                                debug!(
                                    "Failed to delete {} (attempt {}), retrying in {}ms: {}",
                                    orphan_clone.key, attempt, backoff_ms, e
                                );
                                sleep(Duration::from_millis(backoff_ms)).await;
                            }
                        }
                    }
                }

                Err(format!(
                    "Failed to delete {} after {} attempts: {}",
                    orphan_clone.key,
                    max_retries,
                    last_error.unwrap_or_else(|| "unknown error".to_string())
                ))
            });

            tasks.push(task);
        }

        // Wait for all tasks and collect results
        let results = join_all(tasks).await;
        results
            .into_iter()
            .map(|r| r.unwrap_or_else(|e| Err(format!("Task panicked: {}", e))))
            .collect()
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

    #[tokio::test]
    async fn test_deletion_propagation_policy() {
        let gc = GarbageCollector::new(Arc::new(
            rusternetes_storage::etcd::EtcdStorage::new(vec!["http://localhost:2379".to_string()])
                .await
                .unwrap(),
        ));

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
