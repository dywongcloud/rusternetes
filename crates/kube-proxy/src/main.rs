mod iptables;
mod proxy;

use anyhow::Result;
use clap::Parser;
use rusternetes_storage::{StorageBackend, StorageConfig};
use std::sync::Arc;
use tracing::{info, warn, Level};

use proxy::KubeProxy;

#[derive(Parser, Debug)]
#[command(name = "rusternetes-kube-proxy")]
#[command(about = "Rusternetes Kube-proxy - Network proxy for service load balancing")]
struct Args {
    /// Node name
    #[arg(long)]
    node_name: String,

    /// Etcd endpoints (comma-separated)
    #[arg(long, default_value = "http://localhost:2379")]
    etcd_servers: String,

    /// Storage backend: "etcd" or "sqlite"
    #[arg(long, default_value = "etcd")]
    storage_backend: String,

    /// SQLite database path (only used when --storage-backend=sqlite)
    #[arg(long, default_value = "./data/rusternetes.db")]
    data_dir: String,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Sync interval in seconds
    #[arg(long, default_value = "1")]
    sync_interval: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let level = match args.log_level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    tracing_subscriber::fmt().with_max_level(level).init();

    info!(
        "Starting Rusternetes Kube-proxy for node: {}",
        args.node_name
    );

    // Check for iptables availability
    if let Err(e) = check_iptables() {
        warn!("iptables check failed: {}. Some features may not work.", e);
        warn!("Kube-proxy requires iptables to be installed and accessible.");
    }

    // Initialize storage
    let storage_config = match args.storage_backend.as_str() {
        #[cfg(feature = "sqlite")]
        "sqlite" => {
            info!("Using SQLite storage backend at: {}", args.data_dir);
            StorageConfig::Sqlite { path: args.data_dir.clone() }
        }
        _ => {
            let etcd_endpoints: Vec<String> = args
                .etcd_servers
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            info!("Connecting to etcd at: {:?}", etcd_endpoints);
            StorageConfig::Etcd { endpoints: etcd_endpoints }
        }
    };
    let storage = Arc::new(StorageBackend::new(storage_config).await?);

    // Initialize kube-proxy
    let mut kube_proxy = KubeProxy::new(storage)?;

    info!("Kube-proxy initialized successfully");
    info!("Syncing services every {} seconds", args.sync_interval);

    // Main sync loop
    let sync_interval = tokio::time::Duration::from_secs(args.sync_interval);
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

/// Check if iptables is available
fn check_iptables() -> Result<()> {
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
