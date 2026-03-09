use rusternetes_common::{
    resources::{Node, Pod},
    types::Phase,
};
use rusternetes_storage::{build_prefix, Storage};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tracing::{debug, error, info, warn};

pub struct Scheduler {
    storage: Arc<dyn Storage>,
    interval: Duration,
}

impl Scheduler {
    pub fn new(storage: Arc<dyn Storage>, interval_secs: u64) -> Self {
        Self {
            storage,
            interval: Duration::from_secs(interval_secs),
        }
    }

    pub async fn run(&self) -> rusternetes_common::Result<()> {
        info!("Scheduler started, running every {:?}", self.interval);

        let mut interval = tokio::time::interval(self.interval);

        loop {
            interval.tick().await;
            if let Err(e) = self.schedule_pending_pods().await {
                error!("Error scheduling pods: {}", e);
            }
        }
    }

    async fn schedule_pending_pods(&self) -> rusternetes_common::Result<()> {
        debug!("Looking for pending pods to schedule");

        // Get all pods
        let prefix = build_prefix("pods", None);
        let pods: Vec<Pod> = self.storage.list(&prefix).await?;

        // Filter pending pods without a node assignment
        let pending_pods: Vec<Pod> = pods
            .into_iter()
            .filter(|p| {
                p.spec.node_name.is_none()
                    && p.status
                        .as_ref()
                        .map(|s| s.phase == Phase::Pending)
                        .unwrap_or(true)
            })
            .collect();

        if pending_pods.is_empty() {
            debug!("No pending pods to schedule");
            return Ok(());
        }

        info!("Found {} pending pods to schedule", pending_pods.len());

        // Get all nodes
        let nodes_prefix = build_prefix("nodes", None);
        let nodes: Vec<Node> = self.storage.list(&nodes_prefix).await?;

        if nodes.is_empty() {
            warn!("No nodes available for scheduling");
            return Ok(());
        }

        // Simple round-robin scheduling
        for pod in pending_pods {
            if let Some(node) = self.select_node(&pod, &nodes) {
                if let Err(e) = self.bind_pod_to_node(pod, &node.metadata.name).await {
                    error!("Failed to bind pod to node: {}", e);
                }
            } else {
                warn!("No suitable node found for pod {}", pod.metadata.name);
            }
        }

        Ok(())
    }

    fn select_node(&self, pod: &Pod, nodes: &[Node]) -> Option<Node> {
        // Simple scheduling algorithm:
        // 1. Filter out unschedulable nodes
        // 2. Check node selectors
        // 3. Select first available node (round-robin would be better in production)

        let schedulable_nodes: Vec<&Node> = nodes
            .iter()
            .filter(|n| {
                // Check if node is schedulable
                n.spec
                    .as_ref()
                    .and_then(|s| s.unschedulable)
                    .unwrap_or(false)
                    == false
            })
            .collect();

        if schedulable_nodes.is_empty() {
            return None;
        }

        // Check node selectors if specified
        if let Some(node_selector) = &pod.spec.node_selector {
            for node in schedulable_nodes {
                if self.matches_node_selector(node, node_selector) {
                    return Some(node.clone());
                }
            }
            return None;
        }

        // Return first available node
        schedulable_nodes.first().map(|n| (*n).clone())
    }

    fn matches_node_selector(&self, node: &Node, selector: &HashMap<String, String>) -> bool {
        let node_labels = node.metadata.labels.as_ref();

        if node_labels.is_none() {
            return selector.is_empty();
        }

        let labels = node_labels.unwrap();

        for (key, value) in selector {
            if labels.get(key) != Some(value) {
                return false;
            }
        }

        true
    }

    async fn bind_pod_to_node(
        &self,
        mut pod: Pod,
        node_name: &str,
    ) -> rusternetes_common::Result<()> {
        info!(
            "Binding pod {}/{} to node {}",
            pod.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
            pod.metadata.name,
            node_name
        );

        // Update pod spec with node name
        pod.spec.node_name = Some(node_name.to_string());

        // Update pod status to Running
        if let Some(ref mut status) = pod.status {
            status.phase = Phase::Running;
        } else {
            pod.status = Some(rusternetes_common::resources::PodStatus {
                phase: Phase::Running,
                message: Some("Pod scheduled".to_string()),
                reason: None,
                host_ip: None,
                pod_ip: None,
                container_statuses: None,
            });
        }

        // Update pod in storage
        let key = rusternetes_storage::build_key(
            "pods",
            pod.metadata.namespace.as_deref(),
            &pod.metadata.name,
        );
        self.storage.update(&key, &pod).await?;

        info!("Successfully bound pod to node {}", node_name);
        Ok(())
    }
}
