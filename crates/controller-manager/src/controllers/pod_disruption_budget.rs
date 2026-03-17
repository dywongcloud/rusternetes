use rusternetes_common::resources::{
    IntOrString, Pod, PodDisruptionBudget, PodDisruptionBudgetStatus,
};
use rusternetes_common::types::LabelSelector;
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

pub struct PodDisruptionBudgetController<S: Storage> {
    storage: Arc<S>,
}

impl<S: Storage> PodDisruptionBudgetController<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    pub async fn run(&self) -> rusternetes_common::Result<()> {
        info!("Starting PodDisruptionBudget controller");

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;

            if let Err(e) = self.reconcile_all().await {
                error!("Error reconciling PodDisruptionBudgets: {}", e);
            }
        }
    }

    pub async fn reconcile_all(&self) -> rusternetes_common::Result<()> {
        debug!("Reconciling all PodDisruptionBudgets");

        // Get all PDBs
        let prefix = build_prefix("poddisruptionbudgets", None);
        let pdbs: Vec<PodDisruptionBudget> = self.storage.list(&prefix).await?;

        for pdb in pdbs {
            if let Err(e) = self.reconcile_pdb(&pdb).await {
                warn!("Failed to reconcile PDB {}: {}", pdb.metadata.name, e);
            }
        }

        Ok(())
    }

    async fn reconcile_pdb(&self, pdb: &PodDisruptionBudget) -> rusternetes_common::Result<()> {
        let namespace = pdb.metadata.namespace.as_deref().unwrap_or("default");

        info!(
            "Reconciling PodDisruptionBudget: {}/{}",
            namespace, pdb.metadata.name
        );

        // 1. Find all pods matching the selector in the PDB's namespace
        let pods_prefix = build_prefix("pods", Some(namespace));
        let all_pods: Vec<Pod> = self.storage.list(&pods_prefix).await?;

        // 2. Filter pods that match the PDB selector
        let matching_pods: Vec<Pod> = all_pods
            .into_iter()
            .filter(|p| self.pod_matches_selector(p, &pdb.spec.selector))
            .collect();

        // 3. Count healthy vs unhealthy pods
        let total_pods = matching_pods.len() as i32;
        let healthy_pods = matching_pods
            .iter()
            .filter(|p| self.is_pod_healthy(p))
            .count() as i32;

        debug!(
            "PDB {}/{}: total={}, healthy={}",
            namespace, pdb.metadata.name, total_pods, healthy_pods
        );

        // 4. Calculate desired_healthy based on min_available or max_unavailable
        let desired_healthy = self.calculate_desired_healthy(&pdb, total_pods)?;

        // 5. Calculate disruptions_allowed
        // disruptions_allowed = current_healthy - desired_healthy
        let disruptions_allowed = healthy_pods - desired_healthy;

        info!(
            "PDB {}/{}: desired_healthy={}, disruptions_allowed={}",
            namespace, pdb.metadata.name, desired_healthy, disruptions_allowed
        );

        // 6. Update PDB status
        let mut updated_pdb = pdb.clone();
        updated_pdb.status = Some(PodDisruptionBudgetStatus {
            current_healthy: healthy_pods,
            desired_healthy,
            disruptions_allowed,
            expected_pods: total_pods,
            observed_generation: None, // Generation tracking would need to be added to ObjectMeta
            conditions: None,
        });

        // Save updated PDB back to storage
        let key = build_key("poddisruptionbudgets", Some(namespace), &pdb.metadata.name);
        self.storage.update(&key, &updated_pdb).await?;

        Ok(())
    }

    /// Calculate desired_healthy based on min_available or max_unavailable
    fn calculate_desired_healthy(
        &self,
        pdb: &PodDisruptionBudget,
        total_pods: i32,
    ) -> rusternetes_common::Result<i32> {
        if let Some(ref min_available) = pdb.spec.min_available {
            // Use min_available (either int or percentage)
            match min_available {
                IntOrString::Int(value) => Ok(*value),
                IntOrString::String(s) => {
                    // Parse percentage (e.g., "50%")
                    if let Some(stripped) = s.strip_suffix('%') {
                        let percentage: f64 = stripped.parse().map_err(|_| {
                            rusternetes_common::Error::InvalidResource(format!(
                                "Invalid percentage in minAvailable: {}",
                                s
                            ))
                        })?;
                        let desired = ((total_pods as f64) * (percentage / 100.0)).ceil() as i32;
                        Ok(desired)
                    } else {
                        Err(rusternetes_common::Error::InvalidResource(format!(
                            "Invalid minAvailable string format: {}",
                            s
                        )))
                    }
                }
            }
        } else if let Some(ref max_unavailable) = pdb.spec.max_unavailable {
            // Use max_unavailable (either int or percentage)
            let max_unavailable_count = match max_unavailable {
                IntOrString::Int(value) => *value,
                IntOrString::String(s) => {
                    // Parse percentage (e.g., "20%")
                    if let Some(stripped) = s.strip_suffix('%') {
                        let percentage: f64 = stripped.parse().map_err(|_| {
                            rusternetes_common::Error::InvalidResource(format!(
                                "Invalid percentage in maxUnavailable: {}",
                                s
                            ))
                        })?;
                        ((total_pods as f64) * (percentage / 100.0)).floor() as i32
                    } else {
                        return Err(rusternetes_common::Error::InvalidResource(format!(
                            "Invalid maxUnavailable string format: {}",
                            s
                        )));
                    }
                }
            };
            // desired_healthy = total - max_unavailable
            Ok(total_pods - max_unavailable_count)
        } else {
            // No min_available or max_unavailable specified - invalid PDB
            Err(rusternetes_common::Error::InvalidResource(
                "PodDisruptionBudget must specify either minAvailable or maxUnavailable"
                    .to_string(),
            ))
        }
    }

    /// Check if a pod is healthy (Running and Ready)
    fn is_pod_healthy(&self, pod: &Pod) -> bool {
        // Check if pod is in Running phase
        let is_running = pod
            .status
            .as_ref()
            .map(|s| matches!(s.phase, Some(rusternetes_common::types::Phase::Running)))
            .unwrap_or(false);

        if !is_running {
            return false;
        }

        // Check if pod has Ready condition set to True
        // For simplicity, we'll consider a pod ready if it's Running
        // In a full implementation, we'd check pod.status.conditions for Ready=True
        true
    }

    /// Check if a pod matches the PDB selector
    fn pod_matches_selector(&self, pod: &Pod, selector: &LabelSelector) -> bool {
        let pod_labels = match &pod.metadata.labels {
            Some(labels) => labels,
            None => return false,
        };

        // Check match_labels
        if let Some(match_labels) = &selector.match_labels {
            for (key, value) in match_labels {
                if pod_labels.get(key) != Some(value) {
                    return false;
                }
            }
        }

        // TODO: Implement match_expressions support
        // For now, if there are match_expressions, we skip them
        if selector.match_expressions.is_some() {
            debug!("match_expressions not yet implemented for PDB selector matching");
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::resources::{Container, IntOrString, PodDisruptionBudgetSpec, PodSpec};
    use rusternetes_common::types::{ObjectMeta, Phase, TypeMeta};
    use rusternetes_storage::MemoryStorage;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_calculate_desired_healthy_min_available_int() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = PodDisruptionBudgetController::new(storage);

        let spec = PodDisruptionBudgetSpec {
            min_available: Some(IntOrString::Int(3)),
            max_unavailable: None,
            selector: LabelSelector {
                match_labels: Some(HashMap::new()),
                match_expressions: None,
            },
            unhealthy_pod_eviction_policy: None,
        };

        let pdb = PodDisruptionBudget::new("test-pdb", "default", spec);
        let desired = controller.calculate_desired_healthy(&pdb, 5).unwrap();
        assert_eq!(desired, 3);
    }

    #[tokio::test]
    async fn test_calculate_desired_healthy_min_available_percentage() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = PodDisruptionBudgetController::new(storage);

        let spec = PodDisruptionBudgetSpec {
            min_available: Some(IntOrString::String("50%".to_string())),
            max_unavailable: None,
            selector: LabelSelector {
                match_labels: Some(HashMap::new()),
                match_expressions: None,
            },
            unhealthy_pod_eviction_policy: None,
        };

        let pdb = PodDisruptionBudget::new("test-pdb", "default", spec);
        let desired = controller.calculate_desired_healthy(&pdb, 10).unwrap();
        assert_eq!(desired, 5); // 50% of 10 = 5
    }

    #[tokio::test]
    async fn test_calculate_desired_healthy_max_unavailable_int() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = PodDisruptionBudgetController::new(storage);

        let spec = PodDisruptionBudgetSpec {
            min_available: None,
            max_unavailable: Some(IntOrString::Int(2)),
            selector: LabelSelector {
                match_labels: Some(HashMap::new()),
                match_expressions: None,
            },
            unhealthy_pod_eviction_policy: None,
        };

        let pdb = PodDisruptionBudget::new("test-pdb", "default", spec);
        let desired = controller.calculate_desired_healthy(&pdb, 5).unwrap();
        assert_eq!(desired, 3); // 5 - 2 = 3
    }

    #[tokio::test]
    async fn test_pod_matches_selector() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = PodDisruptionBudgetController::new(storage);

        let mut pod = Pod {
            type_meta: TypeMeta {
                kind: "Pod".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: ObjectMeta::new("test-pod"),
            spec: Some(PodSpec {
                containers: vec![Container {
                    name: "test".to_string(),
                    image: "nginx".to_string(),
                    image_pull_policy: None,
                    command: None,
                    args: None,
                    ports: None,
                    env: None,
                    volume_mounts: None,
                    liveness_probe: None,
                    readiness_probe: None,
                    startup_probe: None,
                    resources: None,
                    working_dir: None,
                    security_context: None,
                    restart_policy: None,
                }],
                init_containers: None,
                restart_policy: None,
                node_selector: None,
                node_name: None,
                volumes: None,
                affinity: None,
                tolerations: None,
                service_account_name: None,
                priority: None,
                priority_class_name: None,
                hostname: None,
                subdomain: None,
                host_network: None,
                host_pid: None,
                host_ipc: None,
                automount_service_account_token: None,
                ephemeral_containers: None,
                overhead: None,
                scheduler_name: None,
                topology_spread_constraints: None,
                resource_claims: None,
                active_deadline_seconds: None,
                dns_policy: None,
                dns_config: None,
                security_context: None,
                image_pull_secrets: None,
                share_process_namespace: None,
                readiness_gates: None,
                runtime_class_name: None,
                enable_service_links: None,
                preemption_policy: None,
                host_users: None,
                set_hostname_as_fqdn: None,
                termination_grace_period_seconds: None,
            }),
            status: None,
        };

        pod.metadata.labels = Some(HashMap::from([
            ("app".to_string(), "web".to_string()),
            ("tier".to_string(), "frontend".to_string()),
        ]));

        let selector = LabelSelector {
            match_labels: Some(HashMap::from([("app".to_string(), "web".to_string())])),
            match_expressions: None,
        };

        assert!(controller.pod_matches_selector(&pod, &selector));

        let selector_no_match = LabelSelector {
            match_labels: Some(HashMap::from([("app".to_string(), "api".to_string())])),
            match_expressions: None,
        };

        assert!(!controller.pod_matches_selector(&pod, &selector_no_match));
    }

    #[tokio::test]
    async fn test_is_pod_healthy() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = PodDisruptionBudgetController::new(storage);

        let mut pod = Pod {
            type_meta: TypeMeta {
                kind: "Pod".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: ObjectMeta::new("test-pod"),
            spec: Some(PodSpec {
                containers: vec![Container {
                    name: "test".to_string(),
                    image: "nginx".to_string(),
                    image_pull_policy: None,
                    command: None,
                    args: None,
                    ports: None,
                    env: None,
                    volume_mounts: None,
                    liveness_probe: None,
                    readiness_probe: None,
                    startup_probe: None,
                    resources: None,
                    working_dir: None,
                    security_context: None,
                    restart_policy: None,
                }],
                init_containers: None,
                restart_policy: None,
                node_selector: None,
                node_name: None,
                volumes: None,
                affinity: None,
                tolerations: None,
                service_account_name: None,
                priority: None,
                priority_class_name: None,
                hostname: None,
                subdomain: None,
                host_network: None,
                host_pid: None,
                host_ipc: None,
                automount_service_account_token: None,
                ephemeral_containers: None,
                overhead: None,
                scheduler_name: None,
                topology_spread_constraints: None,
                resource_claims: None,
                active_deadline_seconds: None,
                dns_policy: None,
                dns_config: None,
                security_context: None,
                image_pull_secrets: None,
                share_process_namespace: None,
                readiness_gates: None,
                runtime_class_name: None,
                enable_service_links: None,
                preemption_policy: None,
                host_users: None,
                set_hostname_as_fqdn: None,
                termination_grace_period_seconds: None,
            }),
            status: Some(rusternetes_common::resources::PodStatus {
                phase: Some(Phase::Running),
                message: None,
                reason: None,
                host_ip: None,
                pod_ip: None,
                conditions: None,
                container_statuses: None,
                init_container_statuses: None,
                ephemeral_container_statuses: None,
            }),
        };

        assert!(controller.is_pod_healthy(&pod));

        // Test with Pending pod
        if let Some(ref mut status) = pod.status {
            status.phase = Some(Phase::Pending);
        }
        assert!(!controller.is_pod_healthy(&pod));
    }
}
