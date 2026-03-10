/// Generic PATCH handler that works for any resource type
///
/// This module provides a generic implementation of PATCH operations
/// that can be used across all Kubernetes resource types.

use crate::{
    middleware::AuthContext,
    patch::{apply_patch, PatchType},
    state::ApiServerState,
};
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    Result,
};
use rusternetes_storage::{build_key, Storage};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use tracing::info;

/// Generic PATCH handler for namespaced resources
///
/// # Type Parameters
/// - `T`: The resource type (must implement Serialize + DeserializeOwned)
///
/// # Parameters
/// - `state`: API server state
/// - `auth_ctx`: Authentication context
/// - `namespace`: Resource namespace
/// - `name`: Resource name
/// - `headers`: HTTP headers (contains Content-Type for patch type)
/// - `body`: Patch document
/// - `resource_type`: Resource type name (e.g., "deployments", "services")
/// - `api_group`: API group (e.g., "apps", "" for core)
///
/// # Returns
/// Updated resource after applying patch
pub async fn patch_namespaced_resource<T>(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
    resource_type: &str,
    api_group: &str,
) -> Result<Json<T>>
where
    T: Serialize + DeserializeOwned + Send + Sync,
{
    info!("Patching {} {}/{}", resource_type, namespace, name);

    // Check authorization - use 'patch' verb for RBAC
    let attrs = RequestAttributes::new(auth_ctx.user, "patch", resource_type)
        .with_namespace(&namespace)
        .with_api_group(api_group)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Get Content-Type header
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/strategic-merge-patch+json");

    // Parse patch type
    let patch_type = PatchType::from_content_type(content_type)
        .map_err(|e| rusternetes_common::Error::InvalidResource(e.to_string()))?;

    // Get current resource
    let key = build_key(resource_type, Some(&namespace), &name);
    let current_resource: T = state.storage.get(&key).await?;

    // Convert to JSON for patching
    let current_json = serde_json::to_value(&current_resource)
        .map_err(|e| rusternetes_common::Error::Internal(e.to_string()))?;

    // Parse patch document
    let patch_json: serde_json::Value = serde_json::from_slice(&body).map_err(|e| {
        rusternetes_common::Error::InvalidResource(format!("Invalid patch: {}", e))
    })?;

    // Apply patch
    let patched_json = apply_patch(&current_json, &patch_json, patch_type)
        .map_err(|e| rusternetes_common::Error::InvalidResource(e.to_string()))?;

    // Convert back to resource type
    let patched_resource: T = serde_json::from_value(patched_json).map_err(|e| {
        rusternetes_common::Error::InvalidResource(format!("Invalid result: {}", e))
    })?;

    // For resources with metadata, ensure name/namespace can't be changed via patch
    // This is handled by updating in storage with the original key

    // Update in storage
    let updated = state.storage.update(&key, &patched_resource).await?;

    Ok(Json(updated))
}

/// Generic PATCH handler for cluster-scoped resources
///
/// # Type Parameters
/// - `T`: The resource type (must implement Serialize + DeserializeOwned)
///
/// # Parameters
/// - `state`: API server state
/// - `auth_ctx`: Authentication context
/// - `name`: Resource name
/// - `headers`: HTTP headers (contains Content-Type for patch type)
/// - `body`: Patch document
/// - `resource_type`: Resource type name (e.g., "nodes", "clusterroles")
/// - `api_group`: API group (e.g., "rbac.authorization.k8s.io", "" for core)
///
/// # Returns
/// Updated resource after applying patch
pub async fn patch_cluster_resource<T>(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    headers: HeaderMap,
    body: Bytes,
    resource_type: &str,
    api_group: &str,
) -> Result<Json<T>>
where
    T: Serialize + DeserializeOwned + Send + Sync,
{
    info!("Patching cluster-scoped {} {}", resource_type, name);

    // Check authorization - use 'patch' verb for RBAC
    let attrs = RequestAttributes::new(auth_ctx.user, "patch", resource_type)
        .with_api_group(api_group)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Get Content-Type header
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/strategic-merge-patch+json");

    // Parse patch type
    let patch_type = PatchType::from_content_type(content_type)
        .map_err(|e| rusternetes_common::Error::InvalidResource(e.to_string()))?;

    // Get current resource
    let key = build_key(resource_type, None, &name);
    let current_resource: T = state.storage.get(&key).await?;

    // Convert to JSON for patching
    let current_json = serde_json::to_value(&current_resource)
        .map_err(|e| rusternetes_common::Error::Internal(e.to_string()))?;

    // Parse patch document
    let patch_json: serde_json::Value = serde_json::from_slice(&body).map_err(|e| {
        rusternetes_common::Error::InvalidResource(format!("Invalid patch: {}", e))
    })?;

    // Apply patch
    let patched_json = apply_patch(&current_json, &patch_json, patch_type)
        .map_err(|e| rusternetes_common::Error::InvalidResource(e.to_string()))?;

    // Convert back to resource type
    let patched_resource: T = serde_json::from_value(patched_json).map_err(|e| {
        rusternetes_common::Error::InvalidResource(format!("Invalid result: {}", e))
    })?;

    // Update in storage
    let updated = state.storage.update(&key, &patched_resource).await?;

    Ok(Json(updated))
}

/// Macro to create a PATCH handler for a namespaced resource
///
/// This macro generates a handler function that calls the generic patch implementation
/// with the appropriate resource type and metadata.
///
/// # Example
/// ```ignore
/// patch_handler_namespaced!(patch_deployment, Deployment, "deployments", "apps");
/// ```
#[macro_export]
macro_rules! patch_handler_namespaced {
    ($handler_name:ident, $resource_type:ty, $resource_name:expr, $api_group:expr) => {
        pub async fn $handler_name(
            state: axum::extract::State<std::sync::Arc<$crate::state::ApiServerState>>,
            auth_ctx: axum::Extension<$crate::middleware::AuthContext>,
            path: axum::extract::Path<(String, String)>,
            headers: axum::http::HeaderMap,
            body: axum::body::Bytes,
        ) -> rusternetes_common::Result<axum::Json<$resource_type>> {
            $crate::handlers::generic_patch::patch_namespaced_resource::<$resource_type>(
                state,
                auth_ctx,
                path,
                headers,
                body,
                $resource_name,
                $api_group,
            )
            .await
        }
    };
}

/// Macro to create a PATCH handler for a cluster-scoped resource
///
/// This macro generates a handler function that calls the generic patch implementation
/// with the appropriate resource type and metadata.
///
/// # Example
/// ```ignore
/// patch_handler_cluster!(patch_node, Node, "nodes", "");
/// ```
#[macro_export]
macro_rules! patch_handler_cluster {
    ($handler_name:ident, $resource_type:ty, $resource_name:expr, $api_group:expr) => {
        pub async fn $handler_name(
            state: axum::extract::State<std::sync::Arc<$crate::state::ApiServerState>>,
            auth_ctx: axum::Extension<$crate::middleware::AuthContext>,
            path: axum::extract::Path<String>,
            headers: axum::http::HeaderMap,
            body: axum::body::Bytes,
        ) -> rusternetes_common::Result<axum::Json<$resource_type>> {
            $crate::handlers::generic_patch::patch_cluster_resource::<$resource_type>(
                state,
                auth_ctx,
                path,
                headers,
                body,
                $resource_name,
                $api_group,
            )
            .await
        }
    };
}
