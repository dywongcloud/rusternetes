use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::Endpoints,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::info;

/// Create endpoints
pub async fn create_endpoints(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Json(mut endpoints): Json<Endpoints>,
) -> Result<(StatusCode, Json<Endpoints>)> {
    info!("Creating endpoints: {}/{}", namespace, endpoints.metadata.name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "endpoints")
        .with_namespace(&namespace)
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    endpoints.metadata.namespace = Some(namespace.clone());

    let key = build_key("endpoints", Some(&namespace), &endpoints.metadata.name);
    let created = state.storage.create(&key, &endpoints).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

/// Get endpoints
pub async fn get_endpoints(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<Endpoints>> {
    info!("Getting endpoints: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "endpoints")
        .with_namespace(&namespace)
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("endpoints", Some(&namespace), &name);
    let endpoints = state.storage.get(&key).await?;

    Ok(Json(endpoints))
}

/// List endpoints in namespace
pub async fn list_endpoints(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
) -> Result<Json<Vec<Endpoints>>> {
    info!("Listing endpoints in namespace: {}", namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "endpoints")
        .with_namespace(&namespace)
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("endpoints", Some(&namespace));
    let endpoints = state.storage.list(&prefix).await?;

    Ok(Json(endpoints))
}

/// List all endpoints across all namespaces
pub async fn list_all_endpoints(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<Vec<Endpoints>>> {
    info!("Listing all endpoints");

    // Check authorization (cluster-wide list)
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "endpoints")
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("endpoints", None);
    let endpoints = state.storage.list(&prefix).await?;

    Ok(Json(endpoints))
}

/// Update endpoints
pub async fn update_endpoints(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Json(mut endpoints): Json<Endpoints>,
) -> Result<Json<Endpoints>> {
    info!("Updating endpoints: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "endpoints")
        .with_namespace(&namespace)
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    endpoints.metadata.name = name.clone();
    endpoints.metadata.namespace = Some(namespace.clone());

    let key = build_key("endpoints", Some(&namespace), &name);
    let updated = state.storage.update(&key, &endpoints).await?;

    Ok(Json(updated))
}

/// Delete endpoints
pub async fn delete_endpoints(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<StatusCode> {
    info!("Deleting endpoints: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "endpoints")
        .with_namespace(&namespace)
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("endpoints", Some(&namespace), &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

// Use the macro to create a PATCH handler
crate::patch_handler_namespaced!(patch_endpoints, Endpoints, "endpoints", "");
