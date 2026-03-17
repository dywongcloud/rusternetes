use anyhow::Result;
use rusternetes_common::resources::{
    EndpointAddress, EndpointPort, EndpointReference, EndpointSubset, Endpoints, Pod, Service,
};
use rusternetes_common::types::OwnerReference;
use rusternetes_storage::{build_key, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info};

/// EndpointsController watches Services and Pods to automatically maintain Endpoints resources.
/// It creates/updates Endpoints based on:
/// 1. Service selector matching pod labels
/// 2. Pod readiness status
/// 3. Pod IP assignment
pub struct EndpointsController<S: Storage> {
    storage: Arc<S>,
}

impl<S: Storage> EndpointsController<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    /// Main reconciliation loop - syncs all services with their endpoints
    pub async fn reconcile_all(&self) -> Result<()> {
        debug!("Starting endpoints reconciliation");

        // List all services across all namespaces
        let services: Vec<Service> = self.storage.list("/registry/services/").await?;

        for service in services {
            if let Err(e) = self.reconcile_service(&service).await {
                error!(
                    "Failed to reconcile endpoints for service {}/{}: {}",
                    service
                        .metadata
                        .namespace
                        .as_ref()
                        .unwrap_or(&"default".to_string()),
                    &service.metadata.name,
                    e
                );
            }
        }

        Ok(())
    }

    /// Reconcile endpoints for a single service
    async fn reconcile_service(&self, service: &Service) -> Result<()> {
        let namespace = service
            .metadata
            .namespace
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Service has no namespace"))?;
        let service_name = &service.metadata.name;

        debug!(
            "Reconciling endpoints for service {}/{}",
            namespace, service_name
        );

        // Skip services without selectors (headless services without selector)
        let selector = match &service.spec.selector {
            Some(s) if !s.is_empty() => s,
            _ => {
                debug!(
                    "Service {}/{} has no selector, skipping endpoint creation",
                    namespace, service_name
                );
                return Ok(());
            }
        };

        // Find all pods in the same namespace
        let pod_prefix = format!("/registry/pods/{}/", namespace);
        let all_pods: Vec<Pod> = self.storage.list(&pod_prefix).await?;

        // Filter pods that match the service selector
        let matching_pods: Vec<&Pod> = all_pods
            .iter()
            .filter(|pod| self.pod_matches_selector(pod, selector))
            .collect();

        debug!(
            "Found {} matching pods for service {}/{}",
            matching_pods.len(),
            namespace,
            service_name
        );

        // Build endpoint subsets from matching pods
        let subsets = self.build_endpoint_subsets(&matching_pods, &service.spec.ports);

        // Create or update endpoints
        let endpoints = Endpoints {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "Endpoints".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: rusternetes_common::types::ObjectMeta {
                name: service_name.clone(),
                generate_name: None,
                generation: None,
                managed_fields: None,
                namespace: Some(namespace.clone()),
                uid: String::new(),
                resource_version: None,
                deletion_grace_period_seconds: None,
                finalizers: None,
                owner_references: Some(vec![OwnerReference {
                    api_version: "v1".to_string(),
                    kind: "Service".to_string(),
                    name: service_name.clone(),
                    uid: service.metadata.uid.clone(),
                    controller: Some(true),
                    block_owner_deletion: Some(true),
                }]),
                creation_timestamp: None,
                deletion_timestamp: None,
                labels: service.metadata.labels.clone(),
                annotations: service.metadata.annotations.clone(),
            },
            subsets,
        };

        let endpoints_key = build_key("endpoints", Some(namespace), service_name);
        // Try to update first, if it doesn't exist, create it
        match self.storage.update(&endpoints_key, &endpoints).await {
            Ok(_) => {}
            Err(rusternetes_common::Error::NotFound(_)) => {
                self.storage.create(&endpoints_key, &endpoints).await?;
            }
            Err(e) => return Err(e.into()),
        }

        info!(
            "Updated endpoints for service {}/{} with {} subsets",
            namespace,
            service_name,
            endpoints.subsets.len()
        );

        Ok(())
    }

    /// Check if a pod matches the service selector
    fn pod_matches_selector(&self, pod: &Pod, selector: &HashMap<String, String>) -> bool {
        let pod_labels = match &pod.metadata.labels {
            Some(labels) => labels,
            None => return false,
        };

        // All selector labels must match pod labels
        selector
            .iter()
            .all(|(key, value)| pod_labels.get(key).map(|v| v == value).unwrap_or(false))
    }

    /// Build endpoint subsets from pods, separating ready and not-ready pods
    fn build_endpoint_subsets(
        &self,
        pods: &[&Pod],
        service_ports: &[rusternetes_common::resources::ServicePort],
    ) -> Vec<EndpointSubset> {
        // Separate pods by readiness
        let mut ready_addresses = Vec::new();
        let mut not_ready_addresses = Vec::new();

        for pod in pods {
            // Skip pods without an IP address
            let pod_ip = match &pod.status {
                Some(status) => match &status.pod_ip {
                    Some(ip) if !ip.is_empty() => ip.clone(),
                    _ => {
                        debug!(
                            "Pod {}/{} has no IP, skipping",
                            pod.metadata
                                .namespace
                                .as_ref()
                                .unwrap_or(&"default".to_string()),
                            &pod.metadata.name
                        );
                        continue;
                    }
                },
                None => {
                    debug!(
                        "Pod {}/{} has no status, skipping",
                        pod.metadata
                            .namespace
                            .as_ref()
                            .unwrap_or(&"default".to_string()),
                        &pod.metadata.name
                    );
                    continue;
                }
            };

            let address = EndpointAddress {
                ip: pod_ip,
                hostname: None,
                node_name: pod.spec.as_ref().and_then(|s| s.node_name.clone()),
                target_ref: Some(EndpointReference {
                    kind: Some("Pod".to_string()),
                    namespace: pod.metadata.namespace.clone(),
                    name: Some(pod.metadata.name.clone()),
                    uid: Some(pod.metadata.uid.clone()),
                }),
            };

            // Check if pod is ready
            if self.is_pod_ready(pod) {
                ready_addresses.push(address);
            } else {
                not_ready_addresses.push(address);
            }
        }

        // Convert service ports to endpoint ports
        let endpoint_ports: Vec<EndpointPort> = service_ports
            .iter()
            .map(|sp| EndpointPort {
                name: sp.name.clone(),
                port: sp.target_port.unwrap_or(sp.port),
                protocol: sp.protocol.clone(),
                app_protocol: None,
            })
            .collect();

        // Create a single subset with all addresses and ports
        if ready_addresses.is_empty() && not_ready_addresses.is_empty() {
            vec![]
        } else {
            vec![EndpointSubset {
                addresses: if ready_addresses.is_empty() {
                    None
                } else {
                    Some(ready_addresses)
                },
                not_ready_addresses: if not_ready_addresses.is_empty() {
                    None
                } else {
                    Some(not_ready_addresses)
                },
                ports: if endpoint_ports.is_empty() {
                    None
                } else {
                    Some(endpoint_ports)
                },
            }]
        }
    }

    /// Check if a pod is ready based on its status
    fn is_pod_ready(&self, pod: &Pod) -> bool {
        let status = match &pod.status {
            Some(s) => s,
            None => return false,
        };

        // Pod must be in Running phase
        if status.phase != Some(rusternetes_common::types::Phase::Running) {
            return false;
        }

        // Check container statuses - all containers must be ready
        if let Some(container_statuses) = &status.container_statuses {
            container_statuses.iter().all(|cs| cs.ready)
        } else {
            // If no container statuses, assume not ready
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_storage::MemoryStorage;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_pod_matches_selector() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = EndpointsController { storage };

        let mut pod_labels = HashMap::new();
        pod_labels.insert("app".to_string(), "nginx".to_string());
        pod_labels.insert("tier".to_string(), "frontend".to_string());

        let pod = Pod {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "Pod".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: rusternetes_common::types::ObjectMeta {
                name: "test-pod".to_string(),
                namespace: Some("default".to_string()),
                uid: String::new(),
                resource_version: None,
                deletion_grace_period_seconds: None,
                finalizers: None,
                owner_references: None,
                creation_timestamp: None,
                deletion_timestamp: None,
                labels: Some(pod_labels),
                annotations: None,
                generate_name: None,
                generation: None,
                managed_fields: None,
            },
            spec: Some(rusternetes_common::resources::PodSpec {
                init_containers: None,
                containers: vec![],
                volumes: None,
                restart_policy: None,
                node_name: None,
                node_selector: None,
                service_account_name: None,
                hostname: None,
                subdomain: None,
                host_network: None,
                host_pid: None,
                host_ipc: None,
                affinity: None,
                tolerations: None,
                priority: None,
                priority_class_name: None,
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

        // Test exact match
        let mut selector = HashMap::new();
        selector.insert("app".to_string(), "nginx".to_string());
        assert!(controller.pod_matches_selector(&pod, &selector));

        // Test multiple labels
        selector.insert("tier".to_string(), "frontend".to_string());
        assert!(controller.pod_matches_selector(&pod, &selector));

        // Test mismatch
        selector.insert("app".to_string(), "apache".to_string());
        assert!(!controller.pod_matches_selector(&pod, &selector));

        // Test missing label
        selector.clear();
        selector.insert("nonexistent".to_string(), "value".to_string());
        assert!(!controller.pod_matches_selector(&pod, &selector));
    }

    #[tokio::test]
    async fn test_is_pod_ready() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = EndpointsController { storage };

        // Pod without status
        let pod_no_status = Pod {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "Pod".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: rusternetes_common::types::ObjectMeta {
                name: "test-pod".to_string(),
                namespace: None,
                uid: String::new(),
                resource_version: None,
                deletion_grace_period_seconds: None,
                finalizers: None,
                owner_references: None,
                creation_timestamp: None,
                deletion_timestamp: None,
                labels: None,
                annotations: None,
                generate_name: None,
                generation: None,
                managed_fields: None,
            },
            spec: Some(rusternetes_common::resources::PodSpec {
                init_containers: None,
                containers: vec![],
                volumes: None,
                restart_policy: None,
                node_name: None,
                node_selector: None,
                service_account_name: None,
                hostname: None,
                subdomain: None,
                host_network: None,
                host_pid: None,
                host_ipc: None,
                affinity: None,
                tolerations: None,
                priority: None,
                priority_class_name: None,
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
        assert!(!controller.is_pod_ready(&pod_no_status));

        // Pod in Pending phase
        let pod_pending = Pod {
            status: Some(rusternetes_common::resources::PodStatus {
                phase: Some(rusternetes_common::types::Phase::Pending),
                message: None,
                reason: None,
                host_ip: None,
                pod_ip: None,
                conditions: None,
                container_statuses: None,
                init_container_statuses: None,
                ephemeral_container_statuses: None,
            }),
            ..pod_no_status.clone()
        };
        assert!(!controller.is_pod_ready(&pod_pending));

        // Pod in Running phase with ready container
        let pod_ready = Pod {
            status: Some(rusternetes_common::resources::PodStatus {
                phase: Some(rusternetes_common::types::Phase::Running),
                message: None,
                reason: None,
                host_ip: None,
                pod_ip: None,
                conditions: None,
                container_statuses: Some(vec![rusternetes_common::resources::ContainerStatus {
                    name: "nginx".to_string(),
                    ready: true,
                    restart_count: 0,
                    state: None,
                    image: Some("nginx:latest".to_string()),
                    container_id: Some("container-123".to_string()),
                }]),
                init_container_statuses: None,
                ephemeral_container_statuses: None,
            }),
            ..pod_no_status.clone()
        };
        assert!(controller.is_pod_ready(&pod_ready));

        // Pod in Running phase with not-ready container
        let pod_not_ready = Pod {
            status: Some(rusternetes_common::resources::PodStatus {
                phase: Some(rusternetes_common::types::Phase::Running),
                message: None,
                reason: None,
                host_ip: None,
                pod_ip: None,
                conditions: None,
                container_statuses: Some(vec![rusternetes_common::resources::ContainerStatus {
                    name: "nginx".to_string(),
                    ready: false,
                    restart_count: 0,
                    state: None,
                    image: Some("nginx:latest".to_string()),
                    container_id: Some("container-123".to_string()),
                }]),
                init_container_statuses: None,
                ephemeral_container_statuses: None,
            }),
            ..pod_no_status
        };
        assert!(!controller.is_pod_ready(&pod_not_ready));
    }
}
