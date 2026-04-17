use anyhow::{Context, Result};
use rusternetes_common::resources::Endpoints;
use rusternetes_storage::StorageBackend;
use rusternetes_storage::Storage;
use std::sync::Arc;
use tracing::{info, warn};

/// Get the API server's IP address from the network interface
/// This discovers the container's IP on the Docker/Podman network
fn get_api_server_ip() -> Result<String> {
    // Try to get IP from network interfaces
    // Look for non-loopback IPv4 addresses
    let interfaces = match get_if_addrs::get_if_addrs() {
        Ok(addrs) => addrs,
        Err(e) => {
            warn!("Failed to get network interfaces: {}", e);
            return Err(anyhow::anyhow!("Failed to get network interfaces: {}", e));
        }
    };

    // Find the first non-loopback IPv4 address
    for iface in interfaces {
        if !iface.is_loopback() {
            if let get_if_addrs::IfAddr::V4(addr) = iface.addr {
                let ip = addr.ip.to_string();
                info!(
                    "Discovered API server IP: {} (interface: {})",
                    ip, iface.name
                );
                return Ok(ip);
            }
        }
    }

    Err(anyhow::anyhow!("No non-loopback IPv4 address found"))
}

/// Bootstrap the kubernetes Service and Endpoints in the default namespace
/// This ensures the "kubernetes" service always points to this API server
pub async fn bootstrap_kubernetes_service(
    storage: Arc<StorageBackend>,
    api_server_port: u16,
) -> Result<()> {
    info!("Bootstrapping kubernetes Service and Endpoints");

    // Get the API server's IP address
    let api_server_ip = get_api_server_ip().context("Failed to discover API server IP address")?;

    info!(
        "API server IP: {}, Port: {}",
        api_server_ip, api_server_port
    );

    // Check if the kubernetes Endpoints already exist
    let endpoints_key = "/registry/endpoints/default/kubernetes";

    match storage.get::<Endpoints>(endpoints_key).await {
        Ok(mut endpoints) => {
            // Update existing endpoints
            info!("Updating existing kubernetes Endpoints");
            // endpoints.subsets is Vec<EndpointSubset>
            if let Some(subset) = endpoints.subsets.first_mut() {
                // subset.addresses is Option<Vec<EndpointAddress>>
                if let Some(addresses) = &mut subset.addresses {
                    if let Some(addr) = addresses.first_mut() {
                        if addr.ip != api_server_ip {
                            info!(
                                "Updating API server IP from {} to {}",
                                addr.ip, api_server_ip
                            );
                            addr.ip = api_server_ip;
                            storage
                                .update(endpoints_key, &endpoints)
                                .await
                                .context("Failed to update kubernetes Endpoints")?;
                        } else {
                            info!("API server IP already correct: {}", api_server_ip);
                        }
                    }
                }
            }
        }
        Err(_) => {
            // Create new Endpoints if they don't exist
            info!("kubernetes Endpoints not found - creating new Endpoints");
            use rusternetes_common::resources::{EndpointAddress, EndpointPort, EndpointSubset};
            use rusternetes_common::types::{ObjectMeta, TypeMeta};

            let mut metadata = ObjectMeta::new("kubernetes");
            metadata.namespace = Some("default".to_string());

            let endpoints = Endpoints {
                type_meta: TypeMeta {
                    kind: "Endpoints".to_string(),
                    api_version: "v1".to_string(),
                },
                metadata,
                subsets: vec![EndpointSubset {
                    addresses: Some(vec![EndpointAddress {
                        ip: api_server_ip.clone(),
                        hostname: None,
                        node_name: None,
                        target_ref: None,
                    }]),
                    not_ready_addresses: None,
                    ports: Some(vec![EndpointPort {
                        name: Some("https".to_string()),
                        port: api_server_port,
                        protocol: Some("TCP".to_string()),
                        app_protocol: None,
                    }]),
                }],
            };

            storage
                .create(endpoints_key, &endpoints)
                .await
                .context("Failed to create kubernetes Endpoints")?;
            info!("Created kubernetes Endpoints with IP: {}", api_server_ip);
        }
    }

    Ok(())
}
