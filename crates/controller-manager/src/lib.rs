// Library interface for controller-manager
pub mod controllers;
pub use controllers::*;

use controllers::{
    certificate_signing_request::CertificateSigningRequestController, crd::CRDController,
    cronjob::CronJobController, daemonset::DaemonSetController, deployment::DeploymentController,
    dynamic_provisioner::DynamicProvisionerController, endpoints::EndpointsController,
    endpointslice::EndpointSliceController, events::EventsController,
    garbage_collector::GarbageCollector, hpa::HorizontalPodAutoscalerController,
    ingress::IngressController, job::JobController, loadbalancer::LoadBalancerController,
    namespace::NamespaceController, network_policy::NetworkPolicyController, node::NodeController,
    pod_disruption_budget::PodDisruptionBudgetController, pv_binder::PVBinderController,
    replicaset::ReplicaSetController, replicationcontroller::ReplicationControllerController,
    resource_quota::ResourceQuotaController, service::ServiceController,
    serviceaccount::ServiceAccountController, statefulset::StatefulSetController,
    ttl_controller::TTLController, volume_expansion::VolumeExpansionController,
    volume_snapshot::VolumeSnapshotController, vpa::VerticalPodAutoscalerController,
};
use rusternetes_storage::StorageBackend;
use std::sync::Arc;
use tracing::{error, info};

/// Configuration for the controller-manager component.
pub struct ControllerManagerConfig {
    pub sync_interval: u64,
}

/// Run the controller-manager component.
///
/// Spawns all controllers as tokio tasks and waits for ctrl-c.
pub async fn run(storage: Arc<StorageBackend>, config: ControllerManagerConfig) -> anyhow::Result<()> {
    info!("Starting Rusternetes Controller Manager");

    let interval = config.sync_interval;

    // No leader election in all-in-one mode — single instance
    let cloud_provider: Option<Arc<dyn rusternetes_common::cloud_provider::CloudProvider>> = None;

    // Spawn all controllers
    let s = storage.clone();
    tokio::spawn(async move {
        let c = LoadBalancerController::new(s, cloud_provider, "rusternetes".to_string(), interval);
        if let Err(e) = c.run().await { error!("LoadBalancer controller error: {}", e); }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = DeploymentController::new(s, interval);
        if let Err(e) = c.run().await { error!("Deployment controller error: {}", e); }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = ReplicationControllerController::new(s, interval);
        if let Err(e) = c.run().await { error!("ReplicationController controller error: {}", e); }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = ReplicaSetController::new(s, interval);
        if let Err(e) = c.run().await { error!("ReplicaSet controller error: {}", e); }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = StatefulSetController::new(s);
        if let Err(e) = c.run().await { error!("StatefulSet controller error: {}", e); }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = DaemonSetController::new(s);
        if let Err(e) = c.run().await { error!("DaemonSet controller error: {}", e); }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = JobController::new(s);
        if let Err(e) = c.run().await { error!("Job controller error: {}", e); }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = CronJobController::new(s);
        if let Err(e) = c.run().await { error!("CronJob controller error: {}", e); }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = PVBinderController::new(s);
        if let Err(e) = c.run().await { error!("PV/PVC Binder controller error: {}", e); }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = DynamicProvisionerController::new(s);
        if let Err(e) = c.run().await { error!("Dynamic Provisioner controller error: {}", e); }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = VolumeSnapshotController::new(s);
        if let Err(e) = c.run().await { error!("Volume Snapshot controller error: {}", e); }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = VolumeExpansionController::new(s);
        if let Err(e) = c.run().await { error!("Volume Expansion controller error: {}", e); }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = EndpointsController::new(s);
        loop {
            if let Err(e) = c.reconcile_all().await { error!("Endpoints controller error: {}", e); }
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = EndpointSliceController::new(s);
        loop {
            if let Err(e) = c.reconcile_all().await { error!("EndpointSlice controller error: {}", e); }
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = Arc::new(EventsController::new(s, interval));
        c.run().await;
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = ResourceQuotaController::new(s);
        loop {
            if let Err(e) = c.reconcile_all().await { error!("ResourceQuota controller error: {}", e); }
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = GarbageCollector::new(s);
        c.run().await;
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = HorizontalPodAutoscalerController::new(s);
        if let Err(e) = c.run().await { error!("HPA controller error: {}", e); }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = VerticalPodAutoscalerController::new(s);
        c.run().await;
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = TTLController::new(s);
        c.run().await;
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = PodDisruptionBudgetController::new(s);
        if let Err(e) = c.run().await { error!("PodDisruptionBudget controller error: {}", e); }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = NetworkPolicyController::new(s);
        loop {
            if let Err(e) = c.reconcile_all().await { error!("NetworkPolicy controller error: {}", e); }
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = IngressController::new(s);
        loop {
            if let Err(e) = c.reconcile_all().await { error!("Ingress controller error: {}", e); }
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = CertificateSigningRequestController::new(s);
        loop {
            if let Err(e) = c.reconcile_all().await { error!("CertificateSigningRequest controller error: {}", e); }
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = CRDController::new(s);
        loop {
            if let Err(e) = c.reconcile_all().await { error!("CRD controller error: {}", e); }
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = NamespaceController::new(s);
        loop {
            if let Err(e) = c.reconcile_all().await { error!("Namespace controller error: {}", e); }
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = controllers::taint_eviction::TaintEvictionController::new(s);
        loop {
            if let Err(e) = c.reconcile_all().await { error!("TaintEviction controller error: {}", e); }
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = ServiceAccountController::new(s);
        loop {
            if let Err(e) = c.reconcile_all().await { error!("ServiceAccount controller error: {}", e); }
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = ServiceController::new(s);
        if let Err(e) = c.initialize().await {
            error!("Service controller initialization error: {}", e);
            return;
        }
        loop {
            if let Err(e) = c.reconcile_all().await { error!("Service controller error: {}", e); }
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        }
    });

    let s = storage.clone();
    tokio::spawn(async move {
        let c = NodeController::new(s);
        loop {
            if let Err(e) = c.reconcile_all().await { error!("Node controller error: {}", e); }
            tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        }
    });

    info!("All controllers started successfully");

    // Keep alive until shutdown
    tokio::signal::ctrl_c().await?;
    info!("Shutting down controller manager");

    Ok(())
}
