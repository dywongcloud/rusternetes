use anyhow::{Context, Result};
use rusternetes_common::resources::{Endpoints, Service, ServiceType};
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

        Ok(Self {
            storage,
            iptables,
        })
    }

    /// Sync all services and their endpoints
    pub async fn sync(&mut self) -> Result<()> {
        debug!("Starting kube-proxy sync");

        // Flush existing rules to start fresh
        self.iptables.flush_rules()?;

        // Get all services
        let services: Vec<Service> = self.storage
            .list("/registry/services/")
            .await
            .context("Failed to list services")?;

        // Get all endpoints
        let all_endpoints: Vec<Endpoints> = self.storage
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

        // Process each service
        for service in services {
            if let Err(e) = self.sync_service(&service, &endpoints_map).await {
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
    ) -> Result<()> {
        let namespace = service.metadata.namespace.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Service has no namespace"))?;
        let name = &service.metadata.name;

        debug!("Syncing service {}/{}", namespace, name);

        // Get service type
        let service_type = service.spec.service_type.as_ref()
            .cloned()
            .unwrap_or(ServiceType::ClusterIP);

        // Skip ExternalName services (they don't need iptables rules)
        if service_type == ServiceType::ExternalName {
            debug!("Skipping ExternalName service {}/{}", namespace, name);
            return Ok(());
        }

        // Get ClusterIP (required for ClusterIP/NodePort/LoadBalancer services)
        let cluster_ip = service.spec.cluster_ip.as_ref();
        if cluster_ip.is_none() && service_type != ServiceType::ExternalName {
            warn!("Service {}/{} has no ClusterIP, skipping", namespace, name);
            return Ok(());
        }

        // Get endpoints for this service
        let endpoint_key = format!("{}/{}", namespace, name);
        let endpoints = endpoints_map.get(&endpoint_key);

        // Extract endpoint addresses
        let endpoint_addresses = self.extract_endpoint_addresses(endpoints);

        if endpoint_addresses.is_empty() {
            debug!("Service {}/{} has no ready endpoints", namespace, name);
        }

        // Process each service port
        for service_port in &service.spec.ports {
            let protocol = service_port.protocol.as_deref().unwrap_or("TCP");
            let target_port = service_port.target_port.unwrap_or(service_port.port);

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
                )?;
            }

            // Add NodePort rules (for NodePort and LoadBalancer)
            if matches!(service_type, ServiceType::NodePort | ServiceType::LoadBalancer) {
                if let Some(node_port) = service_port.node_port {
                    self.iptables.add_nodeport_rules(
                        node_port,
                        &endpoints_with_port,
                        protocol,
                    )?;
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
