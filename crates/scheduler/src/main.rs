mod scheduler;

use anyhow::Result;
use clap::Parser;
use rusternetes_storage::etcd::EtcdStorage;
use scheduler::Scheduler;
use std::sync::Arc;
use tracing::{info, Level};

#[derive(Parser, Debug)]
#[command(name = "rusternetes-scheduler")]
#[command(about = "Rusternetes Scheduler - Assigns pods to nodes")]
struct Args {
    /// Etcd endpoints (comma-separated)
    #[arg(long, default_value = "http://localhost:2379")]
    etcd_servers: String,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Scheduling interval in seconds
    #[arg(long, default_value = "5")]
    interval: u64,
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

    info!("Starting Rusternetes Scheduler");

    // Parse etcd endpoints
    let etcd_endpoints: Vec<String> = args
        .etcd_servers
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    // Initialize storage
    let storage = Arc::new(EtcdStorage::new(etcd_endpoints).await?);

    // Create and run scheduler
    let scheduler = Scheduler::new(storage, args.interval);
    scheduler.run().await?;

    Ok(())
}
