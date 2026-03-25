use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::ConfigMap,
    List, Result,
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
    Json(mut configmap): Json<ConfigMap>,
) -> Result<(StatusCode, Json<ConfigMap>)> {
    info!(
        "Creating configmap: {} in namespace: {}",
        configmap.metadata.name, namespace
    );

    // Validate resource name
    crate::handlers::validation::validate_resource_name(&configmap.metadata.name)?;

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "configmaps")
        .with_api_group("")
        .with_namespace(&namespace);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Ensure namespace is set from the URL path
    configmap.metadata.namespace = Some(namespace.clone());

    // Enrich metadata with system fields
    configmap.metadata.ensure_uid();
    configmap.metadata.ensure_creation_timestamp();

    let key = build_key("configmaps", Some(&namespace), &configmap.metadata.name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!(
            "Dry-run: ConfigMap {}/{} validated successfully (not created)",
            namespace, configmap.metadata.name
        );
        return Ok((StatusCode::CREATED, Json(configmap)));
    }

    let created = state.storage.create(&key, &configmap).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<ConfigMap>> {
    info!("Getting configmap: {} in namespace: {}", name, namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "configmaps")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("configmaps", Some(&namespace), &name);
    let configmap = state.storage.get(&key).await?;

    Ok(Json(configmap))
}

pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut configmap): Json<ConfigMap>,
) -> Result<Json<ConfigMap>> {
    info!("Updating configmap: {} in namespace: {}", name, namespace);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "configmaps")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    configmap.metadata.name = name.clone();
    configmap.metadata.namespace = Some(namespace.clone());

    let key = build_key("configmaps", Some(&namespace), &name);

    // Check if existing configmap is immutable
    if let Ok(existing) = state.storage.get::<ConfigMap>(&key).await {
        if existing.immutable == Some(true) {
            return Err(rusternetes_common::Error::InvalidResource(format!(
                "ConfigMap \"{}/{}\" is immutable", namespace, name
            )));
        }
    }

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!(
            "Dry-run: ConfigMap {}/{} validated successfully (not updated)",
            namespace, name
        );
        return Ok(Json(configmap));
    }

    // Try to update first, if not found then create (upsert behavior)
    let result = match state.storage.update(&key, &configmap).await {
        Ok(updated) => updated,
        Err(rusternetes_common::Error::NotFound(_)) => {
            state.storage.create(&key, &configmap).await?
        }
        Err(e) => return Err(e),
    };

    Ok(Json(result))
}

pub async fn delete_configmap(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ConfigMap>> {
    info!("Deleting configmap: {} in namespace: {}", name, namespace);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "configmaps")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("configmaps", Some(&namespace), &name);

    // Get the resource to check if it exists
    let configmap: ConfigMap = state.storage.get(&key).await?;

    // If dry-run, skip delete operation
    if is_dry_run {
        info!(
            "Dry-run: ConfigMap {}/{} validated successfully (not deleted)",
            namespace, name
        );
        return Ok(Json(configmap));
    }

    // Handle deletion with finalizers
    let has_finalizers = crate::handlers::finalizers::handle_delete_with_finalizers(
        &*state.storage,
        &key,
        &configmap,
    )
    .await?;

    if has_finalizers {
        // Resource has finalizers, re-read to get updated version with deletionTimestamp
        let updated: ConfigMap = state.storage.get(&key).await?;
        Ok(Json(updated))
    } else {
        Ok(Json(configmap))
    }
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<axum::response::Response> {
    // Check if this is a watch request
    if params.get("watch").and_then(|v| v.parse::<bool>().ok()).unwrap_or(false) {
        let watch_params = crate::handlers::watch::WatchParams {
            resource_version: crate::handlers::watch::normalize_resource_version(params.get("resourceVersion").cloned()),
            timeout_seconds: params.get("timeoutSeconds").and_then(|v| v.parse::<u64>().ok()),
            label_selector: params.get("labelSelector").map(|s| s.clone()),
            field_selector: params.get("fieldSelector").map(|s| s.clone()),
            watch: Some(true),
            allow_watch_bookmarks: params.get("allowWatchBookmarks").and_then(|v| v.parse::<bool>().ok()),
            send_initial_events: params.get("sendInitialEvents").and_then(|v| v.parse::<bool>().ok()),
        };
        return crate::handlers::watch::watch_namespaced::<ConfigMap>(
            state, auth_ctx, namespace, "configmaps", "", watch_params,
        ).await;
    }

    info!("Listing configmaps in namespace: {}", namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "configmaps")
        .with_api_group("")
        .with_namespace(&namespace);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("configmaps", Some(&namespace));
    let mut configmaps: Vec<ConfigMap> = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut configmaps, &params)?;

    let list = List::new("ConfigMapList", "v1", configmaps);
    Ok(Json(list).into_response())
}

/// List all configmaps across all namespaces
pub async fn list_all_configmaps(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<axum::response::Response> {
    // Check if this is a watch request
    if params.get("watch").and_then(|v| v.parse::<bool>().ok()).unwrap_or(false) {
        let watch_params = crate::handlers::watch::WatchParams {
            resource_version: crate::handlers::watch::normalize_resource_version(params.get("resourceVersion").cloned()),
            timeout_seconds: params.get("timeoutSeconds").and_then(|v| v.parse::<u64>().ok()),
            label_selector: params.get("labelSelector").map(|s| s.clone()),
            field_selector: params.get("fieldSelector").map(|s| s.clone()),
            watch: Some(true),
            allow_watch_bookmarks: params.get("allowWatchBookmarks").and_then(|v| v.parse::<bool>().ok()),
            send_initial_events: params.get("sendInitialEvents").and_then(|v| v.parse::<bool>().ok()),
        };
        return crate::handlers::watch::watch_cluster_scoped::<ConfigMap>(
            state, auth_ctx, "configmaps", "", watch_params,
        ).await;
    }

    info!("Listing all configmaps");

    // Check authorization (cluster-wide list)
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "configmaps").with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("configmaps", None);
    let mut configmaps = state.storage.list::<ConfigMap>(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut configmaps, &params)?;

    let list = List::new("ConfigMapList", "v1", configmaps);
    Ok(Json(list).into_response())
}

// Use the macro to create a PATCH handler
crate::patch_handler_namespaced!(patch, ConfigMap, "configmaps", "");

pub async fn deletecollection_configmaps(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<StatusCode> {
    info!(
        "DeleteCollection configmaps in namespace: {} with params: {:?}",
        namespace, params
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "deletecollection", "configmaps")
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
        info!("Dry-run: ConfigMap collection would be deleted (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get all configmaps in the namespace
    let prefix = build_prefix("configmaps", Some(&namespace));
    let mut items = state.storage.list::<ConfigMap>(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut items, &params)?;

    // Delete each matching resource
    let mut deleted_count = 0;
    for item in items {
        let key = build_key("configmaps", Some(&namespace), &item.metadata.name);

        // Handle deletion with finalizers
        let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
            &state.storage,
            &key,
            &item,
        )
        .await?;

        if deleted_immediately {
            deleted_count += 1;
        }
    }

    info!(
        "DeleteCollection completed: {} configmaps deleted",
        deleted_count
    );
    Ok(StatusCode::OK)
}
