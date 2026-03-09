mod runtime;
mod kubelet;

use anyhow::Result;
use axum::{routing::get, Router};
use clap::Parser;
use kubelet::Kubelet;
use rusternetes_common::observability::MetricsRegistry;
use rusternetes_storage::etcd::EtcdStorage;
use std::sync::Arc;
use tracing::{info, Level};

#[derive(Parser, Debug)]
#[command(name = "rusternetes-kubelet")]
#[command(about = "Rusternetes Kubelet - Node agent that manages containers")]
struct Args {
    /// Node name
    #[arg(long)]
    node_name: String,

    /// Etcd endpoints (comma-separated)
    #[arg(long, default_value = "http://localhost:2379")]
    etcd_servers: String,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Sync interval in seconds
    #[arg(long, default_value = "10")]
    sync_interval: u64,

    /// Metrics server port
    #[arg(long, default_value = "8082")]
    metrics_port: u16,
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

    info!("Starting Rusternetes Kubelet for node: {}", args.node_name);

    // Parse etcd endpoints
    let etcd_endpoints: Vec<String> = args
        .etcd_servers
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    // Initialize storage
    let storage = Arc::new(EtcdStorage::new(etcd_endpoints).await?);

    // Initialize metrics
    let metrics = Arc::new(MetricsRegistry::new().with_kubelet_metrics()?);
    let metrics_clone = metrics.clone();

    // Start metrics server
    let metrics_addr = format!("0.0.0.0:{}", args.metrics_port);
    info!("Starting metrics server on {}", metrics_addr);

    tokio::spawn(async move {
        let app = Router::new()
            .route("/metrics", get(|| async move {
                metrics_clone.gather()
            }));

        let listener = tokio::net::TcpListener::bind(&metrics_addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    // Create and run kubelet
    let kubelet = Kubelet::new(args.node_name, storage, args.sync_interval).await?;
    kubelet.run().await?;

    Ok(())
}
