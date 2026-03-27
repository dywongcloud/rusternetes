use anyhow::Result;
use rusternetes_common::resources::{Node, Pod};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// TaintEvictionController watches for NoExecute taints on nodes and evicts
/// pods that don't tolerate them. This implements the node lifecycle controller's
/// taint-based eviction behavior.
pub struct TaintEvictionController<S: Storage> {
    storage: Arc<S>,
}

impl<S: Storage> TaintEvictionController<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    pub async fn reconcile_all(&self) -> Result<()> {
        debug!("Starting taint eviction reconciliation");

        let nodes: Vec<Node> = self.storage.list("/registry/nodes/").await?;

        for node in &nodes {
            if let Err(e) = self.reconcile_node(node).await {
                warn!("Taint eviction error for node {}: {}", node.metadata.name, e);
            }
        }

        Ok(())
    }

    async fn reconcile_node(&self, node: &Node) -> Result<()> {
        let node_name = &node.metadata.name;

        // Get NoExecute taints on this node
        let no_execute_taints: Vec<&rusternetes_common::resources::Taint> = node
            .spec
            .as_ref()
            .and_then(|s| s.taints.as_ref())
            .map(|taints| {
                taints
                    .iter()
                    .filter(|t| t.effect == "NoExecute")
                    .collect()
            })
            .unwrap_or_default();

        if no_execute_taints.is_empty() {
            return Ok(());
        }

        // Find all pods on this node
        // We need to scan all namespaces for pods assigned to this node
        let all_pods: Vec<Pod> = self.storage.list::<Pod>("/registry/pods/").await.unwrap_or_default();

        let node_pods: Vec<&Pod> = all_pods
            .iter()
            .filter(|pod| {
                pod.spec
                    .as_ref()
                    .and_then(|s| s.node_name.as_deref())
                    == Some(node_name)
            })
            .collect();

        for pod in node_pods {
            // Check if pod tolerates all NoExecute taints
            let tolerates_all = no_execute_taints.iter().all(|taint| {
                self.pod_tolerates_taint(pod, taint)
            });

            if !tolerates_all {
                // Pod doesn't tolerate a NoExecute taint — evict it
                let namespace = pod.metadata.namespace.as_deref().unwrap_or("default");
                let pod_name = &pod.metadata.name;

                // Skip system pods (kube-system namespace)
                if namespace == "kube-system" {
                    debug!("Skipping eviction of system pod {}/{}", namespace, pod_name);
                    continue;
                }

                info!(
                    "Evicting pod {}/{} from node {} due to NoExecute taint",
                    namespace, pod_name, node_name
                );

                let key = build_key("pods", Some(namespace), pod_name);
                match self.storage.delete(&key).await {
                    Ok(_) => {
                        info!("Evicted pod {}/{} from node {}", namespace, pod_name, node_name);
                    }
                    Err(rusternetes_common::Error::NotFound(_)) => {
                        // Already deleted
                    }
                    Err(e) => {
                        warn!("Failed to evict pod {}/{}: {}", namespace, pod_name, e);
                    }
                }
            }
        }

        Ok(())
    }

    fn pod_tolerates_taint(&self, pod: &Pod, taint: &rusternetes_common::resources::Taint) -> bool {
        let tolerations = pod
            .spec
            .as_ref()
            .and_then(|s| s.tolerations.as_ref());

        let tolerations = match tolerations {
            Some(t) => t,
            None => return false, // No tolerations = doesn't tolerate any taint
        };

        tolerations.iter().any(|toleration| {
            // Empty key with Exists operator matches all taints
            if toleration.key.as_ref().map_or(false, |k| k.is_empty())
                || toleration.key.is_none()
            {
                if toleration.operator.as_deref() == Some("Exists") {
                    return true;
                }
            }

            // Key must match
            let key_matches = toleration
                .key
                .as_ref()
                .map_or(false, |k| k == &taint.key);

            if !key_matches {
                return false;
            }

            // Effect must match (or be empty which matches all effects)
            let effect_matches = toleration
                .effect
                .as_ref()
                .map_or(true, |e| e.is_empty() || e == &taint.effect);

            if !effect_matches {
                return false;
            }

            // Operator check
            match toleration.operator.as_deref() {
                Some("Exists") => true, // Key match is sufficient
                Some("Equal") | None => {
                    // Value must match
                    let taint_value = taint.value.as_deref().unwrap_or("");
                    let toleration_value = toleration.value.as_deref().unwrap_or("");
                    taint_value == toleration_value
                }
                _ => false,
            }
        })
    }
}
