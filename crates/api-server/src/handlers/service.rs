use crate::state::ApiServerState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use rusternetes_common::{resources::Service, Result};
use rusternetes_storage::{build_key, build_prefix};
use std::sync::Arc;
use tracing::info;

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Path(namespace): Path<String>,
    Json(mut service): Json<Service>,
) -> Result<(StatusCode, Json<Service>)> {
    info!("Creating service: {}/{}", namespace, service.metadata.name);

    service.metadata.namespace = Some(namespace.clone());

    let key = build_key("services", Some(&namespace), &service.metadata.name);
    let created = state.storage.create(&key, &service).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<Service>> {
    info!("Getting service: {}/{}", namespace, name);

    let key = build_key("services", Some(&namespace), &name);
    let service = state.storage.get(&key).await?;

    Ok(Json(service))
}

pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Path((namespace, name)): Path<(String, String)>,
    Json(mut service): Json<Service>,
) -> Result<Json<Service>> {
    info!("Updating service: {}/{}", namespace, name);

    service.metadata.name = name.clone();
    service.metadata.namespace = Some(namespace.clone());

    let key = build_key("services", Some(&namespace), &name);
    let updated = state.storage.update(&key, &service).await?;

    Ok(Json(updated))
}

pub async fn delete_service(
    State(state): State<Arc<ApiServerState>>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<StatusCode> {
    info!("Deleting service: {}/{}", namespace, name);

    let key = build_key("services", Some(&namespace), &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Path(namespace): Path<String>,
) -> Result<Json<Vec<Service>>> {
    info!("Listing services in namespace: {}", namespace);

    let prefix = build_prefix("services", Some(&namespace));
    let services = state.storage.list(&prefix).await?;

    Ok(Json(services))
}
