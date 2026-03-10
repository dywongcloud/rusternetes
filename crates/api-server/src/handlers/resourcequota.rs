use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::ResourceQuota,
    List,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut quota): Json<ResourceQuota>,
) -> Result<(StatusCode, Json<ResourceQuota>)> {
    info!("Creating ResourceQuota: {} in namespace: {}", quota.metadata.name, namespace);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "resourcequotas")
        .with_api_group("")
        .with_namespace(&namespace);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    quota.metadata.namespace = Some(namespace.clone());

    // Enrich metadata with system fields
    quota.metadata.ensure_uid();
    quota.metadata.ensure_creation_timestamp();

    let key = build_key("resourcequotas", Some(&namespace), &quota.metadata.name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!("Dry-run: ResourceQuota {}/{} validated successfully (not created)", namespace, quota.metadata.name);
        return Ok((StatusCode::CREATED, Json(quota)));
    }

    let created = state.storage.create(&key, &quota).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<ResourceQuota>> {
    info!("Getting ResourceQuota: {} in namespace: {}", name, namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "resourcequotas")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("resourcequotas", Some(&namespace), &name);
    let quota = state.storage.get(&key).await?;

    Ok(Json(quota))
}

pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut quota): Json<ResourceQuota>,
) -> Result<Json<ResourceQuota>> {
    info!("Updating ResourceQuota: {} in namespace: {}", name, namespace);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "resourcequotas")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    quota.metadata.name = name.clone();
    quota.metadata.namespace = Some(namespace.clone());

    let key = build_key("resourcequotas", Some(&namespace), &name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!("Dry-run: ResourceQuota {}/{} validated successfully (not updated)", namespace, name);
        return Ok(Json(quota));
    }

    let result = match state.storage.update(&key, &quota).await {
        Ok(updated) => updated,
        Err(rusternetes_common::Error::NotFound(_)) => {
            state.storage.create(&key, &quota).await?
        }
        Err(e) => return Err(e),
    };

    Ok(Json(result))
}

pub async fn delete(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("Deleting ResourceQuota: {} in namespace: {}", name, namespace);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "resourcequotas")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("resourcequotas", Some(&namespace), &name);

    // If dry-run, skip delete operation
    if is_dry_run {
        info!("Dry-run: ResourceQuota {}/{} validated successfully (not deleted)", namespace, name);
        return Ok(StatusCode::OK);
    }

    // Get the resource quota for finalizer handling
    let quota: ResourceQuota = state.storage.get(&key).await?;

    // Handle deletion with finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &quota,
    )
    .await?;

    if deleted_immediately {
        Ok(StatusCode::NO_CONTENT)
    } else {
        info!(
            "ResourceQuota {}/{} marked for deletion (has finalizers: {:?})",
            namespace,
            name,
            quota.metadata.finalizers
        );
        Ok(StatusCode::OK)
    }
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<List<ResourceQuota>>> {
    info!("Listing ResourceQuotas in namespace: {}", namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "resourcequotas")
        .with_api_group("")
        .with_namespace(&namespace);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("resourcequotas", Some(&namespace));
    let mut quotas = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut quotas, &params)?;

    let list = List::new("ResourceQuotaList", "v1", quotas);
    Ok(Json(list))
}

pub async fn list_all(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<List<ResourceQuota>>> {
    info!("Listing all ResourceQuotas");

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "resourcequotas")
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("resourcequotas", None);
    let mut quotas = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut quotas, &params)?;

    let list = List::new("ResourceQuotaList", "v1", quotas);
    Ok(Json(list))
}

// Use the macro to create a PATCH handler
crate::patch_handler_namespaced!(patch, ResourceQuota, "resourcequotas", "");
