use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use rusternetes_common::resources::{Node, NodeCondition, NodeStatus, Pod, PodStatus};
use rusternetes_common::types::Phase;
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// NodeController monitors node health and manages node lifecycle.
///
/// Responsibilities:
/// 1. Monitor node heartbeats (via status updates)
/// 2. Mark nodes as NotReady when heartbeats are missed
/// 3. Evict pods from failed nodes
/// 4. Manage node taints based on conditions
/// 5. Update node status
const NODE_MONITOR_GRACE_PERIOD_SECONDS: i64 = 40;
const POD_EVICTION_TIMEOUT_SECONDS: i64 = 300; // 5 minutes

pub struct NodeController<S: Storage> {
    storage: Arc<S>,
}

impl<S: Storage> NodeController<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    /// Main reconciliation loop - monitors all nodes
    pub async fn reconcile_all(&self) -> Result<()> {
        debug!("Starting node reconciliation");

        // List all nodes
        let nodes: Vec<Node> = self.storage.list("/registry/nodes/").await?;

        for node in nodes {
            if let Err(e) = self.reconcile_node(&node).await {
                error!("Failed to reconcile node {}: {}", &node.metadata.name, e);
            }
        }

        Ok(())
    }

    /// Reconcile a single node
    async fn reconcile_node(&self, node: &Node) -> Result<()> {
        let node_name = &node.metadata.name;

        // Check if node is ready based on heartbeat
        let is_ready = self.is_node_ready(node);

        // Get current ready condition
        let current_ready_condition = node
            .status
            .as_ref()
            .and_then(|s| s.conditions.as_ref())
            .and_then(|conditions| conditions.iter().find(|c| c.condition_type == "Ready"));

        let needs_update = match current_ready_condition {
            Some(condition) => {
                let current_is_ready = condition.status == "True";
                current_is_ready != is_ready
            }
            None => true, // No ready condition exists, need to create one
        };

        if needs_update {
            info!("Node {} ready status changed to: {}", node_name, is_ready);
            self.update_node_status(node, is_ready).await?;

            // If node became NotReady, start eviction timer
            if !is_ready {
                info!(
                    "Node {} is NotReady, will evict pods after timeout",
                    node_name
                );
            }
        }

        // Evict pods from nodes that have been NotReady for too long
        if !is_ready {
            if self.should_evict_pods(node) {
                info!("Evicting pods from NotReady node {}", node_name);
                self.evict_pods_from_node(node_name).await?;
            }
        }

        Ok(())
    }

    /// Check if a node is ready based on its last heartbeat
    fn is_node_ready(&self, node: &Node) -> bool {
        let status = match &node.status {
            Some(s) => s,
            None => return false,
        };

        // Get the Ready condition
        let ready_condition = match &status.conditions {
            Some(conditions) => conditions.iter().find(|c| c.condition_type == "Ready"),
            None => return false,
        };

        let ready_condition = match ready_condition {
            Some(c) => c,
            None => return false,
        };

        // Check if the condition is "True"
        if ready_condition.status != "True" {
            return false;
        }

        // Check last heartbeat time
        if let Some(last_heartbeat) = &ready_condition.last_heartbeat_time {
            let now = Utc::now();
            let elapsed = now.signed_duration_since(*last_heartbeat);

            // Node is considered ready if heartbeat is within grace period
            return elapsed < Duration::seconds(NODE_MONITOR_GRACE_PERIOD_SECONDS);
        }

        // If we can't determine heartbeat time, consider node not ready
        false
    }

    /// Check if pods should be evicted from a node
    fn should_evict_pods(&self, node: &Node) -> bool {
        let status = match &node.status {
            Some(s) => s,
            None => return false,
        };

        let ready_condition = match &status.conditions {
            Some(conditions) => conditions.iter().find(|c| c.condition_type == "Ready"),
            None => return false,
        };

        let ready_condition = match ready_condition {
            Some(c) => c,
            None => return false,
        };

        // Only evict if node has been NotReady for a while
        if ready_condition.status == "True" {
            return false;
        }

        // Check when the node became NotReady
        if let Some(transition_time) = &ready_condition.last_transition_time {
            let now = Utc::now();
            let elapsed = now.signed_duration_since(*transition_time);

            // Evict pods after timeout
            return elapsed > Duration::seconds(POD_EVICTION_TIMEOUT_SECONDS);
        }

        false
    }

    /// Update node status
    async fn update_node_status(&self, node: &Node, is_ready: bool) -> Result<()> {
        let node_name = &node.metadata.name;
        let node_key = build_key("nodes", None, node_name);

        // Get current node
        let mut updated_node: Node = self.storage.get(&node_key).await?;

        // Initialize status if needed
        if updated_node.status.is_none() {
            updated_node.status = Some(NodeStatus {
                conditions: None,
                addresses: None,
                capacity: None,
                allocatable: None,
                node_info: None,
                images: None,
                volumes_in_use: None,
                volumes_attached: None,
                daemon_endpoints: None,
                config: None,
                features: None,
                runtime_handlers: None,
            });
        }

        let status = updated_node.status.as_mut().unwrap();

        // Initialize conditions if needed
        if status.conditions.is_none() {
            status.conditions = Some(Vec::new());
        }

        let conditions = status.conditions.as_mut().unwrap();

        // Update or create Ready condition
        let now = Utc::now();
        let ready_status = if is_ready { "True" } else { "False" };
        let reason = if is_ready {
            "KubeletReady"
        } else {
            "KubeletNotReady"
        };
        let message = if is_ready {
            "kubelet is posting ready status"
        } else {
            "kubelet stopped posting node status"
        };

        if let Some(ready_condition) = conditions.iter_mut().find(|c| c.condition_type == "Ready") {
            // Update existing condition
            if ready_condition.status != ready_status {
                ready_condition.last_transition_time = Some(now);
            }
            ready_condition.status = ready_status.to_string();
            ready_condition.reason = Some(reason.to_string());
            ready_condition.message = Some(message.to_string());
            ready_condition.last_heartbeat_time = Some(now);
        } else {
            // Create new Ready condition
            conditions.push(NodeCondition {
                condition_type: "Ready".to_string(),
                status: ready_status.to_string(),
                last_heartbeat_time: Some(now),
                last_transition_time: Some(now),
                reason: Some(reason.to_string()),
                message: Some(message.to_string()),
            });
        }

        // Update the node in storage
        self.storage.update(&node_key, &updated_node).await?;

        info!("Updated node {} status to ready={}", node_name, is_ready);
        Ok(())
    }

    /// Evict all pods from a failed node
    async fn evict_pods_from_node(&self, node_name: &str) -> Result<()> {
        info!("Evicting pods from node {}", node_name);

        // List all pods across all namespaces
        let pods: Vec<Pod> = self.storage.list("/registry/pods/").await?;

        // Filter pods running on this node
        let pods_on_node: Vec<&Pod> = pods
            .iter()
            .filter(|pod| {
                pod.spec
                    .as_ref()
                    .and_then(|s| s.node_name.as_ref())
                    .map(|n| n == node_name)
                    .unwrap_or(false)
            })
            .collect();

        info!("Found {} pods on node {}", pods_on_node.len(), node_name);

        // Delete each pod
        for pod in pods_on_node {
            let namespace = pod
                .metadata
                .namespace
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Pod has no namespace"))?;
            let pod_name = &pod.metadata.name;

            let pod_key = build_key("pods", Some(namespace), pod_name);

            match self.storage.delete(&pod_key).await {
                Ok(_) => {
                    info!(
                        "Evicted pod {}/{} from node {}",
                        namespace, pod_name, node_name
                    );
                }
                Err(rusternetes_common::Error::NotFound(_)) => {
                    // Pod already deleted
                    debug!("Pod {}/{} already deleted", namespace, pod_name);
                }
                Err(e) => {
                    warn!("Failed to evict pod {}/{}: {}", namespace, pod_name, e);
                }
            }
        }

        Ok(())
    }

    /// Mark a pod as failed due to node failure
    async fn mark_pod_failed(&self, namespace: &str, pod_name: &str, reason: &str) -> Result<()> {
        let pod_key = build_key("pods", Some(namespace), pod_name);

        let mut pod: Pod = match self.storage.get(&pod_key).await {
            Ok(p) => p,
            Err(rusternetes_common::Error::NotFound(_)) => return Ok(()),
            Err(e) => return Err(e.into()),
        };

        // Initialize status if needed
        if pod.status.is_none() {
            pod.status = Some(PodStatus {
                phase: Some(Phase::Pending),
                message: None,
                reason: None,
                host_ip: None,
                host_i_ps: None,
                pod_ip: None,
                pod_i_ps: None,
                nominated_node_name: None,
                qos_class: None,
                start_time: None,
                conditions: None,
                container_statuses: None,
                init_container_statuses: None,
                ephemeral_container_statuses: None,
                resize: None,
                resource_claim_statuses: None,
                observed_generation: None,
            });
        }

        let status = pod.status.as_mut().unwrap();
        status.phase = Some(Phase::Failed);
        status.reason = Some(reason.to_string());
        status.message = Some(format!("Node {} is not ready", reason));

        // Update pod
        self.storage.update(&pod_key, &pod).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::types::{ObjectMeta, TypeMeta};
    use rusternetes_storage::memory::MemoryStorage;

    #[tokio::test]
    async fn test_node_controller_creation() {
        let storage = Arc::new(MemoryStorage::new());
        let _controller = NodeController::new(storage);
    }

    #[test]
    fn test_node_ready_check() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = NodeController::new(storage);

        // Node with recent heartbeat
        let node_ready = Node {
            type_meta: TypeMeta {
                kind: "Node".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: ObjectMeta {
                name: "test-node".to_string(),
                namespace: None,
                uid: String::new(),
                resource_version: None,
                deletion_grace_period_seconds: None,
                finalizers: None,
                owner_references: None,
                creation_timestamp: None,
                deletion_timestamp: None,
                labels: None,
                annotations: None,
                generate_name: None,
                generation: None,
                managed_fields: None,
            },
            spec: None,
            status: Some(NodeStatus {
                conditions: Some(vec![NodeCondition {
                    condition_type: "Ready".to_string(),
                    status: "True".to_string(),
                    last_heartbeat_time: Some(Utc::now()),
                    last_transition_time: Some(Utc::now()),
                    reason: Some("KubeletReady".to_string()),
                    message: Some("kubelet is ready".to_string()),
                }]),
                addresses: None,
                capacity: None,
                allocatable: None,
                node_info: None,
                images: None,
                volumes_in_use: None,
                volumes_attached: None,
                daemon_endpoints: None,
                config: None,
                features: None,
                runtime_handlers: None,
            }),
        };

        assert!(controller.is_node_ready(&node_ready));

        // Node with old heartbeat
        let old_time = Utc::now() - Duration::seconds(60);
        let node_not_ready = Node {
            status: Some(NodeStatus {
                conditions: Some(vec![NodeCondition {
                    condition_type: "Ready".to_string(),
                    status: "True".to_string(),
                    last_heartbeat_time: Some(old_time),
                    last_transition_time: Some(old_time),
                    reason: Some("KubeletReady".to_string()),
                    message: Some("kubelet is ready".to_string()),
                }]),
                addresses: None,
                capacity: None,
                allocatable: None,
                node_info: None,
                images: None,
                volumes_in_use: None,
                volumes_attached: None,
                daemon_endpoints: None,
                config: None,
                features: None,
                runtime_handlers: None,
            }),
            ..node_ready
        };

        assert!(!controller.is_node_ready(&node_not_ready));
    }
}
