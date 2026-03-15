use crate::eviction::{EvictionManager, EvictionSignal, get_node_stats, get_pod_stats};
use crate::runtime::ContainerRuntime;
use anyhow::Result;
use rusternetes_common::{
    resources::{Node, NodeAddress, NodeCondition, NodeSpec, NodeStatus, Pod, PodStatus},
    types::Phase,
};
use rusternetes_storage::{build_key, build_prefix, etcd::EtcdStorage, Storage};
use std::{collections::HashMap, sync::{Arc, Mutex}, time::Duration};
use tracing::{debug, error, info, warn};

pub struct Kubelet {
    node_name: String,
    storage: Arc<EtcdStorage>,
    runtime: ContainerRuntime,
    sync_interval: Duration,
    eviction_manager: Mutex<EvictionManager>,
}

impl Kubelet {
    pub async fn new(
        node_name: String,
        storage: Arc<EtcdStorage>,
        sync_interval_secs: u64,
        volume_dir: String,
        cluster_dns: String,
        cluster_domain: String,
        network: String,
        kubernetes_service_host: String,
    ) -> Result<Self> {
        let runtime = ContainerRuntime::new(volume_dir, cluster_dns, cluster_domain, network, kubernetes_service_host)
            .await?
            .with_storage(storage.clone());

        Ok(Self {
            node_name,
            storage,
            runtime,
            sync_interval: Duration::from_secs(sync_interval_secs),
            eviction_manager: Mutex::new(EvictionManager::new()),
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!("Kubelet started for node: {}", self.node_name);

        // Register the node
        self.register_node().await?;

        let mut interval = tokio::time::interval(self.sync_interval);

        loop {
            interval.tick().await;

            // Update node status
            if let Err(e) = self.update_node_status().await {
                error!("Error updating node status: {}", e);
            }

            // Sync pods
            if let Err(e) = self.sync_loop().await {
                error!("Error in sync loop: {}", e);
            }
        }
    }

    async fn register_node(&self) -> Result<()> {
        info!("Registering node: {}", self.node_name);

        let mut node = Node::new(&self.node_name);

        // Set node spec to mark it as schedulable
        node.spec = Some(NodeSpec {
            pod_cidr: None,
            provider_id: None,
            unschedulable: Some(false),
            taints: None,
        });

        // Set node status
        node.status = Some(NodeStatus {
            capacity: Some(HashMap::from([
                ("cpu".to_string(), "4".to_string()),
                ("memory".to_string(), "8Gi".to_string()),
                ("pods".to_string(), "110".to_string()),
            ])),
            allocatable: Some(HashMap::from([
                ("cpu".to_string(), "4".to_string()),
                ("memory".to_string(), "8Gi".to_string()),
                ("pods".to_string(), "110".to_string()),
            ])),
            conditions: Some(vec![NodeCondition {
                condition_type: "Ready".to_string(),
                status: "True".to_string(),
                last_heartbeat_time: Some(chrono::Utc::now()),
                last_transition_time: Some(chrono::Utc::now()),
                reason: Some("KubeletReady".to_string()),
                message: Some("kubelet is posting ready status".to_string()),
            }]),
            addresses: Some(vec![
                NodeAddress {
                    address_type: "InternalIP".to_string(),
                    address: "127.0.0.1".to_string(),
                },
                NodeAddress {
                    address_type: "Hostname".to_string(),
                    address: self.node_name.clone(),
                },
            ]),
            node_info: None,
        });

        let key = build_key("nodes", None, &self.node_name);

        // Debug: log what we're trying to store
        let node_json = serde_json::to_string_pretty(&node).unwrap_or_else(|_| "failed to serialize".to_string());
        info!("Registering node with spec: {}", node_json);

        // Try to create, if it exists, update it
        match self.storage.create(&key, &node).await {
            Ok(_) => info!("Node registered successfully"),
            Err(rusternetes_common::Error::AlreadyExists(_)) => {
                self.storage.update(&key, &node).await?;
                info!("Node updated successfully");
            }
            Err(e) => return Err(e.into()),
        }

        Ok(())
    }

    async fn update_node_status(&self) -> Result<()> {
        debug!("Updating node status");

        let key = build_key("nodes", None, &self.node_name);
        let mut node: Node = self.storage.get(&key).await?;

        // Get current node resource statistics
        let node_stats = get_node_stats();

        // Check if eviction is needed
        let mut eviction_manager = self.eviction_manager.lock().unwrap();
        let active_signals = eviction_manager.check_eviction_needed(&node_stats);

        // Update node conditions based on resource pressure
        if !active_signals.is_empty() {
            info!("Resource pressure detected: {:?}", active_signals);
            eviction_manager.update_node_conditions(&mut node, &active_signals)?;
        } else {
            // Clear pressure conditions if no active signals
            eviction_manager.update_node_conditions(&mut node, &[])?;
        }
        drop(eviction_manager); // Release lock before async operations

        // Update heartbeat
        if let Some(ref mut status) = node.status {
            if let Some(ref mut conditions) = status.conditions {
                for condition in conditions.iter_mut() {
                    if condition.condition_type == "Ready" {
                        condition.last_heartbeat_time = Some(chrono::Utc::now());
                    }
                }
            }
        }

        self.storage.update(&key, &node).await?;

        // If eviction is needed, trigger pod eviction
        if !active_signals.is_empty() {
            if let Err(e) = self.handle_eviction(&active_signals).await {
                error!("Error handling eviction: {}", e);
            }
        }

        Ok(())
    }

    async fn sync_loop(&self) -> Result<()> {
        debug!("Running sync loop for node: {}", self.node_name);

        // Get all pods assigned to this node
        let all_pods_prefix = build_prefix("pods", None);
        let all_pods: Vec<Pod> = self.storage.list(&all_pods_prefix).await?;

        let node_pods: Vec<Pod> = all_pods
            .into_iter()
            .filter(|p| {
                p.spec
                    .as_ref()
                    .and_then(|s| s.node_name.as_ref())
                    .map(|n| n == &self.node_name)
                    .unwrap_or(false)
            })
            .collect();

        debug!("Found {} pods assigned to this node", node_pods.len());

        // Sync all pods that should be running
        for pod in &node_pods {
            if let Err(e) = self.sync_pod(pod).await {
                error!("Error syncing pod {}: {}", pod.metadata.name, e);

                // Update pod status to reflect error
                if let Err(update_err) = self.update_pod_status_error(pod, &e.to_string()).await {
                    error!("Failed to update pod status: {}", update_err);
                }
            }
        }

        // Clean up orphaned containers (containers whose pods have been deleted from etcd)
        if let Err(e) = self.cleanup_orphaned_containers(&node_pods).await {
            error!("Error cleaning up orphaned containers: {}", e);
        }

        Ok(())
    }

    async fn cleanup_orphaned_containers(&self, _current_pods: &[Pod]) -> Result<()> {
        debug!("Checking for orphaned containers");

        // Get all pods from etcd (across all namespaces)
        let all_pods_prefix = build_prefix("pods", None);
        let all_existing_pods: Vec<Pod> = self.storage.list(&all_pods_prefix).await?;

        let existing_pod_names: std::collections::HashSet<String> = all_existing_pods
            .iter()
            .map(|p| p.metadata.name.clone())
            .collect();

        debug!("Found {} pods in etcd", existing_pod_names.len());

        // Get list of running pod names from the container runtime
        let running_pods = self.runtime.list_running_pods().await?;
        debug!("Found {} running pods in container runtime", running_pods.len());

        // Check for orphaned pods (running in container runtime but not in etcd)
        for running_pod_name in running_pods {
            if !existing_pod_names.contains(&running_pod_name) {
                info!("Found orphaned pod {} - not in etcd, stopping containers", running_pod_name);
                if let Err(e) = self.runtime.stop_pod(&running_pod_name).await {
                    warn!("Failed to stop orphaned pod {}: {}", running_pod_name, e);
                }
            }
        }

        Ok(())
    }

    async fn sync_pod(&self, pod: &Pod) -> Result<()> {
        let pod_name = &pod.metadata.name;
        let namespace = pod.metadata.namespace.as_deref().unwrap_or("default");

        debug!("Syncing pod: {}/{}", namespace, pod_name);

        // Check current runtime status
        let is_running = self.runtime.is_pod_running(pod_name).await?;

        // Get current phase from pod status
        let current_phase = pod
            .status
            .as_ref()
            .and_then(|s| s.phase.as_ref())
            .unwrap_or(&Phase::Pending);

        match current_phase {
            // If pod is Pending and has been scheduled to this node, start it
            Phase::Pending if !is_running => {
                info!("Starting pod: {}/{}", namespace, pod_name);

                // Update status to indicate we're starting
                self.update_pod_status(pod, Phase::Pending, Some("ContainerCreating"), None).await?;

                // Start the pod
                match self.runtime.start_pod(pod).await {
                    Ok(_) => {
                        info!("Pod {}/{} started successfully", namespace, pod_name);

                        // Get container statuses
                        let container_statuses = self.runtime.get_container_statuses(pod).await.ok();

                        // Get pod IP
                        let pod_ip = self.runtime.get_pod_ip(pod_name).await.ok().flatten();

                        // Update status to Running
                        let mut new_pod = pod.clone();
                        new_pod.status = Some(PodStatus {
                            phase: Some(Phase::Running),
                            message: Some("All containers started".to_string()),
                            reason: None,
                            host_ip: Some("127.0.0.1".to_string()),
                            pod_ip,
                            container_statuses,
                            init_container_statuses: None,
                            ephemeral_container_statuses: None,
                        });

                        let key = build_key("pods", new_pod.metadata.namespace.as_deref(), &new_pod.metadata.name);
                        self.storage.update(&key, &new_pod).await?;
                    }
                    Err(e) => {
                        error!("Failed to start pod {}/{}: {}", namespace, pod_name, e);
                        self.update_pod_status(pod, Phase::Failed, Some("FailedToStart"), Some(&e.to_string())).await?;
                    }
                }
            }
            // If pod is Pending but containers are already running, update to Running
            Phase::Pending if is_running => {
                info!("Pod {}/{} containers are running, updating status to Running", namespace, pod_name);

                // Get container statuses
                let container_statuses = self.runtime.get_container_statuses(pod).await.ok();

                // Get pod IP
                let pod_ip = self.runtime.get_pod_ip(pod_name).await.ok().flatten();

                // Update status to Running
                let mut new_pod = pod.clone();
                new_pod.status = Some(PodStatus {
                    phase: Some(Phase::Running),
                    message: Some("All containers started".to_string()),
                    reason: None,
                    host_ip: Some("127.0.0.1".to_string()),
                    pod_ip,
                    container_statuses,
                    init_container_statuses: None,
                    ephemeral_container_statuses: None,
                });

                let key = build_key("pods", new_pod.metadata.namespace.as_deref(), &new_pod.metadata.name);
                self.storage.update(&key, &new_pod).await?;
            }
            Phase::Running if is_running => {
                debug!("Pod {}/{} is running, checking health", namespace, pod_name);

                // Check liveness probes
                if let Ok(needs_restart) = self.runtime.check_liveness(pod).await {
                    if needs_restart {
                        let restart_policy = pod.spec.as_ref().and_then(|s| s.restart_policy.as_deref()).unwrap_or("Always");

                        match restart_policy {
                            "Always" | "OnFailure" => {
                                warn!("Restarting pod {}/{} due to failed liveness probe", namespace, pod_name);

                                // Stop and restart the pod
                                if let Err(e) = self.runtime.stop_pod(pod_name).await {
                                    error!("Failed to stop pod for restart: {}", e);
                                } else {
                                    // Update status
                                    self.update_pod_status(pod, Phase::Pending, Some("Restarting"), Some("Liveness probe failed")).await?;

                                    // Start again
                                    if let Err(e) = self.runtime.start_pod(pod).await {
                                        error!("Failed to restart pod: {}", e);
                                        self.update_pod_status(pod, Phase::Failed, Some("FailedToRestart"), Some(&e.to_string())).await?;
                                    }
                                }
                            }
                            "Never" => {
                                warn!("Liveness probe failed but restart policy is Never for pod {}/{}", namespace, pod_name);
                                self.update_pod_status(pod, Phase::Failed, Some("LivenessProbeFailedterm"), Some("Restart policy is Never")).await?;
                            }
                            _ => {}
                        }
                    } else {
                        // Update container statuses with readiness info
                        if let Ok(container_statuses) = self.runtime.get_container_statuses(pod).await {
                            let all_ready = container_statuses.iter().all(|s| s.ready);

                            // Get pod IP (important for pods started by docker-compose)
                            let pod_ip = self.runtime.get_pod_ip(pod_name).await.ok().flatten();

                            let mut new_pod = pod.clone();
                            if let Some(ref mut status) = new_pod.status {
                                status.container_statuses = Some(container_statuses);
                                // Update pod IP if we got one and it's different
                                if pod_ip.is_some() && status.pod_ip != pod_ip {
                                    status.pod_ip = pod_ip;
                                }
                                if all_ready {
                                    status.message = Some("All containers ready".to_string());
                                } else {
                                    status.message = Some("Some containers not ready".to_string());
                                }
                            }

                            let key = build_key("pods", new_pod.metadata.namespace.as_deref(), &new_pod.metadata.name);

                            if let Err(e) = self.storage.update(&key, &new_pod).await {
                                debug!("Failed to update pod status: {}", e);
                            }
                        }
                    }
                }
            }
            Phase::Succeeded | Phase::Failed => {
                if is_running {
                    info!("Stopping completed pod: {}/{}", namespace, pod_name);
                    self.runtime.stop_pod(pod_name).await?;
                }
            }
            _ => {
                debug!(
                    "Pod {}/{} is in sync (phase: {:?}, running: {})",
                    namespace, pod_name, current_phase, is_running
                );
            }
        }

        Ok(())
    }

    async fn update_pod_status(
        &self,
        pod: &Pod,
        phase: Phase,
        reason: Option<&str>,
        message: Option<&str>,
    ) -> Result<()> {
        let mut new_pod = pod.clone();

        new_pod.status = Some(PodStatus {
            phase: Some(phase),
            message: message.map(|s| s.to_string()),
            reason: reason.map(|s| s.to_string()),
            host_ip: Some("127.0.0.1".to_string()),
            pod_ip: None,
            container_statuses: None,
            init_container_statuses: None,
            ephemeral_container_statuses: None,
        });

        let key = build_key("pods", new_pod.metadata.namespace.as_deref(), &new_pod.metadata.name);
        self.storage.update(&key, &new_pod).await?;

        Ok(())
    }

    async fn update_pod_status_error(&self, pod: &Pod, error: &str) -> Result<()> {
        self.update_pod_status(pod, Phase::Failed, Some("Error"), Some(error)).await
    }

    /// Handle pod eviction when node resources are exhausted
    async fn handle_eviction(&self, signals: &[EvictionSignal]) -> Result<()> {
        info!("Handling eviction for signals: {:?}", signals);

        // Get all pods assigned to this node
        let all_pods_prefix = build_prefix("pods", None);
        let all_pods: Vec<Pod> = self.storage.list(&all_pods_prefix).await?;

        let node_pods: Vec<Pod> = all_pods
            .into_iter()
            .filter(|p| {
                p.spec
                    .as_ref()
                    .and_then(|s| s.node_name.as_ref())
                    .map(|n| n == &self.node_name)
                    .unwrap_or(false)
            })
            .filter(|p| {
                // Only consider running pods for eviction
                p.status
                    .as_ref()
                    .map(|s| s.phase == Some(Phase::Running))
                    .unwrap_or(false)
            })
            .collect();

        // Get pod resource usage statistics
        let pod_stats = get_pod_stats(&node_pods);

        // For each active signal, select pods for eviction
        for signal in signals {
            let eviction_manager = self.eviction_manager.lock().unwrap();
            let pods_to_evict = eviction_manager.select_pods_for_eviction(
                &node_pods,
                &pod_stats,
                signal,
            );
            drop(eviction_manager); // Release lock

            for pod_key in pods_to_evict {
                warn!("Evicting pod {} due to resource pressure ({:?})", pod_key, signal);

                // Parse namespace and name from key
                let parts: Vec<&str> = pod_key.split('/').collect();
                if parts.len() != 2 {
                    continue;
                }
                let namespace = parts[0];
                let name = parts[1];

                // Find the pod
                if let Some(pod) = node_pods.iter().find(|p| {
                    p.metadata.namespace.as_deref().unwrap_or("default") == namespace
                        && p.metadata.name == name
                }) {
                    // Stop the pod
                    if let Err(e) = self.runtime.stop_pod(&pod.metadata.name).await {
                        error!("Failed to stop evicted pod {}: {}", pod_key, e);
                        continue;
                    }

                    // Update pod status to reflect eviction
                    if let Err(e) = self.update_pod_status(
                        pod,
                        Phase::Failed,
                        Some("Evicted"),
                        Some(&format!("Pod evicted due to resource pressure: {:?}", signal)),
                    )
                    .await
                    {
                        error!("Failed to update evicted pod status: {}", e);
                    }

                    info!("Successfully evicted pod {}", pod_key);
                }
            }
        }

        Ok(())
    }
}
