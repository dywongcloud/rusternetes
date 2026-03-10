use anyhow::Result;
use rusternetes_common::resources::{
    ResourceClaim, ResourceClaimStatus, AllocationResult, DeviceAllocationResult,
    DeviceRequestAllocationResult, ResourceSlice, DeviceClass,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

/// ResourceClaimController manages the allocation of devices for ResourceClaims
///
/// This controller:
/// 1. Watches for unallocated ResourceClaims
/// 2. Finds suitable devices from ResourceSlices based on DeviceClass selectors
/// 3. Allocates devices and updates ResourceClaim status
pub struct ResourceClaimController<S: Storage> {
    storage: Arc<S>,
}

impl<S: Storage> ResourceClaimController<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting ResourceClaim Controller");

        loop {
            if let Err(e) = self.reconcile_all().await {
                error!("Error in ResourceClaim reconciliation loop: {}", e);
            }
            time::sleep(Duration::from_secs(5)).await;
        }
    }

    pub async fn reconcile_all(&self) -> Result<()> {
        // Get all ResourceClaims across all namespaces
        let claims: Vec<ResourceClaim> = self
            .storage
            .list("/registry/resourceclaims/")
            .await?;

        for mut claim in claims {
            if let Err(e) = self.reconcile_claim(&mut claim).await {
                error!(
                    "Failed to reconcile ResourceClaim {}/{}: {}",
                    claim.metadata.as_ref().and_then(|m| m.namespace.as_ref()).unwrap_or(&"".to_string()),
                    claim.metadata.as_ref().and_then(|m| m.name.as_ref()).unwrap_or(&"".to_string()),
                    e
                );
            }
        }

        Ok(())
    }

    async fn reconcile_claim(&self, claim: &mut ResourceClaim) -> Result<()> {
        let metadata = claim.metadata.as_ref()
            .ok_or_else(|| anyhow::anyhow!("ResourceClaim missing metadata"))?;

        let name = metadata.name.as_ref()
            .ok_or_else(|| anyhow::anyhow!("ResourceClaim missing name"))?;

        let namespace = metadata.namespace.as_ref()
            .ok_or_else(|| anyhow::anyhow!("ResourceClaim missing namespace"))?;

        // Skip if already allocated
        if claim.status.as_ref().and_then(|s| s.allocation.as_ref()).is_some() {
            return Ok(());
        }

        info!("Allocating devices for ResourceClaim {}/{}", namespace, name);

        // Process device requests
        let mut allocation_results = Vec::new();

        for request in &claim.spec.devices.requests {
            info!("Processing device request: {}", request.name);

            // Try to allocate from exact request
            if let Some(exact) = &request.exactly {
                let device_class_name = &exact.device_class_name;

                // Get DeviceClass
                let device_class = match self.get_device_class(device_class_name).await {
                    Ok(dc) => dc,
                    Err(e) => {
                        warn!("DeviceClass {} not found: {}", device_class_name, e);
                        continue;
                    }
                };

                info!("Found DeviceClass: {}", device_class_name);

                // Find suitable devices from ResourceSlices
                let devices = self.find_suitable_devices(&device_class, exact.count).await?;

                if devices.is_empty() {
                    warn!("No suitable devices found for request {}", request.name);
                    continue;
                }

                info!("Found {} suitable device(s) for request {}", devices.len(), request.name);

                // Allocate the first suitable device(s)
                for device in devices {
                    allocation_results.push(DeviceRequestAllocationResult {
                        request: request.name.clone(),
                        driver: device.driver,
                        pool: device.pool,
                        device: device.device_name,
                    });
                }
            }
        }

        if allocation_results.is_empty() {
            warn!("No devices could be allocated for ResourceClaim {}/{}", namespace, name);
            return Ok(());
        }

        // Update ResourceClaim status with allocation
        let status = ResourceClaimStatus {
            allocation: Some(AllocationResult {
                devices: DeviceAllocationResult {
                    results: allocation_results,
                    config: vec![],
                },
                node_selector: None,
            }),
            devices: vec![],
            reserved_for: vec![],
            deallocation_requested: None,
        };

        claim.status = Some(status);

        // Save updated ResourceClaim
        let key = build_key("resourceclaims", Some(namespace), name);
        self.storage.update(&key, claim).await?;

        info!("Successfully allocated devices for ResourceClaim {}/{}", namespace, name);
        Ok(())
    }

    async fn get_device_class(&self, name: &str) -> Result<DeviceClass> {
        let key = build_key("deviceclasses", None, name);
        let device_class: DeviceClass = self.storage.get(&key).await?;
        Ok(device_class)
    }

    async fn find_suitable_devices(
        &self,
        device_class: &DeviceClass,
        count: Option<i64>,
    ) -> Result<Vec<AllocatedDevice>> {
        let mut suitable_devices = Vec::new();
        let required_count = count.unwrap_or(1) as usize;

        // Get all ResourceSlices
        let slices: Vec<ResourceSlice> = self
            .storage
            .list("/registry/resourceslices/")
            .await?;

        info!("Checking {} ResourceSlice(s) for suitable devices", slices.len());

        for slice in slices {
            let driver = slice.spec.driver.clone();
            let pool_name = slice.spec.pool.name.clone();

            // Check each device in the slice
            for device in &slice.spec.devices {
                // Simple device selection - in production, this would:
                // 1. Evaluate CEL expressions from DeviceClass selectors
                // 2. Check device attributes against selector requirements
                // 3. Verify device capacity and availability
                // 4. Check node affinity constraints

                // For now, we'll do basic matching
                let is_suitable = self.device_matches_class(&device.name, device_class);

                if is_suitable {
                    suitable_devices.push(AllocatedDevice {
                        driver: driver.clone(),
                        pool: pool_name.clone(),
                        device_name: device.name.clone(),
                    });

                    // Stop if we have enough devices
                    if suitable_devices.len() >= required_count {
                        return Ok(suitable_devices);
                    }
                }
            }
        }

        Ok(suitable_devices)
    }

    /// Basic device matching - checks if device name/attributes match class selectors
    /// In production, this would evaluate CEL expressions
    fn device_matches_class(&self, _device_name: &str, device_class: &DeviceClass) -> bool {
        // If no selectors specified, match all devices
        if device_class.spec.selectors.is_empty() {
            return true;
        }

        // For now, accept all devices if selectors exist
        // In production, this would evaluate CEL expressions like:
        // - device.driver == "nvidia.com/gpu"
        // - device.attributes["model"].string == "A100"
        // - device.capacity["nvidia.com/gpu"].value >= "1"

        // TODO: Implement CEL evaluation using cel-interpreter crate
        true
    }
}

