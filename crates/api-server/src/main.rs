mod handlers;
mod router;
mod state;

use anyhow::Result;
use clap::Parser;
use rusternetes_storage::etcd::EtcdStorage;
use state::ApiServerState;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber;

#[derive(Parser, Debug)]
#[command(name = "rusternetes-api-server")]
#[command(about = "Rusternetes API Server - Kubernetes API reimplemented in Rust")]
struct Args {
    /// Address to bind to
    #[arg(long, default_value = "0.0.0.0:6443")]
    bind_address: String,

    /// Etcd endpoints (comma-separated)
    #[arg(long, default_value = "http://localhost:2379")]
    etcd_servers: String,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,
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

    info!("Starting Rusternetes API Server");

    // Parse etcd endpoints
    let etcd_endpoints: Vec<String> = args
        .etcd_servers
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    // Initialize storage
    info!("Connecting to etcd: {:?}", etcd_endpoints);
    let storage = EtcdStorage::new(etcd_endpoints).await?;

    // Create shared state
    let state = Arc::new(ApiServerState::new(Arc::new(storage)));

    // Build router
    let app = router::build_router(state);

    // Bind and serve
    info!("API Server listening on {}", args.bind_address);
    let listener = tokio::net::TcpListener::bind(&args.bind_address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
