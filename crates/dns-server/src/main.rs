use anyhow::Result;
use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn};
use tracing_subscriber;

mod resolver;
mod server;
mod watcher;

use resolver::KubernetesResolver;
use server::DnsServer;
use watcher::ResourceWatcher;

#[derive(Parser, Debug)]
#[command(name = "rusternetes-dns-server")]
#[command(about = "DNS server for Rusternetes service discovery", long_about = None)]
struct Args {
    #[arg(long, default_value = "http://localhost:2379")]
    etcd_endpoint: String,

    #[arg(long, default_value = "0.0.0.0:53")]
    listen_addr: String,

    #[arg(long, default_value = "cluster.local")]
    cluster_domain: String,

    #[arg(long, default_value = "10")]
    ttl: u32,

    #[arg(long, default_value = "30")]
    sync_interval_secs: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    let args = Args::parse();

    info!("Starting Rusternetes DNS Server");
    info!("  etcd endpoint: {}", args.etcd_endpoint);
    info!("  listen address: {}", args.listen_addr);
    info!("  cluster domain: {}", args.cluster_domain);
    info!("  TTL: {} seconds", args.ttl);
    info!("  sync interval: {} seconds", args.sync_interval_secs);

    // Create etcd storage
    let storage = Arc::new(
        rusternetes_storage::etcd::EtcdStorage::new(vec![args.etcd_endpoint.clone()])
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to etcd: {}", e))?,
    );

    info!("Connected to etcd at {}", args.etcd_endpoint);

    // Create Kubernetes resolver
    let resolver = Arc::new(KubernetesResolver::new(
        args.cluster_domain.clone(),
        args.ttl,
    ));

    // Create and start resource watcher
    let watcher = ResourceWatcher::new(storage.clone(), resolver.clone());
    let watcher_handle = tokio::spawn(async move {
        watcher.watch(args.sync_interval_secs).await;
    });

    // Parse listen address
    let addr: SocketAddr = args
        .listen_addr
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid listen address: {}", e))?;

    // Create and start DNS server
    let dns_server = DnsServer::new(resolver.clone(), addr);
    let server_handle = tokio::spawn(async move {
        if let Err(e) = dns_server.run().await {
            warn!("DNS server error: {}", e);
        }
    });

    info!("DNS server listening on {}", addr);
    info!("DNS server ready to handle queries");

    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Received shutdown signal, stopping DNS server...");
        }
        Err(err) => {
            warn!("Error waiting for shutdown signal: {}", err);
        }
    }

    // Gracefully shutdown
    server_handle.abort();
    watcher_handle.abort();

    info!("DNS server stopped");
    Ok(())
}
