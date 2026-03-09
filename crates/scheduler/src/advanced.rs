use rusternetes_common::{
    resources::{
        Node, NodeSelector, NodeSelectorRequirement, NodeSelectorTerm, Pod,
        Taint, Toleration,
    },
};
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

    let pod_tolerations = match &pod.spec.tolerations {
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
    let affinity = match &pod.spec.affinity {
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

    for container in &pod.spec.containers {
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
