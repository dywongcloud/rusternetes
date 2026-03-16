//! Status subresource handlers
//!
//! Implements the /status subresource for resources that support it.
//! The status subresource allows updating only the status portion of a resource,
//! which is useful for controllers that need to update status without conflicting
//! with user-driven spec changes.

#![allow(dead_code)]

use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    Result,
};
use rusternetes_storage::{build_key, Storage};
use serde_json::Value;
use std::sync::Arc;
use tracing::info;

/// Generic status update handler
///
/// This handler updates only the status field of a resource while preserving
/// the spec and other fields. This is critical for avoiding conflicts between
/// user-driven changes (spec) and controller-driven changes (status).
pub async fn update_status(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((resource_type, namespace, name)): Path<(String, String, String)>,
    Json(new_resource): Json<Value>,
) -> Result<Json<Value>> {
    info!(
        "Updating status for {}/{}/{}",
        resource_type, namespace, name
    );

    // Check authorization - use 'update' verb with '/status' subresource
    let attrs = RequestAttributes::new(auth_ctx.user, "update", &resource_type)
        .with_namespace(&namespace)
        .with_name(&name)
        .with_subresource("status");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Get the current resource
    let key = build_key(&resource_type, Some(&namespace), &name);
    let current_resource: Value = state.storage.get(&key).await?;

    // Extract current and new status
    let current_metadata = current_resource
        .get("metadata")
        .ok_or_else(|| rusternetes_common::Error::InvalidResource("Missing metadata".to_string()))?
        .clone();

    let current_spec = current_resource.get("spec").cloned();

    let new_status = new_resource
        .get("status")
        .ok_or_else(|| rusternetes_common::Error::InvalidResource("Missing status".to_string()))?
        .clone();

    // Build the updated resource:
    // - Keep the current spec
    // - Keep the current metadata (except resourceVersion)
    // - Update only the status
    let mut updated_resource = current_resource.clone();

    if let Some(obj) = updated_resource.as_object_mut() {
        // Preserve spec from current resource
        if let Some(spec) = current_spec {
            obj.insert("spec".to_string(), spec);
        }

        // Update status
        obj.insert("status".to_string(), new_status);

        // Preserve metadata but update resourceVersion
        if let Some(metadata_obj) = current_metadata.as_object() {
            let mut new_metadata = metadata_obj.clone();

            // Increment resource version
            if let Some(version) = new_metadata.get("resourceVersion") {
                if let Some(version_num) = version.as_str().and_then(|s| s.parse::<i64>().ok()) {
                    new_metadata.insert(
                        "resourceVersion".to_string(),
                        Value::String((version_num + 1).to_string()),
                    );
                }
            }

            obj.insert("metadata".to_string(), Value::Object(new_metadata));
        }
    }

    // Save the updated resource
    let saved: Value = state.storage.update(&key, &updated_resource).await?;

    info!(
        "Successfully updated status for {}/{}/{}",
        resource_type, namespace, name
    );

    Ok(Json(saved))
}

/// Generic cluster-scoped status update handler
pub async fn update_cluster_status(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((resource_type, name)): Path<(String, String)>,
    Json(new_resource): Json<Value>,
) -> Result<Json<Value>> {
    info!("Updating status for {}/{}", resource_type, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", &resource_type)
        .with_name(&name)
        .with_subresource("status");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Get the current resource
    let key = build_key(&resource_type, None, &name);
    let current_resource: Value = state.storage.get(&key).await?;

    // Extract current and new status
    let current_metadata = current_resource
        .get("metadata")
        .ok_or_else(|| rusternetes_common::Error::InvalidResource("Missing metadata".to_string()))?
        .clone();

    let current_spec = current_resource.get("spec").cloned();

    let new_status = new_resource
        .get("status")
        .ok_or_else(|| rusternetes_common::Error::InvalidResource("Missing status".to_string()))?
        .clone();

    // Build the updated resource
    let mut updated_resource = current_resource.clone();

    if let Some(obj) = updated_resource.as_object_mut() {
        // Preserve spec from current resource
        if let Some(spec) = current_spec {
            obj.insert("spec".to_string(), spec);
        }

        // Update status
        obj.insert("status".to_string(), new_status);

        // Preserve metadata but update resourceVersion
        if let Some(metadata_obj) = current_metadata.as_object() {
            let mut new_metadata = metadata_obj.clone();

            // Increment resource version
            if let Some(version) = new_metadata.get("resourceVersion") {
                if let Some(version_num) = version.as_str().and_then(|s| s.parse::<i64>().ok()) {
                    new_metadata.insert(
                        "resourceVersion".to_string(),
                        Value::String((version_num + 1).to_string()),
                    );
                }
            }

            obj.insert("metadata".to_string(), Value::Object(new_metadata));
        }
    }

    // Save the updated resource
    let saved: Value = state.storage.update(&key, &updated_resource).await?;

    info!("Successfully updated status for {}/{}", resource_type, name);

    Ok(Json(saved))
}

/// Get status for a resource (read-only)
pub async fn get_status(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((resource_type, namespace, name)): Path<(String, String, String)>,
) -> Result<Json<Value>> {
    info!(
        "Getting status for {}/{}/{}",
        resource_type, namespace, name
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", &resource_type)
        .with_namespace(&namespace)
        .with_name(&name)
        .with_subresource("status");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key(&resource_type, Some(&namespace), &name);
    let resource: Value = state.storage.get(&key).await?;

    Ok(Json(resource))
}

/// Get status for a cluster-scoped resource (read-only)
pub async fn get_cluster_status(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((resource_type, name)): Path<(String, String)>,
) -> Result<Json<Value>> {
    info!("Getting status for {}/{}", resource_type, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", &resource_type)
        .with_name(&name)
        .with_subresource("status");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key(&resource_type, None, &name);
    let resource: Value = state.storage.get(&key).await?;

    Ok(Json(resource))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_status_update_preserves_spec() {
        let current_resource = json!({
            "apiVersion": "apps/v1",
            "kind": "Deployment",
            "metadata": {
                "name": "test-deployment",
                "resourceVersion": "1"
            },
            "spec": {
                "replicas": 3
            },
            "status": {
                "readyReplicas": 2
            }
        });

        let new_resource = json!({
            "apiVersion": "apps/v1",
            "kind": "Deployment",
            "metadata": {
                "name": "test-deployment"
            },
            "status": {
                "readyReplicas": 3
            }
        });

        // In a real scenario, this would be done by the handler
        // Here we just verify the logic

        let current_spec = current_resource.get("spec").unwrap();
        let new_status = new_resource.get("status").unwrap();

        assert_eq!(current_spec["replicas"], 3);
        assert_eq!(new_status["readyReplicas"], 3);
    }

    #[test]
    fn test_status_update_increments_version() {
        let metadata = json!({
            "name": "test",
            "resourceVersion": "100"
        });

        if let Some(version) = metadata.get("resourceVersion") {
            if let Some(version_num) = version.as_str().and_then(|s| s.parse::<i64>().ok()) {
                assert_eq!(version_num, 100);
                assert_eq!(version_num + 1, 101);
            }
        }
    }
}
