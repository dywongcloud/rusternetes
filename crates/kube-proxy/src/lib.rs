pub mod iptables;
pub mod proxy;

use proxy::KubeProxy;
use rusternetes_storage::StorageBackend;
use std::sync::Arc;
use tracing::{info, warn};

/// Configuration for the kube-proxy component.
pub struct KubeProxyConfig {
    pub node_name: String,
    pub sync_interval: u64,
}

/// Run the kube-proxy component.
///
/// This is the main entry point for embedding kube-proxy in the all-in-one binary.
/// It runs the iptables sync loop until cancelled.
pub async fn run(storage: Arc<StorageBackend>, config: KubeProxyConfig) -> anyhow::Result<()> {
    info!("Starting Rusternetes Kube-proxy for node: {}", config.node_name);

    if let Err(e) = check_iptables() {
        warn!("iptables check failed: {}. Some features may not work.", e);
        warn!("Kube-proxy requires iptables to be installed and accessible.");
    }

    let mut kube_proxy = KubeProxy::new(storage)?;

    info!("Kube-proxy initialized successfully");
    info!("Syncing services every {} seconds", config.sync_interval);

    let sync_interval = tokio::time::Duration::from_secs(config.sync_interval);
    let mut interval = tokio::time::interval(sync_interval);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Err(e) = kube_proxy.sync().await {
                    tracing::error!("Sync error: {}", e);
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received shutdown signal");
                break;
            }
        }
    }

    info!("Shutting down kube-proxy");
    Ok(())
}

fn check_iptables() -> anyhow::Result<()> {
    let output = std::process::Command::new("/usr/sbin/iptables-legacy")
        .arg("--version")
        .output()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        info!("iptables version: {}", version.trim());
        Ok(())
    } else {
        Err(anyhow::anyhow!("iptables-legacy not available"))
    }
}
