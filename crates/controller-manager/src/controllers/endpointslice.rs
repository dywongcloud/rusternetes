use anyhow::Result;
use rusternetes_common::resources::endpointslice::{
    Endpoint, EndpointConditions, EndpointPort, EndpointReference,
};
use rusternetes_common::resources::{EndpointSlice, Pod, Service};
use rusternetes_common::types::Phase;
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info};

/// EndpointSliceController builds EndpointSlices directly from Services and Pods,
/// following the same approach as the K8s endpointslice controller.
///
/// Key behavior (matching K8s):
/// - Iterates over Services with selectors
/// - Finds matching pods by label selector
/// - For each pod, computes which service ports the pod actually serves
///   using FindPort logic (matching containerPort to service targetPort)
/// - Groups pods by their port mapping
/// - Creates separate EndpointSlices for each port group
///
/// This ensures that pods only appear in EndpointSlices with the ports
/// they actually serve, fixing the conformance test failure where pods
/// were incorrectly associated with all service ports.
pub struct EndpointSliceController<S: Storage> {
    storage: Arc<S>,
}

impl<S: Storage> EndpointSliceController<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    /// Main reconciliation loop — syncs EndpointSlices for all Services
    pub async fn reconcile_all(&self) -> Result<()> {
        debug!("Starting endpointslice reconciliation");

        // List all services across all namespaces
        let services: Vec<Service> = self
            .storage
            .list(&build_prefix("services", None))
            .await
            .unwrap_or_default();

        // Track which service names we've seen for orphan cleanup
        let mut service_names: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();

        for service in &services {
            let ns = service.metadata.namespace.as_deref().unwrap_or("default");
            service_names.insert((ns.to_string(), service.metadata.name.clone()));

            if let Err(e) = self.reconcile_service(service).await {
                error!(
                    "Failed to reconcile endpointslices for service {}/{}: {}",
                    ns, service.metadata.name, e
                );
            }
        }

        // Clean up orphaned EndpointSlices (whose Service no longer exists)
        let all_slices: Vec<EndpointSlice> = self
            .storage
            .list(&build_prefix("endpointslices", None))
            .await
            .unwrap_or_default();

        for slice in all_slices {
            let ns = slice.metadata.namespace.as_deref().unwrap_or("default");
            let svc_name = slice
                .metadata
                .labels
                .as_ref()
                .and_then(|l| l.get("kubernetes.io/service-name"))
                .map(|s| s.as_str());

            if let Some(svc_name) = svc_name {
                if !service_names.contains(&(ns.to_string(), svc_name.to_string())) {
                    // Only delete slices managed by our controller
                    let is_managed = slice
                        .metadata
                        .labels
                        .as_ref()
                        .and_then(|l| l.get("endpointslice.kubernetes.io/managed-by"))
                        .map(|v| v == "endpointslice-controller.k8s.io")
                        .unwrap_or(false);
                    if is_managed {
                        let key = build_key("endpointslices", Some(ns), &slice.metadata.name);
                        debug!(
                            "Deleting orphaned EndpointSlice {}/{}",
                            ns, slice.metadata.name
                        );
                        let _ = self.storage.delete(&key).await;
                    }
                }
            }
        }

        Ok(())
    }

    /// Reconcile EndpointSlices for a single Service
    async fn reconcile_service(&self, service: &Service) -> Result<()> {
        let namespace = service.metadata.namespace.as_deref().unwrap_or("default");
        let service_name = &service.metadata.name;

        // Skip services without selectors (ExternalName, headless without selector)
        let selector = match &service.spec.selector {
            Some(s) if !s.is_empty() => s,
            _ => return Ok(()),
        };

        // Skip ExternalName services
        if matches!(
            service.spec.service_type,
            Some(rusternetes_common::resources::ServiceType::ExternalName)
        ) {
            return Ok(());
        }

        debug!(
            "Reconciling endpointslices for service {}/{}",
            namespace, service_name
        );

        // Find all pods matching the service's label selector
        let all_pods: Vec<Pod> = self
            .storage
            .list(&build_prefix("pods", Some(namespace)))
            .await
            .unwrap_or_default();

        let matching_pods: Vec<&Pod> = all_pods
            .iter()
            .filter(|pod| {
                // Match label selector
                if let Some(pod_labels) = &pod.metadata.labels {
                    selector.iter().all(|(k, v)| pod_labels.get(k) == Some(v))
                } else {
                    false
                }
            })
            .filter(|pod| {
                // Skip pods that shouldn't be in endpoints
                // (terminated, not yet assigned to a node)
                let phase = pod.status.as_ref().and_then(|s| s.phase.as_ref());
                !matches!(phase, Some(Phase::Succeeded) | Some(Phase::Failed))
                    && pod.metadata.deletion_timestamp.is_none()
            })
            .collect();

        // Group pods by their resolved port mapping.
        // Each group gets its own EndpointSlice with only the ports those pods serve.
        // This follows K8s's reconcileByAddressType / getEndpointPorts pattern.
        // Use (serialized ports, ports) as key since EndpointPort doesn't implement Hash.
        let mut port_groups: HashMap<String, (Vec<EndpointPort>, Vec<Endpoint>)> = HashMap::new();

        for pod in &matching_pods {
            let endpoint_ports = self.get_endpoint_ports(service, pod);
            if endpoint_ports.is_empty() {
                continue; // Pod doesn't serve any of the service's ports
            }

            let pod_ip = pod
                .status
                .as_ref()
                .and_then(|s| s.pod_ip.as_ref())
                .filter(|ip| !ip.is_empty());

            let Some(ip) = pod_ip else {
                continue; // Pod has no IP
            };

            let is_ready = pod
                .status
                .as_ref()
                .and_then(|s| s.conditions.as_ref())
                .map(|conditions| {
                    conditions
                        .iter()
                        .any(|c| c.condition_type == "Ready" && c.status == "True")
                })
                .unwrap_or(false);

            let endpoint = Endpoint {
                addresses: vec![ip.clone()],
                conditions: Some(EndpointConditions {
                    ready: Some(is_ready),
                    serving: Some(is_ready),
                    terminating: Some(false),
                }),
                hostname: pod.spec.as_ref().and_then(|s| {
                    if s.subdomain.is_some() {
                        s.hostname.clone().or(Some(pod.metadata.name.clone()))
                    } else {
                        None
                    }
                }),
                target_ref: Some(EndpointReference {
                    kind: Some("Pod".to_string()),
                    namespace: pod.metadata.namespace.clone(),
                    name: Some(pod.metadata.name.clone()),
                    uid: Some(pod.metadata.uid.clone()),
                    resource_version: None,
                    field_path: None,
                }),
                node_name: pod.spec.as_ref().and_then(|s| s.node_name.clone()),
                zone: None,
                hints: None,
                deprecated_topology: None,
            };

            let port_key = serde_json::to_string(&endpoint_ports).unwrap_or_default();
            port_groups
                .entry(port_key)
                .or_insert_with(|| (endpoint_ports, Vec::new()))
                .1
                .push(endpoint);
        }

        // If no port groups were created but the service has ports,
        // create an empty slice with the service's ports
        if port_groups.is_empty() && !service.spec.ports.is_empty() {
            let ports: Vec<EndpointPort> = service
                .spec
                .ports
                .iter()
                .map(|sp| EndpointPort {
                    name: sp.name.clone(),
                    port: Some(sp.port as i32),
                    protocol: sp.protocol.clone(),
                    app_protocol: sp.app_protocol.clone(),
                })
                .collect();
            let port_key = serde_json::to_string(&ports).unwrap_or_default();
            port_groups.insert(port_key, (ports, Vec::new()));
        }

        // Create/update EndpointSlices for each port group
        for (idx, (_key, (ports, endpoints))) in port_groups.into_iter().enumerate() {
            let slice_name = if idx == 0 {
                service_name.clone()
            } else {
                format!("{}-{}", service_name, idx)
            };

            let mut slice = EndpointSlice::new(&slice_name, "IPv4");
            slice.metadata.namespace = Some(namespace.to_string());
            slice.ports = ports;
            slice.endpoints = endpoints;

            // Set labels
            let labels = slice.metadata.labels.get_or_insert_with(Default::default);
            labels.insert(
                "kubernetes.io/service-name".to_string(),
                service_name.clone(),
            );
            labels.insert(
                "endpointslice.kubernetes.io/managed-by".to_string(),
                "endpointslice-controller.k8s.io".to_string(),
            );

            // Set owner reference to the Service
            slice.metadata.owner_references =
                Some(vec![rusternetes_common::types::OwnerReference {
                    api_version: "v1".to_string(),
                    kind: "Service".to_string(),
                    name: service_name.clone(),
                    uid: service.metadata.uid.clone(),
                    controller: Some(true),
                    block_owner_deletion: Some(true),
                }]);

            let slice_key = build_key("endpointslices", Some(namespace), &slice_name);

            // Check if existing slice matches — skip write if nothing changed
            if let Ok(existing) = self.storage.get::<EndpointSlice>(&slice_key).await {
                if existing.endpoints == slice.endpoints && existing.ports == slice.ports {
                    continue;
                }
                slice.metadata.resource_version = existing.metadata.resource_version;
            }

            match self.storage.update(&slice_key, &slice).await {
                Ok(_) => {
                    debug!(
                        "Updated endpointslice {}/{} for service",
                        namespace, slice_name
                    );
                }
                Err(rusternetes_common::Error::NotFound(_)) => {
                    self.storage.create(&slice_key, &slice).await?;
                    info!(
                        "Created endpointslice {}/{} for service",
                        namespace, slice_name
                    );
                }
                Err(e) => return Err(e.into()),
            }
        }

        Ok(())
    }

    /// Compute endpoint ports for a pod based on the service's port definitions.
    /// Follows K8s's getEndpointPorts() logic:
    /// - For each service port, try to find a matching containerPort on the pod
    /// - If the service uses a named targetPort, look up the container port by name
    /// - If the service uses a numeric targetPort, use it directly
    /// - If no match is found, skip that port (don't include it)
    fn get_endpoint_ports(&self, service: &Service, pod: &Pod) -> Vec<EndpointPort> {
        let mut endpoint_ports = Vec::new();

        for sp in &service.spec.ports {
            let port_num = match self.find_port(pod, sp) {
                Some(p) => p,
                None => continue, // Pod doesn't serve this port
            };

            endpoint_ports.push(EndpointPort {
                name: sp.name.clone(),
                port: Some(port_num),
                protocol: sp.protocol.clone(),
                app_protocol: sp.app_protocol.clone(),
            });
        }

        endpoint_ports
    }

    /// Find the container port on a pod that corresponds to a service port.
    /// Implements K8s's FindPort logic:
    /// - If targetPort is a string (named port), search pod containers for a
    ///   containerPort with that name
    /// - If targetPort is a number, use it directly
    /// - If targetPort is not set, use the service port number
    fn find_port(
        &self,
        pod: &Pod,
        service_port: &rusternetes_common::resources::ServicePort,
    ) -> Option<i32> {
        match &service_port.target_port {
            Some(rusternetes_common::resources::IntOrString::String(name)) => {
                // Named port — search pod containers
                if let Ok(port_num) = name.parse::<i32>() {
                    // It's actually a numeric string
                    return Some(port_num);
                }
                // Look up named port in pod containers
                if let Some(spec) = &pod.spec {
                    for container in &spec.containers {
                        if let Some(ports) = &container.ports {
                            for cp in ports {
                                if cp.name.as_deref() == Some(name.as_str()) {
                                    return Some(cp.container_port as i32);
                                }
                            }
                        }
                    }
                }
                None // Pod doesn't have this named port
            }
            Some(rusternetes_common::resources::IntOrString::Int(port)) => Some(*port),
            None => Some(service_port.port as i32), // Default to service port
        }
    }

    /// Clean up orphaned EndpointSlices
    pub async fn cleanup_orphans(&self) -> Result<()> {
        // Handled in reconcile_all
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::resources::{ContainerPort, ServicePort, ServiceSpec};
    use rusternetes_common::types::ObjectMeta;
    use rusternetes_storage::MemoryStorage;

    #[tokio::test]
    async fn test_endpointslice_controller_creation() {
        let storage = Arc::new(MemoryStorage::new());
        let _controller = EndpointSliceController::new(storage);
    }

    #[tokio::test]
    async fn test_pod_port_filtering_named_ports() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = EndpointSliceController::new(Arc::clone(&storage));

        // Create a service with two named target ports
        let service = Service {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "Service".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: ObjectMeta::new("test-svc").with_namespace("default"),
            spec: ServiceSpec {
                ports: vec![
                    ServicePort {
                        name: Some("portname1".to_string()),
                        port: 80,
                        target_port: Some(rusternetes_common::resources::IntOrString::String(
                            "svc1".to_string(),
                        )),
                        protocol: Some("TCP".to_string()),
                        node_port: None,
                        app_protocol: None,
                    },
                    ServicePort {
                        name: Some("portname2".to_string()),
                        port: 81,
                        target_port: Some(rusternetes_common::resources::IntOrString::String(
                            "svc2".to_string(),
                        )),
                        protocol: Some("TCP".to_string()),
                        node_port: None,
                        app_protocol: None,
                    },
                ],
                selector: Some(HashMap::from([("app".to_string(), "test".to_string())])),
                ..Default::default()
            },
            status: None,
        };

        // Pod1 only has containerPort named "svc1" (serves portname1 only)
        let pod1 = Pod {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "Pod".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: ObjectMeta::new("pod1")
                .with_namespace("default")
                .with_labels(HashMap::from([("app".to_string(), "test".to_string())])),
            spec: Some(rusternetes_common::resources::PodSpec {
                containers: vec![rusternetes_common::resources::Container {
                    name: "c1".to_string(),
                    ports: Some(vec![ContainerPort {
                        container_port: 100,
                        name: Some("svc1".to_string()),
                        protocol: None,
                        host_port: None,
                        host_ip: None,
                    }]),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            status: None,
        };

        // Pod1 should only get portname1 (svc1→100), NOT portname2 (svc2 not found)
        let ports = controller.get_endpoint_ports(&service, &pod1);
        assert_eq!(ports.len(), 1, "Pod1 should only match portname1");
        assert_eq!(ports[0].name, Some("portname1".to_string()));
        assert_eq!(ports[0].port, Some(100));
    }

    #[tokio::test]
    async fn test_orphan_detection_skips_externally_managed_slices() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = EndpointSliceController::new(Arc::clone(&storage));

        // Create an EndpointSlice NOT managed by the controller
        let mut external_slice = EndpointSlice::new("external-slice", "IPv4");
        external_slice.metadata.namespace = Some("default".to_string());
        let key = build_key("endpointslices", Some("default"), "external-slice");
        storage.create(&key, &external_slice).await.unwrap();

        // Create an EndpointSlice managed by the controller
        let mut managed_slice = EndpointSlice::new("managed-slice", "IPv4");
        managed_slice.metadata.namespace = Some("default".to_string());
        let mut labels = HashMap::new();
        labels.insert(
            "endpointslice.kubernetes.io/managed-by".to_string(),
            "endpointslice-controller.k8s.io".to_string(),
        );
        labels.insert(
            "kubernetes.io/service-name".to_string(),
            "nonexistent-service".to_string(),
        );
        managed_slice.metadata.labels = Some(labels);
        let key2 = build_key("endpointslices", Some("default"), "managed-slice");
        storage.create(&key2, &managed_slice).await.unwrap();

        controller.reconcile_all().await.unwrap();

        // External slice should survive
        assert!(
            storage.get::<EndpointSlice>(&key).await.is_ok(),
            "externally-managed EndpointSlice should NOT be deleted"
        );

        // Managed slice should be deleted (orphaned)
        assert!(
            storage.get::<EndpointSlice>(&key2).await.is_err(),
            "controller-managed orphan EndpointSlice should be deleted"
        );
    }
}
