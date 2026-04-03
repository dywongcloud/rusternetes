use chrono::Utc;
use rusternetes_common::{
    resources::{
        Deployment, DeploymentCondition, DeploymentStatus, Pod, ReplicaSet, ReplicaSetSpec,
    },
    types::{ObjectMeta, TypeMeta},
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::{sync::Arc, time::Duration};
use tracing::{debug, error, info};

/// Parse a value that can be either an absolute integer or a percentage string (e.g. "25%" or "1").
/// For percentages, the result is ceil(pct/100 * total). Defaults to 1 if unparseable.
fn parse_int_or_percent(s: &str, total: i32) -> i32 {
    if s.ends_with('%') {
        let pct: f64 = s.trim_end_matches('%').parse().unwrap_or(25.0);
        ((pct / 100.0) * total as f64).ceil() as i32
    } else {
        s.parse().unwrap_or(1)
    }
}

/// Compute the max surge and max unavailable counts for a rolling update.
/// Returns (max_surge, max_unavailable).
fn compute_rolling_update_counts(
    desired: i32,
    max_surge: &str,
    max_unavailable: &str,
) -> (i32, i32) {
    let surge = parse_int_or_percent(max_surge, desired);
    let unavailable = parse_int_or_percent(max_unavailable, desired);
    (surge, unavailable)
}

/// DeploymentController reconciles Deployment resources by creating and managing ReplicaSets
/// This follows the Kubernetes pattern: Deployment -> ReplicaSet -> Pods
pub struct DeploymentController<S: Storage> {
    storage: Arc<S>,
    interval: Duration,
}

impl<S: Storage> DeploymentController<S> {
    pub fn new(storage: Arc<S>, interval_secs: u64) -> Self {
        Self {
            storage,
            interval: Duration::from_secs(interval_secs),
        }
    }

    pub async fn run(&self) -> rusternetes_common::Result<()> {
        info!(
            "Deployment controller started, syncing every {:?}",
            self.interval
        );

        let mut interval = tokio::time::interval(self.interval);

        loop {
            interval.tick().await;
            if let Err(e) = self.reconcile_all().await {
                error!("Error reconciling deployments: {}", e);
            }
        }
    }

    pub async fn reconcile_all(&self) -> rusternetes_common::Result<()> {
        debug!("Reconciling all deployments");

        // Get all deployments
        let prefix = build_prefix("deployments", None);
        let deployments: Vec<Deployment> = self.storage.list(&prefix).await?;

        for deployment in deployments {
            if let Err(e) = self.reconcile_deployment(&deployment).await {
                error!(
                    "Error reconciling deployment {}: {}",
                    deployment.metadata.name, e
                );
            }
        }

        Ok(())
    }

    async fn reconcile_deployment(
        &self,
        deployment: &Deployment,
    ) -> rusternetes_common::Result<()> {
        let namespace = deployment
            .metadata
            .namespace
            .as_deref()
            .unwrap_or("default");

        debug!(
            "Reconciling deployment: {}/{}",
            namespace, deployment.metadata.name
        );

        // Ensure the deployment has a revision annotation
        {
            let annotations = deployment.metadata.annotations.clone().unwrap_or_default();
            if !annotations.contains_key("deployment.kubernetes.io/revision") {
                let mut updated = deployment.clone();
                updated
                    .metadata
                    .annotations
                    .get_or_insert_with(std::collections::HashMap::new)
                    .insert(
                        "deployment.kubernetes.io/revision".to_string(),
                        "1".to_string(),
                    );
                let key = build_key("deployments", Some(namespace), &deployment.metadata.name);
                let _ = self.storage.update(&key, &updated).await;
            }
        }

        // Get all ReplicaSets owned by this deployment
        let rs_prefix = build_prefix("replicasets", Some(namespace));
        let all_replicasets: Vec<ReplicaSet> = self.storage.list(&rs_prefix).await?;

        let mut owned_replicasets: Vec<ReplicaSet> = all_replicasets
            .into_iter()
            .filter(|rs| self.is_owned_by_deployment(rs, deployment))
            .collect();

        info!(
            "Found {} ReplicaSets owned by deployment {}/{}",
            owned_replicasets.len(),
            namespace,
            deployment.metadata.name
        );

        // Find the active ReplicaSet (matches current pod template)
        let active_rs = owned_replicasets
            .iter()
            .find(|rs| self.replicaset_matches_template(rs, deployment));

        let desired_replicas = deployment.spec.replicas.unwrap_or(1);

        // Determine if this is a RollingUpdate strategy
        let is_rolling_update = deployment
            .spec
            .strategy
            .as_ref()
            .map(|s| s.strategy_type == "RollingUpdate")
            .unwrap_or(true); // Default strategy is RollingUpdate

        // Get rolling update parameters
        let (max_surge_str, max_unavailable_str) = deployment
            .spec
            .strategy
            .as_ref()
            .and_then(|s| s.rolling_update.as_ref())
            .map(|ru| {
                let surge = ru
                    .max_surge
                    .as_ref()
                    .and_then(|v| {
                        v.as_str()
                            .map(|s| s.to_string())
                            .or_else(|| v.as_i64().map(|n| n.to_string()))
                    })
                    .unwrap_or_else(|| "25%".to_string());
                let unavail = ru
                    .max_unavailable
                    .as_ref()
                    .and_then(|v| {
                        v.as_str()
                            .map(|s| s.to_string())
                            .or_else(|| v.as_i64().map(|n| n.to_string()))
                    })
                    .unwrap_or_else(|| "25%".to_string());
                (surge, unavail)
            })
            .unwrap_or(("25%".to_string(), "25%".to_string()));

        let (max_surge, max_unavailable) =
            compute_rolling_update_counts(desired_replicas, &max_surge_str, &max_unavailable_str);

        // Calculate total old RS replicas
        let old_rs_total: i32 = owned_replicasets
            .iter()
            .filter(|rs| {
                if let Some(active) = owned_replicasets
                    .iter()
                    .find(|rs| self.replicaset_matches_template(rs, deployment))
                {
                    rs.metadata.name != active.metadata.name
                } else {
                    true
                }
            })
            .map(|rs| rs.spec.replicas)
            .sum();

        if let Some(active) = active_rs {
            let active_name = active.metadata.name.clone();
            let active_replicas = active.spec.replicas;

            if is_rolling_update && active_replicas < desired_replicas && old_rs_total > 0 {
                // Rolling update in progress: gradually scale new RS up and old RS down
                let max_total = desired_replicas + max_surge;
                let min_available = (desired_replicas - max_unavailable).max(0);

                // How many new pods can we add while respecting maxSurge?
                let current_total = active_replicas + old_rs_total;
                let can_add = (max_total - current_total).max(0);
                let want_to_add = desired_replicas - active_replicas;
                let scale_up_by = can_add.min(want_to_add).max(1); // At least 1 to make progress

                let new_active_replicas = (active_replicas + scale_up_by).min(desired_replicas);

                info!(
                    "Rolling update: scaling new ReplicaSet {}/{} from {} to {} (max_total={}, current_total={})",
                    namespace, active_name, active_replicas, new_active_replicas, max_total, current_total
                );
                self.update_replicaset_replicas(
                    &owned_replicasets
                        .iter()
                        .find(|rs| rs.metadata.name == active_name)
                        .unwrap()
                        .clone(),
                    new_active_replicas,
                )
                .await?;

                // How many old pods can we remove while respecting maxUnavailable?
                // Count actual available pods (Running+Ready) across ALL ReplicaSets,
                // not just the desired count. Pods with bad images won't be available,
                // so the old RS must retain replicas until new pods are actually ready.
                let dep_pod_prefix = build_prefix("pods", Some(namespace));
                let all_pods: Vec<Pod> = self.storage.list(&dep_pod_prefix).await?;
                let total_available: i32 = all_pods
                    .iter()
                    .filter(|p| {
                        // Must be owned by this deployment's ReplicaSets
                        let owned = p.metadata.owner_references.as_ref().map_or(false, |refs| {
                            refs.iter().any(|r| {
                                owned_replicasets.iter().any(|rs| r.uid == rs.metadata.uid)
                            })
                        });
                        if !owned {
                            return false;
                        }
                        // Must be Running and Ready, not terminating
                        if p.metadata.deletion_timestamp.is_some() {
                            return false;
                        }
                        let is_ready = p
                            .status
                            .as_ref()
                            .and_then(|s| s.conditions.as_ref())
                            .map(|conds| {
                                conds
                                    .iter()
                                    .any(|c| c.condition_type == "Ready" && c.status == "True")
                            })
                            .unwrap_or(false);
                        is_ready
                    })
                    .count() as i32;
                let can_remove = (total_available - min_available).max(0);
                let scale_down_by = if can_remove > 0 {
                    can_remove.min(old_rs_total)
                } else {
                    0 // Don't scale down old RS if not enough available pods
                };

                let mut remaining_to_remove = scale_down_by;
                for rs in owned_replicasets.iter() {
                    if rs.metadata.name != active_name
                        && rs.spec.replicas > 0
                        && remaining_to_remove > 0
                    {
                        let remove_from_this = rs.spec.replicas.min(remaining_to_remove);
                        let new_replicas = rs.spec.replicas - remove_from_this;
                        info!(
                            "Rolling update: scaling down old ReplicaSet {}/{} from {} to {}",
                            namespace, rs.metadata.name, rs.spec.replicas, new_replicas
                        );
                        self.update_replicaset_replicas(rs, new_replicas).await?;
                        remaining_to_remove -= remove_from_this;
                    }
                }
            } else {
                // No rolling update needed (or already at desired), just ensure correct count
                if active_replicas != desired_replicas {
                    info!(
                        "Updating ReplicaSet {}/{} replicas from {} to {}",
                        namespace, active_name, active_replicas, desired_replicas
                    );
                    self.update_replicaset_replicas(
                        &owned_replicasets
                            .iter()
                            .find(|rs| rs.metadata.name == active_name)
                            .unwrap()
                            .clone(),
                        desired_replicas,
                    )
                    .await?;
                }

                // Scale down old ReplicaSets gradually — only remove old pods
                // when the new RS has enough available replicas to maintain
                // the minimum availability guarantee.
                let new_rs_available = self
                    .count_available_pods_for_rs(&active_name, namespace)
                    .await;
                let min_available = (desired_replicas - max_unavailable).max(0);
                let can_scale_down = (new_rs_available - min_available).max(0);

                if can_scale_down > 0 {
                    let mut remaining = can_scale_down;
                    for rs in owned_replicasets.iter() {
                        if rs.metadata.name != active_name && rs.spec.replicas > 0 && remaining > 0
                        {
                            let remove = rs.spec.replicas.min(remaining);
                            let new_replicas = rs.spec.replicas - remove;
                            info!(
                                "Scaling down old ReplicaSet {}/{} from {} to {} (available={})",
                                namespace,
                                rs.metadata.name,
                                rs.spec.replicas,
                                new_replicas,
                                new_rs_available
                            );
                            self.update_replicaset_replicas(rs, new_replicas).await?;
                            remaining -= remove;
                        }
                    }
                }
            }
        } else {
            // No active ReplicaSet, create one
            info!(
                "Creating new ReplicaSet for deployment {}/{}",
                namespace, deployment.metadata.name
            );

            if !is_rolling_update && old_rs_total > 0 {
                // Recreate strategy: scale down ALL old RSs to 0 FIRST
                for rs in owned_replicasets.iter() {
                    if rs.spec.replicas > 0 {
                        info!(
                            "Recreate: scaling down old ReplicaSet {}/{} to 0",
                            namespace, rs.metadata.name
                        );
                        self.update_replicaset_replicas(rs, 0).await?;
                    }
                }
                // Then create new RS at full desired count
                self.create_replicaset(deployment).await?;
            } else if is_rolling_update && old_rs_total > 0 {
                // Start the rolling update: create new RS with a smaller initial count
                let max_total = desired_replicas + max_surge;
                let initial_replicas = (max_total - old_rs_total).max(1).min(desired_replicas);
                self.create_replicaset_with_replicas(deployment, initial_replicas)
                    .await?;

                // Scale down old RSs gradually
                let min_available = (desired_replicas - max_unavailable).max(0);
                let can_remove = (initial_replicas - min_available).max(0);
                let scale_down_by = can_remove.min(old_rs_total).max(1);

                let mut remaining_to_remove = scale_down_by;
                for rs in owned_replicasets.iter() {
                    if rs.spec.replicas > 0 && remaining_to_remove > 0 {
                        let remove_from_this = rs.spec.replicas.min(remaining_to_remove);
                        let new_replicas = rs.spec.replicas - remove_from_this;
                        info!(
                            "Rolling update: scaling down old ReplicaSet {}/{} from {} to {}",
                            namespace, rs.metadata.name, rs.spec.replicas, new_replicas
                        );
                        self.update_replicaset_replicas(rs, new_replicas).await?;
                        remaining_to_remove -= remove_from_this;
                    }
                }
            } else {
                // No old RSs: create at full desired count
                self.create_replicaset(deployment).await?;
            }
        }

        // Update deployment status
        self.update_deployment_status(deployment).await?;

        Ok(())
    }

    fn is_owned_by_deployment(&self, rs: &ReplicaSet, deployment: &Deployment) -> bool {
        if let Some(owner_refs) = &rs.metadata.owner_references {
            owner_refs.iter().any(|owner| {
                owner.kind == "Deployment"
                    && owner.name == deployment.metadata.name
                    && owner.uid == deployment.metadata.uid
            })
        } else {
            false
        }
    }

    fn replicaset_matches_template(&self, rs: &ReplicaSet, deployment: &Deployment) -> bool {
        // Simple comparison: check if containers match
        // In a full implementation, we'd hash the entire pod template
        if rs.spec.template.spec.containers.len() != deployment.spec.template.spec.containers.len()
        {
            return false;
        }

        for (rs_container, deploy_container) in rs
            .spec
            .template
            .spec
            .containers
            .iter()
            .zip(deployment.spec.template.spec.containers.iter())
        {
            if rs_container.image != deploy_container.image
                || rs_container.name != deploy_container.name
            {
                return false;
            }
        }

        true
    }

    async fn create_replicaset(&self, deployment: &Deployment) -> rusternetes_common::Result<()> {
        self.create_replicaset_with_replicas(deployment, deployment.spec.replicas.unwrap_or(1))
            .await
    }

    /// Generate a deterministic pod-template-hash from the pod template spec.
    /// Uses SHA-256 via serde_json::Value normalization (sorts HashMap keys).
    fn compute_pod_template_hash(deployment: &Deployment) -> String {
        use sha2::{Digest, Sha256};
        // Convert to Value first to normalize HashMap key ordering
        let value = serde_json::to_value(&deployment.spec.template).unwrap_or_default();
        let template_json = serde_json::to_string(&value).unwrap_or_default();
        let hash = Sha256::digest(template_json.as_bytes());
        format!(
            "{:08x}",
            u32::from_be_bytes(hash[..4].try_into().unwrap_or([0u8; 4]))
        )
    }

    async fn create_replicaset_with_replicas(
        &self,
        deployment: &Deployment,
        replicas: i32,
    ) -> rusternetes_common::Result<()> {
        let namespace = deployment
            .metadata
            .namespace
            .as_deref()
            .unwrap_or("default");

        // Generate pod-template-hash from pod template
        let pod_template_hash = Self::compute_pod_template_hash(deployment);

        // Generate ReplicaSet name using the pod-template-hash
        let rs_name = format!("{}-{}", deployment.metadata.name, &pod_template_hash);

        let mut metadata = ObjectMeta::new(&rs_name);
        metadata.namespace = Some(namespace.to_string());

        // Start with template labels, then add pod-template-hash
        let mut labels = deployment
            .spec
            .template
            .metadata
            .as_ref()
            .and_then(|m| m.labels.clone())
            .unwrap_or_default();
        labels.insert("pod-template-hash".to_string(), pod_template_hash.clone());
        metadata.labels = Some(labels);

        // Set revision annotation on the ReplicaSet.
        // Compute the next revision by finding the max among existing ReplicaSets + 1.
        let rs_prefix = build_prefix("replicasets", Some(namespace));
        let all_rs: Vec<ReplicaSet> = self.storage.list(&rs_prefix).await.unwrap_or_default();
        let max_existing_revision = all_rs
            .iter()
            .filter(|rs| {
                rs.metadata
                    .owner_references
                    .as_ref()
                    .map(|refs| refs.iter().any(|r| r.name == deployment.metadata.name))
                    .unwrap_or(false)
            })
            .filter_map(|rs| {
                rs.metadata
                    .annotations
                    .as_ref()
                    .and_then(|a| a.get("deployment.kubernetes.io/revision"))
                    .and_then(|v| v.parse::<i64>().ok())
            })
            .max()
            .unwrap_or(0);
        let new_revision = (max_existing_revision + 1).to_string();

        metadata
            .annotations
            .get_or_insert_with(std::collections::HashMap::new)
            .insert(
                "deployment.kubernetes.io/revision".to_string(),
                new_revision.clone(),
            );

        // Also update the deployment's revision annotation (with CAS retry)
        {
            let dep_key = build_key("deployments", Some(namespace), &deployment.metadata.name);
            let new_rev = new_revision.clone();
            for _ in 0..3 {
                match self.storage.get::<Deployment>(&dep_key).await {
                    Ok(mut dep) => {
                        dep.metadata
                            .annotations
                            .get_or_insert_with(std::collections::HashMap::new)
                            .insert(
                                "deployment.kubernetes.io/revision".to_string(),
                                new_rev.clone(),
                            );
                        match self.storage.update(&dep_key, &dep).await {
                            Ok(_) => break,
                            Err(e) => {
                                debug!("CAS retry updating deployment revision: {}", e);
                                continue;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        }

        // Set owner reference to the deployment
        metadata.owner_references = Some(vec![rusternetes_common::types::OwnerReference {
            api_version: "apps/v1".to_string(),
            kind: "Deployment".to_string(),
            name: deployment.metadata.name.clone(),
            uid: deployment.metadata.uid.clone(),
            controller: Some(true),
            block_owner_deletion: Some(true),
        }]);

        // Add pod-template-hash to the pod template labels
        let mut template = deployment.spec.template.clone();
        let template_labels = template
            .metadata
            .get_or_insert_with(|| ObjectMeta::new(""))
            .labels
            .get_or_insert_with(Default::default);
        template_labels.insert("pod-template-hash".to_string(), pod_template_hash.clone());

        // Add pod-template-hash to the selector matchLabels
        let mut selector = deployment.spec.selector.clone();
        let match_labels = selector.match_labels.get_or_insert_with(Default::default);
        match_labels.insert("pod-template-hash".to_string(), pod_template_hash.clone());

        let replicaset = ReplicaSet {
            type_meta: TypeMeta {
                kind: "ReplicaSet".to_string(),
                api_version: "apps/v1".to_string(),
            },
            metadata,
            spec: ReplicaSetSpec {
                replicas,
                selector,
                template,
                min_ready_seconds: deployment.spec.min_ready_seconds,
            },
            status: None,
        };

        let key = build_key("replicasets", Some(namespace), &rs_name);
        self.storage.create(&key, &replicaset).await?;

        info!(
            "Created ReplicaSet {}/{} with {} replicas for deployment {}",
            namespace, rs_name, replicas, deployment.metadata.name
        );

        Ok(())
    }

    async fn update_replicaset_replicas(
        &self,
        rs: &ReplicaSet,
        replicas: i32,
    ) -> rusternetes_common::Result<()> {
        let namespace = rs.metadata.namespace.as_deref().unwrap_or("default");

        let mut updated_rs = rs.clone();
        updated_rs.spec.replicas = replicas;

        let key = build_key("replicasets", Some(namespace), &rs.metadata.name);
        self.storage.update(&key, &updated_rs).await?;

        info!(
            "Updated ReplicaSet {}/{} replicas to {}",
            namespace, rs.metadata.name, replicas
        );

        Ok(())
    }

    /// Count pods that are Ready for a given ReplicaSet
    async fn count_available_pods_for_rs(&self, rs_name: &str, namespace: &str) -> i32 {
        let pod_prefix = build_prefix("pods", Some(namespace));
        let pods: Vec<Pod> = self.storage.list(&pod_prefix).await.unwrap_or_default();
        pods.iter()
            .filter(|pod| {
                // Pod must be owned by this RS
                let owned = pod
                    .metadata
                    .owner_references
                    .as_ref()
                    .map(|refs| refs.iter().any(|r| r.name == rs_name))
                    .unwrap_or(false);
                if !owned {
                    return false;
                }
                // Pod must be Ready
                pod.status
                    .as_ref()
                    .and_then(|s| s.conditions.as_ref())
                    .map(|c| {
                        c.iter()
                            .any(|cond| cond.condition_type == "Ready" && cond.status == "True")
                    })
                    .unwrap_or(false)
            })
            .count() as i32
    }

    async fn update_deployment_status(
        &self,
        deployment: &Deployment,
    ) -> rusternetes_common::Result<()> {
        let namespace = deployment
            .metadata
            .namespace
            .as_deref()
            .unwrap_or("default");

        // Get all ReplicaSets owned by this deployment
        let rs_prefix = build_prefix("replicasets", Some(namespace));
        let all_replicasets: Vec<ReplicaSet> = self.storage.list(&rs_prefix).await?;

        let owned_replicasets: Vec<ReplicaSet> = all_replicasets
            .into_iter()
            .filter(|rs| self.is_owned_by_deployment(rs, deployment))
            .collect();

        // Aggregate status from all ReplicaSets.
        // If a ReplicaSet has no status yet (controller hasn't run), fall back to
        // counting its pods directly so the deployment status is never stuck at 0.
        let mut total_replicas = 0;
        let mut ready_replicas = 0;
        let mut available_replicas = 0;
        let mut updated_replicas = 0;

        for rs in &owned_replicasets {
            // Always count pods directly for the most accurate status.
            // RS status may be stale if the RS controller hasn't run recently.
            let (pod_total, pod_ready, pod_available) = self.count_pods_for_replicaset(rs).await;

            // Use the higher of RS status or direct pod count
            if let Some(status) = &rs.status {
                total_replicas += std::cmp::max(status.replicas, pod_total);
                ready_replicas += std::cmp::max(status.ready_replicas, pod_ready);
                available_replicas += std::cmp::max(status.available_replicas, pod_available);
            } else {
                total_replicas += pod_total;
                ready_replicas += pod_ready;
                available_replicas += pod_available;
            }

            if self.replicaset_matches_template(rs, deployment) {
                if let Some(status) = &rs.status {
                    updated_replicas += std::cmp::max(status.replicas, pod_total);
                } else {
                    updated_replicas += pod_total;
                }
            }
        }

        let desired_replicas = deployment.spec.replicas.unwrap_or(1);

        let unavailable = if total_replicas > available_replicas {
            total_replicas - available_replicas
        } else {
            0
        };

        // Build status conditions
        let mut conditions = Vec::new();

        // Available condition
        if available_replicas >= desired_replicas {
            conditions.push(DeploymentCondition {
                condition_type: "Available".to_string(),
                status: "True".to_string(),
                last_transition_time: Some(Utc::now()),
                last_update_time: Some(Utc::now()),
                reason: Some("MinimumReplicasAvailable".to_string()),
                message: Some("Deployment has minimum availability.".to_string()),
            });
        } else {
            conditions.push(DeploymentCondition {
                condition_type: "Available".to_string(),
                status: "False".to_string(),
                last_transition_time: Some(Utc::now()),
                last_update_time: Some(Utc::now()),
                reason: Some("MinimumReplicasUnavailable".to_string()),
                message: Some(format!(
                    "Deployment does not have minimum availability. {} of {} available.",
                    available_replicas, desired_replicas
                )),
            });
        }

        // Progressing condition — check progressDeadlineSeconds
        let progress_deadline = deployment.spec.progress_deadline_seconds.unwrap_or(600);
        let deadline_exceeded = if available_replicas < desired_replicas {
            // Check if the deployment has been progressing long enough to exceed the deadline
            deployment
                .metadata
                .creation_timestamp
                .map(|ct| {
                    let elapsed = Utc::now().signed_duration_since(ct).num_seconds();
                    elapsed > progress_deadline as i64
                })
                .unwrap_or(false)
        } else {
            false
        };

        if deadline_exceeded {
            conditions.push(DeploymentCondition {
                condition_type: "Progressing".to_string(),
                status: "False".to_string(),
                last_transition_time: Some(Utc::now()),
                last_update_time: Some(Utc::now()),
                reason: Some("ProgressDeadlineExceeded".to_string()),
                message: Some(format!(
                    "ReplicaSet \"{}\" has timed out progressing.",
                    owned_replicasets
                        .first()
                        .map(|rs| rs.metadata.name.as_str())
                        .unwrap_or("unknown")
                )),
            });
        } else if updated_replicas == desired_replicas && available_replicas >= desired_replicas {
            conditions.push(DeploymentCondition {
                condition_type: "Progressing".to_string(),
                status: "True".to_string(),
                last_transition_time: Some(Utc::now()),
                last_update_time: Some(Utc::now()),
                reason: Some("NewReplicaSetAvailable".to_string()),
                message: Some(format!(
                    "ReplicaSet \"{}\" has successfully progressed.",
                    owned_replicasets
                        .first()
                        .map(|rs| rs.metadata.name.as_str())
                        .unwrap_or("unknown")
                )),
            });
        } else {
            conditions.push(DeploymentCondition {
                condition_type: "Progressing".to_string(),
                status: "True".to_string(),
                last_transition_time: Some(Utc::now()),
                last_update_time: Some(Utc::now()),
                reason: Some("ReplicaSetUpdated".to_string()),
                message: Some(format!(
                    "ReplicaSet \"{}\" is progressing.",
                    owned_replicasets
                        .first()
                        .map(|rs| rs.metadata.name.as_str())
                        .unwrap_or("unknown")
                )),
            });
        }

        let status = DeploymentStatus {
            replicas: Some(total_replicas),
            ready_replicas: Some(ready_replicas),
            available_replicas: Some(available_replicas),
            unavailable_replicas: Some(unavailable),
            updated_replicas: Some(updated_replicas),
            conditions: Some(conditions),
            collision_count: None,
            observed_generation: deployment.metadata.generation,
            terminating_replicas: None,
        };

        let mut updated_deployment = deployment.clone();
        updated_deployment.status = Some(status);

        // Ensure the deployment's revision annotation matches the latest ReplicaSet's revision
        let max_revision = owned_replicasets
            .iter()
            .filter_map(|rs| {
                rs.metadata
                    .annotations
                    .as_ref()
                    .and_then(|a| a.get("deployment.kubernetes.io/revision"))
                    .and_then(|v| v.parse::<i64>().ok())
            })
            .max();
        if let Some(rev) = max_revision {
            updated_deployment
                .metadata
                .annotations
                .get_or_insert_with(std::collections::HashMap::new)
                .insert(
                    "deployment.kubernetes.io/revision".to_string(),
                    rev.to_string(),
                );
        }

        let key = build_key("deployments", Some(namespace), &deployment.metadata.name);
        self.storage.update(&key, &updated_deployment).await?;

        debug!(
            "Updated status for deployment {}/{}: total={}, ready={}, available={}, updated={}",
            namespace,
            deployment.metadata.name,
            total_replicas,
            ready_replicas,
            available_replicas,
            updated_replicas
        );

        Ok(())
    }

    /// Count pods owned by a ReplicaSet directly from storage.
    /// Returns (total, ready, available).
    /// Used as a fallback when the RS has no status yet.
    async fn count_pods_for_replicaset(&self, rs: &ReplicaSet) -> (i32, i32, i32) {
        let namespace = rs.metadata.namespace.as_deref().unwrap_or("default");
        let pods_prefix = build_prefix("pods", Some(namespace));
        let all_pods: Vec<Pod> = self.storage.list(&pods_prefix).await.unwrap_or_default();

        let mut total = 0i32;
        let mut ready = 0i32;
        let mut available = 0i32;

        for pod in &all_pods {
            // Skip terminated or deleting pods
            if pod.metadata.deletion_timestamp.is_some() {
                continue;
            }
            if let Some(ref status) = pod.status {
                if let Some(ref phase) = status.phase {
                    if matches!(
                        phase,
                        rusternetes_common::types::Phase::Failed
                            | rusternetes_common::types::Phase::Succeeded
                    ) {
                        continue;
                    }
                }
            }

            // Check if pod is owned by this RS
            let owned = pod
                .metadata
                .owner_references
                .as_ref()
                .map(|refs| {
                    refs.iter()
                        .any(|r| r.kind == "ReplicaSet" && r.name == rs.metadata.name)
                })
                .unwrap_or(false);

            if !owned {
                // Fallback: check label selector match
                let labels_match =
                    if let Some(match_labels) = rs.spec.selector.match_labels.as_ref() {
                        if let Some(pod_labels) = &pod.metadata.labels {
                            match_labels
                                .iter()
                                .all(|(k, v)| pod_labels.get(k) == Some(v))
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                if !labels_match {
                    continue;
                }
            }

            total += 1;

            // Check readiness
            let is_ready = pod
                .status
                .as_ref()
                .and_then(|s| s.conditions.as_ref())
                .map(|conds| {
                    conds
                        .iter()
                        .any(|c| c.condition_type == "Ready" && c.status == "True")
                })
                .unwrap_or(false);

            if is_ready {
                ready += 1;
                // Check availability (minReadySeconds)
                let min_ready = rs.spec.min_ready_seconds.unwrap_or(0);
                if min_ready > 0 {
                    if let Some(creation) = pod.metadata.creation_timestamp {
                        let elapsed = chrono::Utc::now().signed_duration_since(creation);
                        if elapsed.num_seconds() >= min_ready as i64 {
                            available += 1;
                        }
                    }
                } else {
                    available += 1;
                }
            }
        }

        (total, ready, available)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_int_or_percent_percentage() {
        assert_eq!(parse_int_or_percent("25%", 10), 3); // ceil(2.5) = 3
        assert_eq!(parse_int_or_percent("50%", 10), 5);
        assert_eq!(parse_int_or_percent("100%", 10), 10);
        assert_eq!(parse_int_or_percent("25%", 4), 1); // ceil(1.0) = 1
        assert_eq!(parse_int_or_percent("25%", 1), 1); // ceil(0.25) = 1
    }

    #[test]
    fn test_parse_int_or_percent_absolute() {
        assert_eq!(parse_int_or_percent("1", 10), 1);
        assert_eq!(parse_int_or_percent("3", 10), 3);
        assert_eq!(parse_int_or_percent("0", 10), 0);
    }

    #[test]
    fn test_parse_int_or_percent_invalid() {
        // Invalid strings default to 1
        assert_eq!(parse_int_or_percent("abc", 10), 1);
        assert_eq!(parse_int_or_percent("", 10), 1);
    }

    #[test]
    fn test_parse_int_or_percent_invalid_percentage() {
        // Invalid percentage defaults to 25%
        assert_eq!(parse_int_or_percent("abc%", 10), 3); // ceil(25% of 10) = 3
    }

    #[test]
    fn test_compute_rolling_update_counts_defaults() {
        let (surge, unavailable) = compute_rolling_update_counts(10, "25%", "25%");
        assert_eq!(surge, 3); // ceil(2.5) = 3
        assert_eq!(unavailable, 3);
    }

    #[test]
    fn test_compute_rolling_update_counts_absolute() {
        let (surge, unavailable) = compute_rolling_update_counts(10, "2", "1");
        assert_eq!(surge, 2);
        assert_eq!(unavailable, 1);
    }

    #[test]
    fn test_compute_rolling_update_counts_mixed() {
        let (surge, unavailable) = compute_rolling_update_counts(10, "30%", "1");
        assert_eq!(surge, 3); // ceil(3.0) = 3
        assert_eq!(unavailable, 1);
    }

    #[test]
    fn test_compute_rolling_update_counts_small_deployment() {
        // For a deployment with 1 replica, 25% rounds up to 1
        let (surge, unavailable) = compute_rolling_update_counts(1, "25%", "25%");
        assert_eq!(surge, 1);
        assert_eq!(unavailable, 1);
    }

    /// Helper to create a minimal Container for tests
    fn test_container(name: &str, image: &str) -> rusternetes_common::resources::Container {
        rusternetes_common::resources::Container {
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

    #[tokio::test]
    async fn test_deployment_status_aggregates_from_replicaset_status() {
        use rusternetes_common::resources::{PodTemplateSpec, ReplicaSetStatus};
        use rusternetes_common::types::LabelSelector;
        use rusternetes_storage::memory::MemoryStorage;
        use std::collections::HashMap;

        let storage = Arc::new(MemoryStorage::new());
        let controller = DeploymentController::new(storage.clone(), 5);

        let mut labels = HashMap::new();
        labels.insert("app".to_string(), "test".to_string());

        // Create a deployment
        let deployment = Deployment {
            type_meta: TypeMeta {
                kind: "Deployment".to_string(),
                api_version: "apps/v1".to_string(),
            },
            metadata: ObjectMeta::new("test-deploy").with_namespace("default"),
            spec: rusternetes_common::resources::DeploymentSpec {
                replicas: Some(3),
                selector: LabelSelector {
                    match_labels: Some(labels.clone()),
                    match_expressions: None,
                },
                template: PodTemplateSpec {
                    metadata: Some(ObjectMeta::new("").with_labels(labels.clone())),
                    spec: rusternetes_common::resources::PodSpec {
                        containers: vec![test_container("test", "nginx:latest")],
                        ..Default::default()
                    },
                },
                strategy: None,
                min_ready_seconds: None,
                revision_history_limit: None,
                paused: None,
                progress_deadline_seconds: None,
            },
            status: None,
        };

        let dep_key = build_key("deployments", Some("default"), "test-deploy");
        storage.create(&dep_key, &deployment).await.unwrap();

        // Create an owned ReplicaSet with populated status
        let mut rs_labels = labels.clone();
        rs_labels.insert("pod-template-hash".to_string(), "abc123".to_string());

        let rs = ReplicaSet {
            type_meta: TypeMeta {
                kind: "ReplicaSet".to_string(),
                api_version: "apps/v1".to_string(),
            },
            metadata: ObjectMeta {
                name: "test-deploy-abc123".to_string(),
                namespace: Some("default".to_string()),
                labels: Some(rs_labels.clone()),
                owner_references: Some(vec![rusternetes_common::types::OwnerReference {
                    api_version: "apps/v1".to_string(),
                    kind: "Deployment".to_string(),
                    name: "test-deploy".to_string(),
                    uid: deployment.metadata.uid.clone(),
                    controller: Some(true),
                    block_owner_deletion: Some(true),
                }]),
                ..Default::default()
            },
            spec: ReplicaSetSpec {
                replicas: 3,
                selector: LabelSelector {
                    match_labels: Some(rs_labels.clone()),
                    match_expressions: None,
                },
                template: deployment.spec.template.clone(),
                min_ready_seconds: None,
            },
            status: Some(ReplicaSetStatus {
                replicas: 3,
                ready_replicas: 3,
                available_replicas: 3,
                fully_labeled_replicas: Some(3),
                observed_generation: None,
                conditions: None,
                terminating_replicas: None,
            }),
        };

        let rs_key = build_key("replicasets", Some("default"), "test-deploy-abc123");
        storage.create(&rs_key, &rs).await.unwrap();

        // Run status update
        controller
            .update_deployment_status(&deployment)
            .await
            .unwrap();

        // Read back the deployment and verify status
        let updated: Deployment = storage.get(&dep_key).await.unwrap();
        let status = updated.status.unwrap();
        assert_eq!(status.replicas, Some(3));
        assert_eq!(status.ready_replicas, Some(3));
        assert_eq!(status.available_replicas, Some(3));
        assert_eq!(status.updated_replicas, Some(3));

        // Verify Available condition is True
        let conditions = status.conditions.unwrap();
        let available_cond = conditions
            .iter()
            .find(|c| c.condition_type == "Available")
            .unwrap();
        assert_eq!(available_cond.status, "True");
    }

    #[tokio::test]
    async fn test_deployment_status_fallback_to_pod_count_when_rs_has_no_status() {
        use rusternetes_common::resources::{PodCondition, PodStatus, PodTemplateSpec};
        use rusternetes_common::types::{LabelSelector, Phase};
        use rusternetes_storage::memory::MemoryStorage;
        use std::collections::HashMap;

        let storage = Arc::new(MemoryStorage::new());
        let controller = DeploymentController::new(storage.clone(), 5);

        let mut labels = HashMap::new();
        labels.insert("app".to_string(), "test".to_string());

        let deployment = Deployment {
            type_meta: TypeMeta {
                kind: "Deployment".to_string(),
                api_version: "apps/v1".to_string(),
            },
            metadata: ObjectMeta::new("test-deploy").with_namespace("default"),
            spec: rusternetes_common::resources::DeploymentSpec {
                replicas: Some(2),
                selector: LabelSelector {
                    match_labels: Some(labels.clone()),
                    match_expressions: None,
                },
                template: PodTemplateSpec {
                    metadata: Some(ObjectMeta::new("").with_labels(labels.clone())),
                    spec: rusternetes_common::resources::PodSpec {
                        containers: vec![test_container("test", "nginx:latest")],
                        ..Default::default()
                    },
                },
                strategy: None,
                min_ready_seconds: None,
                revision_history_limit: None,
                paused: None,
                progress_deadline_seconds: None,
            },
            status: None,
        };

        let dep_key = build_key("deployments", Some("default"), "test-deploy");
        storage.create(&dep_key, &deployment).await.unwrap();

        // Create an RS without status (simulates RS controller not having run yet)
        let mut rs_labels = labels.clone();
        rs_labels.insert("pod-template-hash".to_string(), "abc123".to_string());

        let rs = ReplicaSet {
            type_meta: TypeMeta {
                kind: "ReplicaSet".to_string(),
                api_version: "apps/v1".to_string(),
            },
            metadata: ObjectMeta {
                name: "test-deploy-abc123".to_string(),
                namespace: Some("default".to_string()),
                labels: Some(rs_labels.clone()),
                owner_references: Some(vec![rusternetes_common::types::OwnerReference {
                    api_version: "apps/v1".to_string(),
                    kind: "Deployment".to_string(),
                    name: "test-deploy".to_string(),
                    uid: deployment.metadata.uid.clone(),
                    controller: Some(true),
                    block_owner_deletion: Some(true),
                }]),
                ..Default::default()
            },
            spec: ReplicaSetSpec {
                replicas: 2,
                selector: LabelSelector {
                    match_labels: Some(rs_labels.clone()),
                    match_expressions: None,
                },
                template: deployment.spec.template.clone(),
                min_ready_seconds: None,
            },
            status: None, // No status yet!
        };

        let rs_key = build_key("replicasets", Some("default"), "test-deploy-abc123");
        storage.create(&rs_key, &rs).await.unwrap();

        // Create 2 ready pods owned by the RS
        for i in 0..2 {
            let pod_name = format!("test-deploy-abc123-pod-{}", i);
            let pod = Pod {
                type_meta: TypeMeta {
                    kind: "Pod".to_string(),
                    api_version: "v1".to_string(),
                },
                metadata: ObjectMeta {
                    name: pod_name.clone(),
                    namespace: Some("default".to_string()),
                    labels: Some(rs_labels.clone()),
                    owner_references: Some(vec![rusternetes_common::types::OwnerReference {
                        api_version: "apps/v1".to_string(),
                        kind: "ReplicaSet".to_string(),
                        name: "test-deploy-abc123".to_string(),
                        uid: rs.metadata.uid.clone(),
                        controller: Some(true),
                        block_owner_deletion: Some(true),
                    }]),
                    creation_timestamp: Some(chrono::Utc::now() - chrono::Duration::minutes(5)),
                    ..Default::default()
                },
                spec: None,
                status: Some(PodStatus {
                    phase: Some(Phase::Running),
                    conditions: Some(vec![PodCondition {
                        condition_type: "Ready".to_string(),
                        status: "True".to_string(),
                        last_transition_time: None,
                        reason: None,
                        message: None,
                        observed_generation: None,
                    }]),
                    ..Default::default()
                }),
            };
            let pod_key = build_key("pods", Some("default"), &pod_name);
            storage.create(&pod_key, &pod).await.unwrap();
        }

        // Run status update — should pick up pod counts via fallback
        controller
            .update_deployment_status(&deployment)
            .await
            .unwrap();

        let updated: Deployment = storage.get(&dep_key).await.unwrap();
        let status = updated.status.unwrap();

        // Should have counted the pods directly
        assert_eq!(status.replicas, Some(2));
        assert_eq!(status.ready_replicas, Some(2));
        assert_eq!(status.available_replicas, Some(2));
    }
}
