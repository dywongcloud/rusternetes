use anyhow::{Context, Result};
use rusternetes_common::resources::{EndpointSlice, Endpoints, Service, ServiceType};
use rusternetes_storage::{etcd::EtcdStorage, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::iptables::IptablesManager;

/// KubeProxy manages service networking through iptables rules
pub struct KubeProxy {
    storage: Arc<EtcdStorage>,
    iptables: IptablesManager,
}

impl KubeProxy {
    pub fn new(storage: Arc<EtcdStorage>) -> Result<Self> {
        let iptables = IptablesManager::new();
        iptables.initialize()?;

        Ok(Self { storage, iptables })
    }

    /// Sync all services and their endpoints
    pub async fn sync(&mut self) -> Result<()> {
        debug!("Starting kube-proxy sync");

        // Flush existing rules to start fresh
        self.iptables.flush_rules()?;

        // Get all services
        let services: Vec<Service> = self
            .storage
            .list("/registry/services/")
            .await
            .context("Failed to list services")?;

        // Get all endpoints (old-style)
        let all_endpoints: Vec<Endpoints> = self
            .storage
            .list("/registry/endpoints/")
            .await
            .context("Failed to list endpoints")?;

        // Build endpoints map for quick lookup
        let endpoints_map: HashMap<String, Endpoints> = all_endpoints
            .into_iter()
            .filter_map(|ep| {
                let namespace = ep.metadata.namespace.as_ref()?;
                let key = format!("{}/{}", namespace, &ep.metadata.name);
                Some((key, ep))
            })
            .collect();

        // Get all EndpointSlices (new API) as fallback
        let all_endpointslices: Vec<EndpointSlice> = self
            .storage
            .list("/registry/endpointslices/")
            .await
            .unwrap_or_default();

        // Build EndpointSlice map: key = "namespace/service-name" -> list of endpoint IPs
        let mut endpointslice_map: HashMap<String, Vec<String>> = HashMap::new();
        for es in &all_endpointslices {
            let namespace = es.metadata.namespace.as_deref().unwrap_or("default");
            // EndpointSlice's label kubernetes.io/service-name tells us which service it belongs to
            let service_name = es
                .metadata
                .labels
                .as_ref()
                .and_then(|l| l.get("kubernetes.io/service-name"))
                .cloned()
                .unwrap_or_else(|| {
                    // Fall back: strip the random suffix from the EndpointSlice name
                    es.metadata.name.rsplit_once('-').map(|(prefix, _)| prefix.to_string())
                        .unwrap_or_else(|| es.metadata.name.clone())
                });
            let key = format!("{}/{}", namespace, service_name);
            for endpoint in &es.endpoints {
                if let Some(conditions) = &endpoint.conditions {
                    if conditions.ready == Some(false) {
                        continue; // Skip not-ready endpoints
                    }
                }
                for addr in &endpoint.addresses {
                    endpointslice_map.entry(key.clone()).or_default().push(addr.clone());
                }
            }
        }

        // Process each service
        for service in services {
            if let Err(e) = self.sync_service(&service, &endpoints_map, &endpointslice_map).await {
                let namespace = service.metadata.namespace.as_deref().unwrap_or("unknown");
                let name = &service.metadata.name;
                error!("Failed to sync service {}/{}: {}", namespace, name, e);
            }
        }

        info!("Kube-proxy sync completed");
        Ok(())
    }

    /// Sync a single service
    async fn sync_service(
        &mut self,
        service: &Service,
        endpoints_map: &HashMap<String, Endpoints>,
        endpointslice_map: &HashMap<String, Vec<String>>,
    ) -> Result<()> {
        let namespace = service
            .metadata
            .namespace
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Service has no namespace"))?;
        let name = &service.metadata.name;

        debug!("Syncing service {}/{}", namespace, name);

        // Get service type
        let service_type = service
            .spec
            .service_type
            .as_ref()
            .cloned()
            .unwrap_or(ServiceType::ClusterIP);

        // Skip ExternalName services (they don't need iptables rules)
        if service_type == ServiceType::ExternalName {
            debug!("Skipping ExternalName service {}/{}", namespace, name);
            return Ok(());
        }

        // Get ClusterIP (required for ClusterIP/NodePort/LoadBalancer services)
        let cluster_ip = service.spec.cluster_ip.as_ref();
        debug!(
            "Service {}/{} - clusterIP field value: {:?}",
            namespace, name, cluster_ip
        );
        debug!(
            "Service {}/{} - full spec: {:?}",
            namespace, name, service.spec
        );
        let has_valid_cluster_ip = cluster_ip
            .map(|ip| !ip.is_empty() && ip != "None" && ip != "null")
            .unwrap_or(false);
        if !has_valid_cluster_ip && service_type != ServiceType::ExternalName {
            warn!("Service {}/{} has no valid ClusterIP ({:?}), skipping", namespace, name, cluster_ip);
            return Ok(());
        }

        // Get endpoints for this service (try old-style Endpoints first, then EndpointSlices)
        let endpoint_key = format!("{}/{}", namespace, name);
        let endpoints = endpoints_map.get(&endpoint_key);

        // Extract endpoint addresses
        let mut endpoint_addresses = self.extract_endpoint_addresses(endpoints);

        // Fall back to EndpointSlices if no old-style endpoints found
        if endpoint_addresses.is_empty() {
            if let Some(slice_addrs) = endpointslice_map.get(&endpoint_key) {
                endpoint_addresses = slice_addrs.clone();
                debug!("Using EndpointSlice addresses for {}/{}: {:?}", namespace, name, endpoint_addresses);
            }
        }

        if endpoint_addresses.is_empty() {
            debug!("Service {}/{} has no ready endpoints", namespace, name);
        }

        // Check session affinity
        let use_session_affinity = service.spec.session_affinity.as_deref() == Some("ClientIP");

        // Process each service port
        for service_port in &service.spec.ports {
            let protocol = service_port.protocol.as_deref().unwrap_or("TCP");
            let target_port = match &service_port.target_port {
                Some(rusternetes_common::resources::IntOrString::Int(p)) => *p as u16,
                Some(rusternetes_common::resources::IntOrString::String(s)) => {
                    s.parse::<u16>().unwrap_or(service_port.port)
                }
                None => service_port.port,
            };

            // Build list of endpoints with the correct port
            let endpoints_with_port: Vec<(String, u16)> = endpoint_addresses
                .iter()
                .map(|ip| (ip.clone(), target_port))
                .collect();

            // Add ClusterIP rules (for ClusterIP, NodePort, and LoadBalancer)
            if let Some(cluster_ip_str) = cluster_ip {
                self.iptables.add_clusterip_rules(
                    cluster_ip_str,
                    service_port.port,
                    &endpoints_with_port,
                    protocol,
                    use_session_affinity,
                )?;
            }

            // Add NodePort rules (for NodePort and LoadBalancer)
            if matches!(
                service_type,
                ServiceType::NodePort | ServiceType::LoadBalancer
            ) {
                if let Some(node_port) = service_port.node_port {
                    self.iptables
                        .add_nodeport_rules(node_port, &endpoints_with_port, protocol)?;
                }
            }
        }

        Ok(())
    }

    /// Extract ready endpoint IP addresses from Endpoints resource
    fn extract_endpoint_addresses(&self, endpoints: Option<&Endpoints>) -> Vec<String> {
        let endpoints = match endpoints {
            Some(ep) => ep,
            None => return vec![],
        };

        let mut addresses = Vec::new();

        for subset in &endpoints.subsets {
            if let Some(ready_addresses) = &subset.addresses {
                for addr in ready_addresses {
                    addresses.push(addr.ip.clone());
                }
            }
        }

        addresses
    }
}

impl Drop for KubeProxy {
    fn drop(&mut self) {
        info!("Shutting down kube-proxy");
    }
}
