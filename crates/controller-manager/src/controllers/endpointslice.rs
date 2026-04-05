use anyhow::Result;
use rusternetes_common::resources::{EndpointSlice, Endpoints};
use rusternetes_storage::{build_key, Storage};
use std::sync::Arc;
use tracing::{debug, error, info};

/// EndpointSliceController watches Endpoints and automatically maintains EndpointSlice resources.
/// EndpointSlices are the modern replacement for Endpoints in Kubernetes, providing better
/// scalability for services with many endpoints.
///
/// This controller:
/// 1. Watches all Endpoints resources
/// 2. For each Endpoints object, creates corresponding EndpointSlice(s)
/// 3. Syncs changes from Endpoints to EndpointSlices
pub struct EndpointSliceController<S: Storage> {
    storage: Arc<S>,
}

impl<S: Storage> EndpointSliceController<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    /// Main reconciliation loop - syncs all endpoints to endpointslices
    pub async fn reconcile_all(&self) -> Result<()> {
        debug!("Starting endpointslice reconciliation");

        // List all endpoints across all namespaces
        let endpoints_list: Vec<Endpoints> = self.storage.list("/registry/endpoints/").await?;

        // Build a set of existing endpoints names for orphan detection
        let mut endpoints_names: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();
        for ep in &endpoints_list {
            let ns = ep
                .metadata
                .namespace
                .as_deref()
                .unwrap_or("default")
                .to_string();
            endpoints_names.insert((ns, ep.metadata.name.clone()));
        }

        for endpoints in endpoints_list {
            if let Err(e) = self.reconcile_endpoints(&endpoints).await {
                error!(
                    "Failed to reconcile endpointslices for endpoints {}/{}: {}",
                    endpoints
                        .metadata
                        .namespace
                        .as_ref()
                        .unwrap_or(&"default".to_string()),
                    &endpoints.metadata.name,
                    e
                );
            }
        }

        // Clean up orphaned EndpointSlices (whose source Endpoints no longer exist)
        let all_slices: Vec<EndpointSlice> = self
            .storage
            .list("/registry/endpointslices/")
            .await
            .unwrap_or_default();
        for slice in all_slices {
            let ns = slice.metadata.namespace.as_deref().unwrap_or("default");
            let name = &slice.metadata.name;
            // EndpointSlice names match their source Endpoints name (or name-suffix)
            // Only strip the last segment if it's numeric (i.e., a generated suffix like "-1", "-2")
            let source_name = match name.rsplit_once('-') {
                Some((prefix, suffix)) if suffix.chars().all(|c| c.is_ascii_digit()) => {
                    prefix.to_string()
                }
                _ => name.clone(),
            };
            if !endpoints_names.contains(&(ns.to_string(), source_name.clone()))
                && !endpoints_names.contains(&(ns.to_string(), name.clone()))
            {
                // Only delete slices managed by the endpointslice-controller (mirrored from Endpoints)
                // Do NOT delete slices created externally (e.g., by conformance tests or users)
                let is_mirrored = slice
                    .metadata
                    .labels
                    .as_ref()
                    .map(|l| {
                        l.get("endpointslice.kubernetes.io/managed-by")
                            .map(|v| v == "endpointslice-controller.k8s.io")
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);
                if is_mirrored {
                    let key = rusternetes_storage::build_key("endpointslices", Some(ns), name);
                    debug!("Deleting orphaned EndpointSlice {}/{}", ns, name);
                    let _ = self.storage.delete(&key).await;
                }
            }
        }

        Ok(())
    }

    /// Reconcile endpointslices for a single endpoints object
    async fn reconcile_endpoints(&self, endpoints: &Endpoints) -> Result<()> {
        let namespace = endpoints
            .metadata
            .namespace
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Endpoints has no namespace"))?;
        let endpoints_name = &endpoints.metadata.name;

        debug!(
            "Reconciling endpointslices for endpoints {}/{}",
            namespace, endpoints_name
        );

        // Convert Endpoints to EndpointSlice(s)
        let endpointslices = EndpointSlice::from_endpoints(endpoints);

        // For simplicity, we create one EndpointSlice per Endpoints object
        // In real Kubernetes, EndpointSlices are split when they exceed size limits
        for (idx, mut endpointslice) in endpointslices.into_iter().enumerate() {
            // Set the name - for the first slice, use the service name
            // For additional slices, append a suffix
            let slice_name = if idx == 0 {
                endpoints_name.clone()
            } else {
                format!("{}-{}", endpoints_name, idx)
            };

            endpointslice.metadata.name = slice_name.clone();
            endpointslice.metadata.namespace = Some(namespace.clone());

            // Set owner reference to the Endpoints object
            // This ensures EndpointSlices are garbage collected when Endpoints are deleted
            endpointslice.metadata.owner_references =
                Some(vec![rusternetes_common::types::OwnerReference {
                    api_version: "v1".to_string(),
                    kind: "Endpoints".to_string(),
                    name: endpoints_name.clone(),
                    uid: endpoints.metadata.uid.clone(),
                    controller: Some(true),
                    block_owner_deletion: Some(true),
                }]);

            let slice_key = build_key("endpointslices", Some(namespace), &slice_name);

            // Check if existing endpointslice matches — skip write if nothing changed
            if let Ok(existing) = self.storage.get::<EndpointSlice>(&slice_key).await {
                // Compare endpoints and ports (the fields that actually change)
                if existing.endpoints == endpointslice.endpoints
                    && existing.ports == endpointslice.ports
                {
                    debug!(
                        "Endpointslice {}/{} unchanged, skipping write",
                        namespace, slice_name
                    );
                    continue;
                }
                // Preserve resource version for update
                endpointslice.metadata.resource_version = existing.metadata.resource_version;
            }

            // Try to update first, if it doesn't exist, create it
            match self.storage.update(&slice_key, &endpointslice).await {
                Ok(_) => {
                    info!(
                        "Updated endpointslice {}/{} from endpoints",
                        namespace, slice_name
                    );
                }
                Err(rusternetes_common::Error::NotFound(_)) => {
                    self.storage.create(&slice_key, &endpointslice).await?;
                    info!(
                        "Created endpointslice {}/{} from endpoints",
                        namespace, slice_name
                    );
                }
                Err(e) => return Err(e.into()),
            }
        }

        Ok(())
    }

    /// Clean up orphaned EndpointSlices that no longer have corresponding Endpoints
    pub async fn cleanup_orphans(&self) -> Result<()> {
        debug!("Cleaning up orphaned endpointslices");

        // List all endpointslices
        let all_slices: Vec<EndpointSlice> = self.storage.list("/registry/endpointslices/").await?;

        for slice in all_slices {
            let namespace = match &slice.metadata.namespace {
                Some(ns) => ns,
                None => {
                    debug!(
                        "EndpointSlice {} has no namespace, skipping",
                        slice.metadata.name
                    );
                    continue;
                }
            };

            // Check if the slice has the service label
            let service_name = match &slice.metadata.labels {
                Some(labels) => match labels.get("kubernetes.io/service-name") {
                    Some(name) => name,
                    None => continue, // Not managed by us
                },
                None => continue,
            };

            // Check if corresponding endpoints exist
            let endpoints_key = build_key("endpoints", Some(namespace), service_name);
            match self.storage.get::<Endpoints>(&endpoints_key).await {
                Ok(_) => {
                    // Endpoints exist, keep the slice
                    continue;
                }
                Err(rusternetes_common::Error::NotFound(_)) => {
                    // Endpoints don't exist, delete the slice
                    let slice_key =
                        build_key("endpointslices", Some(namespace), &slice.metadata.name);
                    if let Err(e) = self.storage.delete(&slice_key).await {
                        error!(
                            "Failed to delete orphaned endpointslice {}/{}: {}",
                            namespace, slice.metadata.name, e
                        );
                    } else {
                        info!(
                            "Deleted orphaned endpointslice {}/{}",
                            namespace, slice.metadata.name
                        );
                    }
                }
                Err(e) => {
                    error!(
                        "Error checking endpoints {}/{}: {}",
                        namespace, service_name, e
                    );
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_storage::MemoryStorage;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_endpointslice_controller_creation() {
        let storage = Arc::new(MemoryStorage::new());
        let _controller = EndpointSliceController::new(storage);
    }

    /// Helper: extract the source (base) name from an EndpointSlice name,
    /// mirroring the logic used in reconcile_all() for orphan detection.
    fn extract_source_name(name: &str) -> String {
        match name.rsplit_once('-') {
            Some((prefix, suffix)) if suffix.chars().all(|c| c.is_ascii_digit()) => {
                prefix.to_string()
            }
            _ => name.to_string(),
        }
    }

    #[test]
    fn test_base_name_extraction() {
        // Primary slice: name matches the service exactly, no numeric suffix
        assert_eq!(extract_source_name("my-service"), "my-service");

        // Suffixed slice: last segment is numeric
        assert_eq!(extract_source_name("my-service-1"), "my-service");
        assert_eq!(extract_source_name("my-service-42"), "my-service");

        // Multi-dash service name with no numeric suffix
        assert_eq!(
            extract_source_name("my-multi-dash-service"),
            "my-multi-dash-service"
        );

        // Multi-dash service name with numeric suffix
        assert_eq!(
            extract_source_name("my-multi-dash-service-3"),
            "my-multi-dash-service"
        );

        // Single-word name (no dashes at all)
        assert_eq!(extract_source_name("kubernetes"), "kubernetes");

        // Name ending with non-numeric suffix (e.g., a hash)
        assert_eq!(extract_source_name("my-service-abc"), "my-service-abc");
    }

    #[tokio::test]
    async fn test_orphan_detection_skips_externally_managed_slices() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = EndpointSliceController::new(Arc::clone(&storage));

        // Create an EndpointSlice NOT managed by the controller (no managed-by label)
        let mut external_slice = EndpointSlice::new("external-slice", "IPv4");
        external_slice.metadata.namespace = Some("default".to_string());
        let key = build_key("endpointslices", Some("default"), "external-slice");
        storage.create(&key, &external_slice).await.unwrap();

        // Create an EndpointSlice managed by the controller (with managed-by label)
        let mut managed_slice = EndpointSlice::new("managed-slice", "IPv4");
        managed_slice.metadata.namespace = Some("default".to_string());
        let mut labels = HashMap::new();
        labels.insert(
            "endpointslice.kubernetes.io/managed-by".to_string(),
            "endpointslice-controller.k8s.io".to_string(),
        );
        managed_slice.metadata.labels = Some(labels);
        let key2 = build_key("endpointslices", Some("default"), "managed-slice");
        storage.create(&key2, &managed_slice).await.unwrap();

        // No Endpoints exist, so both slices are orphans.
        // Only the managed one should be deleted.
        controller.reconcile_all().await.unwrap();

        // External slice should survive
        assert!(
            storage.get::<EndpointSlice>(&key).await.is_ok(),
            "externally-managed EndpointSlice should NOT be deleted"
        );

        // Managed slice should be deleted
        assert!(
            storage.get::<EndpointSlice>(&key2).await.is_err(),
            "controller-managed orphan EndpointSlice should be deleted"
        );
    }
}
