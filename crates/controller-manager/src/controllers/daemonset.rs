use anyhow::Result;
use rusternetes_common::resources::{DaemonSet, DaemonSetStatus, Node, Pod, PodStatus};
use rusternetes_common::types::Phase;
use rusternetes_storage::{etcd::EtcdStorage, Storage};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info};

pub struct DaemonSetController {
    storage: Arc<EtcdStorage>,
}

impl DaemonSetController {
    pub fn new(storage: Arc<EtcdStorage>) -> Self {
        Self { storage }
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting DaemonSetController");

        loop {
            if let Err(e) = self.reconcile_all().await {
                error!("Error in DaemonSet reconciliation loop: {}", e);
            }
            time::sleep(Duration::from_secs(5)).await;
        }
    }

    pub async fn reconcile_all(&self) -> Result<()> {
        let daemonsets: Vec<DaemonSet> = self
            .storage
            .list("/registry/daemonsets/")
            .await?;

        for mut daemonset in daemonsets {
            if let Err(e) = self.reconcile(&mut daemonset).await {
                error!(
                    "Failed to reconcile DaemonSet {}: {}",
                    daemonset.metadata.name,
                    e
                );
            }
        }

        Ok(())
    }

    async fn reconcile(&self, daemonset: &mut DaemonSet) -> Result<()> {
        let name = &daemonset.metadata.name;
        let namespace = daemonset.metadata.namespace.as_ref().unwrap();

        info!("Reconciling DaemonSet {}/{}", namespace, name);

        // Get all nodes
        let nodes: Vec<Node> = self.storage.list("/registry/nodes/").await?;

        // Filter nodes based on node selector
        let eligible_nodes: Vec<Node> = nodes
            .into_iter()
            .filter(|node| self.matches_node_selector(node, daemonset))
            .collect();

        info!(
            "DaemonSet {}/{}: {} eligible nodes",
            namespace,
            name,
            eligible_nodes.len()
        );

        // Get current pods for this DaemonSet
        let pod_prefix = format!("/registry/pods/{}/", namespace);
        let all_pods: Vec<Pod> = self.storage.list(&pod_prefix).await?;

        // Filter pods that belong to this DaemonSet
        let daemonset_pods: Vec<Pod> = all_pods
            .into_iter()
            .filter(|pod| {
                pod.metadata
                    .labels
                    .as_ref()
                    .and_then(|labels| labels.get("app"))
                    .map(|app| app == name)
                    .unwrap_or(false)
            })
            .collect();

        let mut pods_by_node = std::collections::HashMap::new();
        for pod in daemonset_pods.iter() {
            if let Some(node_name) = pod.spec.as_ref().and_then(|s| s.node_name.as_ref()) {
                pods_by_node.insert(node_name.clone(), pod.clone());
            }
        }

        // Ensure one pod per eligible node
        for node in eligible_nodes.iter() {
            let node_name = &node.metadata.name;

            if !pods_by_node.contains_key(node_name) {
                // Create pod for this node
                self.create_pod(daemonset, node_name, namespace).await?;
                info!("Created DaemonSet pod on node {}", node_name);
            }
        }

        // Remove pods from nodes that are no longer eligible
        let eligible_node_names: std::collections::HashSet<_> = eligible_nodes
            .iter()
            .map(|n| n.metadata.name.as_str())
            .collect();

        for (node_name, pod) in pods_by_node.iter() {
            if !eligible_node_names.contains(node_name.as_str()) {
                let pod_name = &pod.metadata.name;
                let pod_key = format!("/registry/pods/{}/{}", namespace, pod_name);
                self.storage.delete(&pod_key).await?;
                info!(
                    "Deleted DaemonSet pod {} from ineligible node {}",
                    pod_name, node_name
                );
            }
        }

        // Update status
        let current_number_scheduled = pods_by_node.len() as i32;
        let desired_number_scheduled = eligible_nodes.len() as i32;
        let number_ready = pods_by_node
            .values()
            .filter(|pod| {
                pod.status
                    .as_ref()
                    .map(|s| s.phase == Phase::Running)
                    .unwrap_or(false)
            })
            .count() as i32;

        daemonset.status = Some(DaemonSetStatus {
            desired_number_scheduled,
            current_number_scheduled,
            number_ready,
            number_misscheduled: 0,
        });

        // Save updated status
        let key = format!("/registry/daemonsets/{}/{}", namespace, name);
        self.storage.update(&key, daemonset).await?;

        Ok(())
    }

