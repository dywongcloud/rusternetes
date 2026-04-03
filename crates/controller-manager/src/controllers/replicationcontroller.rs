use chrono::Utc;
use rusternetes_common::{
    resources::{Pod, PodStatus, ReplicationController, ReplicationControllerCondition},
    types::{ObjectMeta, OwnerReference, Phase},
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::{sync::Arc, time::Duration};
use tracing::{debug, error, info};

/// ReplicationControllerController reconciles ReplicationController resources
pub struct ReplicationControllerController<S: Storage> {
    storage: Arc<S>,
    interval: Duration,
}

impl<S: Storage> ReplicationControllerController<S> {
    pub fn new(storage: Arc<S>, interval_secs: u64) -> Self {
        Self {
            storage,
            interval: Duration::from_secs(interval_secs),
        }
    }

    pub async fn run(&self) -> rusternetes_common::Result<()> {
        info!(
            "ReplicationController controller started, syncing every {:?}",
            self.interval
        );

        let mut interval = tokio::time::interval(self.interval);

        loop {
            interval.tick().await;
            if let Err(e) = self.reconcile_all().await {
                error!("Error reconciling replicationcontrollers: {}", e);
            }
        }
    }

    pub async fn reconcile_all(&self) -> rusternetes_common::Result<()> {
        debug!("Reconciling all replicationcontrollers");

        // Get all replicationcontrollers
        let prefix = build_prefix("replicationcontrollers", None);
        let rcs: Vec<ReplicationController> = self.storage.list(&prefix).await?;

        for rc in rcs {
            if let Err(e) = self.reconcile_rc(&rc).await {
                error!(
                    "Error reconciling replicationcontroller {}: {}",
                    rc.metadata.name, e
                );
            }
        }

        Ok(())
    }

    async fn reconcile_rc(&self, rc: &ReplicationController) -> rusternetes_common::Result<()> {
        let namespace = rc.metadata.namespace.as_deref().unwrap_or("default");

        // If RC is being deleted with Orphan policy, remove ownerReferences from pods
        // and remove the orphan finalizer, then delete the RC.
        if rc.metadata.deletion_timestamp.is_some() {
            let has_orphan_finalizer = rc
                .metadata
                .finalizers
                .as_ref()
                .map_or(false, |f| f.contains(&"orphan".to_string()));
            if has_orphan_finalizer {
                info!(
                    "RC {}/{} being deleted with orphan policy, removing ownerRefs from pods",
                    namespace, rc.metadata.name
                );
                // Remove ownerReferences from all owned pods
                let pods_prefix = build_prefix("pods", Some(namespace));
                let all_pods: Vec<Pod> = self.storage.list(&pods_prefix).await?;
                for pod in &all_pods {
                    let owned = pod
                        .metadata
                        .owner_references
                        .as_ref()
                        .map_or(false, |refs| refs.iter().any(|r| r.uid == rc.metadata.uid));
                    if owned {
                        let mut updated_pod = pod.clone();
                        updated_pod.metadata.owner_references =
                            updated_pod.metadata.owner_references.map(|refs| {
                                refs.into_iter()
                                    .filter(|r| r.uid != rc.metadata.uid)
                                    .collect()
                            });
                        let pod_key = build_key("pods", Some(namespace), &pod.metadata.name);
                        let _ = self.storage.update(&pod_key, &updated_pod).await;
                    }
                }
                // Remove orphan finalizer and delete the RC
                let mut updated_rc = rc.clone();
                if let Some(ref mut finalizers) = updated_rc.metadata.finalizers {
                    finalizers.retain(|f| f != "orphan");
                }
                let rc_key =
                    build_key("replicationcontrollers", Some(namespace), &rc.metadata.name);
                if updated_rc
                    .metadata
                    .finalizers
                    .as_ref()
                    .map_or(true, |f| f.is_empty())
                {
                    let _ = self.storage.delete(&rc_key).await;
                } else {
                    let _ = self.storage.update(&rc_key, &updated_rc).await;
                }
                return Ok(());
            }
            // If being deleted without orphan, skip reconciliation (let it terminate)
            return Ok(());
        }

        debug!(
            "Reconciling replicationcontroller: {}/{}",
            namespace, rc.metadata.name
        );

        // Get all pods for this replicationcontroller
        let pods_prefix = build_prefix("pods", Some(namespace));
        info!("Querying pods with prefix: {}", pods_prefix);
        let all_pods: Vec<Pod> = self.storage.list(&pods_prefix).await?;
        info!(
            "Found {} total pods in namespace {}",
            all_pods.len(),
            namespace
        );

        // Filter pods that match this replicationcontroller's selector
        // Log pod/selector matching for debugging
        if all_pods.len() > 0 {
            info!(
                "RC {}/{} selector={:?}, checking {} pods",
                namespace,
                rc.metadata.name,
                rc.spec.selector,
                all_pods.len()
            );
            for p in &all_pods {
                let pod_labels = p.metadata.labels.as_ref();
                let matches = self.matches_selector(p, rc);
                if !matches {
                    debug!(
                        "Pod {} labels={:?} does NOT match selector",
                        p.metadata.name, pod_labels
                    );
                }
            }
        }

        let rc_pods: Vec<Pod> = all_pods
            .into_iter()
            .filter(|p| {
                if !self.matches_selector(p, rc) {
                    return false;
                }
                // Only count pods owned by this RC, or orphans (no controller owner)
                let owned_by_this_rc = p
                    .metadata
                    .owner_references
                    .as_ref()
                    .map(|refs| refs.iter().any(|r| r.uid == rc.metadata.uid))
                    .unwrap_or(false);
                let is_orphan = p
                    .metadata
                    .owner_references
                    .as_ref()
                    .map(|refs| refs.is_empty() || !refs.iter().any(|r| r.controller == Some(true)))
                    .unwrap_or(true);
                // Skip pods owned by a different controller
                owned_by_this_rc || is_orphan
            })
            .collect();

        // Adopt orphan pods — set ownerReference on matching pods that don't have one
        for pod in &rc_pods {
            let has_owner = pod
                .metadata
                .owner_references
                .as_ref()
                .map(|refs| refs.iter().any(|r| r.uid == rc.metadata.uid))
                .unwrap_or(false);
            if !has_owner {
                let mut adopted_pod = pod.clone();
                let refs = adopted_pod
                    .metadata
                    .owner_references
                    .get_or_insert_with(Vec::new);
                refs.push(rusternetes_common::types::OwnerReference {
                    api_version: "v1".to_string(),
                    kind: "ReplicationController".to_string(),
                    name: rc.metadata.name.clone(),
                    uid: rc.metadata.uid.clone(),
                    controller: Some(true),
                    block_owner_deletion: Some(true),
                });
                let pod_key = build_key("pods", Some(namespace), &pod.metadata.name);
                let _ = self.storage.update(&pod_key, &adopted_pod).await;
                info!(
                    "Adopted orphan pod {} into RC {}",
                    pod.metadata.name, rc.metadata.name
                );
            }
        }

        let current_replicas = rc_pods.len() as i32;
        let desired_replicas = rc.spec.replicas.unwrap_or(1);

        info!(
            "ReplicationController {}/{}: current={}, desired={} (matched {} pods)",
            namespace,
            rc.metadata.name,
            current_replicas,
            desired_replicas,
            rc_pods.len()
        );

        let mut create_failure: Option<String> = None;
        if current_replicas < desired_replicas {
            // Need to create more pods
            let to_create = desired_replicas - current_replicas;
            for i in 0..to_create {
                if let Err(e) = self.create_pod(rc, i).await {
                    error!("Failed to create pod for RC {}: {}", rc.metadata.name, e);
                    create_failure = Some(e.to_string());
                }
            }
        } else if current_replicas > desired_replicas {
            // Need to delete excess pods
            let to_delete = current_replicas - desired_replicas;
            for pod in rc_pods.iter().take(to_delete as usize) {
                self.delete_pod(&pod.metadata.name, namespace).await?;
            }
        }

        // Re-fetch and recount pods after create/delete operations to get accurate status
        let pods_prefix = build_prefix("pods", Some(namespace));
        let all_pods_after: Vec<Pod> = self.storage.list(&pods_prefix).await?;

        let rc_pods_after: Vec<Pod> = all_pods_after
            .into_iter()
            .filter(|p| {
                if !self.matches_selector(p, rc) {
                    return false;
                }
                // Only count pods owned by this RC or orphans
                let owned_by_this_rc = p
                    .metadata
                    .owner_references
                    .as_ref()
                    .map(|refs| refs.iter().any(|r| r.uid == rc.metadata.uid))
                    .unwrap_or(false);
                let is_orphan = p
                    .metadata
                    .owner_references
                    .as_ref()
                    .map(|refs| refs.is_empty() || !refs.iter().any(|r| r.controller == Some(true)))
                    .unwrap_or(true);
                owned_by_this_rc || is_orphan
            })
            .collect();

        // Count only active (non-Failed, non-Succeeded) pods as replicas
        let active_pods: Vec<&Pod> = rc_pods_after
            .iter()
            .filter(|pod| {
                !matches!(
                    pod.status.as_ref().and_then(|s| s.phase.as_ref()),
                    Some(Phase::Failed) | Some(Phase::Succeeded)
                )
            })
            .collect();

        let final_current_replicas = active_pods.len() as i32;
        let final_ready_replicas = active_pods
            .iter()
            .filter(|pod| {
                pod.status
                    .as_ref()
                    .and_then(|s| s.conditions.as_ref())
                    .map(|conditions| {
                        conditions
                            .iter()
                            .any(|c| c.condition_type == "Ready" && c.status == "True")
                    })
                    .unwrap_or(false)
            })
            .count() as i32;

        // Check for failed pods as a failure signal
        let failed_pods = rc_pods_after
            .iter()
            .filter(|pod| {
                matches!(
                    pod.status.as_ref().and_then(|s| s.phase.as_ref()),
                    Some(Phase::Failed)
                )
            })
            .count();

        // If we had actual pod creation errors, keep them. Only add a new failure
        // message for failed pods — don't set ReplicaFailure just because pods
        // haven't started yet (K8s only sets this on actual creation errors).
        if create_failure.is_none() && final_current_replicas < desired_replicas && failed_pods > 0
        {
            create_failure = Some(format!(
                "pods for rc {}/{} failed: {} pods in Failed phase",
                namespace, rc.metadata.name, failed_pods
            ));
        }

        // Update status with accurate counts
        self.update_status(
            rc,
            final_current_replicas,
            final_ready_replicas,
            create_failure.as_deref(),
        )
        .await?;

        Ok(())
    }

    fn matches_selector(&self, pod: &Pod, rc: &ReplicationController) -> bool {
        // ReplicationController uses simple label matching (not label selectors like Deployment)
        if let Some(selector) = &rc.spec.selector {
            if let Some(pod_labels) = &pod.metadata.labels {
                for (key, value) in selector {
                    if pod_labels.get(key) != Some(value) {
                        return false;
                    }
                }
                return true;
            }
        }
        false
    }

    async fn create_pod(
        &self,
        rc: &ReplicationController,
        _index: i32,
    ) -> rusternetes_common::Result<()> {
        let namespace = rc.metadata.namespace.as_deref().unwrap_or("default");

        let pod_name = format!("{}-{}", rc.metadata.name, uuid::Uuid::new_v4());

        let mut metadata = ObjectMeta::new(&pod_name);
        metadata.namespace = Some(namespace.to_string());
        // Use template labels, falling back to the RC's selector labels.
        // In K8s, template labels must be a superset of selector labels.
        // If the template has no labels at all, use the selector to ensure
        // created pods can be matched by the controller.
        metadata.labels = rc
            .spec
            .template
            .metadata
            .as_ref()
            .and_then(|m| m.labels.clone())
            .or_else(|| rc.spec.selector.clone());
        metadata.owner_references = Some(vec![OwnerReference {
            api_version: "v1".to_string(),
            kind: "ReplicationController".to_string(),
            name: rc.metadata.name.clone(),
            uid: rc.metadata.uid.clone(),
            controller: Some(true),
            block_owner_deletion: Some(true),
        }]);

        let pod = Pod {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "Pod".to_string(),
                api_version: "v1".to_string(),
            },
            metadata,
            spec: Some(rc.spec.template.spec.clone()),
            status: Some(PodStatus {
                phase: Some(Phase::Pending),
                message: None,
                reason: None,
                host_ip: None,
                pod_ip: None,
                conditions: None,
                container_statuses: None,
                init_container_statuses: None,
                ephemeral_container_statuses: None,
                resize: None,
                resource_claim_statuses: None,
                observed_generation: None,
                host_i_ps: None,
                pod_i_ps: None,
                nominated_node_name: None,
                qos_class: None,
                start_time: None,
            }),
        };

        // Check ResourceQuota before creating pod
        super::check_resource_quota(&*self.storage, namespace)
            .await
            .map_err(|e| rusternetes_common::Error::Forbidden(e.to_string()))?;

        let key = build_key("pods", Some(namespace), &pod_name);
        self.storage.create(&key, &pod).await?;

        info!(
            "Created pod {}/{} for replicationcontroller {}",
            namespace, pod_name, rc.metadata.name
        );

        Ok(())
    }

    async fn delete_pod(&self, name: &str, namespace: &str) -> rusternetes_common::Result<()> {
        let key = build_key("pods", Some(namespace), name);
        self.storage.delete(&key).await?;

        info!("Deleted pod {}/{}", namespace, name);

        Ok(())
    }

    async fn update_status(
        &self,
        rc: &ReplicationController,
        current_replicas: i32,
        ready_replicas: i32,
        failure_message: Option<&str>,
    ) -> rusternetes_common::Result<()> {
        let namespace = rc.metadata.namespace.as_deref().unwrap_or("default");
        let key = build_key("replicationcontrollers", Some(namespace), &rc.metadata.name);

        let desired_replicas = rc.spec.replicas.unwrap_or(1);
        let conditions = if let Some(msg) = failure_message {
            // Only set ReplicaFailure when there's an actual failure message
            // (quota exceeded, pod creation failed, etc.)
            Some(vec![ReplicationControllerCondition {
                condition_type: "ReplicaFailure".to_string(),
                status: "True".to_string(),
                last_transition_time: Some(Utc::now()),
                reason: Some("FailedCreate".to_string()),
                message: Some(msg.to_string()),
            }])
        } else {
            // Clear failure conditions when no creation failure — K8s removes
            // the ReplicaFailure condition once pods can be created again
            None
        };

        // Re-read from storage for fresh resourceVersion to avoid CAS conflicts
        let mut updated_rc: ReplicationController = match self.storage.get(&key).await {
            Ok(rc) => rc,
            Err(_) => rc.clone(),
        };
        let conditions_clone = conditions.clone();
        updated_rc.status = Some(rusternetes_common::resources::ReplicationControllerStatus {
            replicas: current_replicas,
            fully_labeled_replicas: Some(current_replicas),
            ready_replicas: Some(ready_replicas),
            available_replicas: Some(ready_replicas),
            observed_generation: updated_rc.metadata.generation,
            conditions,
        });

        if let Err(e) = self.storage.update(&key, &updated_rc).await {
            // CAS conflict — re-read and retry once to ensure condition updates persist
            debug!("RC status update CAS conflict, retrying: {}", e);
            if let Ok(mut fresh_rc) = self.storage.get::<ReplicationController>(&key).await {
                fresh_rc.status =
                    Some(rusternetes_common::resources::ReplicationControllerStatus {
                        replicas: current_replicas,
                        fully_labeled_replicas: Some(current_replicas),
                        ready_replicas: Some(ready_replicas),
                        available_replicas: Some(ready_replicas),
                        observed_generation: fresh_rc.metadata.generation,
                        conditions: conditions_clone,
                    });
                let _ = self.storage.update(&key, &fresh_rc).await;
            }
        }

        debug!(
            "Updated status for replicationcontroller {}/{}",
            namespace, rc.metadata.name
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::resources::workloads::{PodTemplateSpec, ReplicationControllerSpec};
    use rusternetes_common::types::ObjectMeta;
    use rusternetes_storage::MemoryStorage;
    use std::collections::HashMap;

    fn make_rc(
        name: &str,
        ns: &str,
        replicas: i32,
        selector: HashMap<String, String>,
        template_labels: Option<HashMap<String, String>>,
    ) -> ReplicationController {
        ReplicationController {
            type_meta: rusternetes_common::types::TypeMeta {
                kind: "ReplicationController".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: {
                let mut m = ObjectMeta::new(name);
                m.namespace = Some(ns.to_string());
                m
            },
            spec: ReplicationControllerSpec {
                replicas: Some(replicas),
                selector: Some(selector),
                template: PodTemplateSpec {
                    metadata: template_labels.map(|labels| {
                        let mut m = ObjectMeta::new("");
                        m.labels = Some(labels);
                        m
                    }),
                    spec: rusternetes_common::resources::PodSpec {
                        containers: vec![rusternetes_common::resources::Container {
                            name: "test".to_string(),
                            image: "busybox".to_string(),
                            command: None,
                            args: None,
                            working_dir: None,
                            ports: None,
                            env: None,
                            env_from: None,
                            resources: None,
                            volume_mounts: None,
                            volume_devices: None,
                            liveness_probe: None,
                            readiness_probe: None,
                            startup_probe: None,
                            lifecycle: None,
                            termination_message_path: None,
                            termination_message_policy: None,
                            image_pull_policy: None,
                            security_context: None,
                            stdin: None,
                            stdin_once: None,
                            tty: None,
                            resize_policy: None,
                            restart_policy: None,
                        }],
                        init_containers: None,
                        ephemeral_containers: None,
                        restart_policy: Some("Always".to_string()),
                        termination_grace_period_seconds: None,
                        dns_policy: None,
                        node_selector: None,
                        service_account_name: None,
                        service_account: None,
                        automount_service_account_token: None,
                        node_name: None,
                        host_network: None,
                        host_pid: None,
                        host_ipc: None,
                        security_context: None,
                        image_pull_secrets: None,
                        hostname: None,
                        subdomain: None,
                        affinity: None,
                        scheduler_name: None,
                        tolerations: None,
                        host_aliases: None,
                        priority_class_name: None,
                        priority: None,
                        preemption_policy: None,
                        overhead: None,
                        topology_spread_constraints: None,
                        volumes: None,
                        active_deadline_seconds: None,
                        dns_config: None,
                        enable_service_links: None,
                        readiness_gates: None,
                        runtime_class_name: None,
                        os: None,
                        set_hostname_as_fqdn: None,
                        share_process_namespace: None,
                        scheduling_gates: None,
                        resource_claims: None,
                        host_users: None,
                        resources: None,
                    },
                },
                min_ready_seconds: None,
            },
            status: None,
        }
    }

    #[tokio::test]
    async fn test_rc_creates_pods_with_selector_labels_when_template_has_none() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = ReplicationControllerController::new(storage.clone(), 5);

        let mut selector = HashMap::new();
        selector.insert("app".to_string(), "test".to_string());

        // Template has NO labels (metadata is None)
        let rc = make_rc("test-rc", "default", 1, selector.clone(), None);
        storage
            .create("/registry/replicationcontrollers/default/test-rc", &rc)
            .await
            .unwrap();

        controller.reconcile_all().await.unwrap();

        // The pod should exist and have the selector labels
        let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
        assert!(!pods.is_empty(), "RC should have created a pod");

        let pod = &pods[0];
        let pod_labels = pod
            .metadata
            .labels
            .as_ref()
            .expect("Pod should have labels");
        assert_eq!(
            pod_labels.get("app"),
            Some(&"test".to_string()),
            "Pod should inherit selector labels when template has no labels"
        );
    }

    #[tokio::test]
    async fn test_rc_creates_pods_with_template_labels_when_present() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = ReplicationControllerController::new(storage.clone(), 5);

        let mut selector = HashMap::new();
        selector.insert("app".to_string(), "test".to_string());

        let mut template_labels = HashMap::new();
        template_labels.insert("app".to_string(), "test".to_string());
        template_labels.insert("version".to_string(), "v1".to_string());

        let rc = make_rc("test-rc", "default", 1, selector, Some(template_labels));
        storage
            .create("/registry/replicationcontrollers/default/test-rc", &rc)
            .await
            .unwrap();

        controller.reconcile_all().await.unwrap();

        let pods: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
        assert!(!pods.is_empty(), "RC should have created a pod");

        let pod_labels = pods[0].metadata.labels.as_ref().unwrap();
        assert_eq!(pod_labels.get("app"), Some(&"test".to_string()));
        assert_eq!(
            pod_labels.get("version"),
            Some(&"v1".to_string()),
            "Pod should use template labels when present"
        );
    }

    #[tokio::test]
    async fn test_rc_matches_created_pods_on_next_reconcile() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = ReplicationControllerController::new(storage.clone(), 5);

        let mut selector = HashMap::new();
        selector.insert("app".to_string(), "test".to_string());

        let rc = make_rc("test-rc", "default", 2, selector, None);
        storage
            .create("/registry/replicationcontrollers/default/test-rc", &rc)
            .await
            .unwrap();

        // First reconcile: creates pods
        controller.reconcile_all().await.unwrap();

        let pods_after_first: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
        assert_eq!(pods_after_first.len(), 2, "Should create 2 pods");

        // Second reconcile: should match existing pods, not create more
        controller.reconcile_all().await.unwrap();

        let pods_after_second: Vec<Pod> = storage.list("/registry/pods/default/").await.unwrap();
        assert_eq!(
            pods_after_second.len(),
            2,
            "Should still have 2 pods — RC must match its own pods on second reconcile"
        );
    }
}
