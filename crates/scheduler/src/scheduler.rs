use rusternetes_common::{
    resources::{Node, Pod},
    types::Phase,
};
use rusternetes_storage::{build_prefix, etcd::EtcdStorage, Storage};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tracing::{debug, error, info, warn};

use crate::advanced::{
    calculate_resource_score, check_node_affinity, check_taints_tolerations, NodeScore,
};

pub struct Scheduler {
    storage: Arc<EtcdStorage>,
    interval: Duration,
}

impl Scheduler {
    pub fn new(storage: Arc<EtcdStorage>, interval_secs: u64) -> Self {
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
                p.spec.as_ref().and_then(|s| s.node_name.as_ref()).is_none()
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
        // Advanced scheduling algorithm:
        // 1. Filter out unschedulable nodes
        // 2. Check taints and tolerations
        // 3. Check node selectors
        // 4. Check node affinity
        // 5. Calculate resource scores
        // 6. Select node with highest score

        // Phase 1: Filter schedulable nodes
        let schedulable_nodes: Vec<&Node> = nodes
            .iter()
            .filter(|n| {
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

        // Phase 2: Filter by taints and tolerations
        let tolerated_nodes: Vec<&Node> = schedulable_nodes
            .iter()
            .filter(|node| check_taints_tolerations(node, pod))
            .copied()
            .collect();

        if tolerated_nodes.is_empty() {
            debug!("No nodes tolerate pod taints");
            return None;
        }

        // Phase 3: Check node selectors (basic label matching)
        let selector_matched_nodes: Vec<&Node> = if let Some(node_selector) = pod.spec.as_ref().and_then(|s| s.node_selector.as_ref()) {
            tolerated_nodes
                .iter()
                .filter(|node| self.matches_node_selector(node, node_selector))
                .copied()
                .collect()
        } else {
            tolerated_nodes
        };

        if selector_matched_nodes.is_empty() {
            debug!("No nodes match node selector");
            return None;
        }

        // Phase 4 & 5: Score nodes based on affinity and resources
        let mut node_scores: Vec<NodeScore> = Vec::new();

        for node in selector_matched_nodes {
            // Check node affinity (hard requirements and scoring)
            let (affinity_ok, affinity_score) = check_node_affinity(node, pod);
            if !affinity_ok {
                continue; // Skip nodes that don't meet hard affinity requirements
            }

            // Calculate resource-based score
            let resource_score = calculate_resource_score(node, pod);

            // If pod doesn't fit resource-wise, skip
            if resource_score == 0 {
                continue;
            }

            // Priority score (if specified)
            let priority_score = pod.spec.as_ref().and_then(|s| s.priority).unwrap_or(0);

            // Combined score: resource (weight 40%) + affinity (weight 40%) + priority (weight 20%)
            let total_score = (resource_score * 4 / 10)
                + (affinity_score * 4 / 10)
                + (priority_score * 2 / 10);

            node_scores.push(NodeScore {
                node_name: node.metadata.name.clone(),
                score: total_score,
            });
        }

        if node_scores.is_empty() {
            return None;
        }

        // Sort by score (descending) and select best node
        node_scores.sort_by(|a, b| b.score.cmp(&a.score));

        let best_node_name = &node_scores[0].node_name;
        debug!(
            "Selected node {} with score {} for pod {}",
            best_node_name, node_scores[0].score, pod.metadata.name
        );

        nodes
            .iter()
            .find(|n| &n.metadata.name == best_node_name)
            .cloned()
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
        if let Some(ref mut spec) = pod.spec {
            spec.node_name = Some(node_name.to_string());
        }

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