#[derive(Debug, Clone)]
struct AllocatedDevice {
    driver: String,
    pool: String,
    device_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_storage::memory::MemoryStorage;
    use rusternetes_common::resources::{
        DeviceClassSpec, ResourceClaimSpec, DeviceClaim, DeviceRequest,
        ExactDeviceRequest, DeviceAllocationMode, ResourceSliceSpec, ResourcePool, Device,
    };

    #[tokio::test]
    async fn test_device_matching_no_selectors() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = ResourceClaimController::new(storage);

        let device_class = DeviceClass {
            api_version: "resource.k8s.io/v1".to_string(),
            kind: "DeviceClass".to_string(),
            metadata: Some(rusternetes_common::resources::dra::ObjectMeta {
                name: Some("test-class".to_string()),
                ..Default::default()
            }),
            spec: DeviceClassSpec {
                selectors: vec![],
                config: vec![],
                suitable_nodes: None,
            },
        };

        // Should match any device when no selectors
        assert!(controller.device_matches_class("gpu-0", &device_class));
        assert!(controller.device_matches_class("any-device", &device_class));
    }

    #[tokio::test]
    async fn test_reconcile_already_allocated() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = ResourceClaimController::new(storage.clone());

        let mut claim = ResourceClaim {
            api_version: "resource.k8s.io/v1".to_string(),
            kind: "ResourceClaim".to_string(),
            metadata: Some(rusternetes_common::resources::dra::ObjectMeta {
                name: Some("test-claim".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            }),
            spec: ResourceClaimSpec {
                devices: DeviceClaim::default(),
            },
            status: Some(ResourceClaimStatus {
                allocation: Some(AllocationResult {
                    devices: DeviceAllocationResult {
                        results: vec![],
                        config: vec![],
                    },
                    node_selector: None,
                }),
                devices: vec![],
                reserved_for: vec![],
                deallocation_requested: None,
            }),
        };

        // Should skip already allocated claims
        let result = controller.reconcile_claim(&mut claim).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_find_suitable_devices_empty_slices() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = ResourceClaimController::new(storage);

        let device_class = DeviceClass {
            api_version: "resource.k8s.io/v1".to_string(),
            kind: "DeviceClass".to_string(),
            metadata: Some(rusternetes_common::resources::dra::ObjectMeta {
                name: Some("test-class".to_string()),
                ..Default::default()
            }),
            spec: DeviceClassSpec::default(),
        };

        let devices = controller.find_suitable_devices(&device_class, Some(1)).await.unwrap();
        assert_eq!(devices.len(), 0);
    }
}
