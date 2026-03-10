use rusternetes_common::{
    resources::{
        Node, NodeSelector, NodeSelectorRequirement, NodeSelectorTerm, Pod,
        Taint, Toleration, TopologySpreadConstraint,
    },
};
use std::collections::HashMap;
use tracing::debug;

/// Scoring result for a node
#[derive(Debug, Clone)]
pub struct NodeScore {
    pub node_name: String,
    pub score: i32,
}

/// Check if pod tolerates all node taints
pub fn check_taints_tolerations(node: &Node, pod: &Pod) -> bool {
    let node_taints = match &node.spec {
        Some(spec) => match &spec.taints {
            Some(taints) => taints,
            None => return true, // No taints, pod can be scheduled
        },
        None => return true, // No spec, no taints
    };

    let pod_tolerations = match &pod.spec.as_ref().unwrap().tolerations {
        Some(tolerations) => tolerations,
        None => {
            // No tolerations, check if there are any NoSchedule or NoExecute taints
            return !node_taints
                .iter()
                .any(|t| t.effect == "NoSchedule" || t.effect == "NoExecute");
        }
    };

    // Check each taint to see if there's a matching toleration
    for taint in node_taints {
        if !taint_is_tolerated(taint, pod_tolerations) {
            debug!(
                "Pod {} does not tolerate taint {:?} on node {}",
                pod.metadata.name, taint, node.metadata.name
            );
            return false;
        }
    }

    true
}

/// Check if a specific taint is tolerated by any of the tolerations
fn taint_is_tolerated(taint: &Taint, tolerations: &[Toleration]) -> bool {
    // PreferNoSchedule is a soft constraint, always tolerated for hard scheduling
    if taint.effect == "PreferNoSchedule" {
        return true;
    }

    for toleration in tolerations {
        if toleration_matches_taint(toleration, taint) {
            return true;
        }
    }

    false
}

/// Check if a toleration matches a taint
fn toleration_matches_taint(toleration: &Toleration, taint: &Taint) -> bool {
    let operator = toleration.operator.as_deref().unwrap_or("Equal");

    // Check effect
    if let Some(ref effect) = toleration.effect {
        if effect != &taint.effect {
            return false;
        }
    }

    // Check operator
    match operator {
        "Exists" => {
            // If key is empty, tolerate all taints
            if toleration.key.is_none() {
                return true;
            }
            // Otherwise check key matches
            toleration.key.as_ref() == Some(&taint.key)
        }
        "Equal" => {
            // Both key and value must match
            toleration.key.as_ref() == Some(&taint.key)
                && toleration.value.as_ref() == taint.value.as_ref()
        }
        _ => false,
    }
}

/// Check node affinity requirements
pub fn check_node_affinity(node: &Node, pod: &Pod) -> (bool, i32) {
    let affinity = match &pod.spec.as_ref().unwrap().affinity {
        Some(a) => a,
        None => return (true, 0), // No affinity requirements
    };

    let node_affinity = match &affinity.node_affinity {
        Some(na) => na,
        None => return (true, 0),
    };

    // Check required node affinity (hard requirement)
    if let Some(ref required) = node_affinity.required_during_scheduling_ignored_during_execution {
        if !matches_node_selector(node, required) {
            return (false, 0);
        }
    }

    // Calculate score from preferred node affinity (soft requirement)
    let mut score = 0;
    if let Some(ref preferred) = node_affinity.preferred_during_scheduling_ignored_during_execution
    {
        for pref in preferred {
            if matches_node_selector_term(node, &pref.preference) {
                score += pref.weight;
            }
        }
    }

    (true, score)
}

