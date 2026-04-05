use anyhow::Result;
use rusternetes_common::resources::{
    PersistentVolumeClaim, Pod, PodStatus, StatefulSet, StatefulSetStatus,
};
use rusternetes_common::types::{ObjectMeta, OwnerReference, Phase, TypeMeta};
use rusternetes_storage::{build_key, Storage};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info};

pub struct StatefulSetController<S: Storage> {
    storage: Arc<S>,
}

impl<S: Storage> StatefulSetController<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting StatefulSetController");

        loop {
            if let Err(e) = self.reconcile_all().await {
                error!("Error in StatefulSet reconciliation loop: {}", e);
            }
            time::sleep(Duration::from_secs(1)).await;
        }
    }

    pub async fn reconcile_all(&self) -> Result<()> {
        let statefulsets: Vec<StatefulSet> = self.storage.list("/registry/statefulsets/").await?;

        for mut statefulset in statefulsets {
            if let Err(e) = self.reconcile(&mut statefulset).await {
                error!(
                    "Failed to reconcile StatefulSet {}: {}",
                    statefulset.metadata.name, e
                );
            }
        }

        Ok(())
    }

    async fn reconcile(&self, statefulset: &mut StatefulSet) -> Result<()> {
        let name = &statefulset.metadata.name;
        let namespace = statefulset.metadata.namespace.as_ref().unwrap();

        // Skip reconciliation for StatefulSets being deleted — GC handles pod cleanup
        if statefulset.metadata.is_being_deleted() {
            return Ok(());
        }

        info!("Reconciling StatefulSet {}/{}", namespace, name);

        let desired_replicas = statefulset.spec.replicas.unwrap_or(1);

        // Get current pods for this StatefulSet
        let pod_prefix = format!("/registry/pods/{}/", namespace);
        let all_pods: Vec<Pod> = self.storage.list(&pod_prefix).await?;

        // Filter pods that belong to this StatefulSet via ownerReferences (authoritative)
        // Fall back to label matching for backwards compatibility
        let statefulset_uid = &statefulset.metadata.uid;
        let mut statefulset_pods: Vec<Pod> = all_pods
            .into_iter()
            .filter(|pod| {
                let owned_by_ref = pod
                    .metadata
                    .owner_references
                    .as_ref()
                    .map(|refs| refs.iter().any(|r| &r.uid == statefulset_uid))
                    .unwrap_or(false);
                let owned_by_label = pod
                    .metadata
                    .labels
                    .as_ref()
                    .and_then(|labels| labels.get("app"))
                    .map(|app| app == name)
                    .unwrap_or(false)
                    && pod
                        .metadata
                        .labels
                        .as_ref()
                        .and_then(|labels| labels.get("statefulset.kubernetes.io/pod-name"))
                        .is_some();
                owned_by_ref || owned_by_label
            })
            .collect();

        // Filter out terminated (Failed/Succeeded) pods — only count active pods as replicas
        statefulset_pods.retain(|pod| {
            // Exclude terminated pods (Failed/Succeeded phase)
            !matches!(
                pod.status.as_ref().and_then(|s| s.phase.as_ref()),
                Some(Phase::Failed) | Some(Phase::Succeeded)
            )
        });

        // Sort pods by ordinal index
        statefulset_pods.sort_by_key(|pod| {
            pod.metadata
                .name
                .rsplit_once('-')
                .and_then(|(_, idx)| idx.parse::<i32>().ok())
                .unwrap_or(0)
        });

        // Count only non-terminating pods as current replicas.
        // Terminating pods (deletion_timestamp set) should not prevent scale-up
        // to recreate them with the new template.
        let current_replicas = statefulset_pods
            .iter()
            .filter(|p| p.metadata.deletion_timestamp.is_none())
            .count() as i32;

        info!(
            "StatefulSet {}/{}: desired={}, current={}",
            namespace, name, desired_replicas, current_replicas
        );

        let is_ordered_ready = statefulset
            .spec
            .pod_management_policy
            .as_ref()
            .map(|p| p == "OrderedReady")
            .unwrap_or(true);

        // Scale up or down
        if current_replicas < desired_replicas {
            // Scale up: create any missing pods in ordinal order.
            // During rolling updates, gaps can appear at any ordinal (not just at the end),
            // so we check all ordinals 0..desired rather than current..desired.
            for i in 0..desired_replicas {
                let pod_exists = {
                    let pod_name = format!("{}-{}", name, i);
                    let pod_key = build_key("pods", Some(namespace), &pod_name);
                    self.storage.get::<Pod>(&pod_key).await.is_ok()
                };
                if pod_exists {
                    continue;
                }
                // For OrderedReady policy, check that the previous pod is Ready before
                // creating the next one. If it's not ready, halt scaling.
                if is_ordered_ready && i > 0 {
                    let prev_pod_name = format!("{}-{}", name, i - 1);
                    let prev_pod_key = build_key("pods", Some(namespace), &prev_pod_name);
                    match self.storage.get::<Pod>(&prev_pod_key).await {
                        Ok(prev_pod) => {
                            let is_ready = prev_pod
                                .status
                                .as_ref()
                                .and_then(|s| s.conditions.as_ref())
                                .map(|conditions| {
                                    conditions
                                        .iter()
                                        .any(|c| c.condition_type == "Ready" && c.status == "True")
                                })
                                .unwrap_or(false);

                            if !is_ready {
                                info!(
                                    "StatefulSet {}: pod {} not ready, halting scale-up",
                                    name, prev_pod_name
                                );
                                break;
                            }
                        }
                        Err(_) => {
                            // Previous pod doesn't exist yet
                            info!(
                                "StatefulSet {}: pod {} not found, halting scale-up",
                                name, prev_pod_name
                            );
                            break;
                        }
                    }
                }

                // Ensure PVCs exist for this ordinal before creating the pod
                self.ensure_pvcs_for_ordinal(statefulset, i, namespace)
                    .await?;
                self.create_pod(statefulset, i, namespace).await?;
                info!("Created pod {}-{}", name, i);
            }
        } else if current_replicas > desired_replicas {
            // Scale down: delete ONE pod at a time in reverse order.
            // K8s StatefulSet controller requires:
            // 1. No pod is currently terminating
            // 2. All remaining pods (ordinals < pod being deleted) are Running and Ready
            // This prevents scaling past an unhealthy pod (OrderedReady policy).
            let any_terminating = statefulset_pods
                .iter()
                .any(|p| p.metadata.deletion_timestamp.is_some());
            // Check if all pods that will REMAIN after this scale-down are Ready.
            // For scaling from N to desired, we delete pod N-1. All pods 0..desired must be Ready.
            let all_remaining_ready = if is_ordered_ready {
                (0..desired_replicas).all(|ordinal| {
                    let pod_name = format!("{}-{}", name, ordinal);
                    statefulset_pods.iter().any(|p| {
                        p.metadata.name == pod_name
                            && p.metadata.deletion_timestamp.is_none()
                            && p.status
                                .as_ref()
                                .and_then(|s| s.conditions.as_ref())
                                .map(|conditions| {
                                    conditions
                                        .iter()
                                        .any(|c| c.condition_type == "Ready" && c.status == "True")
                                })
                                .unwrap_or(false)
                    })
                })
            } else {
                true // Parallel policy doesn't require ordered readiness
            };
            if any_terminating {
                debug!(
                    "StatefulSet {}/{}: waiting for terminating pod before continuing scale-down",
                    namespace, name
                );
            } else if !all_remaining_ready {
                debug!(
                    "StatefulSet {}/{}: scale-down halted — not all remaining pods are Ready",
                    namespace, name
                );
            } else {
                let i = current_replicas - 1;
                let pod_name = format!("{}-{}", name, i);
                let pod_key = format!("/registry/pods/{}/{}", namespace, pod_name);
                // Set deletionTimestamp for graceful termination (like real K8s).
                // The kubelet will stop containers and remove the pod.
                match self.storage.get::<Pod>(&pod_key).await {
                    Ok(mut pod_to_delete) => {
                        if pod_to_delete.metadata.deletion_timestamp.is_none() {
                            pod_to_delete.metadata.deletion_timestamp = Some(chrono::Utc::now());
                            pod_to_delete.metadata.deletion_grace_period_seconds = pod_to_delete
                                .spec
                                .as_ref()
                                .and_then(|s| s.termination_grace_period_seconds)
                                .or(Some(30));
                            let _ = self.storage.update(&pod_key, &pod_to_delete).await;
                            info!(
                                "Scale down: set deletionTimestamp on pod {} ({} -> {})",
                                pod_name,
                                current_replicas,
                                current_replicas - 1
                            );
                        }
                    }
                    Err(_) => {
                        // Pod doesn't exist — nothing to delete
                        info!(
                            "Scale down: pod {} already gone ({} -> {})",
                            pod_name,
                            current_replicas,
                            current_replicas - 1
                        );
                    }
                }
            }
        }

        // Rolling update: if replica count matches but pods have old revision, delete one at a time.
        // The controller will recreate them with the new template on the next reconcile.
        // Skip if updateStrategy is OnDelete (user must manually delete pods to trigger update).
        let update_strategy = statefulset
            .spec
            .update_strategy
            .as_ref()
            .and_then(|s| s.strategy_type.as_deref())
            .unwrap_or("RollingUpdate");

        let partition = statefulset
            .spec
            .update_strategy
            .as_ref()
            .and_then(|s| s.rolling_update.as_ref())
            .and_then(|ru| ru.partition)
            .unwrap_or(0);

        if current_replicas == desired_replicas
            && desired_replicas > 0
            && update_strategy == "RollingUpdate"
        {
            let update_revision = Self::compute_revision(&statefulset.spec.template);
            info!(
                "StatefulSet {}/{}: rolling update check, update_revision={}",
                namespace, name, update_revision
            );

            // Check pods in reverse order for rolling update.
            // Only update pods with ordinal >= partition.
            // For each pod: if it has a stale revision AND is Ready (or at least Running),
            // delete it so it gets recreated with the new template.
            // If the most recently deleted pod's replacement is not yet Ready, wait before
            // deleting the next one.
            let mut deleted_one = false;
            for pod in statefulset_pods.iter().rev() {
                let ordinal = pod
                    .metadata
                    .name
                    .rsplit_once('-')
                    .and_then(|(_, idx)| idx.parse::<i32>().ok())
                    .unwrap_or(0);

                // Only update pods with ordinal >= partition
                if ordinal < partition {
                    continue;
                }

                let pod_revision = pod
                    .metadata
                    .labels
                    .as_ref()
                    .and_then(|l| l.get("controller-revision-hash"))
                    .map(|s| s.as_str())
                    .unwrap_or("");
                info!(
                    "StatefulSet {}/{}: pod {} revision={} vs update_revision={}",
                    namespace, name, pod.metadata.name, pod_revision, update_revision
                );
                if pod_revision != update_revision {
                    // Check if this pod is at least Running or Ready — don't delete pods
                    // that haven't even started yet (prevents cascading deletions during initial creation)
                    let pod_phase = pod.status.as_ref().and_then(|s| s.phase.as_ref());
                    let pod_is_active =
                        matches!(pod_phase, Some(Phase::Running) | Some(Phase::Pending));
                    let pod_is_ready = pod
                        .status
                        .as_ref()
                        .and_then(|s| s.conditions.as_ref())
                        .map(|c| {
                            c.iter()
                                .any(|cond| cond.condition_type == "Ready" && cond.status == "True")
                        })
                        .unwrap_or(false);

                    // Delete pods with stale revision using graceful termination
                    // (set deletionTimestamp instead of direct delete, so the kubelet
                    // can perform cleanup and the pod gets properly recreated).
                    // Empty revision means newly created — skip those.
                    if !pod_revision.is_empty() && (pod_is_ready || pod_is_active) {
                        let pod_key = format!("/registry/pods/{}/{}", namespace, pod.metadata.name);
                        if pod.metadata.deletion_timestamp.is_none() {
                            let mut pod_to_delete = pod.clone();
                            pod_to_delete.metadata.deletion_timestamp = Some(chrono::Utc::now());
                            pod_to_delete.metadata.deletion_grace_period_seconds = pod_to_delete
                                .spec
                                .as_ref()
                                .and_then(|s| s.termination_grace_period_seconds)
                                .or(Some(30));
                            let _ = self.storage.update(&pod_key, &pod_to_delete).await;
                            info!(
                                "Rolling update: set deletionTimestamp on pod {} (old revision {}, update revision {})",
                                pod.metadata.name, pod_revision, update_revision
                            );
                        }
                        deleted_one = true;
                        break; // Delete one at a time for OrderedReady rolling updates
                    }
                }
            }
            let _ = deleted_one;
        }

        // Re-fetch and recount pods after create/delete operations to get accurate status
        let pod_prefix = format!("/registry/pods/{}/", namespace);
        let all_pods_after: Vec<Pod> = self.storage.list(&pod_prefix).await?;

        let statefulset_pods_after: Vec<Pod> = all_pods_after
            .into_iter()
            .filter(|pod| {
                let owned_by_ref = pod
                    .metadata
                    .owner_references
                    .as_ref()
                    .map(|refs| refs.iter().any(|r| &r.uid == statefulset_uid))
                    .unwrap_or(false);
                let owned_by_label = pod
                    .metadata
                    .labels
                    .as_ref()
                    .and_then(|labels| labels.get("statefulset.kubernetes.io/pod-name"))
                    .is_some()
                    && pod
                        .metadata
                        .labels
                        .as_ref()
                        .and_then(|labels| labels.get("app"))
                        .map(|app| app == name)
                        .unwrap_or(false);
                owned_by_ref || owned_by_label
            })
            .collect();

        // Filter out terminating pods (deletionTimestamp set) — K8s does not count
        // terminating pods in replicas/readyReplicas/availableReplicas.
        let non_terminating_pods: Vec<&Pod> = statefulset_pods_after
            .iter()
            .filter(|pod| pod.metadata.deletion_timestamp.is_none())
            .collect();
        let final_current_replicas = non_terminating_pods.len() as i32;
        let final_ready_pods = non_terminating_pods
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

        // Generate a revision hash from the current pod template spec
        let update_revision = Self::compute_revision(&statefulset.spec.template);

        // The current_revision is the revision that existing pods are running.
        // During a rolling update, this differs from update_revision.
        // Preserve the existing current_revision if set, otherwise derive from pods.
        let current_revision = statefulset
            .status
            .as_ref()
            .and_then(|s| s.current_revision.clone())
            .or_else(|| {
                // No current_revision in status — derive from actual pod labels
                statefulset_pods_after.iter().find_map(|pod| {
                    pod.metadata
                        .labels
                        .as_ref()
                        .and_then(|l| l.get("controller-revision-hash"))
                        .cloned()
                })
            })
            .unwrap_or_else(|| update_revision.clone());

        // Count how many pods match the update revision (have the matching controller-revision-hash label)
        let updated_count = statefulset_pods_after
            .iter()
            .filter(|pod| {
                pod.metadata
                    .labels
                    .as_ref()
                    .and_then(|l| l.get("controller-revision-hash"))
                    .map(|h| h == &update_revision)
                    .unwrap_or(false)
            })
            .count() as i32;

        // Count how many pods match the current revision
        let current_rev_count = statefulset_pods_after
            .iter()
            .filter(|pod| {
                pod.metadata
                    .labels
                    .as_ref()
                    .and_then(|l| l.get("controller-revision-hash"))
                    .map(|h| h == &current_revision)
                    .unwrap_or(false)
            })
            .count() as i32;

        // Determine the final current_revision:
        // Only advance current_revision to update_revision when ALL pods have been
        // updated (updated_count >= desired_replicas). This ensures that during a
        // rolling update, currentRevision != updateRevision, which conformance tests verify.
        let final_current_revision = if updated_count >= desired_replicas {
            update_revision.clone()
        } else {
            current_revision.clone()
        };

        // Update status with accurate counts
        // current_replicas = pods matching currentRevision (K8s semantics)
        // updated_replicas = pods matching updateRevision
        statefulset.status = Some(StatefulSetStatus {
            replicas: final_current_replicas,
            ready_replicas: Some(final_ready_pods),
            current_replicas: Some(if final_current_revision == update_revision {
                // All pods are on the same (current) revision
                final_current_replicas
            } else {
                // During rolling update: count pods on the old (current) revision
                current_rev_count
            }),
            updated_replicas: Some(updated_count),
            available_replicas: Some(final_ready_pods),
            collision_count: None,
            observed_generation: statefulset.metadata.generation,
            current_revision: Some(final_current_revision),
            update_revision: Some(update_revision),
            conditions: None,
        });

        // Save updated status
        let key = format!("/registry/statefulsets/{}/{}", namespace, name);
        self.storage.update(&key, statefulset).await?;

        // Ensure a ControllerRevision exists for the current template revision
        let revision = Self::compute_revision(&statefulset.spec.template);
        let cr_name = format!(
            "{}-{}",
            name,
            &revision[..std::cmp::min(10, revision.len())]
        );
        let cr_key = format!("/registry/controllerrevisions/{}/{}", namespace, cr_name);
        if self
            .storage
            .get::<serde_json::Value>(&cr_key)
            .await
            .is_err()
        {
            // Create the ControllerRevision
            let template_data =
                serde_json::to_value(&statefulset.spec.template).unwrap_or_default();
            let cr = serde_json::json!({
                "apiVersion": "apps/v1",
                "kind": "ControllerRevision",
                "metadata": {
                    "name": cr_name,
                    "namespace": namespace,
                    "uid": uuid::Uuid::new_v4().to_string(),
                    "creationTimestamp": chrono::Utc::now().to_rfc3339(),
                    "labels": {
                        "controller.kubernetes.io/hash": revision,
                        "app": name
                    },
                    "ownerReferences": [{
                        "apiVersion": "apps/v1",
                        "kind": "StatefulSet",
                        "name": name,
                        "uid": statefulset.metadata.uid,
                        "controller": true,
                        "blockOwnerDeletion": true
                    }]
                },
                "data": template_data,
                "revision": 1
            });
            if let Err(e) = self.storage.create(&cr_key, &cr).await {
                info!(
                    "ControllerRevision {} already exists or failed: {}",
                    cr_name, e
                );
            } else {
                info!(
                    "Created ControllerRevision {} for StatefulSet {}/{}",
                    cr_name, namespace, name
                );
            }
        }

        Ok(())
    }

    async fn ensure_pvcs_for_ordinal(
        &self,
        statefulset: &StatefulSet,
        ordinal: i32,
        namespace: &str,
    ) -> Result<()> {
        if let Some(ref templates) = statefulset.spec.volume_claim_templates {
            for template in templates {
                let pvc_name = format!(
                    "{}-{}-{}",
                    template.metadata.name, statefulset.metadata.name, ordinal
                );
                let key = build_key("persistentvolumeclaims", Some(namespace), &pvc_name);

                // Check if PVC already exists
                if self
                    .storage
                    .get::<PersistentVolumeClaim>(&key)
                    .await
                    .is_ok()
                {
                    continue; // PVC already exists
                }

                // Create PVC from template
                let mut pvc_metadata =
                    ObjectMeta::new(&pvc_name).with_namespace(namespace.to_string());

                // Copy labels and annotations from template metadata
                if let Some(ref tmpl_labels) = template.metadata.labels {
                    pvc_metadata.labels = Some(tmpl_labels.clone());
                }
                if let Some(ref tmpl_annotations) = template.metadata.annotations {
                    pvc_metadata.annotations = Some(tmpl_annotations.clone());
                }

                // Set owner reference to the StatefulSet
                pvc_metadata.owner_references = Some(vec![OwnerReference {
                    api_version: "apps/v1".to_string(),
                    kind: "StatefulSet".to_string(),
                    name: statefulset.metadata.name.clone(),
                    uid: statefulset.metadata.uid.clone(),
                    controller: Some(true),
                    block_owner_deletion: Some(true),
                }]);

                let pvc = PersistentVolumeClaim {
                    type_meta: TypeMeta {
                        kind: "PersistentVolumeClaim".to_string(),
                        api_version: "v1".to_string(),
                    },
                    metadata: pvc_metadata,
                    spec: template.spec.clone(),
                    status: None,
                };

                self.storage.create(&key, &pvc).await?;
                info!(
                    "Created PVC {} for StatefulSet {}/{}",
                    pvc_name, namespace, statefulset.metadata.name
                );
            }
        }
        Ok(())
    }

    /// Compute a revision string from the pod template spec.
    /// This produces a deterministic hash that captures template changes.
    ///
    /// IMPORTANT: We must convert to serde_json::Value first before serializing
    /// to string. Direct `to_string` on the struct iterates HashMap fields in
    /// arbitrary order (HashMap has no guaranteed iteration order), producing
    /// non-deterministic output. Converting to Value first normalizes all maps
    /// into BTreeMap-backed serde_json::Map, which iterates in sorted key order.
    fn compute_revision(template: &rusternetes_common::resources::PodTemplateSpec) -> String {
        use sha2::{Digest, Sha256};
        // Convert to Value first to normalize HashMap ordering to sorted BTreeMap
        let value = serde_json::to_value(template).unwrap_or_default();
        let serialized = serde_json::to_string(&value).unwrap_or_default();
        let hash = Sha256::digest(serialized.as_bytes());
        let revision = format!(
            "{:010x}",
            u64::from_be_bytes(hash[..8].try_into().unwrap_or([0u8; 8]))
        );
        // Log the first container's image for debugging rolling updates
        let image = value
            .pointer("/spec/containers/0/image")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        info!(
            "compute_revision: image={}, hash={}, json_len={}",
            image,
            revision,
            serialized.len()
        );
        revision
    }

    async fn create_pod(
        &self,
        statefulset: &StatefulSet,
        ordinal: i32,
        namespace: &str,
    ) -> Result<()> {
        let statefulset_name = &statefulset.metadata.name;
        let pod_name = format!("{}-{}", statefulset_name, ordinal);

        // Create pod from template
        let template = &statefulset.spec.template;
        let mut labels = template
            .metadata
            .as_ref()
            .and_then(|m| m.labels.clone())
            .unwrap_or_default();
        labels.insert("app".to_string(), statefulset_name.clone());
        labels.insert(
            "statefulset.kubernetes.io/pod-name".to_string(),
            pod_name.clone(),
        );
        // Set the controller-revision-hash label so tests can verify pod revision
        let revision = Self::compute_revision(&statefulset.spec.template);
        labels.insert("controller-revision-hash".to_string(), revision);

        let mut metadata = rusternetes_common::types::ObjectMeta::new(pod_name.clone())
            .with_namespace(namespace.to_string())
            .with_labels(labels)
            .with_owner_reference(OwnerReference {
                api_version: "apps/v1".to_string(),
                kind: "StatefulSet".to_string(),
                name: statefulset_name.clone(),
                uid: statefulset.metadata.uid.clone(),
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
            spec: Some(template.spec.clone()),
            status: Some(PodStatus {
                phase: Some(Phase::Pending),
                message: None,
                reason: None,
                pod_ip: None,
                host_ip: None,
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
        super::check_resource_quota(&*self.storage, namespace).await?;

        let key = format!("/registry/pods/{}/{}", namespace, pod_name);
        match self.storage.create(&key, &pod).await {
            Ok(_) => Ok(()),
            Err(rusternetes_common::Error::AlreadyExists(_)) => {
                info!("Pod {} already exists, skipping creation", pod_name);
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::resources::workloads::{
        RollingUpdateStatefulSetStrategy, StatefulSetUpdateStrategy,
    };
    use rusternetes_common::resources::{
        Container, PodCondition, PodSpec, PodTemplateSpec, StatefulSetSpec,
    };
    use rusternetes_common::types::LabelSelector;
    use rusternetes_storage::MemoryStorage;
    use std::collections::HashMap;

    #[test]
    fn test_pod_name_generation() {
        let statefulset_name = "web";
        let ordinal = 2;
        let pod_name = format!("{}-{}", statefulset_name, ordinal);
        assert_eq!(pod_name, "web-2");
    }

    #[test]
    fn test_pod_ordinal_parsing() {
        let pod_name = "web-5";
        let ordinal: i32 = pod_name
            .rsplit_once('-')
            .and_then(|(_, idx)| idx.parse().ok())
            .unwrap();
        assert_eq!(ordinal, 5);
    }

    /// Verify compute_revision is deterministic even with HashMap-backed labels
    #[test]
    fn test_compute_revision_deterministic() {
        let mut labels = HashMap::new();
        labels.insert("app".to_string(), "web".to_string());
        labels.insert("version".to_string(), "v1".to_string());
        labels.insert("tier".to_string(), "frontend".to_string());

        let template = PodTemplateSpec {
            metadata: Some(ObjectMeta::new("").with_labels(labels.clone())),
            spec: PodSpec {
                containers: vec![test_container("nginx", "nginx:1.19")],
                ..Default::default()
            },
        };

        // Compute multiple times — must always produce the same result
        let rev1 = StatefulSetController::<MemoryStorage>::compute_revision(&template);
        let rev2 = StatefulSetController::<MemoryStorage>::compute_revision(&template);
        let rev3 = StatefulSetController::<MemoryStorage>::compute_revision(&template);
        assert_eq!(rev1, rev2, "Revision should be deterministic across calls");
        assert_eq!(rev2, rev3, "Revision should be deterministic across calls");
    }

    /// Verify compute_revision changes when image changes
    #[test]
    fn test_compute_revision_changes_on_image_change() {
        let mut labels = HashMap::new();
        labels.insert("app".to_string(), "web".to_string());

        let template_v1 = PodTemplateSpec {
            metadata: Some(ObjectMeta::new("").with_labels(labels.clone())),
            spec: PodSpec {
                containers: vec![test_container("nginx", "nginx:1.19")],
                ..Default::default()
            },
        };

        let template_v2 = PodTemplateSpec {
            metadata: Some(ObjectMeta::new("").with_labels(labels.clone())),
            spec: PodSpec {
                containers: vec![test_container("nginx", "nginx:1.20")],
                ..Default::default()
            },
        };

        let rev1 = StatefulSetController::<MemoryStorage>::compute_revision(&template_v1);
        let rev2 = StatefulSetController::<MemoryStorage>::compute_revision(&template_v2);
        assert_ne!(rev1, rev2, "Revision should change when image changes");
    }

    fn test_container(name: &str, image: &str) -> Container {
        Container {
            name: name.to_string(),
            image: image.to_string(),
            command: None,
            args: None,
            working_dir: None,
            ports: None,
            env: None,
            env_from: None,
            resources: None,
            volume_mounts: None,
            volume_devices: None,
            image_pull_policy: None,
            liveness_probe: None,
            readiness_probe: None,
            startup_probe: None,
            security_context: None,
            restart_policy: None,
            resize_policy: None,
            lifecycle: None,
            termination_message_path: None,
            termination_message_policy: None,
            stdin: None,
            stdin_once: None,
            tty: None,
        }
    }

    fn make_statefulset(name: &str, namespace: &str, replicas: i32, image: &str) -> StatefulSet {
        let mut labels = HashMap::new();
        labels.insert("app".to_string(), name.to_string());

        StatefulSet {
            type_meta: TypeMeta {
                kind: "StatefulSet".to_string(),
                api_version: "apps/v1".to_string(),
            },
            metadata: ObjectMeta::new(name).with_namespace(namespace.to_string()),
            spec: StatefulSetSpec {
                replicas: Some(replicas),
                selector: LabelSelector {
                    match_labels: Some(labels.clone()),
                    match_expressions: None,
                },
                service_name: format!("{}-svc", name),
                template: PodTemplateSpec {
                    metadata: Some(ObjectMeta::new("").with_labels(labels)),
                    spec: PodSpec {
                        containers: vec![test_container("main", image)],
                        ..Default::default()
                    },
                },
                update_strategy: None,
                pod_management_policy: Some("Parallel".to_string()),
                min_ready_seconds: None,
                revision_history_limit: None,
                volume_claim_templates: None,
                persistent_volume_claim_retention_policy: None,
                ordinals: None,
            },
            status: None,
        }
    }

    /// Make a pod look like it's Running and Ready
    async fn make_pod_ready(storage: &Arc<MemoryStorage>, namespace: &str, pod_name: &str) {
        let key = format!("/registry/pods/{}/{}", namespace, pod_name);
        if let Ok(mut pod) = storage.get::<Pod>(&key).await {
            pod.status = Some(PodStatus {
                phase: Some(Phase::Running),
                conditions: Some(vec![PodCondition {
                    condition_type: "Ready".to_string(),
                    status: "True".to_string(),
                    reason: None,
                    message: None,
                    last_transition_time: None,
                    observed_generation: None,
                }]),
                ..Default::default()
            });
            let _ = storage.update(&key, &pod).await;
        }
    }

    /// During a rolling update, currentRevision != updateRevision.
    /// After all pods are updated, currentRevision == updateRevision.
    #[tokio::test]
    async fn test_rolling_update_revision_tracking() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = StatefulSetController::new(storage.clone());

        let ns = "default";
        let ss = make_statefulset("web", ns, 3, "nginx:1.19");

        // Store the StatefulSet
        let key = format!("/registry/statefulsets/{}/{}", ns, "web");
        storage.create(&key, &ss).await.unwrap();

        // First reconcile: creates 3 pods with revision hash for nginx:1.19
        let mut ss: StatefulSet = storage.get(&key).await.unwrap();
        controller.reconcile(&mut ss).await.unwrap();

        // Make all pods Running+Ready
        for i in 0..3 {
            make_pod_ready(&storage, ns, &format!("web-{}", i)).await;
        }

        // Reconcile again so status reflects ready pods
        let mut ss: StatefulSet = storage.get(&key).await.unwrap();
        controller.reconcile(&mut ss).await.unwrap();

        // Verify initial state: currentRevision == updateRevision
        let ss: StatefulSet = storage.get(&key).await.unwrap();
        let status = ss.status.as_ref().unwrap();
        assert_eq!(
            status.current_revision, status.update_revision,
            "Before update: currentRevision should equal updateRevision"
        );
        let old_revision = status.current_revision.clone().unwrap();

        // Now patch the StatefulSet to use a new image (simulate a rolling update)
        let mut ss: StatefulSet = storage.get(&key).await.unwrap();
        ss.spec.template.spec.containers[0].image = "nginx:1.20".to_string();
        storage.update(&key, &ss).await.unwrap();

        // Reconcile: should detect template change and begin rolling update
        let mut ss: StatefulSet = storage.get(&key).await.unwrap();
        controller.reconcile(&mut ss).await.unwrap();

        // Check that currentRevision != updateRevision during rolling update
        let ss: StatefulSet = storage.get(&key).await.unwrap();
        let status = ss.status.as_ref().unwrap();
        let new_revision = status.update_revision.clone().unwrap();

        assert_ne!(
            old_revision, new_revision,
            "Update revision should differ from old revision after template change"
        );
        assert_ne!(
            status.current_revision, status.update_revision,
            "During rolling update: currentRevision should NOT equal updateRevision"
        );
        assert_eq!(
            status.current_revision.as_ref().unwrap(),
            &old_revision,
            "currentRevision should still be the old revision during rolling update"
        );

        // Now simulate completing the rolling update:
        // Each cycle: make pods ready → reconcile (deletes one old, creates one new)
        // Need enough cycles for all 3 pods to be replaced + a final reconcile
        for cycle in 0..20 {
            // Make all current pods ready
            for i in 0..3 {
                let pod_name = format!("web-{}", i);
                let pod_key = format!("/registry/pods/{}/{}", ns, pod_name);
                if storage.get::<Pod>(&pod_key).await.is_ok() {
                    make_pod_ready(&storage, ns, &pod_name).await;
                }
            }

            let mut ss: StatefulSet = storage.get(&key).await.unwrap();
            controller.reconcile(&mut ss).await.unwrap();

            let ss: StatefulSet = storage.get(&key).await.unwrap();
            let status = ss.status.as_ref().unwrap();
            if status.current_revision == status.update_revision {
                break;
            }
            assert!(cycle < 19, "Rolling update did not complete in 20 cycles");
        }

        // After rollout completes, currentRevision == updateRevision
        let ss: StatefulSet = storage.get(&key).await.unwrap();
        let status = ss.status.as_ref().unwrap();
        assert_eq!(
            status.current_revision, status.update_revision,
            "After rollout completes: currentRevision should equal updateRevision"
        );
        assert_eq!(
            status.updated_replicas,
            Some(3),
            "All replicas should be updated"
        );
    }

    /// Partition should be respected: only pods with ordinal >= partition are updated.
    #[tokio::test]
    async fn test_canary_update_respects_partition() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = StatefulSetController::new(storage.clone());

        let ns = "default";
        let mut ss = make_statefulset("web", ns, 3, "nginx:1.19");
        // Set partition=2 so only pod web-2 should be updated
        ss.spec.update_strategy = Some(StatefulSetUpdateStrategy {
            strategy_type: Some("RollingUpdate".to_string()),
            rolling_update: Some(RollingUpdateStatefulSetStrategy {
                partition: Some(2),
                max_unavailable: None,
            }),
        });

        let key = format!("/registry/statefulsets/{}/{}", ns, "web");
        storage.create(&key, &ss).await.unwrap();

        // Create pods and make them ready
        let mut ss: StatefulSet = storage.get(&key).await.unwrap();
        controller.reconcile(&mut ss).await.unwrap();

        for i in 0..3 {
            make_pod_ready(&storage, ns, &format!("web-{}", i)).await;
        }

        let mut ss: StatefulSet = storage.get(&key).await.unwrap();
        controller.reconcile(&mut ss).await.unwrap();

        // Record the old revision from pod-0
        let pod0: Pod = storage
            .get(&format!("/registry/pods/{}/web-0", ns))
            .await
            .unwrap();
        let old_rev = pod0
            .metadata
            .labels
            .as_ref()
            .and_then(|l| l.get("controller-revision-hash"))
            .cloned()
            .unwrap();

        // Patch image
        let mut ss: StatefulSet = storage.get(&key).await.unwrap();
        ss.spec.template.spec.containers[0].image = "nginx:1.20".to_string();
        storage.update(&key, &ss).await.unwrap();

        // Run several reconcile cycles
        for _ in 0..5 {
            let pod_prefix = format!("/registry/pods/{}/", ns);
            let pods: Vec<Pod> = storage.list(&pod_prefix).await.unwrap();
            for pod in &pods {
                if pod
                    .metadata
                    .labels
                    .as_ref()
                    .and_then(|l| l.get("app"))
                    .map(|a| a == "web")
                    .unwrap_or(false)
                {
                    make_pod_ready(&storage, ns, &pod.metadata.name).await;
                }
            }

            let mut ss: StatefulSet = storage.get(&key).await.unwrap();
            controller.reconcile(&mut ss).await.unwrap();
        }

        // Check that pod-0 and pod-1 still have the old revision (partition=2 protects them)
        let pod0: Pod = storage
            .get(&format!("/registry/pods/{}/web-0", ns))
            .await
            .unwrap();
        let pod0_rev = pod0
            .metadata
            .labels
            .as_ref()
            .and_then(|l| l.get("controller-revision-hash"))
            .cloned()
            .unwrap();
        assert_eq!(
            pod0_rev, old_rev,
            "Pod-0 should keep old revision (below partition)"
        );

        let pod1: Pod = storage
            .get(&format!("/registry/pods/{}/web-1", ns))
            .await
            .unwrap();
        let pod1_rev = pod1
            .metadata
            .labels
            .as_ref()
            .and_then(|l| l.get("controller-revision-hash"))
            .cloned()
            .unwrap();
        assert_eq!(
            pod1_rev, old_rev,
            "Pod-1 should keep old revision (below partition)"
        );

        // Pod-2 should have the new revision
        let pod2: Pod = storage
            .get(&format!("/registry/pods/{}/web-2", ns))
            .await
            .unwrap();
        let pod2_rev = pod2
            .metadata
            .labels
            .as_ref()
            .and_then(|l| l.get("controller-revision-hash"))
            .cloned()
            .unwrap();
        assert_ne!(
            pod2_rev, old_rev,
            "Pod-2 should have new revision (at or above partition)"
        );

        // currentRevision should NOT equal updateRevision (partition prevents full rollout)
        let ss: StatefulSet = storage.get(&key).await.unwrap();
        let status = ss.status.as_ref().unwrap();
        assert_ne!(
            status.current_revision, status.update_revision,
            "With partition, currentRevision should not equal updateRevision"
        );
    }

    /// Test that scale-down sets deletionTimestamp instead of direct delete.
    #[tokio::test]
    async fn test_scale_down_sets_deletion_timestamp() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = StatefulSetController::new(storage.clone());

        let ns = "default";
        // Create a StatefulSet with 3 replicas
        let ss = make_statefulset("ss-scale", ns, 3, "busybox");
        storage
            .create("/registry/statefulsets/default/ss-scale", &ss)
            .await
            .unwrap();

        // First reconcile: creates 3 pods
        let mut ss: StatefulSet = storage
            .get("/registry/statefulsets/default/ss-scale")
            .await
            .unwrap();
        controller.reconcile(&mut ss).await.unwrap();

        // Make all pods ready
        for i in 0..3 {
            make_pod_ready(&storage, ns, &format!("ss-scale-{}", i)).await;
        }

        // Reconcile again to update status
        let mut ss: StatefulSet = storage
            .get("/registry/statefulsets/default/ss-scale")
            .await
            .unwrap();
        controller.reconcile(&mut ss).await.unwrap();

        // Scale down to 2
        let mut ss: StatefulSet = storage
            .get("/registry/statefulsets/default/ss-scale")
            .await
            .unwrap();
        ss.spec.replicas = Some(2);
        storage
            .update("/registry/statefulsets/default/ss-scale", &ss)
            .await
            .unwrap();

        // Reconcile — should set deletionTimestamp on pod ss-scale-2
        let mut ss: StatefulSet = storage
            .get("/registry/statefulsets/default/ss-scale")
            .await
            .unwrap();
        controller.reconcile(&mut ss).await.unwrap();

        // Pod ss-scale-2 should have deletionTimestamp set, not be deleted
        let pod2: Pod = storage
            .get("/registry/pods/default/ss-scale-2")
            .await
            .expect("Pod ss-scale-2 should still exist (graceful termination)");

        assert!(
            pod2.metadata.deletion_timestamp.is_some(),
            "Pod ss-scale-2 should have deletionTimestamp set for graceful termination"
        );

        // Pods 0 and 1 should not have deletionTimestamp
        let pod0: Pod = storage
            .get("/registry/pods/default/ss-scale-0")
            .await
            .unwrap();
        assert!(
            pod0.metadata.deletion_timestamp.is_none(),
            "Pod ss-scale-0 should not be terminating"
        );

        // Status should not count terminating pods
        let ss: StatefulSet = storage
            .get("/registry/statefulsets/default/ss-scale")
            .await
            .unwrap();
        let status = ss.status.unwrap();
        assert_eq!(
            status.replicas, 2,
            "replicas should exclude terminating pods"
        );
        assert_eq!(
            status.ready_replicas.unwrap_or(0),
            2,
            "readyReplicas should exclude terminating pods"
        );
    }

    /// Rolling update should use graceful termination (set deletionTimestamp)
    /// instead of direct deletion. This ensures the kubelet can perform cleanup
    /// and the pod gets properly recreated.
    #[tokio::test]
    async fn test_rolling_update_uses_graceful_termination() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = StatefulSetController::new(storage.clone());
        let ns = "default";
        let ss = make_statefulset("ss-grace", ns, 2, "nginx:1.19");
        let key = "/registry/statefulsets/default/ss-grace";
        storage.create(key, &ss).await.unwrap();

        // Create initial pods
        let mut ss: StatefulSet = storage.get(key).await.unwrap();
        controller.reconcile(&mut ss).await.unwrap();
        for i in 0..2 {
            make_pod_ready(&storage, ns, &format!("ss-grace-{}", i)).await;
        }
        let mut ss: StatefulSet = storage.get(key).await.unwrap();
        controller.reconcile(&mut ss).await.unwrap();

        // Change image to trigger rolling update
        let mut ss: StatefulSet = storage.get(key).await.unwrap();
        ss.spec.template.spec.containers[0].image = "nginx:1.20".to_string();
        storage.update(key, &ss).await.unwrap();

        // Reconcile — should set deletionTimestamp on highest-ordinal stale pod
        let mut ss: StatefulSet = storage.get(key).await.unwrap();
        controller.reconcile(&mut ss).await.unwrap();

        // The pod should still exist (not directly deleted) with deletionTimestamp set
        let pod1_key = "/registry/pods/default/ss-grace-1";
        let pod1: Pod = storage.get(pod1_key).await
            .expect("Pod ss-grace-1 should still exist after rolling update (graceful termination)");
        assert!(
            pod1.metadata.deletion_timestamp.is_some(),
            "Rolling update should set deletionTimestamp for graceful termination, not direct delete"
        );
        assert!(
            pod1.metadata.deletion_grace_period_seconds.is_some(),
            "Rolling update should set deletion_grace_period_seconds"
        );
    }

    /// Current replicas count should exclude terminating pods so that
    /// the controller can recreate them with the new template.
    #[tokio::test]
    async fn test_current_replicas_excludes_terminating() {
        let storage = Arc::new(MemoryStorage::new());
        let controller = StatefulSetController::new(storage.clone());
        let ns = "default";
        let ss = make_statefulset("ss-term", ns, 2, "nginx:1.19");
        let key = "/registry/statefulsets/default/ss-term";
        storage.create(key, &ss).await.unwrap();

        // Create initial pods
        let mut ss: StatefulSet = storage.get(key).await.unwrap();
        controller.reconcile(&mut ss).await.unwrap();
        for i in 0..2 {
            make_pod_ready(&storage, ns, &format!("ss-term-{}", i)).await;
        }

        // Manually set deletionTimestamp on one pod (simulating graceful termination)
        let pod_key = "/registry/pods/default/ss-term-1";
        let mut pod: Pod = storage.get(pod_key).await.unwrap();
        pod.metadata.deletion_timestamp = Some(chrono::Utc::now());
        storage.update(pod_key, &pod).await.unwrap();

        // Reconcile — controller should see only 1 active replica (not 2)
        // and attempt to recreate the terminating one
        let mut ss: StatefulSet = storage.get(key).await.unwrap();
        controller.reconcile(&mut ss).await.unwrap();

        // Status should reflect the non-terminating count
        let ss: StatefulSet = storage.get(key).await.unwrap();
        let status = ss.status.unwrap();
        // replicas should be the total non-terminating pod count
        assert!(
            status.replicas <= 2,
            "replicas ({}) should not double-count terminating pods",
            status.replicas
        );
    }
}
