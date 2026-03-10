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
    List,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::{info, warn};

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    Json(mut pod): Json<Pod>,
) -> Result<(StatusCode, Json<Pod>)> {
    info!("Creating pod: {}/{}", namespace, pod.metadata.name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

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

    // Inject service account token (built-in admission controller)
    if let Err(e) = crate::admission::inject_service_account_token(&state.storage, &namespace, &mut pod).await {
        warn!("Error injecting service account token for pod {}/{}: {}", namespace, pod.metadata.name, e);
        // Continue anyway - don't fail pod creation if SA injection fails
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

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!("Dry-run: Pod {}/{} validated successfully (not created)", namespace, pod.metadata.name);
        return Ok((StatusCode::CREATED, Json(pod)));
    }

    match state.storage.create(&key, &pod).await {
        Ok(created) => {
            info!("Pod created successfully: {}/{}", namespace, pod.metadata.name);
            Ok((StatusCode::CREATED, Json(created)))
        }
        Err(e) => {
            warn!("Failed to create pod {}/{}: {}", namespace, pod.metadata.name, e);
            Err(e)
        }
    }
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
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    Json(mut pod): Json<Pod>,
) -> Result<Json<Pod>> {
    info!("Updating pod: {}/{}", namespace, name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

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

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!("Dry-run: Pod {}/{} validated successfully (not updated)", namespace, name);
        return Ok(Json(pod));
    }

    let updated = state.storage.update(&key, &pod).await?;

    Ok(Json(updated))
}

pub async fn delete_pod(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("Deleting pod: {}/{}", namespace, name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

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

    // Get the pod to check for finalizers
    let pod: Pod = state.storage.get(&key).await?;

    // If dry-run, skip delete operation
    if is_dry_run {
        info!("Dry-run: Pod {}/{} validated successfully (not deleted)", namespace, name);
        return Ok(StatusCode::OK);
    }

    // Handle deletion with finalizers
    // If the pod has finalizers, it will be marked for deletion (deletionTimestamp set)
    // and remain in storage until controllers remove the finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &pod,
    )
    .await?;

    if deleted_immediately {
        // Pod had no finalizers and was deleted immediately
        Ok(StatusCode::NO_CONTENT)
    } else {
        // Pod has finalizers and was marked for deletion
        info!(
            "Pod {}/{} marked for deletion (has finalizers: {:?})",
            namespace,
            name,
            pod.metadata.finalizers
        );
        Ok(StatusCode::OK)
    }
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<axum::response::Response> {
    // Check if this is a watch request
    if params.get("watch").and_then(|v| v.parse::<bool>().ok()).unwrap_or(false) {
        info!("Starting watch for pods in namespace: {}", namespace);
        // Parse WatchParams from the query parameters
        let watch_params = crate::handlers::watch::WatchParams {
            resource_version: params.get("resourceVersion").map(|s| s.clone()),
            timeout_seconds: params.get("timeoutSeconds").and_then(|v| v.parse::<u64>().ok()),
            label_selector: params.get("labelSelector").map(|s| s.clone()),
            field_selector: params.get("fieldSelector").map(|s| s.clone()),
            watch: Some(true),
            allow_watch_bookmarks: params.get("allowWatchBookmarks").and_then(|v| v.parse::<bool>().ok()),
        };
        return crate::handlers::watch::watch_namespaced::<Pod>(
            state,
            auth_ctx,
            namespace,
            "pods",
            "",
            watch_params,
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

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut pods, &params)?;

    // Parse pagination parameters
    let limit = params.get("limit")
        .and_then(|l| l.parse::<i64>().ok());
    let continue_token = params.get("continue").cloned();

    let pagination_params = rusternetes_common::PaginationParams {
        limit,
        continue_token,
    };

    // Get a resource version for consistency
    // In a real implementation, this would be from etcd or storage layer
    let resource_version = "1"; // Simplified for now

    // Apply pagination
    let paginated = rusternetes_common::paginate(pods, pagination_params, resource_version)
        .map_err(|e| rusternetes_common::Error::InvalidResource(e))?;

    // Check if table format is requested
    let accept = headers.get("accept").and_then(|v| v.to_str().ok());
    if crate::handlers::table::wants_table(accept) {
        let table = crate::handlers::table::pods_table(
            paginated.items,
            Some(resource_version.to_string()),
        );
        return Ok(axum::Json(table).into_response());
    }

    // Wrap in proper List object with pagination metadata
    let mut list = List::new("PodList", "v1", paginated.items);
    list.metadata.continue_token = paginated.continue_token;
    list.metadata.remaining_item_count = paginated.remaining_item_count;
    list.metadata.resource_version = Some(resource_version.to_string());

    Ok(axum::Json(list).into_response())
}

/// List all pods across all namespaces
pub async fn list_all_pods(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<axum::response::Response> {
    // Check if this is a watch request
    if params.get("watch").and_then(|v| v.parse::<bool>().ok()).unwrap_or(false) {
        info!("Watch request for all pods");
        // Parse WatchParams from the query parameters
        let watch_params = crate::handlers::watch::WatchParams {
            resource_version: params.get("resourceVersion").map(|s| s.clone()),
            timeout_seconds: params.get("timeoutSeconds").and_then(|v| v.parse::<u64>().ok()),
            label_selector: params.get("labelSelector").map(|s| s.clone()),
            field_selector: params.get("fieldSelector").map(|s| s.clone()),
            watch: Some(true),
            allow_watch_bookmarks: params.get("allowWatchBookmarks").and_then(|v| v.parse::<bool>().ok()),
        };
        return crate::handlers::watch::watch_cluster_scoped::<Pod>(
            state,
            auth_ctx,
            "pods",
            "",
            watch_params,
        )
        .await;
    }

    info!("Listing all pods");

    // Check authorization (cluster-wide list)
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "pods")
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("pods", None);
    let mut pods = state.storage.list::<Pod>(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut pods, &params)?;

    // Parse pagination parameters
    let limit = params.get("limit")
        .and_then(|l| l.parse::<i64>().ok());
    let continue_token = params.get("continue").cloned();

    let pagination_params = rusternetes_common::PaginationParams {
        limit,
        continue_token,
    };

    // Get a resource version for consistency
    let resource_version = "1"; // Simplified for now

    // Apply pagination
    let paginated = rusternetes_common::paginate(pods, pagination_params, resource_version)
        .map_err(|e| rusternetes_common::Error::InvalidResource(e))?;

    // Check if table format is requested
    let accept = headers.get("accept").and_then(|v| v.to_str().ok());
    if crate::handlers::table::wants_table(accept) {
        let table = crate::handlers::table::pods_table(
            paginated.items,
            Some(resource_version.to_string()),
        );
        return Ok(axum::Json(table).into_response());
    }

    // Wrap in proper List object with pagination metadata
    let mut list = List::new("PodList", "v1", paginated.items);
    list.metadata.continue_token = paginated.continue_token;
    list.metadata.remaining_item_count = paginated.remaining_item_count;
    list.metadata.resource_version = Some(resource_version.to_string());

    Ok(axum::Json(list).into_response())
}

pub async fn patch(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
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

    // Check if this is a server-side apply request
    if let Some(field_manager) = params.get("fieldManager") {
        use rusternetes_common::server_side_apply::{server_side_apply, ApplyParams, ApplyResult};

        info!("Server-side apply for pod {}/{} by manager {}", namespace, name, field_manager);

        // Get current resource (if exists)
        let key = build_key("pods", Some(&namespace), &name);
        let current_json = match state.storage.get::<Pod>(&key).await {
            Ok(current) => Some(serde_json::to_value(&current)
                .map_err(|e| rusternetes_common::Error::Internal(e.to_string()))?),
            Err(rusternetes_common::Error::NotFound(_)) => None,
            Err(e) => return Err(e),
        };

        // Parse desired resource
        let desired_json: serde_json::Value = serde_json::from_slice(&body)
            .map_err(|e| rusternetes_common::Error::InvalidResource(format!("Invalid resource: {}", e)))?;

        // Apply with server-side apply semantics
        let force = params.get("force")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);

        let apply_params = if force {
            ApplyParams::new(field_manager.clone()).with_force()
        } else {
            ApplyParams::new(field_manager.clone())
        };

        let result = server_side_apply(current_json.as_ref(), &desired_json, &apply_params)
            .map_err(|e| rusternetes_common::Error::InvalidResource(e.to_string()))?;

        match result {
            ApplyResult::Success(applied_json) => {
                // Convert to Pod type
                let mut applied_pod: Pod = serde_json::from_value(applied_json)
                    .map_err(|e| rusternetes_common::Error::InvalidResource(format!("Invalid result: {}", e)))?;

                // Ensure metadata matches URL
                applied_pod.metadata.name = name.clone();
                applied_pod.metadata.namespace = Some(namespace.clone());

                // Save to storage (create or update)
                let saved = if current_json.is_some() {
                    state.storage.update(&key, &applied_pod).await?
                } else {
                    applied_pod.metadata.ensure_uid();
                    applied_pod.metadata.ensure_creation_timestamp();
                    state.storage.create(&key, &applied_pod).await?
                };

                return Ok(Json(saved));
            }
            ApplyResult::Conflicts(conflicts) => {
                // Return 409 Conflict with details
                let conflict_details: Vec<String> = conflicts
                    .iter()
                    .map(|c| {
                        format!(
                            "Field '{}' is owned by '{}' (applying as '{}')",
                            c.field, c.current_manager, c.applying_manager
                        )
                    })
                    .collect();

                return Err(rusternetes_common::Error::Conflict(format!(
                    "Apply conflict: {}. Use force=true to override.",
                    conflict_details.join("; ")
                )));
            }
        }
    }

    // Standard PATCH operation (not server-side apply)
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
