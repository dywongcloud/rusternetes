use rusternetes_common::{
    resources::{Node, Pod, PriorityClass},
    types::Phase,
};
use rusternetes_storage::{build_prefix, etcd::EtcdStorage, Storage};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tracing::{debug, error, info, warn};

use crate::advanced::{
    calculate_resource_score, check_node_affinity, check_pod_affinity, check_pod_anti_affinity,
    check_preemption, check_taints_tolerations, check_topology_spread_constraints, NodeScore,
};

pub struct Scheduler {
    storage: Arc<EtcdStorage>,
    interval: Duration,
    /// Name of this scheduler (default "default-scheduler")
    scheduler_name: String,
}

impl Scheduler {
    pub fn new(storage: Arc<EtcdStorage>, interval_secs: u64) -> Self {
        Self::new_with_name(storage, interval_secs, "default-scheduler".to_string())
    }

    pub fn new_with_name(
        storage: Arc<EtcdStorage>,
        interval_secs: u64,
        scheduler_name: String,
    ) -> Self {
        Self {
            storage,
            interval: Duration::from_secs(interval_secs),
            scheduler_name,
        }
    }

    pub async fn run(&self) -> rusternetes_common::Result<()> {
        info!(
            "Scheduler '{}' started, running every {:?}",
            self.scheduler_name, self.interval
        );

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

        // Filter pending pods without a node assignment that are assigned to this scheduler
        let pending_pods: Vec<Pod> = all_pods
            .iter()
            .filter(|p| {
                // Check if pod is pending and unscheduled
                // A pod is considered pending if:
                // 1. It has no node assignment, AND
                // 2. Either it has no status, OR phase is None, OR phase is Pending
                let is_pending = p.spec.as_ref().and_then(|s| s.node_name.as_ref()).is_none()
                    && p.status
                        .as_ref()
                        .map(|s| s.phase.is_none() || s.phase == Some(Phase::Pending))
                        .unwrap_or(true);

                if !is_pending {
                    return false;
                }

                // Check if pod is assigned to this scheduler
                // If schedulerName is not specified, defaults to "default-scheduler"
                let pod_scheduler_name = p
                    .spec
                    .as_ref()
                    .and_then(|s| s.scheduler_name.as_deref())
                    .unwrap_or("default-scheduler");

                pod_scheduler_name == self.scheduler_name
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
            if let Some(node) = self
                .select_node(&pod, &nodes, &all_pods, &priority_classes)
                .await
            {
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
                    warn!(
                        "No suitable node found for pod {} (even with preemption)",
                        pod.metadata.name
                    );
                }
            }
        }

        Ok(())
    }

