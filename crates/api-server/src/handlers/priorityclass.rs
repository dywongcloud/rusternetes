use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::PriorityClass,
    List, Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut priority_class): Json<PriorityClass>,
) -> Result<(StatusCode, Json<PriorityClass>)> {
    info!("Creating PriorityClass: {}", priority_class.metadata.name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "priorityclasses")
        .with_api_group("scheduling.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Enrich metadata with system fields
    priority_class.metadata.ensure_uid();
    priority_class.metadata.ensure_creation_timestamp();

    let key = build_key("priorityclasses", None, &priority_class.metadata.name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!(
            "Dry-run: PriorityClass {} validated successfully (not created)",
            priority_class.metadata.name
        );
        return Ok((StatusCode::CREATED, Json(priority_class)));
    }

    let created = state.storage.create(&key, &priority_class).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<PriorityClass>> {
    info!("Getting PriorityClass: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "priorityclasses")
        .with_api_group("scheduling.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("priorityclasses", None, &name);
    let priority_class = state.storage.get(&key).await?;

    Ok(Json(priority_class))
}

pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut priority_class): Json<PriorityClass>,
) -> Result<Json<PriorityClass>> {
    info!("Updating PriorityClass: {}", name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "priorityclasses")
        .with_api_group("scheduling.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    priority_class.metadata.name = name.clone();

    let key = build_key("priorityclasses", None, &name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!(
            "Dry-run: PriorityClass {} validated successfully (not updated)",
            name
        );
        return Ok(Json(priority_class));
    }

    let result = match state.storage.update(&key, &priority_class).await {
        Ok(updated) => updated,
        Err(rusternetes_common::Error::NotFound(_)) => {
            state.storage.create(&key, &priority_class).await?
        }
        Err(e) => return Err(e),
    };

    Ok(Json(result))
}

pub async fn delete(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<PriorityClass>> {
    info!("Deleting PriorityClass: {}", name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "priorityclasses")
        .with_api_group("scheduling.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("priorityclasses", None, &name);

    // Get the priority class for finalizer handling
    let priority_class: PriorityClass = state.storage.get(&key).await?;

    // If dry-run, skip delete operation
    if is_dry_run {
        info!(
            "Dry-run: PriorityClass {} validated successfully (not deleted)",
            name
        );
        return Ok(Json(priority_class));
    }

    // Handle deletion with finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &priority_class,
    )
    .await?;

    if deleted_immediately {
        Ok(Json(priority_class))
    } else {
        // Resource has finalizers, re-read to get updated version with deletionTimestamp
        let updated: PriorityClass = state.storage.get(&key).await?;
        Ok(Json(updated))
    }
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<List<PriorityClass>>> {
    info!("Listing PriorityClasses");

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "priorityclasses")
        .with_api_group("scheduling.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("priorityclasses", None);
    let mut priority_classes = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut priority_classes, &params)?;

    let list = List::new(
        "PriorityClassList",
        "scheduling.k8s.io/v1",
        priority_classes,
    );
    Ok(Json(list))
}

// Use the macro to create a PATCH handler
crate::patch_handler_cluster!(patch, PriorityClass, "priorityclasses", "scheduling.k8s.io");

pub async fn deletecollection_priorityclasses(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("DeleteCollection priorityclasses with params: {:?}", params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "deletecollection", "priorityclasses")
        .with_api_group("scheduling.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: PriorityClass collection would be deleted (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get all priorityclasses
    let prefix = build_prefix("priorityclasses", None);
    let mut items = state.storage.list::<PriorityClass>(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut items, &params)?;

    // Delete each matching resource
    let mut deleted_count = 0;
    for item in items {
        let key = build_key("priorityclasses", None, &item.metadata.name);

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
        "DeleteCollection completed: {} priorityclasses deleted",
        deleted_count
    );
    Ok(StatusCode::OK)
}
