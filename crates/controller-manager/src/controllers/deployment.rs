use rusternetes_common::{
    resources::{Deployment, DeploymentStatus, ReplicaSet, ReplicaSetSpec},
    types::{ObjectMeta, TypeMeta},
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::{sync::Arc, time::Duration};
use tracing::{debug, error, info};

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

        if let Some(active) = active_rs {
            // Active ReplicaSet exists, ensure it has correct replica count
            if active.spec.replicas != desired_replicas {
                info!(
                    "Updating ReplicaSet {}/{} replicas from {} to {}",
                    namespace, active.metadata.name, active.spec.replicas, desired_replicas
                );
                self.update_replicaset_replicas(active, desired_replicas)
                    .await?;
            }

            // Scale down old ReplicaSets to 0
            for rs in owned_replicasets.iter() {
                if rs.metadata.name != active.metadata.name && rs.spec.replicas > 0 {
                    info!(
                        "Scaling down old ReplicaSet {}/{} to 0",
                        namespace, rs.metadata.name
                    );
                    self.update_replicaset_replicas(rs, 0).await?;
                }
            }
        } else {
            // No active ReplicaSet, create one
            info!(
                "Creating new ReplicaSet for deployment {}/{}",
                namespace, deployment.metadata.name
            );
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
        let namespace = deployment
            .metadata
            .namespace
            .as_deref()
            .unwrap_or("default");

        // Generate ReplicaSet name with hash suffix (simplified - just use UUID)
        let rs_name = format!(
            "{}-{}",
            deployment.metadata.name,
            uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
        );

        let mut metadata = ObjectMeta::new(&rs_name);
        metadata.namespace = Some(namespace.to_string());
        metadata.labels = deployment
            .spec
            .template
            .metadata
            .as_ref()
            .and_then(|m| m.labels.clone());

        // Set owner reference to the deployment
        metadata.owner_references = Some(vec![rusternetes_common::types::OwnerReference {
            api_version: "apps/v1".to_string(),
            kind: "Deployment".to_string(),
            name: deployment.metadata.name.clone(),
            uid: deployment.metadata.uid.clone(),
            controller: Some(true),
            block_owner_deletion: Some(true),
        }]);

        let replicaset = ReplicaSet {
            type_meta: TypeMeta {
                kind: "ReplicaSet".to_string(),
                api_version: "apps/v1".to_string(),
            },
            metadata,
            spec: ReplicaSetSpec {
                replicas: deployment.spec.replicas.unwrap_or(1),
                selector: deployment.spec.selector.clone(),
                template: deployment.spec.template.clone(),
                min_ready_seconds: deployment.spec.min_ready_seconds,
            },
            status: None,
        };

        let key = build_key("replicasets", Some(namespace), &rs_name);
        self.storage.create(&key, &replicaset).await?;

        info!(
            "Created ReplicaSet {}/{} for deployment {}",
            namespace, rs_name, deployment.metadata.name
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

        let unavailable = if total_replicas > available_replicas {
            total_replicas - available_replicas
        } else {
            0
        };

        let status = DeploymentStatus {
            replicas: Some(total_replicas),
            ready_replicas: Some(ready_replicas),
            available_replicas: Some(available_replicas),
            unavailable_replicas: Some(unavailable),
            updated_replicas: Some(updated_replicas),
            conditions: None,
            collision_count: None,
            observed_generation: None,
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
