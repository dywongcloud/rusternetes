use anyhow::Result;
use rusternetes_common::resources::pod::{SecretVolumeSource, Volume, VolumeMount};
use rusternetes_common::resources::{DaemonSet, DaemonSetStatus, Node, Pod, PodStatus};
use rusternetes_common::types::{OwnerReference, Phase};
use rusternetes_storage::Storage;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

pub struct DaemonSetController<S: Storage> {
    storage: Arc<S>,
}

impl<S: Storage> DaemonSetController<S> {
    pub fn new(storage: Arc<S>) -> Self {
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
        let daemonsets: Vec<DaemonSet> = self.storage.list("/registry/daemonsets/").await?;

        for mut daemonset in daemonsets {
            if let Err(e) = self.reconcile(&mut daemonset).await {
                error!(
                    "Failed to reconcile DaemonSet {}: {}",
                    daemonset.metadata.name, e
                );
            }
        }

        Ok(())
    }

    async fn reconcile(&self, daemonset: &mut DaemonSet) -> Result<()> {
        let name = &daemonset.metadata.name;
        let namespace = daemonset.metadata.namespace.as_ref().unwrap();

        // Skip reconciliation for DaemonSets being deleted — GC handles pod cleanup
        if daemonset.metadata.is_being_deleted() {
            return Ok(());
        }

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

        // Get current pods for this DaemonSet using owner references
        let pod_prefix = format!("/registry/pods/{}/", namespace);
        let all_pods: Vec<Pod> = self.storage.list(&pod_prefix).await?;

        // Find pods owned by this DaemonSet via ownerReferences (authoritative)
        // Fall back to label matching for backwards compatibility with pods created before this fix
        let daemonset_uid = &daemonset.metadata.uid;
        let daemonset_pods: Vec<Pod> = all_pods
            .into_iter()
            .filter(|pod| {
                let owned_by_ref = pod
                    .metadata
                    .owner_references
                    .as_ref()
                    .map(|refs| refs.iter().any(|r| &r.uid == daemonset_uid))
                    .unwrap_or(false);
                let owned_by_label = pod
                    .metadata
                    .labels
                    .as_ref()
                    .and_then(|labels| labels.get("controller-uid"))
                    .map(|uid| uid == daemonset_uid)
                    .unwrap_or(false);
                owned_by_ref || owned_by_label
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

        // Re-fetch pods after creating/deleting to get accurate count for status
        let all_pods_after: Vec<Pod> = self.storage.list(&pod_prefix).await?;
        let daemonset_pods_after: Vec<Pod> = all_pods_after
            .into_iter()
            .filter(|pod| {
                let owned_by_ref = pod
                    .metadata
                    .owner_references
                    .as_ref()
                    .map(|refs| refs.iter().any(|r| &r.uid == daemonset_uid))
                    .unwrap_or(false);
                let owned_by_label = pod
                    .metadata
                    .labels
                    .as_ref()
                    .and_then(|labels| labels.get("controller-uid"))
                    .map(|uid| uid == daemonset_uid)
                    .unwrap_or(false);
                owned_by_ref || owned_by_label
            })
            .collect();

        let mut final_pods_by_node = std::collections::HashMap::new();
        for pod in daemonset_pods_after.iter() {
            if let Some(node_name) = pod.spec.as_ref().and_then(|s| s.node_name.as_ref()) {
                final_pods_by_node.insert(node_name.clone(), pod.clone());
            }
        }

        // Update status with accurate counts
        let current_number_scheduled = final_pods_by_node.len() as i32;
        let desired_number_scheduled = eligible_nodes.len() as i32;
        let number_ready = final_pods_by_node
            .values()
            .filter(|pod| {
                pod.status
                    .as_ref()
                    .and_then(|s| s.phase.as_ref())
                    .map(|phase| *phase == Phase::Running)
                    .unwrap_or(false)
            })
            .count() as i32;

        daemonset.status = Some(DaemonSetStatus {
            desired_number_scheduled,
            current_number_scheduled,
            number_ready,
            number_misscheduled: 0,
            number_available: None,
            number_unavailable: None,
            updated_number_scheduled: None,
            observed_generation: None,
            collision_count: None,
            conditions: None,
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
                    node_labels
                        .get(k)
                        .map(|node_v| node_v == v)
                        .unwrap_or(false)
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
        let mut labels = template
            .metadata
            .as_ref()
            .and_then(|m| m.labels.clone())
            .unwrap_or_default();
        labels.insert("app".to_string(), daemonset_name.clone());
        labels.insert("controller-uid".to_string(), daemonset.metadata.uid.clone());

        let mut spec = template.spec.clone();

        // CRITICAL: Assign the pod to the specific node
        spec.node_name = Some(node_name.to_string());

        // Debug: Check if NODE_NAME env var has valueFrom before and after
        info!("Before injection - Checking environment variables in pod template:");
        for container in &spec.containers {
            if let Some(env) = &container.env {
                for env_var in env {
                    if env_var.name.contains("NODE_NAME")
                        || env_var.name.contains("SONOBUOY_NS")
                        || env_var.name.contains("SONOBUOY_PLUGIN_POD")
                    {
                        info!(
                            "  Container '{}': {} - value={:?}, value_from.field_ref={:?}",
                            container.name,
                            env_var.name,
                            env_var.value,
                            env_var
                                .value_from
                                .as_ref()
                                .and_then(|vf| vf.field_ref.as_ref())
                        );
                    }
                }
            }
        }

        // Inject service account token volume
        self.inject_service_account_token(&mut spec, namespace);

        // Debug: Check again after injection
        info!("After injection - Checking environment variables:");
        for container in &spec.containers {
            if let Some(env) = &container.env {
                for env_var in env {
                    if env_var.name.contains("NODE_NAME")
                        || env_var.name.contains("SONOBUOY_NS")
                        || env_var.name.contains("SONOBUOY_PLUGIN_POD")
                    {
                        info!(
                            "  Container '{}': {} - value={:?}, value_from.field_ref={:?}",
                            container.name,
                            env_var.name,
                            env_var.value,
                            env_var
                                .value_from
                                .as_ref()
                                .and_then(|vf| vf.field_ref.as_ref())
                        );
                    }
                }
            }
        }

        let mut metadata = rusternetes_common::types::ObjectMeta::new(pod_name.clone())
            .with_namespace(namespace.to_string())
            .with_labels(labels)
            .with_owner_reference(OwnerReference {
                api_version: "apps/v1".to_string(),
                kind: "DaemonSet".to_string(),
                name: daemonset_name.clone(),
                uid: daemonset.metadata.uid.clone(),
                controller: Some(true),
                block_owner_deletion: Some(true),
            });

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
                phase: Some(Phase::Pending),
                message: None,
                reason: None,
                pod_ip: None,
                pod_i_ps: None,
                nominated_node_name: None,
                qos_class: None,
                start_time: None,
                host_ip: None,
                host_i_ps: None,
                conditions: None,
                container_statuses: None,
                init_container_statuses: None,
                ephemeral_container_statuses: None,
            }),
        };

        let key = format!("/registry/pods/{}/{}", namespace, pod_name);
        self.storage.create(&key, &pod).await?;

        Ok(())
    }

    fn inject_service_account_token(
        &self,
        spec: &mut rusternetes_common::resources::PodSpec,
        namespace: &str,
    ) {
        // Get service account name, default to "default"
        let sa_name = spec.service_account_name.as_deref().unwrap_or("default");

        // The service account token secret name follows the pattern: {sa-name}-token
        let token_secret_name = format!("{}-token", sa_name);

        // Define the service account token volume
        let sa_token_volume = Volume {
            name: "kube-api-access".to_string(),
            empty_dir: None,
            host_path: None,
            config_map: None,
            secret: Some(SecretVolumeSource {
                secret_name: Some(token_secret_name.clone()),
                items: None,
                default_mode: None,
                optional: None,
            }),
            persistent_volume_claim: None,
            downward_api: None,
            csi: None,
            ephemeral: None,
        };

        // Add volume to pod spec
        if let Some(volumes) = &mut spec.volumes {
            // Check if volume already exists
            if !volumes.iter().any(|v| v.name == "kube-api-access") {
                volumes.push(sa_token_volume);
                info!(
                    "Injected service account token volume for DaemonSet pod in namespace {}",
                    namespace
                );
            }
        } else {
            spec.volumes = Some(vec![sa_token_volume]);
            info!(
                "Injected service account token volume for DaemonSet pod in namespace {}",
                namespace
            );
        }

        // Define the volume mount for the token
        let sa_token_mount = VolumeMount {
            name: "kube-api-access".to_string(),
            mount_path: "/var/run/secrets/kubernetes.io/serviceaccount".to_string(),
            read_only: Some(true),
            sub_path: None,
        };

        // Add volume mount to all containers
        for container in &mut spec.containers {
            if let Some(mounts) = &mut container.volume_mounts {
                // Check if mount already exists
                if !mounts
                    .iter()
                    .any(|m| m.mount_path == "/var/run/secrets/kubernetes.io/serviceaccount")
                {
                    mounts.push(sa_token_mount.clone());
                }
            } else {
                container.volume_mounts = Some(vec![sa_token_mount.clone()]);
            }
        }

        // Also add to init containers if present
        if let Some(init_containers) = &mut spec.init_containers {
            for container in init_containers {
                if let Some(mounts) = &mut container.volume_mounts {
                    if !mounts
                        .iter()
                        .any(|m| m.mount_path == "/var/run/secrets/kubernetes.io/serviceaccount")
                    {
                        mounts.push(sa_token_mount.clone());
                    }
                } else {
                    container.volume_mounts = Some(vec![sa_token_mount.clone()]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::resources::PodSpec;
    use rusternetes_storage::memory::MemoryStorage;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_node_selector_matching() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = DaemonSetController::new(storage);

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
                generate_name: None,
                generation: None,
                managed_fields: None,
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
                generate_name: None,
                generation: None,
                managed_fields: None,
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
                        generate_name: None,
                        generation: None,
                        managed_fields: None,
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
                        subdomain: None,
                        host_network: None,
                        host_pid: None,
                        host_ipc: None,
                        automount_service_account_token: None,
                        ephemeral_containers: None,
                        overhead: None,
                        scheduler_name: None,
                        topology_spread_constraints: None,
                        resource_claims: None,
                        active_deadline_seconds: None,
                        dns_policy: None,
                        dns_config: None,
                        security_context: None,
                        image_pull_secrets: None,
                        share_process_namespace: None,
                        readiness_gates: None,
                        runtime_class_name: None,
                        enable_service_links: None,
                        preemption_policy: None,
                        host_users: None,
                        set_hostname_as_fqdn: None,
                        termination_grace_period_seconds: None,
                        host_aliases: None,
                        os: None,
                        scheduling_gates: None,
                    },
                },
                update_strategy: None,
            min_ready_seconds: None,
            revision_history_limit: None,
            },
            status: None,
        };

        assert!(controller.matches_node_selector(&node, &ds_no_selector));
    }
}
