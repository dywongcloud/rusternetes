use rusternetes_common::{
    resources::{Node, Pod, PriorityClass},
    types::Phase,
};
use rusternetes_storage::{build_prefix, etcd::EtcdStorage, Storage};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tracing::{debug, error, info, warn};

use crate::advanced::{
    calculate_resource_score, check_node_affinity, check_pod_affinity, check_pod_anti_affinity,
    check_preemption, check_taints_tolerations, NodeScore,
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
        let all_pods: Vec<Pod> = self.storage.list(&prefix).await?;

        // Filter pending pods without a node assignment
        let pending_pods: Vec<Pod> = all_pods
            .iter()
            .filter(|p| {
                p.spec.as_ref().and_then(|s| s.node_name.as_ref()).is_none()
                    && p.status
                        .as_ref()
                        .map(|s| s.phase == Phase::Pending)
                        .unwrap_or(true)
            })
            .cloned()
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

        // Load all PriorityClasses for pod priority resolution
        let priority_classes = self.load_priority_classes().await?;

        // Simple round-robin scheduling
        for pod in pending_pods {
            if let Some(node) = self.select_node(&pod, &nodes, &all_pods, &priority_classes) {
                if let Err(e) = self.bind_pod_to_node(pod, &node.metadata.name).await {
                    error!("Failed to bind pod to node: {}", e);
                }
            } else {
                // No suitable node found, try preemption if pod has priority
                if let Some(preemption_result) = self.try_preempt(&pod, &nodes, &all_pods).await {
                    let (node_name, pods_to_evict) = preemption_result;
                    info!(
                        "Preempting {} pods on node {} for high-priority pod {}",
                        pods_to_evict.len(),
                        node_name,
                        pod.metadata.name
                    );

                    // Evict lower-priority pods
                    for pod_name in pods_to_evict {
                        if let Err(e) = self.evict_pod(&pod_name).await {
                            error!("Failed to evict pod {}: {}", pod_name, e);
                        }
                    }

                    // Bind the high-priority pod
                    if let Err(e) = self.bind_pod_to_node(pod, &node_name).await {
                        error!("Failed to bind preempting pod to node: {}", e);
                    }
                } else {
                    warn!("No suitable node found for pod {} (even with preemption)", pod.metadata.name);
                }
            }
        }

        Ok(())
    }

    fn select_node(&self, pod: &Pod, nodes: &[Node], all_pods: &[Pod], priority_classes: &HashMap<String, PriorityClass>) -> Option<Node> {
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

        // Phase 4, 5 & 6: Score nodes based on affinity, pod affinity/anti-affinity, and resources
        let mut node_scores: Vec<NodeScore> = Vec::new();

        for node in selector_matched_nodes {
            // Check node affinity (hard requirements and scoring)
            let (affinity_ok, node_affinity_score) = check_node_affinity(node, pod);
            if !affinity_ok {
                continue; // Skip nodes that don't meet hard affinity requirements
            }

            // Check pod affinity (hard requirements and scoring)
            let (pod_affinity_ok, pod_affinity_score) = check_pod_affinity(node, pod, all_pods);
            if !pod_affinity_ok {
                continue; // Skip nodes that don't meet hard pod affinity requirements
            }

            // Check pod anti-affinity (hard requirements and penalty scoring)
            let (pod_anti_affinity_ok, pod_anti_affinity_penalty) =
                check_pod_anti_affinity(node, pod, all_pods);
            if !pod_anti_affinity_ok {
                continue; // Skip nodes that violate hard pod anti-affinity requirements
            }

            // Calculate resource-based score
            let resource_score = calculate_resource_score(node, pod);

            // If pod doesn't fit resource-wise, skip
            if resource_score == 0 {
                continue;
            }

            // Priority score (resolve from PriorityClass if needed)
            let priority_score = self.get_pod_priority_sync(pod, priority_classes);

            // Combined score:
            // - resource (weight 30%)
            // - node affinity (weight 25%)
            // - pod affinity (weight 20%)
            // - priority (weight 15%)
            // - pod anti-affinity penalty (weight 10%)
            let total_score = (resource_score * 30 / 100)
                + (node_affinity_score * 25 / 100)
                + (pod_affinity_score * 20 / 100)
                + (priority_score * 15 / 100)
                - (pod_anti_affinity_penalty * 10 / 100); // Penalty reduces score

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
                init_container_statuses: None,
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

    /// Try to preempt lower-priority pods to make room for a high-priority pod
    /// Returns Some((node_name, pods_to_evict)) if preemption is possible, None otherwise
    async fn try_preempt(
        &self,
        pod: &Pod,
        nodes: &[Node],
        all_pods: &[Pod],
    ) -> Option<(String, Vec<String>)> {
        // Check each node to see if preemption is possible
        for node in nodes {
            let (can_preempt, pods_to_evict) = check_preemption(node, pod, all_pods);
            if can_preempt && !pods_to_evict.is_empty() {
                return Some((node.metadata.name.clone(), pods_to_evict));
            }
        }
        None
    }

    /// Evict a pod by deleting it from storage
    async fn evict_pod(&self, pod_name: &str) -> rusternetes_common::Result<()> {
        // Find the pod in all namespaces
        let prefix = build_prefix("pods", None);
        let all_pods: Vec<Pod> = self.storage.list(&prefix).await?;

        for pod in all_pods {
            if pod.metadata.name == pod_name {
                let key = rusternetes_storage::build_key(
                    "pods",
                    pod.metadata.namespace.as_deref(),
                    pod_name,
                );
                self.storage.delete(&key).await?;
                info!("Evicted pod {} for preemption", pod_name);
                return Ok(());
            }
        }

        warn!("Pod {} not found for eviction", pod_name);
        Ok(())
    }

    /// Load all PriorityClasses from storage into a HashMap for fast lookup
    async fn load_priority_classes(&self) -> rusternetes_common::Result<HashMap<String, PriorityClass>> {
        let prefix = build_prefix("priorityclasses", None);
        let priority_classes: Vec<PriorityClass> = self.storage.list(&prefix).await?;

        let mut map = HashMap::new();
        for pc in priority_classes {
            map.insert(pc.metadata.name.clone(), pc);
        }

        Ok(map)
    }

    /// Get the priority value for a pod (synchronous version using pre-loaded PriorityClasses)
    /// If pod.spec.priority is set, use it directly
    /// Otherwise, look up the PriorityClass specified by pod.spec.priorityClassName
    /// If neither is set, return 0 (default priority)
    fn get_pod_priority_sync(&self, pod: &Pod, priority_classes: &HashMap<String, PriorityClass>) -> i32 {
        let spec = match pod.spec.as_ref() {
            Some(s) => s,
            None => return 0,
        };

        // If priority is explicitly set, use it
        if let Some(priority) = spec.priority {
            return priority;
        }

        // If priorityClassName is set, look it up
        if let Some(class_name) = &spec.priority_class_name {
            if let Some(priority_class) = priority_classes.get(class_name) {
                debug!(
                    "Resolved priority {} from PriorityClass {} for pod {}",
                    priority_class.value, class_name, pod.metadata.name
                );
                return priority_class.value;
            } else {
                warn!(
                    "PriorityClass {} not found for pod {}, using default priority 0",
                    class_name, pod.metadata.name
                );
                return 0;
            }
        }

        // No priority specified
        0
    }
}