/// Check pod affinity requirements
/// Returns (passes_hard_requirements, score)
pub fn check_pod_affinity(
    node: &Node,
    pod: &Pod,
    all_pods: &[Pod],
) -> (bool, i32) {
    let affinity = match &pod.spec.as_ref().unwrap().affinity {
        Some(a) => a,
        None => return (true, 0), // No affinity requirements
    };

    let pod_affinity = match &affinity.pod_affinity {
        Some(pa) => pa,
        None => return (true, 0),
    };

    // Check required pod affinity (hard requirement)
    if let Some(ref required) = pod_affinity.required_during_scheduling_ignored_during_execution {
        for term in required {
            if !matches_pod_affinity_term(node, pod, term, all_pods, true) {
                debug!(
                    "Pod {} does not meet hard pod affinity requirement on node {}",
                    pod.metadata.name, node.metadata.name
                );
                return (false, 0);
            }
        }
    }

    // Calculate score from preferred pod affinity (soft requirement)
    let mut score = 0;
    if let Some(ref preferred) = pod_affinity.preferred_during_scheduling_ignored_during_execution {
        for weighted_term in preferred {
            if matches_pod_affinity_term(
                node,
                pod,
                &weighted_term.pod_affinity_term,
                all_pods,
                true,
            ) {
                score += weighted_term.weight;
            }
        }
    }

    (true, score)
}

/// Check pod anti-affinity requirements
/// Returns (passes_hard_requirements, score_penalty)
pub fn check_pod_anti_affinity(
    node: &Node,
    pod: &Pod,
    all_pods: &[Pod],
) -> (bool, i32) {
    let affinity = match &pod.spec.as_ref().unwrap().affinity {
        Some(a) => a,
        None => return (true, 0), // No anti-affinity requirements
    };

    let pod_anti_affinity = match &affinity.pod_anti_affinity {
        Some(paa) => paa,
        None => return (true, 0),
    };

    // Check required pod anti-affinity (hard requirement)
    if let Some(ref required) = pod_anti_affinity.required_during_scheduling_ignored_during_execution {
        for term in required {
            // For anti-affinity, we check if matching pods exist
            // If they do, we CANNOT schedule on this node
            if matches_pod_affinity_term(node, pod, term, all_pods, false) {
                debug!(
                    "Pod {} violates hard pod anti-affinity requirement on node {}",
                    pod.metadata.name, node.metadata.name
                );
                return (false, 0);
            }
        }
    }

    // Calculate score penalty from preferred pod anti-affinity (soft requirement)
    let mut penalty = 0;
    if let Some(ref preferred) = pod_anti_affinity.preferred_during_scheduling_ignored_during_execution {
        for weighted_term in preferred {
            if matches_pod_affinity_term(
                node,
                pod,
                &weighted_term.pod_affinity_term,
                all_pods,
                false,
            ) {
                penalty += weighted_term.weight;
            }
        }
    }

    (true, penalty)
}

/// Check if node matches a node selector
fn matches_node_selector(node: &Node, selector: &NodeSelector) -> bool {
    // At least one term must match (OR logic)
    selector
        .node_selector_terms
        .iter()
        .any(|term| matches_node_selector_term(node, term))
}

/// Check if node matches a single node selector term
fn matches_node_selector_term(node: &Node, term: &NodeSelectorTerm) -> bool {
    // Check match expressions (labels)
    if let Some(ref expressions) = term.match_expressions {
        if !expressions
            .iter()
            .all(|expr| matches_node_selector_requirement(node, expr, true))
        {
            return false;
        }
    }

    // Check match fields
    if let Some(ref fields) = term.match_fields {
        if !fields
            .iter()
            .all(|expr| matches_node_selector_requirement(node, expr, false))
        {
            return false;
        }
    }

    true
}

