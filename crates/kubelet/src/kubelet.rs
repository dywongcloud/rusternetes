use crate::runtime::ContainerRuntime;
use anyhow::Result;
use rusternetes_common::{
    resources::{Node, NodeAddress, NodeCondition, NodeStatus, Pod},
    types::Phase,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tracing::{debug, error, info, warn};

pub struct Kubelet {
    node_name: String,
    storage: Arc<dyn Storage>,
    runtime: ContainerRuntime,
    sync_interval: Duration,
}

impl Kubelet {
    pub async fn new(
        node_name: String,
        storage: Arc<dyn Storage>,
        sync_interval_secs: u64,
    ) -> Result<Self> {
        let runtime = ContainerRuntime::new().await?;

        Ok(Self {
            node_name,
            storage,
            runtime,
            sync_interval: Duration::from_secs(sync_interval_secs),
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!("Kubelet started for node: {}", self.node_name);

        // Register the node
        self.register_node().await?;

        let mut interval = tokio::time::interval(self.sync_interval);

        loop {
            interval.tick().await;

            if let Err(e) = self.sync_loop().await {
                error!("Error in sync loop: {}", e);
            }
        }
    }

    async fn register_node(&self) -> Result<()> {
        info!("Registering node: {}", self.node_name);

        let mut node = Node::new(&self.node_name);

        // Set node status
        node.status = Some(NodeStatus {
            capacity: Some(HashMap::from([
                ("cpu".to_string(), "4".to_string()),
                ("memory".to_string(), "8Gi".to_string()),
            ])),
            allocatable: Some(HashMap::from([
                ("cpu".to_string(), "4".to_string()),
                ("memory".to_string(), "8Gi".to_string()),
            ])),
            conditions: Some(vec![NodeCondition {
                condition_type: "Ready".to_string(),
                status: "True".to_string(),
                last_heartbeat_time: Some(chrono::Utc::now()),
                last_transition_time: Some(chrono::Utc::now()),
                reason: Some("KubeletReady".to_string()),
                message: Some("kubelet is posting ready status".to_string()),
            }]),
            addresses: Some(vec![NodeAddress {
                address_type: "InternalIP".to_string(),
                address: "127.0.0.1".to_string(),
            }]),
            node_info: None,
        });

        let key = build_key("nodes", None, &self.node_name);

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

    async fn sync_loop(&self) -> Result<()> {
        debug!("Running sync loop for node: {}", self.node_name);

        // Get all pods assigned to this node
        let all_pods_prefix = build_prefix("pods", None);
        let all_pods: Vec<Pod> = self.storage.list(&all_pods_prefix).await?;

        let node_pods: Vec<Pod> = all_pods
            .into_iter()
            .filter(|p| {
                p.spec
                    .node_name
                    .as_ref()
                    .map(|n| n == &self.node_name)
                    .unwrap_or(false)
            })
            .collect();

        debug!("Found {} pods assigned to this node", node_pods.len());

        for pod in node_pods {
            if let Err(e) = self.sync_pod(&pod).await {
                error!("Error syncing pod {}: {}", pod.metadata.name, e);
            }
        }

        Ok(())
    }

    async fn sync_pod(&self, pod: &Pod) -> Result<()> {
        let pod_name = &pod.metadata.name;
        let namespace = pod
            .metadata
            .namespace
            .as_deref()
            .unwrap_or("default");

        debug!("Syncing pod: {}/{}", namespace, pod_name);

        // Check current status
        let is_running = self.runtime.is_pod_running(pod_name).await?;

        let desired_phase = pod
            .status
            .as_ref()
            .map(|s| &s.phase)
            .unwrap_or(&Phase::Pending);

        match desired_phase {
            Phase::Running if !is_running => {
                info!("Starting pod: {}/{}", namespace, pod_name);
                self.runtime.start_pod(pod).await?;
            }
            Phase::Failed | Phase::Succeeded if is_running => {
                info!("Stopping completed pod: {}/{}", namespace, pod_name);
                self.runtime.stop_pod(pod_name).await?;
            }
            _ => {
                debug!(
                    "Pod {}/{} is in sync (phase: {:?}, running: {})",
                    namespace, pod_name, desired_phase, is_running
                );
            }
        }

        Ok(())
    }
}
