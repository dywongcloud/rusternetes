use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::ResourceSlice,
    List,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

pub async fn create_resourceslice(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut slice): Json<ResourceSlice>,
) -> Result<(StatusCode, Json<ResourceSlice>)> {
    info!("Creating ResourceSlice: {}", slice.metadata.as_ref().map(|m| m.name.as_ref().map(|n| n.as_str()).unwrap_or("")).unwrap_or(""));

    // Check authorization (cluster-scoped)
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "resourceslices")
        .with_api_group("resource.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Ensure metadata exists and set defaults
    let metadata = slice.metadata.get_or_insert_with(Default::default);

    // Generate UID and timestamp if not present
    if metadata.uid.is_none() {
        metadata.uid = Some(uuid::Uuid::new_v4().to_string());
    }
    if metadata.creation_timestamp.is_none() {
        metadata.creation_timestamp = Some(chrono::Utc::now());
    }

    let name = metadata.name.as_ref()
        .ok_or_else(|| rusternetes_common::Error::InvalidResource("metadata.name is required".to_string()))?;

    // Check for dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: ResourceSlice validated successfully (not created)");
        return Ok((StatusCode::CREATED, Json(slice)));
    }

    let key = build_key("resourceslices", None, name);
    let created = state.storage.create(&key, &slice).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_resourceslice(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<ResourceSlice>> {
    info!("Getting ResourceSlice: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "get", "resourceslices")
        .with_api_group("resource.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("resourceslices", None, &name);
    let slice = state.storage.get(&key).await?;

    Ok(Json(slice))
}

pub async fn list_resourceslices(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<List<ResourceSlice>>> {
    info!("Listing all ResourceSlices");

    let attrs = RequestAttributes::new(auth_ctx.user, "list", "resourceslices")
        .with_api_group("resource.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("resourceslices", None);
    let mut slices = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut slices, &params)?;

    let list = List::new("ResourceSliceList", "resource.k8s.io/v1", slices);
    Ok(Json(list))
}

pub async fn update_resourceslice(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut slice): Json<ResourceSlice>,
) -> Result<Json<ResourceSlice>> {
    info!("Updating ResourceSlice: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "update", "resourceslices")
        .with_api_group("resource.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Ensure metadata and set name
    let metadata = slice.metadata.get_or_insert_with(Default::default);
    metadata.name = Some(name.clone());

    // Check for dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: ResourceSlice validated successfully (not updated)");
        return Ok(Json(slice));
    }

    let key = build_key("resourceslices", None, &name);
    let updated = state.storage.update(&key, &slice).await?;

    Ok(Json(updated))
}

pub async fn delete_resourceslice(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("Deleting ResourceSlice: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "resourceslices")
        .with_api_group("resource.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("resourceslices", None, &name);

    // Check for dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: ResourceSlice validated successfully (not deleted)");
        return Ok(StatusCode::OK);
    }

    // NOTE: DRA resources use dra::ObjectMeta which is incompatible with finalizers.
    // We perform a simple delete without finalizer support.
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

// Use the macro to create a PATCH handler
crate::patch_handler_cluster!(patch_resourceslice, ResourceSlice, "resourceslices", "resource.k8s.io");