    async fn select_node(
        &self,
        pod: &Pod,
        nodes: &[Node],
        all_pods: &[Pod],
        priority_classes: &HashMap<String, PriorityClass>,
    ) -> Option<Node> {
        // Advanced scheduling algorithm:
        // 1. Filter out unschedulable nodes
        // 2. Check taints and tolerations
        // 3. Check node selectors
        // 4. Check DRA device availability
        // 5. Check node affinity
        // 6. Calculate resource scores
        // 7. Select node with highest score

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
        let selector_matched_nodes: Vec<&Node> =
            if let Some(node_selector) = pod.spec.as_ref().and_then(|s| s.node_selector.as_ref()) {
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

        // Phase 4: Check DRA device availability
        // Filter nodes that have required devices for ResourceClaims
        let mut dra_matched_nodes: Vec<&Node> = Vec::new();
        for node in selector_matched_nodes {
            if self.check_dra_device_availability(node, pod).await {
                dra_matched_nodes.push(node);
            } else {
                debug!(
                    "Node {} does not have required DRA devices for pod {}",
                    node.metadata.name, pod.metadata.name
                );
            }
        }

        if dra_matched_nodes.is_empty() {
            debug!("No nodes have required DRA devices");
            return None;
        }

        // Phase 5, 6 & 7: Score nodes based on affinity, pod affinity/anti-affinity, topology spread, and resources
        let mut node_scores: Vec<NodeScore> = Vec::new();

        for node in dra_matched_nodes {
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

            // Check topology spread constraints (hard requirements and penalty scoring)
            let (topology_ok, topology_penalty) =
                check_topology_spread_constraints(node, pod, all_pods, nodes);
            if !topology_ok {
                continue; // Skip nodes that violate hard topology spread constraints
            }

            // Calculate resource-based score (accounting for pod overhead if specified)
            let resource_score = self.calculate_resource_score_with_overhead(node, pod);

            // If pod doesn't fit resource-wise, skip
            if resource_score == 0 {
                continue;
            }

            // Priority score (resolve from PriorityClass if needed)
            let priority_score = self.get_pod_priority_sync(pod, priority_classes);

            // Combined score:
            // - resource (weight 25%)
            // - node affinity (weight 20%)
            // - pod affinity (weight 18%)
            // - priority (weight 15%)
            // - pod anti-affinity penalty (weight 12%)
            // - topology spread penalty (weight 10%)
            let total_score = (resource_score * 25 / 100)
                + (node_affinity_score * 20 / 100)
                + (pod_affinity_score * 18 / 100)
                + (priority_score * 15 / 100)
                - (pod_anti_affinity_penalty * 12 / 100) // Penalty reduces score
                - (topology_penalty * 10 / 100); // Penalty reduces score

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
            pod.metadata
                .namespace
                .as_ref()
                .unwrap_or(&"default".to_string()),
            pod.metadata.name,
            node_name
        );

        // Update pod spec with node name
        if let Some(ref mut spec) = pod.spec {
            spec.node_name = Some(node_name.to_string());
        }

        // Update pod status to Pending (kubelet will update to Running after starting containers)
        if let Some(ref mut status) = pod.status {
            status.phase = Some(Phase::Pending);
            status.message = Some("Pod scheduled".to_string());
        } else {
            pod.status = Some(rusternetes_common::resources::PodStatus {
                phase: Some(Phase::Pending),
                message: Some("Pod scheduled".to_string()),
                reason: None,
                host_ip: None,
                pod_ip: None,
                container_statuses: None,
                init_container_statuses: None,
                ephemeral_container_statuses: None,
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
    async fn load_priority_classes(
        &self,
    ) -> rusternetes_common::Result<HashMap<String, PriorityClass>> {
        let prefix = build_prefix("priorityclasses", None);
        let priority_classes: Vec<PriorityClass> = self.storage.list(&prefix).await?;

        let mut map = HashMap::new();
        for pc in priority_classes {
            map.insert(pc.metadata.name.clone(), pc);
        }

        Ok(map)
    }

    /// Calculate resource score with pod overhead
    /// Pod overhead represents additional resources required beyond container requests
    fn calculate_resource_score_with_overhead(&self, node: &Node, pod: &Pod) -> i32 {
        use crate::advanced::calculate_resource_score;

        // Get base resource score
        let base_score = calculate_resource_score(node, pod);

        // If no overhead specified, return base score
        let overhead = match &pod.spec {
            Some(spec) => match &spec.overhead {
                Some(o) => o,
                None => return base_score,
            },
            None => return base_score,
        };

        // Parse overhead resources
        let mut cpu_overhead = 0i64;
        let mut memory_overhead = 0i64;

        if let Some(cpu) = overhead.get("cpu") {
            cpu_overhead = self.parse_resource_quantity(cpu, "cpu");
        }
        if let Some(memory) = overhead.get("memory") {
            memory_overhead = self.parse_resource_quantity(memory, "memory");
        }

        // Get node allocatable resources
        let allocatable = match &node.status {
            Some(status) => match &status.allocatable {
                Some(a) => a,
                None => return base_score,
            },
            None => return base_score,
        };

        let available_cpu = allocatable
            .get("cpu")
            .map(|s| self.parse_resource_quantity(s, "cpu"))
            .unwrap_or(0);
        let available_memory = allocatable
            .get("memory")
            .map(|s| self.parse_resource_quantity(s, "memory"))
            .unwrap_or(0);

        // Check if overhead alone would prevent scheduling
        if cpu_overhead > available_cpu || memory_overhead > available_memory {
            return 0; // Can't schedule
        }

        // Reduce the base score proportionally to overhead impact
        let cpu_overhead_ratio = if available_cpu > 0 {
            (cpu_overhead * 100 / available_cpu) as i32
        } else {
            0
        };
        let memory_overhead_ratio = if available_memory > 0 {
            (memory_overhead * 100 / available_memory) as i32
        } else {
            0
        };

        let overhead_penalty = (cpu_overhead_ratio + memory_overhead_ratio) / 2;

        // Return score minus overhead penalty (but not less than 0)
        (base_score - overhead_penalty).max(0)
    }

    /// Parse resource quantity (helper method)
    fn parse_resource_quantity(&self, quantity: &str, resource_type: &str) -> i64 {
        let quantity = quantity.trim();

        if resource_type == "cpu" {
            // CPU: support m (millicores) and plain numbers
            if let Some(stripped) = quantity.strip_suffix('m') {
                stripped.parse().unwrap_or(0)
            } else {
                quantity.parse::<i64>().unwrap_or(0) * 1000
            }
        } else {
            // Memory: support Ki, Mi, Gi
            if let Some(stripped) = quantity.strip_suffix("Ki") {
                stripped.parse::<i64>().unwrap_or(0) * 1024
            } else if let Some(stripped) = quantity.strip_suffix("Mi") {
                stripped.parse::<i64>().unwrap_or(0) * 1024 * 1024
            } else if let Some(stripped) = quantity.strip_suffix("Gi") {
                stripped.parse::<i64>().unwrap_or(0) * 1024 * 1024 * 1024
            } else {
                quantity.parse().unwrap_or(0)
            }
        }
    }

    /// Get the priority value for a pod (synchronous version using pre-loaded PriorityClasses)
    /// If pod.spec.priority is set, use it directly
    /// Otherwise, look up the PriorityClass specified by pod.spec.priorityClassName
    /// If neither is set, return 0 (default priority)
    fn get_pod_priority_sync(
        &self,
        pod: &Pod,
        priority_classes: &HashMap<String, PriorityClass>,
    ) -> i32 {
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

    // DRA (Dynamic Resource Allocation) Integration Methods

    /// Check if node has available devices for DRA ResourceClaims
    /// Returns true if all required devices are available on the node, or if no resource claims are specified
    async fn check_dra_device_availability(&self, node: &Node, pod: &Pod) -> bool {
        use rusternetes_common::resources::{DeviceClass, ResourceClaim, ResourceSlice};

        // Extract resourceClaims from pod.spec
        let spec = match &pod.spec {
            Some(s) => s,
            None => return true, // No spec, no claims to check
        };

        let resource_claims_refs = match &spec.resource_claims {
            Some(claims) => claims,
            None => return true, // No resource claims, all nodes are suitable
        };

        if resource_claims_refs.is_empty() {
            return true;
        }

        let pod_namespace = pod.metadata.namespace.as_deref().unwrap_or("default");

        // For each claim reference, resolve the ResourceClaim object
        for claim_ref in resource_claims_refs {
            let claim_name = match &claim_ref.source {
                Some(source) => {
                    if let Some(name) = &source.resource_claim_name {
                        name.as_str()
                    } else if let Some(template_name) = &source.resource_claim_template_name {
                        // TODO: In a full implementation, we'd need to resolve the template
                        // and create a ResourceClaim from it. For now, we'll treat the template
                        // name as the claim name (simplified)
                        debug!(
                            "ResourceClaimTemplate '{}' referenced, treating as claim name",
                            template_name
                        );
                        template_name.as_str()
                    } else {
                        warn!("ResourceClaim reference has no name or template");
                        return false;
                    }
                }
                None => {
                    warn!("ResourceClaim reference has no source");
                    return false;
                }
            };

            // Get the ResourceClaim from storage
            let claim_key =
                rusternetes_storage::build_key("resourceclaims", Some(pod_namespace), claim_name);

            let claim: ResourceClaim = match self.storage.get(&claim_key).await {
                Ok(c) => c,
                Err(e) => {
                    warn!(
                        "Failed to get ResourceClaim {}/{}: {}",
                        pod_namespace, claim_name, e
                    );
                    return false;
                }
            };

            // Check if the claim is allocated
            let allocation = match &claim.status {
                Some(status) => match &status.allocation {
                    Some(alloc) => alloc,
                    None => {
                        debug!(
                            "ResourceClaim {}/{} is not yet allocated",
                            pod_namespace, claim_name
                        );
                        return false; // Claim not allocated yet
                    }
                },
                None => {
                    debug!(
                        "ResourceClaim {}/{} has no status",
                        pod_namespace, claim_name
                    );
                    return false;
                }
            };

            // Check if the allocation has a node selector and if this node matches
            if let Some(node_selector) = &allocation.node_selector {
                // Check if node matches the node selector
                let node_labels = node.metadata.labels.as_ref();
                if let Some(required_labels) = &node_selector.node_selector_terms.first() {
                    if let Some(match_expressions) = &required_labels.match_expressions {
                        for expr in match_expressions {
                            let node_label_value = node_labels.and_then(|l| l.get(&expr.key));

                            match expr.operator.as_str() {
                                "In" => {
                                    if let Some(values) = &expr.values {
                                        let matches = node_label_value
                                            .map(|v| values.contains(v))
                                            .unwrap_or(false);
                                        if !matches {
                                            debug!(
                                                "Node {} does not match ResourceClaim node selector (key={}, operator=In)",
                                                node.metadata.name, expr.key
                                            );
                                            return false;
                                        }
                                    }
                                }
                                "NotIn" => {
                                    if let Some(values) = &expr.values {
                                        let matches = node_label_value
                                            .map(|v| !values.contains(v))
                                            .unwrap_or(true);
                                        if !matches {
                                            debug!(
                                                "Node {} does not match ResourceClaim node selector (key={}, operator=NotIn)",
                                                node.metadata.name, expr.key
                                            );
                                            return false;
                                        }
                                    }
                                }
                                "Exists" => {
                                    if node_label_value.is_none() {
                                        debug!(
                                            "Node {} does not match ResourceClaim node selector (key={}, operator=Exists)",
                                            node.metadata.name, expr.key
                                        );
                                        return false;
                                    }
                                }
                                "DoesNotExist" => {
                                    if node_label_value.is_some() {
                                        debug!(
                                            "Node {} does not match ResourceClaim node selector (key={}, operator=DoesNotExist)",
                                            node.metadata.name, expr.key
                                        );
                                        return false;
                                    }
                                }
                                _ => {
                                    warn!("Unknown node selector operator: {}", expr.operator);
                                }
                            }
                        }
                    }
                }
            }

            // Verify devices are available on this node
            // Check each allocated device to ensure it's on this node
            for device_result in &allocation.devices.results {
                // Get ResourceSlices to find which node has this device
                let slices_prefix = build_prefix("resourceslices", None);
                let slices: Vec<ResourceSlice> = match self.storage.list(&slices_prefix).await {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("Failed to list ResourceSlices: {}", e);
                        return false;
                    }
                };

                let mut device_found_on_node = false;

                for slice in slices {
                    // Check if this slice is for the right driver and pool
                    if slice.spec.driver != device_result.driver {
                        continue;
                    }
                    if slice.spec.pool.name != device_result.pool {
                        continue;
                    }

                    // Check if slice has node name specified
                    let slice_node_name = match &slice.spec.node_name {
                        Some(name) => name,
                        None => continue, // Slice not associated with a specific node
                    };

                    // Check if this is the target node
                    if slice_node_name != &node.metadata.name {
                        continue;
                    }

                    // Check if the device exists in this slice
                    for device in &slice.spec.devices {
                        if device.name == device_result.device {
                            device_found_on_node = true;
                            break;
                        }
                    }

                    if device_found_on_node {
                        break;
                    }
                }

                if !device_found_on_node {
                    debug!(
                        "Device {} from pool {} (driver {}) not found on node {}",
                        device_result.device,
                        device_result.pool,
                        device_result.driver,
                        node.metadata.name
                    );
                    return false;
                }
            }
        }

        // All resource claims are satisfied on this node
        true
    }
}
