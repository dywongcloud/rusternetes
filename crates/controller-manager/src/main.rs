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
    dynamic_provisioner::DynamicProvisionerController,
    volume_snapshot::VolumeSnapshotController,
    endpoints::EndpointsController,
    loadbalancer::LoadBalancerController,
};
use rusternetes_storage::etcd::EtcdStorage;
use std::sync::Arc;
use tracing::{info, Level};
use rusternetes_common::cloud_provider::CloudProvider;

#[cfg(feature = "cloud-providers")]
use rusternetes_cloud_providers;

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

    /// Cloud provider (aws, gcp, azure, or none)
    #[arg(long)]
    cloud_provider: Option<String>,

    /// Cluster name for cloud provider resources
    #[arg(long, default_value = "rusternetes")]
    cluster_name: String,

    /// Cloud provider region
    #[arg(long)]
    cloud_region: Option<String>,
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

    // Initialize cloud provider if configured
    #[cfg(feature = "cloud-providers")]
    let cloud_provider: Option<Arc<dyn CloudProvider>> = {
        if let Some(provider_str) = &args.cloud_provider {
            use rusternetes_common::cloud_provider::CloudProviderType;

            let provider_type = CloudProviderType::from_str(provider_str)
                .ok_or_else(|| anyhow::anyhow!("Invalid cloud provider: {}", provider_str))?;

            if provider_type != CloudProviderType::None {
                let mut config = std::collections::HashMap::new();
                if let Some(region) = &args.cloud_region {
                    config.insert("region".to_string(), region.clone());
                }

                info!("Initializing {} cloud provider", provider_type.as_str());
                let provider = rusternetes_cloud_providers::create_provider(
                    provider_type,
                    args.cluster_name.clone(),
                    config,
                ).await?;
                Some(provider)
            } else {
                None
            }
        } else {
            // Auto-detect cloud provider
            let detected = rusternetes_cloud_providers::detect_cloud_provider();
            if detected != rusternetes_common::cloud_provider::CloudProviderType::None {
                info!("Auto-detected cloud provider: {}", detected.as_str());
                let mut config = std::collections::HashMap::new();
                if let Some(region) = &args.cloud_region {
                    config.insert("region".to_string(), region.clone());
                }
                let provider = rusternetes_cloud_providers::create_provider(
                    detected,
                    args.cluster_name.clone(),
                    config,
                ).await.ok();
                provider
            } else {
                None
            }
        }
    };

    #[cfg(not(feature = "cloud-providers"))]
    let cloud_provider: Option<Arc<dyn CloudProvider>> = None;

    // Start LoadBalancer controller
    let lb_controller = LoadBalancerController::new(
        storage.clone(),
        cloud_provider.clone(),
        args.cluster_name.clone(),
        args.sync_interval,
    );
    tokio::spawn(async move {
        if let Err(e) = lb_controller.run().await {
            tracing::error!("LoadBalancer controller error: {}", e);
        }
    });

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

    // Start Dynamic Provisioner controller
    let dynamic_provisioner_controller = DynamicProvisionerController::new(storage.clone());
    tokio::spawn(async move {
        if let Err(e) = dynamic_provisioner_controller.run().await {
            tracing::error!("Dynamic Provisioner controller error: {}", e);
        }
    });

    // Start Volume Snapshot controller
    let volume_snapshot_controller = VolumeSnapshotController::new(storage.clone());
    tokio::spawn(async move {
        if let Err(e) = volume_snapshot_controller.run().await {
            tracing::error!("Volume Snapshot controller error: {}", e);
        }
    });

    // Start Endpoints controller
    let endpoints_controller = EndpointsController::new(storage.clone());
    let sync_interval_secs = args.sync_interval;
    tokio::spawn(async move {
        loop {
            if let Err(e) = endpoints_controller.reconcile_all().await {
                tracing::error!("Endpoints controller error: {}", e);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(sync_interval_secs)).await;
        }
    });

    info!("All controllers started successfully");

    // Keep the main thread alive
    tokio::signal::ctrl_c().await?;
    info!("Shutting down controller manager");

    Ok(())
}
