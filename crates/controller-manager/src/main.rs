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
    volume_expansion::VolumeExpansionController,
    endpoints::EndpointsController,
    loadbalancer::LoadBalancerController,
    events::EventsController,
    resource_quota::ResourceQuotaController,
};
use rusternetes_storage::etcd::EtcdStorage;
use std::sync::Arc;
use tracing::{info, warn, Level};
use rusternetes_common::cloud_provider::CloudProvider;
use rusternetes_common::leader_election::{LeaderElector, LeaderElectionConfig};

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

    /// Enable leader election (for HA)
    #[arg(long)]
    enable_leader_election: bool,

    /// Leader election identity (unique for each instance)
    #[arg(long)]
    leader_election_identity: Option<String>,

    /// Leader election lock key
    #[arg(long, default_value = "/rusternetes/controller-manager/leader")]
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

    info!("Starting Rusternetes Controller Manager");

    // Parse etcd endpoints
    let etcd_endpoints: Vec<String> = args
        .etcd_servers
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    // Initialize storage
    let storage = Arc::new(EtcdStorage::new(etcd_endpoints.clone()).await?);

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

    // Initialize leader election if enabled
    let leader_elector = if args.enable_leader_election {
        let identity = args.leader_election_identity.unwrap_or_else(|| {
            format!("controller-manager-{}", uuid::Uuid::new_v4())
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

        Some(elector)
    } else {
        warn!("Leader election disabled - running in single-instance mode");
        None
    };

    // Macro to spawn controllers with leader election support
    macro_rules! spawn_controller {
        ($name:expr, $elector:expr, $fut:expr) => {{
            let elector_clone = $elector.clone();
            tokio::spawn(async move {
                // Wait until we're the leader (if leader election is enabled)
                if let Some(ref elector) = elector_clone {
                    loop {
                        while !elector.is_leader().await {
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        }
                        info!("{} starting (leader acquired)", $name);

                        // Run controller
                        $fut.await;

                        // Check if we're still the leader
                        if !elector.is_leader().await {
                            warn!("{} stopped (lost leadership)", $name);
                            continue;
                        }
                        break;
                    }
                } else {
                    // No leader election, run directly
                    $fut.await;
                }
            })
        }};
    }

    // Start LoadBalancer controller
    let lb_controller = Arc::new(LoadBalancerController::new(
        storage.clone(),
        cloud_provider.clone(),
        args.cluster_name.clone(),
        args.sync_interval,
    ));
    spawn_controller!(
        "LoadBalancer controller",
        leader_elector,
        {
            let controller = lb_controller.clone();
            async move {
                if let Err(e) = controller.run().await {
                    tracing::error!("LoadBalancer controller error: {}", e);
                }
            }
        }
    );

    // Start deployment controller
    let deployment_controller = Arc::new(DeploymentController::new(storage.clone(), args.sync_interval));
    spawn_controller!(
        "Deployment controller",
        leader_elector,
        {
            let controller = deployment_controller.clone();
            async move {
                if let Err(e) = controller.run().await {
                    tracing::error!("Deployment controller error: {}", e);
                }
            }
        }
    );

    // Start StatefulSet controller
    let statefulset_controller = Arc::new(StatefulSetController::new(storage.clone()));
    spawn_controller!(
        "StatefulSet controller",
        leader_elector,
        {
            let controller = statefulset_controller.clone();
            async move {
                if let Err(e) = controller.run().await {
                    tracing::error!("StatefulSet controller error: {}", e);
                }
            }
        }
    );

    // Start DaemonSet controller
    let daemonset_controller = Arc::new(DaemonSetController::new(storage.clone()));
    spawn_controller!(
        "DaemonSet controller",
        leader_elector,
        {
            let controller = daemonset_controller.clone();
            async move {
                if let Err(e) = controller.run().await {
                    tracing::error!("DaemonSet controller error: {}", e);
                }
            }
        }
    );

    // Start Job controller
    let job_controller = Arc::new(JobController::new(storage.clone()));
    spawn_controller!(
        "Job controller",
        leader_elector,
        {
            let controller = job_controller.clone();
            async move {
                if let Err(e) = controller.run().await {
                    tracing::error!("Job controller error: {}", e);
                }
            }
        }
    );

    // Start CronJob controller
    let cronjob_controller = Arc::new(CronJobController::new(storage.clone()));
    spawn_controller!(
        "CronJob controller",
        leader_elector,
        {
            let controller = cronjob_controller.clone();
            async move {
                if let Err(e) = controller.run().await {
                    tracing::error!("CronJob controller error: {}", e);
                }
            }
        }
    );

    // Start PV/PVC Binder controller
    let pv_binder_controller = Arc::new(PVBinderController::new(storage.clone()));
    spawn_controller!(
        "PV/PVC Binder controller",
        leader_elector,
        {
            let controller = pv_binder_controller.clone();
            async move {
                if let Err(e) = controller.run().await {
                    tracing::error!("PV/PVC Binder controller error: {}", e);
                }
            }
        }
    );

    // Start Dynamic Provisioner controller
    let dynamic_provisioner_controller = Arc::new(DynamicProvisionerController::new(storage.clone()));
    spawn_controller!(
        "Dynamic Provisioner controller",
        leader_elector,
        {
            let controller = dynamic_provisioner_controller.clone();
            async move {
                if let Err(e) = controller.run().await {
                    tracing::error!("Dynamic Provisioner controller error: {}", e);
                }
            }
        }
    );

    // Start Volume Snapshot controller
    let volume_snapshot_controller = Arc::new(VolumeSnapshotController::new(storage.clone()));
    spawn_controller!(
        "Volume Snapshot controller",
        leader_elector,
        {
            let controller = volume_snapshot_controller.clone();
            async move {
                if let Err(e) = controller.run().await {
                    tracing::error!("Volume Snapshot controller error: {}", e);
                }
            }
        }
    );

    // Start Volume Expansion controller
    let volume_expansion_controller = Arc::new(VolumeExpansionController::new(storage.clone()));
    spawn_controller!(
        "Volume Expansion controller",
        leader_elector,
        {
            let controller = volume_expansion_controller.clone();
            async move {
                if let Err(e) = controller.run().await {
                    tracing::error!("Volume Expansion controller error: {}", e);
                }
            }
        }
    );

    // Start Endpoints controller
    let endpoints_controller = Arc::new(EndpointsController::new(storage.clone()));
    let sync_interval_secs = args.sync_interval;
    spawn_controller!(
        "Endpoints controller",
        leader_elector,
        {
            let controller = endpoints_controller.clone();
            async move {
                loop {
                    if let Err(e) = controller.reconcile_all().await {
                        tracing::error!("Endpoints controller error: {}", e);
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(sync_interval_secs)).await;
                }
            }
        }
    );

    // Start Events controller
    let events_controller = Arc::new(EventsController::new(storage.clone(), args.sync_interval));
    spawn_controller!(
        "Events controller",
        leader_elector,
        {
            let controller = events_controller.clone();
            async move {
                controller.run().await;
            }
        }
    );

    // Start ResourceQuota controller
    let resource_quota_controller = Arc::new(ResourceQuotaController::new(storage.clone()));
    let sync_interval_secs2 = args.sync_interval;
    spawn_controller!(
        "ResourceQuota controller",
        leader_elector,
        {
            let controller = resource_quota_controller.clone();
            async move {
                loop {
                    if let Err(e) = controller.reconcile_all().await {
                        tracing::error!("ResourceQuota controller error: {}", e);
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(sync_interval_secs2)).await;
                }
            }
        }
    );

    info!("All controllers started successfully");

    // Keep the main thread alive
    tokio::signal::ctrl_c().await?;
    info!("Shutting down controller manager");

    Ok(())
}
