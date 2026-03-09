use crate::state::ApiServerState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use rusternetes_common::{resources::Node, Result};
use rusternetes_storage::{build_key, build_prefix};
use std::sync::Arc;
use tracing::info;

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Json(node): Json<Node>,
) -> Result<(StatusCode, Json<Node>)> {
    info!("Creating node: {}", node.metadata.name);

    let key = build_key("nodes", None, &node.metadata.name);
    let created = state.storage.create(&key, &node).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Path(name): Path<String>,
) -> Result<Json<Node>> {
    info!("Getting node: {}", name);

    let key = build_key("nodes", None, &name);
    let node = state.storage.get(&key).await?;

    Ok(Json(node))
}

pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Path(name): Path<String>,
    Json(mut node): Json<Node>,
) -> Result<Json<Node>> {
    info!("Updating node: {}", name);

    node.metadata.name = name.clone();

    let key = build_key("nodes", None, &name);
    let updated = state.storage.update(&key, &node).await?;

    Ok(Json(updated))
}

pub async fn delete_node(
    State(state): State<Arc<ApiServerState>>,
    Path(name): Path<String>,
) -> Result<StatusCode> {
    info!("Deleting node: {}", name);

    let key = build_key("nodes", None, &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list(State(state): State<Arc<ApiServerState>>) -> Result<Json<Vec<Node>>> {
    info!("Listing nodes");

    let prefix = build_prefix("nodes", None);
    let nodes = state.storage.list(&prefix).await?;

    Ok(Json(nodes))
}
