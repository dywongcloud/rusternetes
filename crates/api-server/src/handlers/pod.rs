use crate::{
    middleware::AuthContext,
    patch::{apply_patch, PatchType},
    state::ApiServerState,
};
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use rusternetes_common::{
    admission::{AdmissionResponse, GroupVersionKind, GroupVersionResource, Operation},
    authz::{Decision, RequestAttributes},
    resources::Pod,
    List, Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::{info, warn};

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    body: Bytes,
) -> Result<(StatusCode, Json<Pod>)> {
    // Parse the body manually so we can do strict field validation against the raw bytes
    let mut pod: Pod = serde_json::from_slice(&body).map_err(|e| {
        rusternetes_common::Error::InvalidResource(format!("failed to decode: {}", e))
    })?;

    info!("Creating pod: {}/{}", namespace, pod.metadata.name);

    // Strict field validation: reject unknown fields when requested
    crate::handlers::validation::validate_strict_fields(&params, &body, &pod)?;

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

    // Validate pod spec
    if let Some(ref spec) = pod.spec {
        if spec.containers.is_empty() {
            return Err(rusternetes_common::Error::InvalidResource(
                "spec.containers: Required value: must have at least one container".to_string(),
            ));
        }
        for (i, container) in spec.containers.iter().enumerate() {
            if container.image.is_empty() {
                return Err(rusternetes_common::Error::InvalidResource(format!(
                    "spec.containers[{}].image: Required value",
                    i
                )));
            }
            if container.name.is_empty() {
                return Err(rusternetes_common::Error::InvalidResource(format!(
                    "spec.containers[{}].name: Required value",
                    i
                )));
            }
        }
    }

    // Set defaults
    if let Some(ref mut spec) = pod.spec {
        if spec.restart_policy.is_none() {
            spec.restart_policy = Some("Always".to_string());
        }
        if spec.dns_policy.is_none() {
            spec.dns_policy = Some("ClusterFirst".to_string());
        }
        if spec.termination_grace_period_seconds.is_none() {
            spec.termination_grace_period_seconds = Some(30);
        }
        for container in &mut spec.containers {
            if container.termination_message_path.is_none() {
                container.termination_message_path = Some("/dev/termination-log".to_string());
            }
            if container.termination_message_policy.is_none() {
                container.termination_message_policy = Some("File".to_string());
            }
            if container.image_pull_policy.is_none() {
                // Default based on image tag
                if container.image.contains(":latest") || !container.image.contains(':') {
                    container.image_pull_policy = Some("Always".to_string());
                } else {
                    container.image_pull_policy = Some("IfNotPresent".to_string());
                }
            }
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

    // Debug: Log pod_value to see if valueFrom is present
    if pod.metadata.name.contains("test-env-fieldref") || pod.metadata.name.contains("sonobuoy") {
        info!(
            "POD CREATE - Before webhooks - pod_value: {}",
            serde_json::to_string(&pod_value).unwrap()
        );
    }

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
                info!(
                    "Pod mutated by webhooks: {}/{}",
                    namespace, pod.metadata.name
                );
            }
        }
    }

    // Inject service account token (built-in admission controller)
    if let Err(e) =
        crate::admission::inject_service_account_token(&state.storage, &namespace, &mut pod).await
    {
        warn!(
            "Error injecting service account token for pod {}/{}: {}",
            namespace, pod.metadata.name, e
        );
        // Continue anyway - don't fail pod creation if SA injection fails
    }

    // Apply LimitRange defaults and validate constraints
    match crate::admission::apply_limit_range(&state.storage, &namespace, &mut pod).await {
        Ok(true) => {
            info!(
                "LimitRange admission passed for pod {}/{}",
                namespace, pod.metadata.name
            );
        }
        Ok(false) => {
            warn!(
                "LimitRange admission denied for pod {}/{}",
                namespace, pod.metadata.name
            );
            return Err(rusternetes_common::Error::Forbidden(
                "Pod violates LimitRange constraints".to_string(),
            ));
        }
        Err(e) => {
            warn!(
                "Error checking LimitRange for pod {}/{}: {}",
                namespace, pod.metadata.name, e
            );
            // Continue anyway - don't fail pod creation if LimitRange check fails
        }
    }

    // Check ResourceQuota
    match crate::admission::check_resource_quota(&state.storage, &namespace, &pod).await {
        Ok(true) => {
            info!(
                "ResourceQuota admission passed for pod {}/{}",
                namespace, pod.metadata.name
            );
        }
        Ok(false) => {
            warn!(
                "ResourceQuota admission denied for pod {}/{}",
                namespace, pod.metadata.name
            );
            return Err(rusternetes_common::Error::Forbidden(
                "Pod creation would exceed ResourceQuota".to_string(),
            ));
        }
        Err(e) => {
            warn!(
                "Error checking ResourceQuota for pod {}/{}: {}",
                namespace, pod.metadata.name, e
            );
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
            info!(
                "Validating webhooks passed for pod {}/{}",
                namespace, pod.metadata.name
            );
        }
    }

    pod.metadata.ensure_uid();
    pod.metadata.ensure_creation_timestamp();
    crate::handlers::lifecycle::set_initial_generation(&mut pod.metadata);

    // Set initial status to Pending (Kubernetes always sets this on creation)
    if pod.status.is_none()
        || pod
            .status
            .as_ref()
            .and_then(|s| s.phase.as_ref())
            .is_none()
    {
        let mut status = pod.status.take().unwrap_or_default();
        status.phase = Some(rusternetes_common::types::Phase::Pending);
        pod.status = Some(status);
    }

    let key = build_key("pods", Some(&namespace), &pod.metadata.name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!(
            "Dry-run: Pod {}/{} validated successfully (not created)",
            namespace, pod.metadata.name
        );
        return Ok((StatusCode::CREATED, Json(pod)));
    }

    match state.storage.create(&key, &pod).await {
        Ok(created) => {
            info!(
                "Pod created successfully: {}/{}",
                namespace, pod.metadata.name
            );
            Ok((StatusCode::CREATED, Json(created)))
        }
        Err(e) => {
            warn!(
                "Failed to create pod {}/{}: {}",
                namespace, pod.metadata.name, e
            );
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
    body: Bytes,
) -> Result<Json<Pod>> {
    // Parse the body manually for better error handling — axum's Json extractor
    // returns 422 Unprocessable Entity on failure, but Kubernetes expects a proper
    // Status object. Manual parsing also tolerates unknown fields gracefully.
    let mut pod: Pod = serde_json::from_slice(&body).map_err(|e| {
        rusternetes_common::Error::InvalidResource(format!("failed to decode: {}", e))
    })?;

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

    // Get the old pod for webhook comparison and concurrency control
    let key = build_key("pods", Some(&namespace), &name);
    let old_pod: Pod = state.storage.get(&key).await?;

    // Check resourceVersion for optimistic concurrency control
    crate::handlers::lifecycle::check_resource_version(
        old_pod.metadata.resource_version.as_deref(),
        pod.metadata.resource_version.as_deref(),
        &name,
    )?;

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
            Some(old_pod_value.clone()),
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

    // Increment generation if spec changed
    let new_pod_value = serde_json::to_value(&pod)
        .map_err(|e| rusternetes_common::Error::Internal(e.to_string()))?;
    crate::handlers::lifecycle::maybe_increment_generation(
        &old_pod_value,
        &new_pod_value,
        &mut pod.metadata,
    );

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!(
            "Dry-run: Pod {}/{} validated successfully (not updated)",
            namespace, name
        );
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
) -> Result<Json<Pod>> {
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
        info!(
            "Dry-run: Pod {}/{} validated successfully (not deleted)",
            namespace, name
        );
        return Ok(Json(pod));
    }

    // Kubernetes pod deletion: set deletionTimestamp and let the kubelet
    // handle graceful shutdown. The pod remains in storage until the kubelet
    // confirms termination. This is different from other resources where
    // immediate deletion is acceptable.
    let mut updated_pod = pod.clone();

    // Set deletionTimestamp if not already set
    if updated_pod.metadata.deletion_timestamp.is_none() {
        updated_pod.metadata.deletion_timestamp = Some(chrono::Utc::now());
    }

    // Parse gracePeriodSeconds from query params or use pod spec default
    let grace_period = params
        .get("gracePeriodSeconds")
        .and_then(|v| v.parse::<i64>().ok())
        .or(updated_pod.spec.as_ref().and_then(|s| s.termination_grace_period_seconds))
        .unwrap_or(30);
    updated_pod.metadata.deletion_grace_period_seconds = Some(grace_period);

    // If grace period is 0, delete immediately (force delete)
    if grace_period == 0 {
        state.storage.delete(&key).await?;
        return Ok(Json(updated_pod));
    }

    // Update the pod in storage with deletionTimestamp set
    // The kubelet will detect this and handle graceful shutdown
    let saved: Pod = state.storage.update(&key, &updated_pod).await?;
    Ok(Json(saved))
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<axum::response::Response> {
    // Check if this is a watch request
    if params
        .get("watch")
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false)
    {
        info!("Starting watch for pods in namespace: {}", namespace);
        // Parse WatchParams from the query parameters
        let watch_params = crate::handlers::watch::WatchParams {
            resource_version: crate::handlers::watch::normalize_resource_version(params.get("resourceVersion").cloned()),
            timeout_seconds: params
                .get("timeoutSeconds")
                .and_then(|v| v.parse::<u64>().ok()),
            label_selector: params.get("labelSelector").map(|s| s.clone()),
            field_selector: params.get("fieldSelector").map(|s| s.clone()),
            watch: Some(true),
            allow_watch_bookmarks: params
                .get("allowWatchBookmarks")
                .and_then(|v| v.parse::<bool>().ok()),

            send_initial_events: params.get("sendInitialEvents").and_then(|v| v.parse::<bool>().ok()),        };
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
    let limit = params.get("limit").and_then(|l| l.parse::<i64>().ok());
    let continue_token = params.get("continue").cloned();

    let pagination_params = rusternetes_common::PaginationParams {
        limit,
        continue_token,
    };

    // Get a resource version for consistency
    // In a real implementation, this would be from etcd or storage layer
    let resource_version = chrono::Utc::now().timestamp().to_string();

    // Apply pagination
    let paginated = rusternetes_common::paginate(pods, pagination_params, &resource_version)
        .map_err(|e| rusternetes_common::Error::InvalidResource(e))?;

    // Check if table format is requested
    let accept = headers.get("accept").and_then(|v| v.to_str().ok());
    if crate::handlers::table::wants_table(accept) {
        let table =
            crate::handlers::table::pods_table(paginated.items, Some(resource_version.clone()));
        return Ok(axum::Json(table).into_response());
    }

    // Wrap in proper List object with pagination metadata
    let mut list = List::new("PodList", "v1", paginated.items);
    list.metadata.continue_token = paginated.continue_token;
    list.metadata.remaining_item_count = paginated.remaining_item_count;
    list.metadata.resource_version = Some(resource_version);

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
    if params
        .get("watch")
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false)
    {
        info!("Watch request for all pods");
        // Parse WatchParams from the query parameters
        let watch_params = crate::handlers::watch::WatchParams {
            resource_version: crate::handlers::watch::normalize_resource_version(params.get("resourceVersion").cloned()),
            timeout_seconds: params
                .get("timeoutSeconds")
                .and_then(|v| v.parse::<u64>().ok()),
            label_selector: params.get("labelSelector").map(|s| s.clone()),
            field_selector: params.get("fieldSelector").map(|s| s.clone()),
            watch: Some(true),
            allow_watch_bookmarks: params
                .get("allowWatchBookmarks")
                .and_then(|v| v.parse::<bool>().ok()),

            send_initial_events: params.get("sendInitialEvents").and_then(|v| v.parse::<bool>().ok()),        };
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
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "pods").with_api_group("");

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
    let limit = params.get("limit").and_then(|l| l.parse::<i64>().ok());
    let continue_token = params.get("continue").cloned();

    let pagination_params = rusternetes_common::PaginationParams {
        limit,
        continue_token,
    };

    // Get a resource version for consistency
    let resource_version = chrono::Utc::now().timestamp().to_string();

    // Apply pagination
    let paginated = rusternetes_common::paginate(pods, pagination_params, &resource_version)
        .map_err(|e| rusternetes_common::Error::InvalidResource(e))?;

    // Check if table format is requested
    let accept = headers.get("accept").and_then(|v| v.to_str().ok());
    if crate::handlers::table::wants_table(accept) {
        let table =
            crate::handlers::table::pods_table(paginated.items, Some(resource_version.clone()));
        return Ok(axum::Json(table).into_response());
    }

    // Wrap in proper List object with pagination metadata
    let mut list = List::new("PodList", "v1", paginated.items);
    list.metadata.continue_token = paginated.continue_token;
    list.metadata.remaining_item_count = paginated.remaining_item_count;
    list.metadata.resource_version = Some(resource_version);

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

        info!(
            "Server-side apply for pod {}/{} by manager {}",
            namespace, name, field_manager
        );

        // Get current resource (if exists)
        let key = build_key("pods", Some(&namespace), &name);
        let current_json = match state.storage.get::<Pod>(&key).await {
            Ok(current) => Some(
                serde_json::to_value(&current)
                    .map_err(|e| rusternetes_common::Error::Internal(e.to_string()))?,
            ),
            Err(rusternetes_common::Error::NotFound(_)) => None,
            Err(e) => return Err(e),
        };

        // Parse desired resource
        let desired_json: serde_json::Value = serde_json::from_slice(&body).map_err(|e| {
            rusternetes_common::Error::InvalidResource(format!("Invalid resource: {}", e))
        })?;

        // Apply with server-side apply semantics
        let force = params
            .get("force")
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
                let mut applied_pod: Pod = serde_json::from_value(applied_json).map_err(|e| {
                    rusternetes_common::Error::InvalidResource(format!("Invalid result: {}", e))
                })?;

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
    let mut patched_pod: Pod = serde_json::from_value(patched_json).map_err(|e| {
        rusternetes_common::Error::InvalidResource(format!("Invalid result: {}", e))
    })?;

    // Ensure metadata matches URL (prevent changing name/namespace via patch)
    patched_pod.metadata.name = name.clone();
    patched_pod.metadata.namespace = Some(namespace.clone());

    // Update in storage
    let updated = state.storage.update(&key, &patched_pod).await?;

    Ok(Json(updated))
}

pub async fn deletecollection_pods(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<StatusCode> {
    info!(
        "DeleteCollection pods in namespace: {} with params: {:?}",
        namespace, params
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "deletecollection", "pods")
        .with_namespace(&namespace)
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: Pod collection would be deleted (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get all pods in the namespace
    let prefix = build_prefix("pods", Some(&namespace));
    let mut pods = state.storage.list::<Pod>(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut pods, &params)?;

    // Delete each matching pod
    let mut deleted_count = 0;
    for pod in pods {
        let key = build_key("pods", Some(&namespace), &pod.metadata.name);

        // Handle deletion with finalizers
        let deleted_immediately =
            !crate::handlers::finalizers::handle_delete_with_finalizers(&state.storage, &key, &pod)
                .await?;

        if deleted_immediately {
            deleted_count += 1;
        }
    }

    info!("DeleteCollection completed: {} pods deleted", deleted_count);
    Ok(StatusCode::OK)
}
