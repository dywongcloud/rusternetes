use anyhow::Result;
use rusternetes_common::resources::{Pod, ResourceQuota, ResourceQuotaStatus};
use rusternetes_common::types::Phase;
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info};

/// ResourceQuotaController tracks resource usage per namespace and enforces quota limits.
/// It:
/// 1. Watches ResourceQuotas across all namespaces
/// 2. Calculates current resource usage (pods, cpu, memory, etc.)
/// 3. Updates ResourceQuota status with used vs hard limits
pub struct ResourceQuotaController<S: Storage> {
    storage: Arc<S>,
}

impl<S: Storage> ResourceQuotaController<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    /// Main reconciliation loop - syncs all resource quotas
    pub async fn reconcile_all(&self) -> Result<()> {
        debug!("Starting resource quota reconciliation");

        // List all resource quotas across all namespaces
        let quotas: Vec<ResourceQuota> = self
            .storage
            .list(&build_prefix("resourcequotas", None))
            .await?;

        for quota in quotas {
            if let Err(e) = self.reconcile_quota(&quota).await {
                error!(
                    "Failed to reconcile quota {}/{}: {}",
                    quota
                        .metadata
                        .namespace
                        .as_ref()
                        .unwrap_or(&"default".to_string()),
                    &quota.metadata.name,
                    e
                );
            }
        }

        Ok(())
    }

    /// Reconcile a single resource quota
    async fn reconcile_quota(&self, quota: &ResourceQuota) -> Result<()> {
        let namespace = quota
            .metadata
            .namespace
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("ResourceQuota has no namespace"))?;
        let quota_name = &quota.metadata.name;

        debug!("Reconciling quota {}/{}", namespace, quota_name);

        // Calculate current resource usage in the namespace
        let used = self.calculate_usage(namespace).await?;

        // Update quota status
        let mut updated_quota = quota.clone();
        updated_quota.status = Some(ResourceQuotaStatus {
            hard: quota.spec.hard.clone(),
            used: Some(used),
        });

        // Save updated quota
        let key = build_key("resourcequotas", Some(namespace), quota_name);
        self.storage.update(&key, &updated_quota).await?;

        info!("Updated quota {}/{} status", namespace, quota_name);

        Ok(())
    }

    /// Calculate resource usage in a namespace
    async fn calculate_usage(&self, namespace: &str) -> Result<HashMap<String, String>> {
        let mut usage = HashMap::new();

        // Count active pods (not Failed/Succeeded)
        let pod_prefix = format!("/registry/pods/{}/", namespace);
        let all_pods: Vec<Pod> = self.storage.list(&pod_prefix).await?;
        let pods: Vec<&Pod> = all_pods.iter()
            .filter(|p| {
                let phase = p.status.as_ref().and_then(|s| s.phase.as_ref());
                !matches!(phase, Some(Phase::Failed) | Some(Phase::Succeeded))
            })
            .collect();
        usage.insert("pods".to_string(), pods.len().to_string());

        // Calculate CPU and memory requests
        let mut total_cpu_requests = 0i64;
        let mut total_memory_requests = 0i64;
        let mut total_cpu_limits = 0i64;
        let mut total_memory_limits = 0i64;

        for pod in pods.iter() {
            if let Some(spec) = &pod.spec {
                for container in &spec.containers {
                    // Count CPU requests
                    if let Some(resources) = &container.resources {
                        if let Some(requests) = &resources.requests {
                            if let Some(cpu) = requests.get("cpu") {
                                if let Ok(millis) = self.parse_cpu_to_millicores(cpu) {
                                    total_cpu_requests += millis;
                                }
                            }
                            if let Some(memory) = requests.get("memory") {
                                if let Ok(bytes) = self.parse_memory_to_bytes(memory) {
                                    total_memory_requests += bytes;
                                }
                            }
                        }
                        if let Some(limits) = &resources.limits {
                            if let Some(cpu) = limits.get("cpu") {
                                if let Ok(millis) = self.parse_cpu_to_millicores(cpu) {
                                    total_cpu_limits += millis;
                                }
                            }
                            if let Some(memory) = limits.get("memory") {
                                if let Ok(bytes) = self.parse_memory_to_bytes(memory) {
                                    total_memory_limits += bytes;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Convert to Kubernetes resource format
        if total_cpu_requests > 0 {
            usage.insert(
                "requests.cpu".to_string(),
                format!("{}m", total_cpu_requests),
            );
        }
        if total_memory_requests > 0 {
            usage.insert(
                "requests.memory".to_string(),
                self.bytes_to_memory_string(total_memory_requests),
            );
        }
        if total_cpu_limits > 0 {
            usage.insert("limits.cpu".to_string(), format!("{}m", total_cpu_limits));
        }
        if total_memory_limits > 0 {
            usage.insert(
                "limits.memory".to_string(),
                self.bytes_to_memory_string(total_memory_limits),
            );
        }

        Ok(usage)
    }

    /// Parse CPU string to millicores (e.g., "1" -> 1000, "500m" -> 500)
    fn parse_cpu_to_millicores(&self, cpu: &str) -> Result<i64> {
        if cpu.ends_with('m') {
            // Already in millicores
            let millis = cpu.trim_end_matches('m').parse::<i64>()?;
            Ok(millis)
        } else {
            // In cores, convert to millicores
            let cores = cpu.parse::<f64>()?;
            Ok((cores * 1000.0) as i64)
        }
    }

    /// Parse memory string to bytes (e.g., "1Gi" -> 1073741824, "512Mi" -> 536870912)
    fn parse_memory_to_bytes(&self, memory: &str) -> Result<i64> {
        let memory = memory.trim();

        if memory.ends_with("Gi") {
            let value = memory.trim_end_matches("Gi").parse::<f64>()?;
            Ok((value * 1024.0 * 1024.0 * 1024.0) as i64)
        } else if memory.ends_with("Mi") {
            let value = memory.trim_end_matches("Mi").parse::<f64>()?;
            Ok((value * 1024.0 * 1024.0) as i64)
        } else if memory.ends_with("Ki") {
            let value = memory.trim_end_matches("Ki").parse::<f64>()?;
            Ok((value * 1024.0) as i64)
        } else if memory.ends_with("G") {
            let value = memory.trim_end_matches("G").parse::<f64>()?;
            Ok((value * 1000.0 * 1000.0 * 1000.0) as i64)
        } else if memory.ends_with("M") {
            let value = memory.trim_end_matches("M").parse::<f64>()?;
            Ok((value * 1000.0 * 1000.0) as i64)
        } else if memory.ends_with("K") {
            let value = memory.trim_end_matches("K").parse::<f64>()?;
            Ok((value * 1000.0) as i64)
        } else {
            // Assume bytes
            Ok(memory.parse::<i64>()?)
        }
    }

    /// Convert bytes to human-readable memory string
    fn bytes_to_memory_string(&self, bytes: i64) -> String {
        const GI: i64 = 1024 * 1024 * 1024;
        const MI: i64 = 1024 * 1024;
        const KI: i64 = 1024;

        if bytes >= GI && bytes % GI == 0 {
            format!("{}Gi", bytes / GI)
        } else if bytes >= MI && bytes % MI == 0 {
            format!("{}Mi", bytes / MI)
        } else if bytes >= KI && bytes % KI == 0 {
            format!("{}Ki", bytes / KI)
        } else {
            format!("{}", bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_storage::memory::MemoryStorage;

    #[test]
    fn test_parse_cpu_to_millicores() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = ResourceQuotaController::new(storage);

        assert_eq!(controller.parse_cpu_to_millicores("1").unwrap(), 1000);
        assert_eq!(controller.parse_cpu_to_millicores("500m").unwrap(), 500);
        assert_eq!(controller.parse_cpu_to_millicores("0.5").unwrap(), 500);
        assert_eq!(controller.parse_cpu_to_millicores("2").unwrap(), 2000);
    }

    #[test]
    fn test_parse_memory_to_bytes() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = ResourceQuotaController::new(storage);

        assert_eq!(controller.parse_memory_to_bytes("1Gi").unwrap(), 1073741824);
        assert_eq!(
            controller.parse_memory_to_bytes("512Mi").unwrap(),
            536870912
        );
        assert_eq!(controller.parse_memory_to_bytes("1024Ki").unwrap(), 1048576);
        assert_eq!(controller.parse_memory_to_bytes("1000").unwrap(), 1000);
    }

    #[test]
    fn test_bytes_to_memory_string() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = ResourceQuotaController::new(storage);

        assert_eq!(controller.bytes_to_memory_string(1073741824), "1Gi");
        assert_eq!(controller.bytes_to_memory_string(536870912), "512Mi");
        assert_eq!(controller.bytes_to_memory_string(1048576), "1Mi"); // 1048576 = 1024 * 1024 = 1 MiB
        assert_eq!(controller.bytes_to_memory_string(1024), "1Ki"); // 1024 bytes = 1 KiB
        assert_eq!(controller.bytes_to_memory_string(1000), "1000");
    }
}