    fn matches_node_selector(&self, node: &Node, daemonset: &DaemonSet) -> bool {
        // Check if node matches the DaemonSet's node selector
        let node_labels = match &node.metadata.labels {
            Some(labels) => labels,
            None => return daemonset.spec.template.spec.node_selector.is_none(),
        };

        match &daemonset.spec.template.spec.node_selector {
            Some(selector) => {
                // All selector labels must match node labels
                selector.iter().all(|(k, v)| {
                    node_labels.get(k).map(|node_v| node_v == v).unwrap_or(false)
                })
            }
            None => true, // No selector means all nodes match
        }
    }

    async fn create_pod(
        &self,
        daemonset: &DaemonSet,
        node_name: &str,
        namespace: &str,
    ) -> Result<()> {
        let daemonset_name = &daemonset.metadata.name;
        let pod_name = format!("{}-{}", daemonset_name, &node_name.replace('.', "-"));

        // Create pod from template
        let template = &daemonset.spec.template;
        let mut labels = template.metadata.as_ref()
            .and_then(|m| m.labels.clone())
            .unwrap_or_default();
        labels.insert("app".to_string(), daemonset_name.clone());
        labels.insert(
            "controller-uid".to_string(),
            daemonset.metadata.uid.clone(),
        );

        let spec = template.spec.clone();
        // Note: In production, would set node_name here but PodSpec doesn't have this field yet

        let mut metadata = rusternetes_common::types::ObjectMeta::new(pod_name.clone())
            .with_namespace(namespace.to_string())
            .with_labels(labels);

        if let Some(template_meta) = &template.metadata {
            if let Some(ref annotations) = template_meta.annotations {
                metadata.annotations = Some(annotations.clone());
            }
        }

        let pod = Pod {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "Pod".to_string(),
                api_version: "v1".to_string(),
            },
            metadata,
            spec: Some(spec),
            status: Some(PodStatus {
                phase: Phase::Pending,
                message: None,
                reason: None,
                pod_ip: None,
                host_ip: None,
                container_statuses: None,
                init_container_statuses: None,
            }),
        };

        let key = format!("/registry/pods/{}/{}", namespace, pod_name);
        self.storage.create(&key, &pod).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::resources::PodSpec;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_node_selector_matching() {
        let storage = Arc::new(EtcdStorage::new(vec!["http://localhost:2379".to_string()]).await.unwrap());
        let controller = DaemonSetController { storage };

        let mut node_labels = HashMap::new();
        node_labels.insert("disktype".to_string(), "ssd".to_string());
        node_labels.insert("region".to_string(), "us-west".to_string());

        let node = Node {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "Node".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: rusternetes_common::types::ObjectMeta {
                name: "node-1".to_string(),
                namespace: None,
                labels: Some(node_labels),
                annotations: None,
                uid: uuid::Uuid::new_v4().to_string(),
                creation_timestamp: None,
                deletion_timestamp: None,
                resource_version: None,
                deletion_grace_period_seconds: None,
                finalizers: None,
                owner_references: None,
            },
            spec: Some(rusternetes_common::resources::NodeSpec {
                pod_cidr: None,
                provider_id: None,
                unschedulable: None,
                taints: None,
            }),
            status: None,
        };

        // Test: no selector = all nodes match
        let ds_no_selector = DaemonSet {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "DaemonSet".to_string(),
                api_version: "apps/v1".to_string(),
            },
            metadata: rusternetes_common::types::ObjectMeta {
                name: "test-ds".to_string(),
                namespace: Some("default".to_string()),
                labels: None,
                annotations: None,
                uid: uuid::Uuid::new_v4().to_string(),
                creation_timestamp: None,
                deletion_timestamp: None,
                resource_version: None,
                deletion_grace_period_seconds: None,
                finalizers: None,
                owner_references: None,
            },
            spec: rusternetes_common::resources::DaemonSetSpec {
                selector: rusternetes_common::types::LabelSelector {
                    match_labels: None,
                    match_expressions: None,
                },
                template: rusternetes_common::resources::PodTemplateSpec {
                    metadata: Some(rusternetes_common::types::ObjectMeta {
                        name: "".to_string(),
                        namespace: None,
                        labels: None,
                        annotations: None,
                        uid: uuid::Uuid::new_v4().to_string(),
                        creation_timestamp: None,
                        deletion_timestamp: None,
                        resource_version: None,
                deletion_grace_period_seconds: None,
                finalizers: None,
                owner_references: None,
                    }),
                    spec: PodSpec {
                        init_containers: None,
                containers: vec![],
                        node_name: None,
                        node_selector: None,
                        restart_policy: None,
                        service_account_name: None,
                        volumes: None,
                        affinity: None,
                        tolerations: None,
                        priority: None,
                        priority_class_name: None,
                        hostname: None,
                        host_network: None,
                        host_pid: None,
                        host_ipc: None,
                    },
                },
                update_strategy: None,
            },
            status: None,
        };

        assert!(controller.matches_node_selector(&node, &ds_no_selector));
    }
}
