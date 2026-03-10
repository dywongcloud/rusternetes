use anyhow::{Context, Result};
use rusternetes_common::{
    cloud_provider::{CloudProvider, LoadBalancerPort, LoadBalancerService as CloudLBService},
    resources::{
        Service, ServiceType, Node,
        service::{LoadBalancerStatus, ServiceStatus, LoadBalancerIngress},
    },
};
use rusternetes_storage::{etcd::EtcdStorage, Storage};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info, warn};

/// LoadBalancerController reconciles LoadBalancer-type Services with cloud provider load balancers
pub struct LoadBalancerController {
    storage: Arc<EtcdStorage>,
    cloud_provider: Option<Arc<dyn CloudProvider>>,
    cluster_name: String,
    sync_interval: Duration,
}

impl LoadBalancerController {
    pub fn new(
        storage: Arc<EtcdStorage>,
        cloud_provider: Option<Arc<dyn CloudProvider>>,
        cluster_name: String,
        sync_interval_secs: u64,
    ) -> Self {
        Self {
            storage,
            cloud_provider,
            cluster_name,
            sync_interval: Duration::from_secs(sync_interval_secs),
        }
    }

    /// Start the controller reconciliation loop
    pub async fn run(mut self) -> Result<()> {
        info!("Starting LoadBalancer controller");

        if self.cloud_provider.is_none() {
            warn!("No cloud provider configured. LoadBalancer services will not be provisioned.");
            warn!("Set CLOUD_PROVIDER environment variable to enable cloud load balancers.");
        }

        let mut interval = time::interval(self.sync_interval);

        loop {
            interval.tick().await;

            if let Err(e) = self.reconcile_all().await {
                error!("Error during LoadBalancer reconciliation: {}", e);
            }
        }
    }

    /// Reconcile all LoadBalancer services
    pub async fn reconcile_all(&mut self) -> Result<()> {
        debug!("Reconciling LoadBalancer services");

        // If no cloud provider, skip
        let cloud_provider = match &self.cloud_provider {
            Some(p) => p,
            None => {
                debug!("Skipping reconciliation - no cloud provider configured");
                return Ok(());
            }
        };

        // Get all services
        let services: Vec<Service> = self.storage
            .list("/registry/services/")
            .await
            .context("Failed to list services")?;

        // Get all nodes for IP addresses
        let nodes: Vec<Node> = self.storage
            .list("/registry/nodes/")
            .await
            .context("Failed to list nodes")?;

        let node_addresses: Vec<String> = nodes
            .iter()
            .filter_map(|node| {
                node.status.as_ref()
                    .and_then(|s| s.addresses.as_ref())
                    .and_then(|addrs| addrs.iter().find(|a| a.address_type == "InternalIP"))
                    .map(|addr| addr.address.clone())
            })
            .collect();

        // Filter to LoadBalancer services
        let lb_services: Vec<&Service> = services
            .iter()
            .filter(|s| {
                matches!(
                    s.spec.service_type.as_ref().unwrap_or(&ServiceType::ClusterIP),
                    ServiceType::LoadBalancer
                )
            })
            .collect();

        info!("Found {} LoadBalancer services to reconcile", lb_services.len());

        for service in lb_services {
            if let Err(e) = self.reconcile_service(service, cloud_provider.as_ref(), &node_addresses).await {
                let namespace = service.metadata.namespace.as_deref().unwrap_or("unknown");
                error!("Failed to reconcile service {}/{}: {}", namespace, service.metadata.name, e);
            }
        }

        Ok(())
    }

    /// Reconcile a single LoadBalancer service
    async fn reconcile_service(
        &self,
        service: &Service,
        cloud_provider: &dyn CloudProvider,
        node_addresses: &[String],
    ) -> Result<()> {
        let namespace = service.metadata.namespace.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Service has no namespace"))?;
        let name = &service.metadata.name;

        debug!("Reconciling LoadBalancer service {}/{}", namespace, name);

        // Ensure NodePorts are allocated
        let has_node_ports = service.spec.ports.iter().all(|p| p.node_port.is_some());

        if !has_node_ports {
            warn!("Service {}/{} is LoadBalancer type but missing NodePorts", namespace, name);
            // In a real implementation, we should allocate NodePorts here
            return Ok(());
        }

        // Convert to cloud provider service format
        let cloud_lb_service = CloudLBService {
            namespace: namespace.clone(),
            name: name.clone(),
            cluster_name: self.cluster_name.clone(),
            ports: service.spec.ports.iter().map(|p| LoadBalancerPort {
                name: p.name.clone(),
                protocol: p.protocol.clone().unwrap_or_else(|| "TCP".to_string()),
                port: p.port,
                node_port: p.node_port.unwrap(),
            }).collect(),
            node_addresses: node_addresses.to_vec(),
            session_affinity: service.spec.session_affinity.clone(),
            annotations: service.metadata.annotations.clone().unwrap_or_default(),
        };

        // Ensure load balancer exists
        let lb_status = cloud_provider
            .ensure_load_balancer(&cloud_lb_service)
            .await
            .context("Failed to ensure load balancer")?;

        // Update service status with load balancer information
        self.update_service_status(namespace, name, lb_status).await?;

        info!("Successfully reconciled LoadBalancer service {}/{}", namespace, name);

        Ok(())
    }

    /// Update service status with load balancer information
    async fn update_service_status(
        &self,
        namespace: &str,
        name: &str,
        lb_status: rusternetes_common::cloud_provider::LoadBalancerStatus,
    ) -> Result<()> {
        let key = rusternetes_storage::build_key("services", Some(namespace), name);

        // Get current service
        let mut service: Service = self.storage
            .get(&key)
            .await
            .context("Failed to get service")?;

        // Convert cloud provider status to service status
        let service_lb_status = LoadBalancerStatus {
            ingress: lb_status.ingress.iter().map(|ing| {
                LoadBalancerIngress {
                    ip: ing.ip.clone(),
                    hostname: ing.hostname.clone(),
                }
            }).collect(),
        };

        // Update status
        service.status = Some(ServiceStatus {
            load_balancer: Some(service_lb_status),
        });

        // Save updated service
        self.storage.update(&key, &service).await
            .context("Failed to update service status")?;

        debug!("Updated status for service {}/{}", namespace, name);

        Ok(())
    }

    /// Delete load balancer for a service (called when service is deleted)
    #[allow(dead_code)]
    pub async fn cleanup_service(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<()> {
        let cloud_provider = match &self.cloud_provider {
            Some(p) => p,
            None => return Ok(()), // No cloud provider, nothing to clean up
        };

        info!("Cleaning up LoadBalancer for service {}/{}", namespace, name);

        cloud_provider
            .delete_load_balancer(namespace, name)
            .await
            .context("Failed to delete load balancer")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_creation() {
        // Test that we can create a controller without cloud provider
        let storage = Arc::new(unsafe { std::mem::zeroed() });
        let controller = LoadBalancerController::new(
            storage,
            None,
            "test-cluster".to_string(),
            30,
        );

        assert_eq!(controller.cluster_name, "test-cluster");
        assert!(controller.cloud_provider.is_none());
    }
}
