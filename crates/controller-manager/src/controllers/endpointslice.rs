use anyhow::Result;
use rusternetes_common::resources::{Endpoints, EndpointSlice};
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

        for endpoints in endpoints_list {
            if let Err(e) = self.reconcile_endpoints(&endpoints).await {
                error!(
                    "Failed to reconcile endpointslices for endpoints {}/{}: {}",
                    endpoints.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
                    &endpoints.metadata.name,
                    e
                );
            }
        }

        Ok(())
    }

    /// Reconcile endpointslices for a single endpoints object
    async fn reconcile_endpoints(&self, endpoints: &Endpoints) -> Result<()> {
        let namespace = endpoints.metadata.namespace.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Endpoints has no namespace"))?;
        let endpoints_name = &endpoints.metadata.name;

        debug!("Reconciling endpointslices for endpoints {}/{}", namespace, endpoints_name);

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
            endpointslice.metadata.owner_references = Some(vec![
                rusternetes_common::types::OwnerReference {
                    api_version: "v1".to_string(),
                    kind: "Endpoints".to_string(),
                    name: endpoints_name.clone(),
                    uid: endpoints.metadata.uid.clone(),
                    controller: Some(true),
                    block_owner_deletion: Some(true),
                },
            ]);

            let slice_key = build_key("endpointslices", Some(namespace), &slice_name);

            // Try to update first, if it doesn't exist, create it
            match self.storage.update(&slice_key, &endpointslice).await {
                Ok(_) => {
                    info!(
                        "Updated endpointslice {}/{} from endpoints",
                        namespace,
                        slice_name
                    );
                },
                Err(rusternetes_common::Error::NotFound(_)) => {
                    self.storage.create(&slice_key, &endpointslice).await?;
                    info!(
                        "Created endpointslice {}/{} from endpoints",
                        namespace,
                        slice_name
                    );
                },
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
                    debug!("EndpointSlice {} has no namespace, skipping", slice.metadata.name);
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
                    let slice_key = build_key("endpointslices", Some(namespace), &slice.metadata.name);
                    if let Err(e) = self.storage.delete(&slice_key).await {
                        error!(
                            "Failed to delete orphaned endpointslice {}/{}: {}",
                            namespace,
                            slice.metadata.name,
                            e
                        );
                    } else {
                        info!(
                            "Deleted orphaned endpointslice {}/{}",
                            namespace,
                            slice.metadata.name
                        );
                    }
                }
                Err(e) => {
                    error!("Error checking endpoints {}/{}: {}", namespace, service_name, e);
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

    #[tokio::test]
    async fn test_endpointslice_controller_creation() {
        let storage = Arc::new(MemoryStorage::new());
        let _controller = EndpointSliceController::new(storage);
    }
}
