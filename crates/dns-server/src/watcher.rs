use rusternetes_common::resources::{Endpoints, Pod, Service};
use rusternetes_common::types::Phase;
use rusternetes_storage::etcd::EtcdStorage;
use rusternetes_storage::Storage;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info};

use crate::resolver::{KubernetesResolver, ServiceEndpoint};

pub struct ResourceWatcher {
    storage: Arc<EtcdStorage>,
    resolver: Arc<KubernetesResolver>,
}

impl ResourceWatcher {
    pub fn new(storage: Arc<EtcdStorage>, resolver: Arc<KubernetesResolver>) -> Self {
        Self { storage, resolver }
    }

    pub async fn watch(&self, interval_secs: u64) {
        info!("Starting resource watcher with {}-second interval", interval_secs);

        loop {
            if let Err(e) = self.sync_all().await {
                error!("Error syncing resources: {}", e);
            }

            sleep(Duration::from_secs(interval_secs)).await;
        }
    }

    async fn sync_all(&self) -> anyhow::Result<()> {
        // Sync services and endpoints
        self.sync_services().await?;

        // Sync pods
        self.sync_pods().await?;

        let (names, records) = self.resolver.stats();
        debug!("DNS cache stats: {} unique names, {} total records", names, records);

        Ok(())
    }

    async fn sync_services(&self) -> anyhow::Result<()> {
        // List all namespaces to find services
        let namespaces = self.list_namespaces().await?;

        for namespace in namespaces {
            // List services in this namespace
            let services: Result<Vec<Service>, _> = self
                .storage
                .list(&format!("/registry/services/{}/", namespace))
                .await;

            let services = services.unwrap_or_default();

            for service in services {
                let service_name = service.metadata.name.as_str();
                let namespace = service.metadata.namespace.as_deref().unwrap_or("default");

                // Get cluster IP
                let cluster_ip = service.spec.cluster_ip.as_deref();

                // Get endpoints for this service
                let endpoints = self.get_service_endpoints(service_name, namespace).await;

                // Update DNS records
                self.resolver.update_service(
                    service_name,
                    namespace,
                    cluster_ip,
                    endpoints,
                );
            }
        }

        Ok(())
    }

    async fn sync_pods(&self) -> anyhow::Result<()> {
        // List all namespaces to find pods
        let namespaces = self.list_namespaces().await?;

        for namespace in namespaces {
            // List pods in this namespace
            let pods: Result<Vec<Pod>, _> = self
                .storage
                .list(&format!("/registry/pods/{}/", namespace))
                .await;

            let pods = pods.unwrap_or_default();

            for pod in pods {
                let pod_name = pod.metadata.name.as_str();
                let namespace = pod.metadata.namespace.as_deref().unwrap_or("default");

                // Only add DNS records for pods that have an IP and are running
                if let Some(ref status) = pod.status {
                    if let Some(pod_ip) = &status.pod_ip {
                        if status.phase == Phase::Running {
                            self.resolver.update_pod(pod_name, namespace, pod_ip);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn get_service_endpoints(&self, service_name: &str, namespace: &str) -> Vec<ServiceEndpoint> {
        let key = format!("/registry/endpoints/{}/{}", namespace, service_name);

        match self.storage.get::<Endpoints>(&key).await {
            Ok(endpoints) => {
                let mut result = Vec::new();

                // Process subsets
                for subset in &endpoints.subsets {
                        // Get ready addresses
                        let addresses = subset.addresses.as_ref().cloned().unwrap_or_default();

                        // Get ports
                        let ports = subset.ports.as_ref().cloned().unwrap_or_default();

                        for addr in addresses {
                            for port in &ports {
                                result.push(ServiceEndpoint {
                                    ip: addr.ip.clone(),
                                    port: port.port as u16,
                                    port_name: port.name.clone(),
                                    protocol: port.protocol.clone(),
                                    pod_name: addr.target_ref.as_ref().and_then(|r| r.name.clone()),
                                });
                            }
                        }
                }

                result
            }
            Err(_) => Vec::new(),
        }
    }

    async fn list_namespaces(&self) -> anyhow::Result<Vec<String>> {
        let namespaces: Result<Vec<rusternetes_common::resources::Namespace>, _> = self
            .storage
            .list("/registry/namespaces/")
            .await;

        let namespaces = namespaces.unwrap_or_default();

        let mut names = vec!["default".to_string()];
        for ns in namespaces {
            let name = &ns.metadata.name;
            if name != "default" {
                names.push(name.clone());
            }
        }

        Ok(names)
    }
}
