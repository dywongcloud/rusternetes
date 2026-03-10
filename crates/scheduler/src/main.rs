mod scheduler;
mod advanced;

use anyhow::Result;
use axum::{routing::get, Router};
use clap::Parser;
use rusternetes_common::observability::MetricsRegistry;
use rusternetes_common::leader_election::{LeaderElector, LeaderElectionConfig};
use rusternetes_storage::etcd::EtcdStorage;
use scheduler::Scheduler;
use std::sync::Arc;
use tracing::{info, warn, Level};
use uuid::Uuid;

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

    /// Metrics server port
    #[arg(long, default_value = "8081")]
    metrics_port: u16,

    /// Enable leader election (for HA)
    #[arg(long)]
    enable_leader_election: bool,

    /// Leader election identity (unique for each instance)
    #[arg(long)]
    leader_election_identity: Option<String>,

    /// Leader election lock key
    #[arg(long, default_value = "/rusternetes/scheduler/leader")]
    leader_election_lock_key: String,

    /// Leader election lease duration in seconds
    #[arg(long, default_value = "15")]
    leader_election_lease_duration: u64,
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
    let storage = Arc::new(EtcdStorage::new(etcd_endpoints.clone()).await?);

    // Initialize metrics
    let metrics = Arc::new(MetricsRegistry::new().with_scheduler_metrics()?);
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

    // Initialize leader election if enabled
    if args.enable_leader_election {
        let identity = args.leader_election_identity.unwrap_or_else(|| {
            format!("scheduler-{}", Uuid::new_v4())
        });

        let config = LeaderElectionConfig {
            identity: identity.clone(),
            lock_key: args.leader_election_lock_key.clone(),
            lease_duration: args.leader_election_lease_duration,
            renew_interval: args.leader_election_lease_duration / 3,
            retry_interval: 2,
        };

        info!(
            identity = %identity,
            "Leader election enabled - starting in follower mode"
        );

        let elector = Arc::new(LeaderElector::new(etcd_endpoints.clone(), config).await?);

        // Start leader election in background
        let elector_clone = elector.clone();
        tokio::spawn(async move {
            if let Err(e) = elector_clone.run().await {
                tracing::error!("Leader election error: {}", e);
            }
        });

        // Create scheduler
        let scheduler = Scheduler::new(storage, args.interval);

        // Wait to become leader before running scheduler
        loop {
            while !elector.is_leader().await {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
            info!("Scheduler starting (leader acquired)");

            // Run scheduler
            if let Err(e) = scheduler.run().await {
                tracing::error!("Scheduler error: {}", e);
            }

            // Check if we're still the leader
            if !elector.is_leader().await {
                warn!("Scheduler stopped (lost leadership)");
                continue;
            }
            break;
        }
    } else {
        warn!("Leader election disabled - running in single-instance mode");

        // Create and run scheduler directly
        let scheduler = Scheduler::new(storage, args.interval);
        scheduler.run().await?;
    }

    Ok(())
}
