use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::StorageClass,
    List, Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

pub async fn create_storageclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut sc): Json<StorageClass>,
) -> Result<(StatusCode, Json<StorageClass>)> {
    info!("Creating StorageClass: {}", sc.metadata.name);

    // Check authorization (cluster-scoped)
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "storageclasses")
        .with_api_group("storage.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    sc.metadata.ensure_uid();
    sc.metadata.ensure_creation_timestamp();

    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: StorageClass validated successfully (not created)");
        return Ok((StatusCode::CREATED, Json(sc)));
    }

    let key = build_key("storageclasses", None, &sc.metadata.name);
    let created = state.storage.create(&key, &sc).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_storageclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<StorageClass>> {
    info!("Getting StorageClass: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "get", "storageclasses")
        .with_api_group("storage.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("storageclasses", None, &name);
    let sc = state.storage.get(&key).await?;

    Ok(Json(sc))
}

pub async fn list_storageclasses(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<List<StorageClass>>> {
    info!("Listing all StorageClasses");

    let attrs = RequestAttributes::new(auth_ctx.user, "list", "storageclasses")
        .with_api_group("storage.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("storageclasses", None);
    let mut scs = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut scs, &params)?;

    let list = List::new("StorageClassList", "storage.k8s.io/v1", scs);
    Ok(Json(list))
}

pub async fn update_storageclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut sc): Json<StorageClass>,
) -> Result<Json<StorageClass>> {
    info!("Updating StorageClass: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "update", "storageclasses")
        .with_api_group("storage.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    sc.metadata.name = name.clone();

    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: StorageClass validated successfully (not updated)");
        return Ok(Json(sc));
    }

    let key = build_key("storageclasses", None, &name);
    let updated = state.storage.update(&key, &sc).await?;

    Ok(Json(updated))
}

pub async fn delete_storageclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("Deleting StorageClass: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "storageclasses")
        .with_api_group("storage.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("storageclasses", None, &name);

    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: StorageClass validated successfully (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get the resource for finalizer handling
    let resource: StorageClass = state.storage.get(&key).await?;

    // Handle deletion with finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &resource,
    )
    .await?;

    if deleted_immediately {
        Ok(StatusCode::NO_CONTENT)
    } else {
        info!(
            "StorageClass marked for deletion (has finalizers: {:?})",
            resource.metadata.finalizers
        );
        Ok(StatusCode::OK)
    }
}

// Use the macro to create a PATCH handler
crate::patch_handler_cluster!(
    patch_storageclass,
    StorageClass,
    "storageclasses",
    "storage.k8s.io"
);

pub async fn deletecollection_storageclasses(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("DeleteCollection storageclasses with params: {:?}", params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "deletecollection", "storageclasses")
        .with_api_group("storage.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: StorageClass collection would be deleted (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get all storageclasses
    let prefix = build_prefix("storageclasses", None);
    let mut items = state.storage.list::<StorageClass>(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut items, &params)?;

    // Delete each matching resource
    let mut deleted_count = 0;
    for item in items {
        let key = build_key("storageclasses", None, &item.metadata.name);

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
        "DeleteCollection completed: {} storageclasses deleted",
        deleted_count
    );
    Ok(StatusCode::OK)
}
