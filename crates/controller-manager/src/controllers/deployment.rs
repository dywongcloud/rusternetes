use chrono::Utc;
use rusternetes_common::{
    resources::{Deployment, DeploymentCondition, DeploymentStatus, ReplicaSet, ReplicaSetSpec},
    types::{ObjectMeta, TypeMeta},
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
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
                let key =
                    build_key("deployments", Some(namespace), &deployment.metadata.name);
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
                (
                    ru.max_surge.as_deref().unwrap_or("25%"),
                    ru.max_unavailable.as_deref().unwrap_or("25%"),
                )
            })
            .unwrap_or(("25%", "25%"));

        let (max_surge, max_unavailable) =
            compute_rolling_update_counts(desired_replicas, max_surge_str, max_unavailable_str);

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
                // Available = new pods that are running (assume all new pods are available for simplicity)
                let available_after_scaleup = new_active_replicas;
                let can_remove = (available_after_scaleup - min_available).max(0);
                let scale_down_by = can_remove.min(old_rs_total).max(1); // At least 1 to make progress

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

                // Scale down old ReplicaSets to 0
                for rs in owned_replicasets.iter() {
                    if rs.metadata.name != active_name && rs.spec.replicas > 0 {
                        info!(
                            "Scaling down old ReplicaSet {}/{} to 0",
                            namespace, rs.metadata.name
                        );
                        self.update_replicaset_replicas(rs, 0).await?;
                    }
                }
            }
        } else {
            // No active ReplicaSet, create one
            info!(
                "Creating new ReplicaSet for deployment {}/{}",
                namespace, deployment.metadata.name
            );

            if is_rolling_update && old_rs_total > 0 {
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
                // No old RSs or Recreate strategy: create at full desired count
                self.create_replicaset(deployment).await?;

                // Scale down all old ReplicaSets
                for rs in owned_replicasets.iter() {
                    if rs.spec.replicas > 0 {
                        info!(
                            "Scaling down old ReplicaSet {}/{} to 0",
                            namespace, rs.metadata.name
                        );
                        self.update_replicaset_replicas(rs, 0).await?;
                    }
                }
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

    /// Generate a pod-template-hash from the pod template spec.
    fn compute_pod_template_hash(deployment: &Deployment) -> String {
        let template_json = serde_json::to_string(&deployment.spec.template)
            .unwrap_or_default();
        let mut hasher = DefaultHasher::new();
        template_json.hash(&mut hasher);
        let hash = hasher.finish();
        format!("{:x}", hash as u32 & 0xFFFFFFFF)
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
        let rs_name = format!(
            "{}-{}",
            deployment.metadata.name,
            &pod_template_hash
        );

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

        // Set revision annotation on the ReplicaSet
        let revision = deployment
            .metadata
            .annotations
            .as_ref()
            .and_then(|a| a.get("deployment.kubernetes.io/revision"))
            .cloned()
            .unwrap_or_else(|| "1".to_string());
        metadata
            .annotations
            .get_or_insert_with(std::collections::HashMap::new)
            .insert(
                "deployment.kubernetes.io/revision".to_string(),
                revision,
            );

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

        // Aggregate status from all ReplicaSets
        let mut total_replicas = 0;
        let mut ready_replicas = 0;
        let mut available_replicas = 0;
        let mut updated_replicas = 0;

        for rs in &owned_replicasets {
            if let Some(status) = &rs.status {
                total_replicas += status.replicas;
                ready_replicas += status.ready_replicas;
                available_replicas += status.available_replicas;

                // Count replicas from ReplicaSets matching current template as "updated"
                if self.replicaset_matches_template(rs, deployment) {
                    updated_replicas += status.replicas;
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

        // Progressing condition
        conditions.push(DeploymentCondition {
            condition_type: "Progressing".to_string(),
            status: "True".to_string(),
            last_transition_time: Some(Utc::now()),
            last_update_time: Some(Utc::now()),
            reason: Some("NewReplicaSetAvailable".to_string()),
            message: Some("ReplicaSet has successfully progressed.".to_string()),
        });

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
}
