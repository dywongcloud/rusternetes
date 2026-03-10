//! Scale subresource handlers
//!
//! Implements the /scale subresource for resources that support scaling.
//! The scale subresource allows getting and setting the replica count
//! for workload resources like Deployments, StatefulSets, and ReplicaSets.

use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    Error, Result,
};
use rusternetes_storage::{build_key, Storage};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::info;

/// Scale represents the scale of a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scale {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub kind: String,
    pub metadata: ScaleMetadata,
    pub spec: ScaleSpec,
    pub status: ScaleStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaleMetadata {
    pub name: String,
    pub namespace: String,
    #[serde(rename = "resourceVersion", skip_serializing_if = "Option::is_none")]
    pub resource_version: Option<String>,
    #[serde(rename = "creationTimestamp", skip_serializing_if = "Option::is_none")]
    pub creation_timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaleSpec {
    pub replicas: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaleStatus {
    pub replicas: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
}

/// GET /apis/{group}/{version}/namespaces/{namespace}/{resource}/{name}/scale
/// Returns the scale subresource for a resource
pub async fn get_scale(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((group, version, namespace, resource, name)): Path<(
        String,
        String,
        String,
        String,
        String,
    )>,
) -> Result<Json<Scale>> {
    info!(
        "Getting scale for {}/{}/{}/{}",
        group, resource, namespace, name
    );

    // Check authorization
    let resource_type = format!("{}.{}", resource, group);
    let attrs = RequestAttributes::new(auth_ctx.user, "get", &resource_type)
        .with_namespace(&namespace)
        .with_name(&name)
        .with_subresource("scale");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(Error::Forbidden(reason));
        }
    }

    // Get the resource
    let key = build_key(&resource, Some(&namespace), &name);
    let resource_obj: Value = state.storage.get(&key).await?;

    // Extract scale information
    let scale = extract_scale(&resource_obj, &namespace, &name, &group, &version)?;

    Ok(Json(scale))
}

/// PUT /apis/{group}/{version}/namespaces/{namespace}/{resource}/{name}/scale
/// Updates the scale subresource for a resource
pub async fn update_scale(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((group, version, namespace, resource, name)): Path<(
        String,
        String,
        String,
        String,
        String,
    )>,
    Json(scale): Json<Scale>,
) -> Result<Json<Scale>> {
    info!(
        "Updating scale for {}/{}/{}/{}",
        group, resource, namespace, name
    );

    // Check authorization
    let resource_type = format!("{}.{}", resource, group);
    let attrs = RequestAttributes::new(auth_ctx.user, "update", &resource_type)
        .with_namespace(&namespace)
        .with_name(&name)
        .with_subresource("scale");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(Error::Forbidden(reason));
        }
    }

    // Get the current resource
    let key = build_key(&resource, Some(&namespace), &name);
    let mut resource_obj: Value = state.storage.get(&key).await?;

    // Update the replicas in the spec
    if let Some(spec) = resource_obj.get_mut("spec") {
        if let Some(spec_obj) = spec.as_object_mut() {
            spec_obj.insert("replicas".to_string(), Value::Number(scale.spec.replicas.into()));
        }
    }

    // Save the updated resource
    let updated_resource: Value = state.storage.update(&key, &resource_obj).await?;

    // Extract and return the updated scale
    let updated_scale = extract_scale(&updated_resource, &namespace, &name, &group, &version)?;

    info!(
        "Successfully updated scale for {}/{}/{}/{}",
        group, resource, namespace, name
    );

    Ok(Json(updated_scale))
}

/// PATCH /apis/{group}/{version}/namespaces/{namespace}/{resource}/{name}/scale
/// Patches the scale subresource for a resource
pub async fn patch_scale(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((group, version, namespace, resource, name)): Path<(
        String,
        String,
        String,
        String,
        String,
    )>,
    Json(scale): Json<Scale>,
) -> Result<Json<Scale>> {
    // For scale, patch is the same as update
    update_scale(
        State(state),
        Extension(auth_ctx),
        Path((group, version, namespace, resource, name)),
        Json(scale),
    )
    .await
}

/// Extract scale information from a resource object
fn extract_scale(
    resource: &Value,
    namespace: &str,
    name: &str,
    group: &str,
    version: &str,
) -> Result<Scale> {
    let metadata = resource
        .get("metadata")
        .ok_or_else(|| Error::InvalidResource("Missing metadata".to_string()))?;

    let spec = resource
        .get("spec")
        .ok_or_else(|| Error::InvalidResource("Missing spec".to_string()))?;

    let status = resource.get("status");

    let resource_version = metadata
        .get("resourceVersion")
        .and_then(|v| v.as_str())
        .map(String::from);

    let creation_timestamp = metadata
        .get("creationTimestamp")
        .and_then(|v| v.as_str())
        .map(String::from);

    let replicas_spec = spec
        .get("replicas")
        .and_then(|v| v.as_i64())
        .unwrap_or(1) as i32;

    let replicas_status = status
        .and_then(|s| s.get("replicas"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    // Extract selector from spec
    let selector = spec
        .get("selector")
        .and_then(|s| {
            // Try to serialize the selector as a string
            serde_json::to_string(s).ok()
        });

    Ok(Scale {
        api_version: format!("{}/{}", group, version),
        kind: "Scale".to_string(),
        metadata: ScaleMetadata {
            name: name.to_string(),
            namespace: namespace.to_string(),
            resource_version,
            creation_timestamp,
        },
        spec: ScaleSpec {
            replicas: replicas_spec,
        },
        status: ScaleStatus {
            replicas: replicas_status,
            selector,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_scale() {
        let resource = json!({
            "apiVersion": "apps/v1",
            "kind": "Deployment",
            "metadata": {
                "name": "test-deployment",
                "namespace": "default",
                "resourceVersion": "100",
                "creationTimestamp": "2026-03-10T00:00:00Z"
            },
            "spec": {
                "replicas": 3,
                "selector": {
                    "matchLabels": {
                        "app": "test"
                    }
                }
            },
            "status": {
                "replicas": 3,
                "readyReplicas": 3
            }
        });

        let scale =
            extract_scale(&resource, "default", "test-deployment", "apps", "v1").unwrap();

        assert_eq!(scale.kind, "Scale");
        assert_eq!(scale.api_version, "apps/v1");
        assert_eq!(scale.metadata.name, "test-deployment");
        assert_eq!(scale.metadata.namespace, "default");
        assert_eq!(scale.spec.replicas, 3);
        assert_eq!(scale.status.replicas, 3);
    }

    #[test]
    fn test_extract_scale_no_status() {
        let resource = json!({
            "apiVersion": "apps/v1",
            "kind": "Deployment",
            "metadata": {
                "name": "test-deployment",
                "namespace": "default"
            },
            "spec": {
                "replicas": 5
            }
        });

        let scale =
            extract_scale(&resource, "default", "test-deployment", "apps", "v1").unwrap();

        assert_eq!(scale.spec.replicas, 5);
        assert_eq!(scale.status.replicas, 0); // No status, so 0
    }
}
