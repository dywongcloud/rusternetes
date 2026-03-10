mod controllers;

use anyhow::Result;
use clap::Parser;
use controllers::{
    deployment::DeploymentController,
    statefulset::StatefulSetController,
    daemonset::DaemonSetController,
    job::JobController,
    cronjob::CronJobController,
    pv_binder::PVBinderController,
};
use rusternetes_storage::etcd::EtcdStorage;
use std::sync::Arc;
use tracing::{info, Level};

#[derive(Parser, Debug)]
#[command(name = "rusternetes-controller-manager")]
#[command(about = "Rusternetes Controller Manager - Runs controller loops")]
struct Args {
    /// Etcd endpoints (comma-separated)
    #[arg(long, default_value = "http://localhost:2379")]
    etcd_servers: String,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Controller sync interval in seconds
    #[arg(long, default_value = "10")]
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

    info!("Starting Rusternetes Controller Manager");

    // Parse etcd endpoints
    let etcd_endpoints: Vec<String> = args
        .etcd_servers
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    // Initialize storage
    let storage = Arc::new(EtcdStorage::new(etcd_endpoints).await?);

    // Start deployment controller
    let deployment_controller = DeploymentController::new(storage.clone(), args.sync_interval);
    tokio::spawn(async move {
        if let Err(e) = deployment_controller.run().await {
            tracing::error!("Deployment controller error: {}", e);
        }
    });

    // Start StatefulSet controller
    let statefulset_controller = StatefulSetController::new(storage.clone());
    tokio::spawn(async move {
        if let Err(e) = statefulset_controller.run().await {
            tracing::error!("StatefulSet controller error: {}", e);
        }
    });

    // Start DaemonSet controller
    let daemonset_controller = DaemonSetController::new(storage.clone());
    tokio::spawn(async move {
        if let Err(e) = daemonset_controller.run().await {
            tracing::error!("DaemonSet controller error: {}", e);
        }
    });

    // Start Job controller
    let job_controller = JobController::new(storage.clone());
    tokio::spawn(async move {
        if let Err(e) = job_controller.run().await {
            tracing::error!("Job controller error: {}", e);
        }
    });

    // Start CronJob controller
    let cronjob_controller = CronJobController::new(storage.clone());
    tokio::spawn(async move {
        if let Err(e) = cronjob_controller.run().await {
            tracing::error!("CronJob controller error: {}", e);
        }
    });

    // Start PV/PVC Binder controller
    let pv_binder_controller = PVBinderController::new(storage.clone());
    tokio::spawn(async move {
        if let Err(e) = pv_binder_controller.run().await {
            tracing::error!("PV/PVC Binder controller error: {}", e);
        }
    });

    info!("All controllers started successfully");

    // Keep the main thread alive
    tokio::signal::ctrl_c().await?;
    info!("Shutting down controller manager");

    Ok(())
}
