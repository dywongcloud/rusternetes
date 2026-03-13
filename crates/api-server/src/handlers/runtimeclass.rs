use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::RuntimeClass,
    List,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

pub async fn create_runtimeclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut runtime_class): Json<RuntimeClass>,
) -> Result<(StatusCode, Json<RuntimeClass>)> {
    info!("Creating RuntimeClass: {}", runtime_class.metadata.name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "runtimeclasses")
        .with_api_group("node.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Enrich metadata with system fields
    runtime_class.metadata.ensure_uid();
    runtime_class.metadata.ensure_creation_timestamp();

    let key = build_key("runtimeclasses", None, &runtime_class.metadata.name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!("Dry-run: RuntimeClass {} validated successfully (not created)", runtime_class.metadata.name);
        return Ok((StatusCode::CREATED, Json(runtime_class)));
    }

    let created = state.storage.create(&key, &runtime_class).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_runtimeclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<RuntimeClass>> {
    info!("Getting RuntimeClass: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "runtimeclasses")
        .with_api_group("node.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("runtimeclasses", None, &name);
    let runtime_class = state.storage.get(&key).await?;

    Ok(Json(runtime_class))
}

pub async fn update_runtimeclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut runtime_class): Json<RuntimeClass>,
) -> Result<Json<RuntimeClass>> {
    info!("Updating RuntimeClass: {}", name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "runtimeclasses")
        .with_api_group("node.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    runtime_class.metadata.name = name.clone();

    let key = build_key("runtimeclasses", None, &name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!("Dry-run: RuntimeClass {} validated successfully (not updated)", name);
        return Ok(Json(runtime_class));
    }

    // Try to update first, if not found then create (upsert behavior)
    let result = match state.storage.update(&key, &runtime_class).await {
        Ok(updated) => updated,
        Err(rusternetes_common::Error::NotFound(_)) => {
            state.storage.create(&key, &runtime_class).await?
        }
        Err(e) => return Err(e),
    };

    Ok(Json(result))
}

pub async fn delete_runtimeclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("Deleting RuntimeClass: {}", name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "runtimeclasses")
        .with_api_group("node.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("runtimeclasses", None, &name);

    // If dry-run, skip delete operation
    if is_dry_run {
        info!("Dry-run: RuntimeClass {} validated successfully (not deleted)", name);
        return Ok(StatusCode::OK);
    }

    // Get the runtime class for finalizer handling
    let runtime_class: RuntimeClass = state.storage.get(&key).await?;

    // Handle deletion with finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &runtime_class,
    )
    .await?;

    if deleted_immediately {
        Ok(StatusCode::NO_CONTENT)
    } else {
        info!(
            "RuntimeClass {} marked for deletion (has finalizers: {:?})",
            name,
            runtime_class.metadata.finalizers
        );
        Ok(StatusCode::OK)
    }
}

pub async fn list_runtimeclasses(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<List<RuntimeClass>>> {
    info!("Listing RuntimeClasses");

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "runtimeclasses")
        .with_api_group("node.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("runtimeclasses", None);
    let runtime_classes = state.storage.list(&prefix).await?;

    let list = List::new("RuntimeClassList", "node.k8s.io/v1", runtime_classes);
    Ok(Json(list))
}

// Use the macro to create a PATCH handler for cluster-scoped RuntimeClass
crate::patch_handler_cluster!(patch_runtimeclass, RuntimeClass, "runtimeclasses", "node.k8s.io");

pub async fn deletecollection_runtimeclasses(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("DeleteCollection runtimeclasses with params: {:?}", params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "deletecollection", "runtimeclasses")
        .with_api_group("node.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: RuntimeClass collection would be deleted (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get all runtimeclasses
    let prefix = build_prefix("runtimeclasses", None);
    let mut items = state.storage.list::<RuntimeClass>(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut items, &params)?;

    // Delete each matching resource
    let mut deleted_count = 0;
    for item in items {
        let key = build_key("runtimeclasses", None, &item.metadata.name);

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

    info!("DeleteCollection completed: {} runtimeclasses deleted", deleted_count);
    Ok(StatusCode::OK)
}
