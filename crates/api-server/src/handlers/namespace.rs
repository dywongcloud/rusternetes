use crate::state::ApiServerState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use rusternetes_common::{resources::Namespace, Result};
use rusternetes_storage::{build_key, build_prefix};
use std::sync::Arc;
use tracing::info;

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Json(namespace): Json<Namespace>,
) -> Result<(StatusCode, Json<Namespace>)> {
    info!("Creating namespace: {}", namespace.metadata.name);

    let key = build_key("namespaces", None, &namespace.metadata.name);
    let created = state.storage.create(&key, &namespace).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Path(name): Path<String>,
) -> Result<Json<Namespace>> {
    info!("Getting namespace: {}", name);

    let key = build_key("namespaces", None, &name);
    let namespace = state.storage.get(&key).await?;

    Ok(Json(namespace))
}

pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Path(name): Path<String>,
    Json(mut namespace): Json<Namespace>,
) -> Result<Json<Namespace>> {
    info!("Updating namespace: {}", name);

    namespace.metadata.name = name.clone();

    let key = build_key("namespaces", None, &name);
    let updated = state.storage.update(&key, &namespace).await?;

    Ok(Json(updated))
}

pub async fn delete_ns(
    State(state): State<Arc<ApiServerState>>,
    Path(name): Path<String>,
) -> Result<StatusCode> {
    info!("Deleting namespace: {}", name);

    let key = build_key("namespaces", None, &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list(State(state): State<Arc<ApiServerState>>) -> Result<Json<Vec<Namespace>>> {
    info!("Listing namespaces");

    let prefix = build_prefix("namespaces", None);
    let namespaces = state.storage.list(&prefix).await?;

    Ok(Json(namespaces))
}
