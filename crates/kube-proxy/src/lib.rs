pub mod iptables;
pub mod proxy;

use futures::StreamExt;
use proxy::KubeProxy;
use rusternetes_storage::{Storage, StorageBackend};
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

    let mut kube_proxy = KubeProxy::new(Arc::clone(&storage))?;

    info!("Kube-proxy initialized successfully");
    info!("Syncing services every {} seconds", config.sync_interval);

    let sync_interval = tokio::time::Duration::from_secs(config.sync_interval);

    loop {
        // Initial sync on each watch (re)connection
        if let Err(e) = kube_proxy.sync().await {
            tracing::error!("Initial sync error: {}", e);
        }

        // Watch services (primary trigger for iptables changes)
        let watch_result = storage.watch("/registry/services/").await;
        let mut watch = match watch_result {
            Ok(w) => w,
            Err(e) => {
                tracing::error!("Failed to establish watch: {}, retrying", e);
                tokio::time::sleep(sync_interval).await;
                continue;
            }
        };

        // Periodic resync as a safety net — 15s keeps iptables fresh even if
        // watch events are missed or endpoints change without a service change.
        let mut resync = tokio::time::interval(std::time::Duration::from_secs(15));
        resync.tick().await; // consume the immediate first tick

        let mut watch_broken = false;
        while !watch_broken {
            tokio::select! {
                event = watch.next() => {
                    match event {
                        Some(Ok(_)) => {
                            if let Err(e) = kube_proxy.sync().await {
                                tracing::error!("Sync error: {}", e);
                            }
                        }
                        Some(Err(e)) => {
                            tracing::warn!("Watch error: {}, reconnecting", e);
                            watch_broken = true;
                        }
                        None => {
                            tracing::warn!("Watch stream ended, reconnecting");
                            watch_broken = true;
                        }
                    }
                }
                _ = resync.tick() => {
                    if let Err(e) = kube_proxy.sync().await {
                        tracing::error!("Periodic sync error: {}", e);
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("Received shutdown signal");
                    return Ok(());
                }
            }
        }
    }
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