/// Check if node matches a selector requirement
fn matches_node_selector_requirement(
    node: &Node,
    requirement: &NodeSelectorRequirement,
    is_label: bool,
) -> bool {
    let value = if is_label {
        // Get from node labels
        node.metadata
            .labels
            .as_ref()
            .and_then(|labels| labels.get(&requirement.key))
            .map(|s| s.as_str())
    } else {
        // Get from node fields
        get_node_field(node, &requirement.key)
    };

    let values = requirement.values.as_deref().unwrap_or(&[]);

    match requirement.operator.as_str() {
        "In" => value.map(|v| values.contains(&v.to_string())).unwrap_or(false),
        "NotIn" => !value.map(|v| values.contains(&v.to_string())).unwrap_or(false),
        "Exists" => value.is_some(),
        "DoesNotExist" => value.is_none(),
        "Gt" => {
            if let Some(v) = value {
                if let Ok(node_val) = v.parse::<i64>() {
                    if !values.is_empty() {
                        if let Ok(req_val) = values[0].parse::<i64>() {
                            return node_val > req_val;
                        }
                    }
                }
            }
            false
        }
        "Lt" => {
            if let Some(v) = value {
                if let Ok(node_val) = v.parse::<i64>() {
                    if !values.is_empty() {
                        if let Ok(req_val) = values[0].parse::<i64>() {
                            return node_val < req_val;
                        }
                    }
                }
            }
            false
        }
        _ => false,
    }
}

/// Get a field value from a node
fn get_node_field<'a>(node: &'a Node, field: &str) -> Option<&'a str> {
    match field {
        "metadata.name" => Some(&node.metadata.name),
        "metadata.namespace" => node.metadata.namespace.as_deref(),
        _ => None,
    }
}

/// Match a label selector against pod labels
fn match_selector(
    selector: &rusternetes_common::types::LabelSelector,
    labels: &Option<std::collections::HashMap<String, String>>,
) -> bool {
    // Check matchLabels
    if let Some(ref match_labels) = selector.match_labels {
        let pod_labels = match labels {
            Some(l) => l,
            None => return match_labels.is_empty(),
        };

        for (key, value) in match_labels {
            if pod_labels.get(key) != Some(value) {
                return false;
            }
        }
    }

    // Check matchExpressions
    if let Some(ref match_expressions) = selector.match_expressions {
        let pod_labels = labels.as_ref();

        for expr in match_expressions {
            let label_value = pod_labels.and_then(|l| l.get(&expr.key));
            let values = expr.values.as_deref().unwrap_or(&[]);

            let matches = match expr.operator.as_str() {
                "In" => {
                    label_value.map(|v| values.contains(&v.as_str().to_string())).unwrap_or(false)
                }
                "NotIn" => {
                    !label_value.map(|v| values.contains(&v.as_str().to_string())).unwrap_or(false)
                }
                "Exists" => label_value.is_some(),
                "DoesNotExist" => label_value.is_none(),
                _ => false,
            };

            if !matches {
                return false;
            }
        }
    }

    true
}

/// Check if a pod affinity term matches
/// For affinity (is_affinity=true): returns true if matching pods exist on node's topology
/// For anti-affinity (is_affinity=false): returns true if matching pods exist (indicating a conflict)
fn matches_pod_affinity_term(
    node: &Node,
    _pod: &Pod,
    term: &rusternetes_common::resources::PodAffinityTerm,
    all_pods: &[Pod],
    _is_affinity: bool,
) -> bool {
    // Get the topology key value from the node
    let _topology_value = match node.metadata.labels.as_ref() {
        Some(labels) => match labels.get(&term.topology_key) {
            Some(v) => v,
            None => {
                // Node doesn't have the topology key label
                return false;
            }
        },
        None => return false,
    };

    // Find all pods scheduled on nodes with the same topology value
    let matching_pods: Vec<&Pod> = all_pods
        .iter()
        .filter(|p| {
            // Skip pods that aren't scheduled yet
            if p.spec.as_ref().and_then(|s| s.node_name.as_ref()).is_none() {
                return false;
            }

            // Check if pod matches the label selector
            if !match_selector(&term.label_selector, &p.metadata.labels) {
                return false;
            }

            // Check namespace constraint
            if let Some(ref namespaces) = term.namespaces {
                let pod_ns = p.metadata.namespace.as_deref().unwrap_or("default");
                if !namespaces.contains(&pod_ns.to_string()) {
                    return false;
                }
            }

            // TODO: Check if the pod is on a node with matching topology value
            // For now, we simplify by checking if any matching pod exists
            true
        })
        .collect();

    !matching_pods.is_empty()
}

