use crate::state::ApiServerState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use rusternetes_common::{resources::Deployment, Result};
use rusternetes_storage::{build_key, build_prefix};
use std::sync::Arc;
use tracing::info;

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Path(namespace): Path<String>,
    Json(mut deployment): Json<Deployment>,
) -> Result<(StatusCode, Json<Deployment>)> {
    info!(
        "Creating deployment: {}/{}",
        namespace, deployment.metadata.name
    );

    deployment.metadata.namespace = Some(namespace.clone());

    let key = build_key("deployments", Some(&namespace), &deployment.metadata.name);
    let created = state.storage.create(&key, &deployment).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<Deployment>> {
    info!("Getting deployment: {}/{}", namespace, name);

    let key = build_key("deployments", Some(&namespace), &name);
    let deployment = state.storage.get(&key).await?;

    Ok(Json(deployment))
}

pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Path((namespace, name)): Path<(String, String)>,
    Json(mut deployment): Json<Deployment>,
) -> Result<Json<Deployment>> {
    info!("Updating deployment: {}/{}", namespace, name);

    deployment.metadata.name = name.clone();
    deployment.metadata.namespace = Some(namespace.clone());

    let key = build_key("deployments", Some(&namespace), &name);
    let updated = state.storage.update(&key, &deployment).await?;

    Ok(Json(updated))
}

pub async fn delete_deployment(
    State(state): State<Arc<ApiServerState>>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<StatusCode> {
    info!("Deleting deployment: {}/{}", namespace, name);

    let key = build_key("deployments", Some(&namespace), &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Path(namespace): Path<String>,
) -> Result<Json<Vec<Deployment>>> {
    info!("Listing deployments in namespace: {}", namespace);

    let prefix = build_prefix("deployments", Some(&namespace));
    let deployments = state.storage.list(&prefix).await?;

    Ok(Json(deployments))
}
