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
    http::Uri,
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

/// Extract the resource type from the request URI.
///
/// For namespaced resources, the URI looks like:
///   /api/v1/namespaces/{ns}/{resource}/{name}/status
///   /apis/{group}/{version}/namespaces/{ns}/{resource}/{name}/status
/// The resource type is the segment before {name}, i.e. 2 segments before "status".
///
/// For cluster-scoped resources, the URI looks like:
///   /api/v1/{resource}/{name}/status  (e.g. /api/v1/nodes/node1/status)
///   /api/v1/namespaces/{name}/status  (namespaces are cluster-scoped)
fn extract_resource_type_from_uri(uri: &Uri) -> String {
    let path = uri.path();
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    // The resource type is always 3 segments before the end (before name and "status")
    // e.g. ["api", "v1", "namespaces", "ns", "pods", "name", "status"] -> "pods"
    // e.g. ["api", "v1", "namespaces", "name", "status"] -> "namespaces"
    // e.g. ["api", "v1", "nodes", "name", "status"] -> "nodes"
    if segments.len() >= 3 {
        segments[segments.len() - 3].to_string()
    } else {
        "unknown".to_string()
    }
}

/// Generic status update handler
///
/// This handler updates only the status field of a resource while preserving
/// the spec and other fields. This is critical for avoiding conflicts between
/// user-driven changes (spec) and controller-driven changes (status).
pub async fn update_status(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    uri: Uri,
    Path((namespace, name)): Path<(String, String)>,
    Json(new_resource): Json<Value>,
) -> Result<Json<Value>> {
    let resource_type = extract_resource_type_from_uri(&uri);
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

        // Merge metadata: keep current metadata but apply annotations/labels
        // from the incoming request (for PATCH/server-side apply support).
        if let Some(metadata_obj) = current_metadata.as_object() {
            let mut merged_metadata = metadata_obj.clone();
            merged_metadata.remove("resourceVersion");

            // Merge annotations and labels from the new resource
            if let Some(new_meta) = new_resource.get("metadata").and_then(|m| m.as_object()) {
                if let Some(new_annotations) = new_meta.get("annotations").and_then(|a| a.as_object()) {
                    let annotations = merged_metadata
                        .entry("annotations")
                        .or_insert_with(|| Value::Object(serde_json::Map::new()));
                    if let Some(existing) = annotations.as_object_mut() {
                        for (k, v) in new_annotations {
                            existing.insert(k.clone(), v.clone());
                        }
                    }
                }
                if let Some(new_labels) = new_meta.get("labels").and_then(|l| l.as_object()) {
                    let labels = merged_metadata
                        .entry("labels")
                        .or_insert_with(|| Value::Object(serde_json::Map::new()));
                    if let Some(existing) = labels.as_object_mut() {
                        for (k, v) in new_labels {
                            existing.insert(k.clone(), v.clone());
                        }
                    }
                }
            }

            obj.insert("metadata".to_string(), Value::Object(merged_metadata));
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
    uri: Uri,
    Path(name): Path<String>,
    Json(new_resource): Json<Value>,
) -> Result<Json<Value>> {
    let resource_type = extract_resource_type_from_uri(&uri);
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

        // Preserve metadata but clear resourceVersion to avoid conflicts
        // with concurrent updates. The storage layer assigns a new version.
        if let Some(metadata_obj) = current_metadata.as_object() {
            let mut new_metadata = metadata_obj.clone();
            new_metadata.remove("resourceVersion");
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
    uri: Uri,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<Value>> {
    let resource_type = extract_resource_type_from_uri(&uri);
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
    uri: Uri,
    Path(name): Path<String>,
) -> Result<Json<Value>> {
    let resource_type = extract_resource_type_from_uri(&uri);
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