/// Calculate resource-based node score
pub fn calculate_resource_score(node: &Node, pod: &Pod) -> i32 {
    let allocatable = match &node.status {
        Some(status) => match &status.allocatable {
            Some(a) => a,
            None => return 50, // Default score if no resource info
        },
        None => return 50,
    };

    // Get pod resource requests
    let mut cpu_request = 0i64;
    let mut memory_request = 0i64;

    for container in &pod.spec.as_ref().unwrap().containers {
        if let Some(ref resources) = container.resources {
            if let Some(ref requests) = resources.requests {
                if let Some(cpu) = requests.get("cpu") {
                    cpu_request += parse_resource_quantity(cpu, "cpu");
                }
                if let Some(memory) = requests.get("memory") {
                    memory_request += parse_resource_quantity(memory, "memory");
                }
            }
        }
    }

    // Calculate available resources
    let available_cpu = allocatable
        .get("cpu")
        .map(|s| parse_resource_quantity(s, "cpu"))
        .unwrap_or(0);
    let available_memory = allocatable
        .get("memory")
        .map(|s| parse_resource_quantity(s, "memory"))
        .unwrap_or(0);

    // If node can't fit the pod, return 0
    if cpu_request > available_cpu || memory_request > available_memory {
        return 0;
    }

    // Calculate score based on remaining capacity (0-100)
    // Higher remaining capacity = higher score (balanced scheduling)
    let cpu_score = if available_cpu > 0 {
        ((available_cpu - cpu_request) * 100 / available_cpu) as i32
    } else {
        0
    };

    let memory_score = if available_memory > 0 {
        ((available_memory - memory_request) * 100 / available_memory) as i32
    } else {
        0
    };

    // Return average score
    (cpu_score + memory_score) / 2
}

