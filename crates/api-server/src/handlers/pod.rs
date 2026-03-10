use crate::{middleware::AuthContext, patch::{PatchType, apply_patch}, state::ApiServerState};
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
    Extension, Json,
};
use rusternetes_common::{
    admission::{AdmissionResponse, GroupVersionKind, GroupVersionResource, Operation},
    authz::{Decision, RequestAttributes},
    resources::Pod,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::{info, warn};

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Json(mut pod): Json<Pod>,
) -> Result<(StatusCode, Json<Pod>)> {
    info!("Creating pod: {}/{}", namespace, pod.metadata.name);

    // Build user info for admission webhooks early (before auth_ctx.user is moved)
    let user_info = rusternetes_common::admission::UserInfo {
        username: auth_ctx.user.username.clone(),
        uid: auth_ctx.user.uid.clone(),
        groups: auth_ctx.user.groups.clone(),
    };

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "pods")
        .with_namespace(&namespace)
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Ensure namespace is set correctly
    pod.metadata.namespace = Some(namespace.clone());

    // Define GVK and GVR for Pod
    let gvk = GroupVersionKind {
        group: "".to_string(),
        version: "v1".to_string(),
        kind: "Pod".to_string(),
    };

    let gvr = GroupVersionResource {
        group: "".to_string(),
        version: "v1".to_string(),
        resource: "pods".to_string(),
    };

    // Run mutating webhooks BEFORE other admission checks
    let pod_value = serde_json::to_value(&pod)
        .map_err(|e| rusternetes_common::Error::Internal(e.to_string()))?;

    let (mutation_response, mutated_pod_value) = state
        .webhook_manager
        .run_mutating_webhooks(
            &Operation::Create,
            &gvk,
            &gvr,
            Some(&namespace),
            &pod.metadata.name,
            Some(pod_value),
            None,
            &user_info,
        )
        .await?;

    // Check if mutating webhooks denied the request
    match mutation_response {
        AdmissionResponse::Deny(reason) => {
            warn!("Mutating webhooks denied pod creation: {}", reason);
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
        AdmissionResponse::Allow | AdmissionResponse::AllowWithPatch(_) => {
            // Continue with the mutated object
            if let Some(mutated_value) = mutated_pod_value {
                pod = serde_json::from_value(mutated_value)
                    .map_err(|e| rusternetes_common::Error::Internal(e.to_string()))?;
                info!("Pod mutated by webhooks: {}/{}", namespace, pod.metadata.name);
            }
        }
    }

    // Apply LimitRange defaults and validate constraints
    match crate::admission::apply_limit_range(&state.storage, &namespace, &mut pod).await {
        Ok(true) => {
            info!("LimitRange admission passed for pod {}/{}", namespace, pod.metadata.name);
        }
        Ok(false) => {
            warn!("LimitRange admission denied for pod {}/{}", namespace, pod.metadata.name);
            return Err(rusternetes_common::Error::Forbidden(
                "Pod violates LimitRange constraints".to_string(),
            ));
        }
        Err(e) => {
            warn!("Error checking LimitRange for pod {}/{}: {}", namespace, pod.metadata.name, e);
            // Continue anyway - don't fail pod creation if LimitRange check fails
        }
    }

    // Check ResourceQuota
    match crate::admission::check_resource_quota(&state.storage, &namespace, &pod).await {
        Ok(true) => {
            info!("ResourceQuota admission passed for pod {}/{}", namespace, pod.metadata.name);
        }
        Ok(false) => {
            warn!("ResourceQuota admission denied for pod {}/{}", namespace, pod.metadata.name);
            return Err(rusternetes_common::Error::Forbidden(
                "Pod creation would exceed ResourceQuota".to_string(),
            ));
        }
        Err(e) => {
            warn!("Error checking ResourceQuota for pod {}/{}: {}", namespace, pod.metadata.name, e);
            // Continue anyway - don't fail pod creation if quota check fails
        }
    }

    // Run validating webhooks AFTER mutations and other admission checks
    let final_pod_value = serde_json::to_value(&pod)
        .map_err(|e| rusternetes_common::Error::Internal(e.to_string()))?;

    let validation_response = state
        .webhook_manager
        .run_validating_webhooks(
            &Operation::Create,
            &gvk,
            &gvr,
            Some(&namespace),
            &pod.metadata.name,
            Some(final_pod_value),
            None,
            &user_info,
        )
        .await?;

    // Check if validating webhooks denied the request
    match validation_response {
        AdmissionResponse::Deny(reason) => {
            warn!("Validating webhooks denied pod creation: {}", reason);
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
        AdmissionResponse::Allow | AdmissionResponse::AllowWithPatch(_) => {
            info!("Validating webhooks passed for pod {}/{}", namespace, pod.metadata.name);
        }
    }

    pod.metadata.ensure_uid();
    pod.metadata.ensure_creation_timestamp();

    let key = build_key("pods", Some(&namespace), &pod.metadata.name);
    let created = state.storage.create(&key, &pod).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<Pod>> {
    info!("Getting pod: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "pods")
        .with_namespace(&namespace)
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("pods", Some(&namespace), &name);
    let pod = state.storage.get(&key).await?;

    Ok(Json(pod))
}

pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Json(mut pod): Json<Pod>,
) -> Result<Json<Pod>> {
    info!("Updating pod: {}/{}", namespace, name);

    // Build user info for admission webhooks early (before auth_ctx.user is moved)
    let user_info = rusternetes_common::admission::UserInfo {
        username: auth_ctx.user.username.clone(),
        uid: auth_ctx.user.uid.clone(),
        groups: auth_ctx.user.groups.clone(),
    };

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "pods")
        .with_namespace(&namespace)
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Ensure metadata matches URL
    pod.metadata.name = name.clone();
    pod.metadata.namespace = Some(namespace.clone());

    // Get the old pod for webhook comparison
    let key = build_key("pods", Some(&namespace), &name);
    let old_pod: Pod = state.storage.get(&key).await?;
    let old_pod_value = serde_json::to_value(&old_pod)
        .map_err(|e| rusternetes_common::Error::Internal(e.to_string()))?;

    // Define GVK and GVR for Pod
    let gvk = GroupVersionKind {
        group: "".to_string(),
        version: "v1".to_string(),
        kind: "Pod".to_string(),
    };

    let gvr = GroupVersionResource {
        group: "".to_string(),
        version: "v1".to_string(),
        resource: "pods".to_string(),
    };

    // Run mutating webhooks
    let pod_value = serde_json::to_value(&pod)
        .map_err(|e| rusternetes_common::Error::Internal(e.to_string()))?;

    let (mutation_response, mutated_pod_value) = state
        .webhook_manager
        .run_mutating_webhooks(
            &Operation::Update,
            &gvk,
            &gvr,
            Some(&namespace),
            &name,
            Some(pod_value),
            Some(old_pod_value.clone()),
            &user_info,
        )
        .await?;

    // Check if mutating webhooks denied the request
    match mutation_response {
        AdmissionResponse::Deny(reason) => {
            warn!("Mutating webhooks denied pod update: {}", reason);
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
        AdmissionResponse::Allow | AdmissionResponse::AllowWithPatch(_) => {
            // Continue with the mutated object
            if let Some(mutated_value) = mutated_pod_value {
                pod = serde_json::from_value(mutated_value)
                    .map_err(|e| rusternetes_common::Error::Internal(e.to_string()))?;
                info!("Pod mutated by webhooks: {}/{}", namespace, name);
            }
        }
    }

    // Run validating webhooks
    let final_pod_value = serde_json::to_value(&pod)
        .map_err(|e| rusternetes_common::Error::Internal(e.to_string()))?;

    let validation_response = state
        .webhook_manager
        .run_validating_webhooks(
            &Operation::Update,
            &gvk,
            &gvr,
            Some(&namespace),
            &name,
            Some(final_pod_value),
            Some(old_pod_value),
            &user_info,
        )
        .await?;

    // Check if validating webhooks denied the request
    match validation_response {
        AdmissionResponse::Deny(reason) => {
            warn!("Validating webhooks denied pod update: {}", reason);
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
        AdmissionResponse::Allow | AdmissionResponse::AllowWithPatch(_) => {
            info!("Validating webhooks passed for pod {}/{}", namespace, name);
        }
    }

    let updated = state.storage.update(&key, &pod).await?;

    Ok(Json(updated))
}

pub async fn delete_pod(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<StatusCode> {
    info!("Deleting pod: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "pods")
        .with_namespace(&namespace)
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("pods", Some(&namespace), &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<axum::response::Response> {
    // Check if this is a watch request
    if params.get("watch").and_then(|v| v.parse::<bool>().ok()).unwrap_or(false) {
        info!("Starting watch for pods in namespace: {}", namespace);
        return crate::handlers::watch::watch_namespaced::<Pod>(
            state,
            auth_ctx,
            namespace,
            "pods",
            "",
        )
        .await;
    }

    info!("Listing pods in namespace: {}", namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "pods")
        .with_namespace(&namespace)
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("pods", Some(&namespace));
    let mut pods: Vec<Pod> = state.storage.list(&prefix).await?;

    // Apply field selector filtering if provided
    if let Some(field_selector_str) = params.get("fieldSelector") {
        use rusternetes_common::field_selector::FieldSelector;

        match FieldSelector::parse(field_selector_str) {
            Ok(selector) => {
                if !selector.is_empty() {
                    // Filter pods by field selector
                    pods.retain(|pod| {
                        let pod_json = serde_json::to_value(pod).unwrap_or_default();
                        selector.matches(&pod_json)
                    });
                }
            }
            Err(e) => {
                return Err(rusternetes_common::Error::InvalidResource(format!(
                    "Invalid field selector: {}",
                    e
                )));
            }
        }
    }

    Ok(axum::Json(pods).into_response())
}

pub async fn patch(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Pod>> {
    info!("Patching pod: {}/{}", namespace, name);

    // Check authorization - use 'patch' verb for RBAC
    let attrs = RequestAttributes::new(auth_ctx.user, "patch", "pods")
        .with_namespace(&namespace)
        .with_api_group("")
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
    let key = build_key("pods", Some(&namespace), &name);
    let current_pod: Pod = state.storage.get(&key).await?;

    // Convert to JSON for patching
    let current_json = serde_json::to_value(&current_pod)
        .map_err(|e| rusternetes_common::Error::Internal(e.to_string()))?;

    // Parse patch document
    let patch_json: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| rusternetes_common::Error::InvalidResource(format!("Invalid patch: {}", e)))?;

    // Apply patch
    let patched_json = apply_patch(&current_json, &patch_json, patch_type)
        .map_err(|e| rusternetes_common::Error::InvalidResource(e.to_string()))?;

    // Convert back to Pod
    let mut patched_pod: Pod = serde_json::from_value(patched_json)
        .map_err(|e| rusternetes_common::Error::InvalidResource(format!("Invalid result: {}", e)))?;

    // Ensure metadata matches URL (prevent changing name/namespace via patch)
    patched_pod.metadata.name = name.clone();
    patched_pod.metadata.namespace = Some(namespace.clone());

    // Update in storage
    let updated = state.storage.update(&key, &patched_pod).await?;

    Ok(Json(updated))
}
