use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::ResourceClaim,
    List,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

pub async fn create_resourceclaim(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut claim): Json<ResourceClaim>,
) -> Result<(StatusCode, Json<ResourceClaim>)> {
    info!("Creating ResourceClaim: {}/{}", namespace, claim.metadata.as_ref().map(|m| m.name.as_ref().map(|n| n.as_str()).unwrap_or("")).unwrap_or(""));

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "resourceclaims")
        .with_api_group("resource.k8s.io")
        .with_namespace(&namespace);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Ensure metadata exists and set defaults
    let metadata = claim.metadata.get_or_insert_with(Default::default);
    metadata.namespace = Some(namespace.clone());

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
        info!("Dry-run: ResourceClaim validated successfully (not created)");
        return Ok((StatusCode::CREATED, Json(claim)));
    }

    let key = build_key("resourceclaims", Some(&namespace), name);
    let created = state.storage.create(&key, &claim).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_resourceclaim(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<ResourceClaim>> {
    info!("Getting ResourceClaim: {}/{}", namespace, name);

    let attrs = RequestAttributes::new(auth_ctx.user, "get", "resourceclaims")
        .with_api_group("resource.k8s.io")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("resourceclaims", Some(&namespace), &name);
    let claim = state.storage.get(&key).await?;

    Ok(Json(claim))
}

pub async fn list_resourceclaims(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<List<ResourceClaim>>> {
    info!("Listing ResourceClaims in namespace: {}", namespace);

    let attrs = RequestAttributes::new(auth_ctx.user, "list", "resourceclaims")
        .with_api_group("resource.k8s.io")
        .with_namespace(&namespace);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("resourceclaims", Some(&namespace));
    let mut claims = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut claims, &params)?;

    let list = List::new("ResourceClaimList", "resource.k8s.io/v1", claims);
    Ok(Json(list))
}

pub async fn list_all_resourceclaims(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<List<ResourceClaim>>> {
    info!("Listing all ResourceClaims");

    let attrs = RequestAttributes::new(auth_ctx.user, "list", "resourceclaims")
        .with_api_group("resource.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("resourceclaims", None);
    let mut claims = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut claims, &params)?;

    let list = List::new("ResourceClaimList", "resource.k8s.io/v1", claims);
    Ok(Json(list))
}

pub async fn update_resourceclaim(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut claim): Json<ResourceClaim>,
) -> Result<Json<ResourceClaim>> {
    info!("Updating ResourceClaim: {}/{}", namespace, name);

    let attrs = RequestAttributes::new(auth_ctx.user, "update", "resourceclaims")
        .with_api_group("resource.k8s.io")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Ensure metadata and set namespace/name
    let metadata = claim.metadata.get_or_insert_with(Default::default);
    metadata.namespace = Some(namespace.clone());
    metadata.name = Some(name.clone());

    // Check for dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: ResourceClaim validated successfully (not updated)");
        return Ok(Json(claim));
    }

    let key = build_key("resourceclaims", Some(&namespace), &name);
    let updated = state.storage.update(&key, &claim).await?;

    Ok(Json(updated))
}

pub async fn update_resourceclaim_status(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Json(mut claim): Json<ResourceClaim>,
) -> Result<Json<ResourceClaim>> {
    info!("Updating ResourceClaim status: {}/{}", namespace, name);

    let attrs = RequestAttributes::new(auth_ctx.user, "update", "resourceclaims/status")
        .with_api_group("resource.k8s.io")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Ensure metadata
    let metadata = claim.metadata.get_or_insert_with(Default::default);
    metadata.namespace = Some(namespace.clone());
    metadata.name = Some(name.clone());

    let key = build_key("resourceclaims", Some(&namespace), &name);

    // Get existing claim to preserve spec
    let mut existing: ResourceClaim = state.storage.get(&key).await?;

    // Only update status
    existing.status = claim.status;

    let updated = state.storage.update(&key, &existing).await?;

    Ok(Json(updated))
}

pub async fn delete_resourceclaim(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("Deleting ResourceClaim: {}/{}", namespace, name);

    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "resourceclaims")
        .with_api_group("resource.k8s.io")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("resourceclaims", Some(&namespace), &name);

    // Check for dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: ResourceClaim validated successfully (not deleted)");
        return Ok(StatusCode::OK);
    }

    // NOTE: DRA resources use dra::ObjectMeta which is incompatible with finalizers.
    // We perform a simple delete without finalizer support.
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

// Use the macro to create a PATCH handler (namespace-scoped)
crate::patch_handler_namespaced!(patch_resourceclaim, ResourceClaim, "resourceclaims", "resource.k8s.io");