/// Parse resource quantity (simplified)
fn parse_resource_quantity(quantity: &str, resource_type: &str) -> i64 {
    // Remove units and parse
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


/// Check if preemption should occur and return pods to evict
/// Returns (should_preempt, pods_to_evict)
pub fn check_preemption(
    node: &Node,
    pod: &Pod,
    all_pods: &[Pod],
) -> (bool, Vec<String>) {
    // Get the priority of the incoming pod
    let incoming_priority = pod.spec.as_ref().and_then(|s| s.priority).unwrap_or(0);

    // If incoming pod has priority <= 0, don't preempt
    if incoming_priority <= 0 {
        return (false, vec![]);
    }

    // Find pods running on this node
    let node_pods: Vec<&Pod> = all_pods
        .iter()
        .filter(|p| {
            p.spec
                .as_ref()
                .and_then(|s| s.node_name.as_ref())
                .map(|n| n == &node.metadata.name)
                .unwrap_or(false)
        })
        .collect();

    // Find pods with lower priority that could be evicted
    let mut candidates: Vec<(&Pod, i32)> = node_pods
        .iter()
        .filter_map(|p| {
            let pod_priority = p.spec.as_ref().and_then(|s| s.priority).unwrap_or(0);
            if pod_priority < incoming_priority {
                Some((*p, pod_priority))
            } else {
                None
            }
        })
        .collect();

    // If no candidates, can't preempt
    if candidates.is_empty() {
        return (false, vec![]);
    }

    // Sort by priority (lowest first) for eviction
    candidates.sort_by_key(|(_, priority)| *priority);

    // Calculate resources needed by incoming pod
    let mut cpu_needed = 0i64;
    let mut memory_needed = 0i64;

    if let Some(spec) = &pod.spec {
        for container in &spec.containers {
            if let Some(ref resources) = container.resources {
                if let Some(ref requests) = resources.requests {
                    if let Some(cpu) = requests.get("cpu") {
                        cpu_needed += parse_resource_quantity(cpu, "cpu");
                    }
                    if let Some(memory) = requests.get("memory") {
                        memory_needed += parse_resource_quantity(memory, "memory");
                    }
                }
            }
        }
    }

    // Get node's allocatable resources
    let (available_cpu, available_memory) = if let Some(status) = &node.status {
        if let Some(allocatable) = &status.allocatable {
            let cpu = allocatable
                .get("cpu")
                .map(|s| parse_resource_quantity(s, "cpu"))
                .unwrap_or(0);
            let memory = allocatable
                .get("memory")
                .map(|s| parse_resource_quantity(s, "memory"))
                .unwrap_or(0);
            (cpu, memory)
        } else {
            return (false, vec![]);
        }
    } else {
        return (false, vec![]);
    };

    // Check if we have enough resources even with preemption
    if cpu_needed > available_cpu || memory_needed > available_memory {
        return (false, vec![]);
    }

    // Try to find a minimal set of pods to evict
    // Simple strategy: evict lowest priority pods until we have enough resources
    let mut pods_to_evict = Vec::new();
    let mut freed_cpu = 0i64;
    let mut freed_memory = 0i64;

    for (candidate_pod, _) in candidates {
        // Calculate resources used by this pod
        if let Some(spec) = &candidate_pod.spec {
            for container in &spec.containers {
                if let Some(ref resources) = container.resources {
                    if let Some(ref requests) = resources.requests {
                        if let Some(cpu) = requests.get("cpu") {
                            freed_cpu += parse_resource_quantity(cpu, "cpu");
                        }
                        if let Some(memory) = requests.get("memory") {
                            freed_memory += parse_resource_quantity(memory, "memory");
                        }
                    }
                }
            }
        }

        pods_to_evict.push(candidate_pod.metadata.name.clone());

        // Check if we've freed enough resources
        if freed_cpu >= cpu_needed && freed_memory >= memory_needed {
            debug!(
                "Preemption possible on node {}: evicting {} pods",
                node.metadata.name,
                pods_to_evict.len()
            );
            return (true, pods_to_evict);
        }
    }

    // Even after evicting all lower-priority pods, not enough resources
    (false, vec![])
}

/// Check topology spread constraints for a pod
/// Returns (passes_hard_constraints, score_penalty)
pub fn check_topology_spread_constraints(
    node: &Node,
    pod: &Pod,
    all_pods: &[Pod],
    all_nodes: &[Node],
) -> (bool, i32) {
    let constraints = match &pod.spec {
        Some(spec) => match &spec.topology_spread_constraints {
            Some(c) => c,
            None => return (true, 0), // No constraints
        },
        None => return (true, 0),
    };

    let mut total_penalty = 0;

    for constraint in constraints {
        let (passes, penalty) = check_single_topology_constraint(node, pod, constraint, all_pods, all_nodes);

        if !passes {
            return (false, 0); // Hard constraint failed
        }

        total_penalty += penalty;
    }

    (true, total_penalty)
}

/// Check a single topology spread constraint
fn check_single_topology_constraint(
    node: &Node,
    pod: &Pod,
    constraint: &TopologySpreadConstraint,
    all_pods: &[Pod],
    all_nodes: &[Node],
) -> (bool, i32) {
    // Get the topology value for the candidate node
    let node_topology_value = match node.metadata.labels.as_ref() {
        Some(labels) => match labels.get(&constraint.topology_key) {
            Some(v) => v.clone(),
            None => {
                // Node doesn't have the topology key
                // If whenUnsatisfiable is DoNotSchedule, we can't schedule here
                if constraint.when_unsatisfiable == "DoNotSchedule" {
                    return (false, 0);
                }
                return (true, 0);
            }
        },
        None => {
            if constraint.when_unsatisfiable == "DoNotSchedule" {
                return (false, 0);
            }
            return (true, 0);
        }
    };

    // Find all pods that match the label selector
    let matching_pods: Vec<&Pod> = all_pods
        .iter()
        .filter(|p| {
            // Skip unscheduled pods
            if p.spec.as_ref().and_then(|s| s.node_name.as_ref()).is_none() {
                return false;
            }

            // Check if pod matches the label selector
            if let Some(ref selector) = constraint.label_selector {
                match_selector(selector, &p.metadata.labels)
            } else {
                // No label selector means match all pods
                true
            }
        })
        .collect();

    // Count pods per topology domain
    let mut domain_counts: HashMap<String, i32> = HashMap::new();

    // Initialize counts for all domains
    for n in all_nodes {
        if let Some(labels) = &n.metadata.labels {
            if let Some(topology_value) = labels.get(&constraint.topology_key) {
                domain_counts.entry(topology_value.clone()).or_insert(0);
            }
        }
    }

    // Count matching pods per domain
    for p in &matching_pods {
        if let Some(spec) = &p.spec {
            if let Some(node_name) = &spec.node_name {
                // Find the node this pod is on
                if let Some(pod_node) = all_nodes.iter().find(|n| &n.metadata.name == node_name) {
                    if let Some(labels) = &pod_node.metadata.labels {
                        if let Some(topology_value) = labels.get(&constraint.topology_key) {
                            *domain_counts.entry(topology_value.clone()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }

    // Calculate skew if we place this pod on the candidate node
    let current_count = domain_counts.get(&node_topology_value).copied().unwrap_or(0);
    let new_count = current_count + 1;

    // Find min and max counts
    let min_count = domain_counts.values().min().copied().unwrap_or(0);
    let max_count = domain_counts.values().max().copied().unwrap_or(0);

    // Calculate skew after placing pod
    let skew = if new_count > min_count {
        new_count - min_count
    } else {
        max_count - min_count
    };

    // Check if skew exceeds max_skew
    if skew > constraint.max_skew {
        if constraint.when_unsatisfiable == "DoNotSchedule" {
            debug!(
                "Topology spread constraint violated: skew {} > max_skew {} for topology key {}",
                skew, constraint.max_skew, constraint.topology_key
            );
            return (false, 0);
        } else {
            // ScheduleAnyway - allow but penalize
            let penalty = (skew - constraint.max_skew) * 10; // Penalty proportional to skew violation
            return (true, penalty);
        }
    }

    // Check minDomains if specified
    if let Some(min_domains) = constraint.min_domains {
        let num_domains = domain_counts.len() as i32;
        if num_domains < min_domains {
            if constraint.when_unsatisfiable == "DoNotSchedule" {
                return (false, 0);
            } else {
                let penalty = (min_domains - num_domains) * 5;
                return (true, penalty);
            }
        }
    }

    // Constraint satisfied - add small penalty based on imbalance to prefer better spread
    let imbalance_penalty = ((new_count as f32 - min_count as f32) * 2.0) as i32;
    (true, imbalance_penalty.max(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpu_quantity() {
        assert_eq!(parse_resource_quantity("100m", "cpu"), 100);
        assert_eq!(parse_resource_quantity("1", "cpu"), 1000);
        assert_eq!(parse_resource_quantity("2", "cpu"), 2000);
    }

    #[test]
    fn test_parse_memory_quantity() {
        assert_eq!(parse_resource_quantity("1Ki", "memory"), 1024);
        assert_eq!(parse_resource_quantity("1Mi", "memory"), 1024 * 1024);
        assert_eq!(parse_resource_quantity("1Gi", "memory"), 1024 * 1024 * 1024);
    }

    #[test]
    fn test_toleration_matches_taint() {
        let taint = Taint {
            key: "key1".to_string(),
            value: Some("value1".to_string()),
            effect: "NoSchedule".to_string(),
        };

        let toleration = Toleration {
            key: Some("key1".to_string()),
            operator: Some("Equal".to_string()),
            value: Some("value1".to_string()),
            effect: Some("NoSchedule".to_string()),
            toleration_seconds: None,
        };

        assert!(toleration_matches_taint(&toleration, &taint));
    }
}
