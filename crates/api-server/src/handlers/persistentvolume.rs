use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::PersistentVolume,
    List,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

pub async fn create_pv(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut pv): Json<PersistentVolume>,
) -> Result<(StatusCode, Json<PersistentVolume>)> {
    info!("Creating PersistentVolume: {}", pv.metadata.name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization (cluster-scoped)
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "persistentvolumes")
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    pv.metadata.ensure_uid();
    pv.metadata.ensure_creation_timestamp();

    let key = build_key("persistentvolumes", None, &pv.metadata.name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!("Dry-run: PersistentVolume {} validated successfully (not created)", pv.metadata.name);
        return Ok((StatusCode::CREATED, Json(pv)));
    }

    let created = state.storage.create(&key, &pv).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_pv(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<PersistentVolume>> {
    info!("Getting PersistentVolume: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "get", "persistentvolumes")
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("persistentvolumes", None, &name);
    let pv = state.storage.get(&key).await?;

    Ok(Json(pv))
}

pub async fn list_pvs(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<List<PersistentVolume>>> {
    info!("Listing all PersistentVolumes");

    let attrs = RequestAttributes::new(auth_ctx.user, "list", "persistentvolumes")
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("persistentvolumes", None);
    let mut pvs = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut pvs, &params)?;

    let list = List::new("PersistentVolumeList", "v1", pvs);
    Ok(Json(list))
}

pub async fn update_pv(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut pv): Json<PersistentVolume>,
) -> Result<Json<PersistentVolume>> {
    info!("Updating PersistentVolume: {}", name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    let attrs = RequestAttributes::new(auth_ctx.user, "update", "persistentvolumes")
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    pv.metadata.name = name.clone();

    let key = build_key("persistentvolumes", None, &name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!("Dry-run: PersistentVolume {} validated successfully (not updated)", name);
        return Ok(Json(pv));
    }

    let updated = state.storage.update(&key, &pv).await?;

    Ok(Json(updated))
}

pub async fn delete_pv(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("Deleting PersistentVolume: {}", name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "persistentvolumes")
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("persistentvolumes", None, &name);

    // Get the resource to check if it exists
    let pv: PersistentVolume = state.storage.get(&key).await?;

    // If dry-run, skip delete operation
    if is_dry_run {
        info!("Dry-run: PersistentVolume {} validated successfully (not deleted)", name);
        return Ok(StatusCode::OK);
    }

    crate::handlers::finalizers::handle_delete_with_finalizers(&*state.storage, &key, &pv).await?;

    Ok(StatusCode::NO_CONTENT)
}

// Use the macro to create a PATCH handler
crate::patch_handler_cluster!(patch_pv, PersistentVolume, "persistentvolumes", "");
