use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::CronJob,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::info;

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Json(mut cronjob): Json<CronJob>,
) -> Result<(StatusCode, Json<CronJob>)> {
    info!(
        "Creating cronjob: {}/{}",
        namespace, cronjob.metadata.name
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "cronjobs")
        .with_namespace(&namespace)
        .with_api_group("batch");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    cronjob.metadata.namespace = Some(namespace.clone());

    let key = build_key("cronjobs", Some(&namespace), &cronjob.metadata.name);
    let created = state.storage.create(&key, &cronjob).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<CronJob>> {
    info!("Getting cronjob: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "cronjobs")
        .with_namespace(&namespace)
        .with_api_group("batch")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("cronjobs", Some(&namespace), &name);
    let cronjob = state.storage.get(&key).await?;

    Ok(Json(cronjob))
}

pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Json(mut cronjob): Json<CronJob>,
) -> Result<Json<CronJob>> {
    info!("Updating cronjob: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "cronjobs")
        .with_namespace(&namespace)
        .with_api_group("batch")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    cronjob.metadata.name = name.clone();
    cronjob.metadata.namespace = Some(namespace.clone());

    let key = build_key("cronjobs", Some(&namespace), &name);
    let updated = state.storage.update(&key, &cronjob).await?;

    Ok(Json(updated))
}

pub async fn delete_cronjob(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<StatusCode> {
    info!("Deleting cronjob: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "cronjobs")
        .with_namespace(&namespace)
        .with_api_group("batch")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("cronjobs", Some(&namespace), &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
) -> Result<Json<Vec<CronJob>>> {
    info!("Listing cronjobs in namespace: {}", namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "cronjobs")
        .with_namespace(&namespace)
        .with_api_group("batch");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("cronjobs", Some(&namespace));
    let cronjobs = state.storage.list(&prefix).await?;

    Ok(Json(cronjobs))
}

// Use the macro to create a PATCH handler
crate::patch_handler_namespaced!(patch, CronJob, "cronjobs", "batch");
