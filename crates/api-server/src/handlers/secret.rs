use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::Secret,
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
    Json(mut secret): Json<Secret>,
) -> Result<(StatusCode, Json<Secret>)> {
    info!(
        "Creating secret: {} in namespace: {}",
        secret.metadata.name, namespace
    );

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "secrets")
        .with_api_group("")
        .with_namespace(&namespace);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Enrich metadata with system fields
    secret.metadata.ensure_uid();
    secret.metadata.ensure_creation_timestamp();

    // Normalize: convert stringData to base64-encoded data
    secret.normalize();

    let key = build_key("secrets", Some(&namespace), &secret.metadata.name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!("Dry-run: Secret {}/{} validated successfully (not created)", namespace, secret.metadata.name);
        return Ok((StatusCode::CREATED, Json(secret)));
    }

    let created = state.storage.create(&key, &secret).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<Secret>> {
    info!("Getting secret: {} in namespace: {}", name, namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "secrets")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("secrets", Some(&namespace), &name);
    let secret = state.storage.get(&key).await?;

    Ok(Json(secret))
}

pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut secret): Json<Secret>,
) -> Result<Json<Secret>> {
    info!("Updating secret: {} in namespace: {}", name, namespace);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "secrets")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    secret.metadata.name = name.clone();
    secret.metadata.namespace = Some(namespace.clone());

    // Normalize: convert stringData to base64-encoded data
    secret.normalize();

    let key = build_key("secrets", Some(&namespace), &name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!("Dry-run: Secret {}/{} validated successfully (not updated)", namespace, name);
        return Ok(Json(secret));
    }

    // Try to update first, if not found then create (upsert behavior)
    let result = match state.storage.update(&key, &secret).await {
        Ok(updated) => updated,
        Err(rusternetes_common::Error::NotFound(_)) => {
            state.storage.create(&key, &secret).await?
        }
        Err(e) => return Err(e),
    };

    Ok(Json(result))
}

pub async fn delete_secret(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("Deleting secret: {} in namespace: {}", name, namespace);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "secrets")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("secrets", Some(&namespace), &name);

    // Get the resource to check if it exists
    let secret: Secret = state.storage.get(&key).await?;

    // If dry-run, skip delete operation
    if is_dry_run {
        info!("Dry-run: Secret {}/{} validated successfully (not deleted)", namespace, name);
        return Ok(StatusCode::OK);
    }

    crate::handlers::finalizers::handle_delete_with_finalizers(&*state.storage, &key, &secret).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<List<Secret>>> {
    info!("Listing secrets in namespace: {}", namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "secrets")
        .with_api_group("")
        .with_namespace(&namespace);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("secrets", Some(&namespace));
    let mut secrets = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut secrets, &params)?;

    let list = List::new("SecretList", "v1", secrets);
    Ok(Json(list))
}

/// List all secrets across all namespaces
pub async fn list_all_secrets(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<List<Secret>>> {
    info!("Listing all secrets");

    // Check authorization (cluster-wide list)
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "secrets")
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("secrets", None);
    let mut secrets = state.storage.list::<Secret>(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut secrets, &params)?;

    let list = List::new("SecretList", "v1", secrets);
    Ok(Json(list))
}

// Use the macro to create a PATCH handler
crate::patch_handler_namespaced!(patch, Secret, "secrets", "");
