use crate::state::ApiServerState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use rusternetes_common::{resources::Pod, Result};
use rusternetes_storage::{build_key, build_prefix};
use std::sync::Arc;
use tracing::info;

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Path(namespace): Path<String>,
    Json(mut pod): Json<Pod>,
) -> Result<(StatusCode, Json<Pod>)> {
    info!("Creating pod: {}/{}", namespace, pod.metadata.name);

    // Ensure namespace is set correctly
    pod.metadata.namespace = Some(namespace.clone());

    let key = build_key("pods", Some(&namespace), &pod.metadata.name);
    let created = state.storage.create(&key, &pod).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<Pod>> {
    info!("Getting pod: {}/{}", namespace, name);

    let key = build_key("pods", Some(&namespace), &name);
    let pod = state.storage.get(&key).await?;

    Ok(Json(pod))
}

pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Path((namespace, name)): Path<(String, String)>,
    Json(mut pod): Json<Pod>,
) -> Result<Json<Pod>> {
    info!("Updating pod: {}/{}", namespace, name);

    // Ensure metadata matches URL
    pod.metadata.name = name.clone();
    pod.metadata.namespace = Some(namespace.clone());

    let key = build_key("pods", Some(&namespace), &name);
    let updated = state.storage.update(&key, &pod).await?;

    Ok(Json(updated))
}

pub async fn delete_pod(
    State(state): State<Arc<ApiServerState>>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<StatusCode> {
    info!("Deleting pod: {}/{}", namespace, name);

    let key = build_key("pods", Some(&namespace), &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Path(namespace): Path<String>,
) -> Result<Json<Vec<Pod>>> {
    info!("Listing pods in namespace: {}", namespace);

    let prefix = build_prefix("pods", Some(&namespace));
    let pods = state.storage.list(&prefix).await?;

    Ok(Json(pods))
}
