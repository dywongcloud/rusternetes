use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::ReplicationController,
    List, Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

pub async fn create_replicationcontroller(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Json(mut rc): Json<ReplicationController>,
) -> Result<(StatusCode, Json<ReplicationController>)> {
    info!(
        "Creating replicationcontroller: {} in namespace: {}",
        rc.metadata.name, namespace
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "replicationcontrollers")
        .with_api_group("")
        .with_namespace(&namespace);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Enrich metadata with system fields
    rc.metadata.ensure_uid();
    rc.metadata.ensure_creation_timestamp();

    let key = build_key(
        "replicationcontrollers",
        Some(&namespace),
        &rc.metadata.name,
    );
    let created = state.storage.create(&key, &rc).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_replicationcontroller(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<ReplicationController>> {
    info!(
        "Getting replicationcontroller: {} in namespace: {}",
        name, namespace
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "replicationcontrollers")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("replicationcontrollers", Some(&namespace), &name);
    let rc = state.storage.get(&key).await?;

    Ok(Json(rc))
}

pub async fn update_replicationcontroller(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Json(mut rc): Json<ReplicationController>,
) -> Result<Json<ReplicationController>> {
    info!(
        "Updating replicationcontroller: {} in namespace: {}",
        name, namespace
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "replicationcontrollers")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    rc.metadata.name = name.clone();
    rc.metadata.namespace = Some(namespace.clone());

    let key = build_key("replicationcontrollers", Some(&namespace), &name);

    // Try to update first, if not found then create (upsert behavior)
    let result = match state.storage.update(&key, &rc).await {
        Ok(updated) => updated,
        Err(rusternetes_common::Error::NotFound(_)) => state.storage.create(&key, &rc).await?,
        Err(e) => return Err(e),
    };

    Ok(Json(result))
}

pub async fn delete_replicationcontroller(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ReplicationController>> {
    info!(
        "Deleting replicationcontroller: {} in namespace: {}",
        name, namespace
    );

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "replicationcontrollers")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("replicationcontrollers", Some(&namespace), &name);

    // Get the resource to check if it exists
    let rc: ReplicationController = state.storage.get(&key).await?;

    // If dry-run, skip delete operation
    if is_dry_run {
        info!(
            "Dry-run: ReplicationController {}/{} validated successfully (not deleted)",
            namespace, name
        );
        return Ok(Json(rc));
    }

    let has_finalizers =
        crate::handlers::finalizers::handle_delete_with_finalizers(&*state.storage, &key, &rc)
            .await?;

    if has_finalizers {
        // Resource has finalizers, re-read to get updated version with deletionTimestamp
        let updated: ReplicationController = state.storage.get(&key).await?;
        Ok(Json(updated))
    } else {
        Ok(Json(rc))
    }
}

pub async fn list_replicationcontrollers(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
) -> Result<Json<List<ReplicationController>>> {
    info!("Listing replicationcontrollers in namespace: {}", namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "replicationcontrollers")
        .with_api_group("")
        .with_namespace(&namespace);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("replicationcontrollers", Some(&namespace));
    let rcs = state.storage.list(&prefix).await?;

    let list = List::new("ReplicationControllerList", "v1", rcs);
    Ok(Json(list))
}

/// List all replicationcontrollers across all namespaces
pub async fn list_all_replicationcontrollers(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<List<ReplicationController>>> {
    info!("Listing all replicationcontrollers");

    // Check authorization (cluster-wide list)
    let attrs =
        RequestAttributes::new(auth_ctx.user, "list", "replicationcontrollers").with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("replicationcontrollers", None);
    let rcs = state.storage.list::<ReplicationController>(&prefix).await?;

    let list = List::new("ReplicationControllerList", "v1", rcs);
    Ok(Json(list))
}

// Use the macro to create a PATCH handler
crate::patch_handler_namespaced!(
    patch_replicationcontroller,
    ReplicationController,
    "replicationcontrollers",
    ""
);

pub async fn deletecollection_replicationcontrollers(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<StatusCode> {
    info!(
        "DeleteCollection replicationcontrollers in namespace: {} with params: {:?}",
        namespace, params
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "deletecollection", "replicationcontrollers")
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
        info!("Dry-run: ReplicationController collection would be deleted (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get all replicationcontrollers in the namespace
    let prefix = build_prefix("replicationcontrollers", Some(&namespace));
    let mut items = state.storage.list::<ReplicationController>(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut items, &params)?;

    // Delete each matching resource
    let mut deleted_count = 0;
    for item in items {
        let key = build_key(
            "replicationcontrollers",
            Some(&namespace),
            &item.metadata.name,
        );

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
        "DeleteCollection completed: {} replicationcontrollers deleted",
        deleted_count
    );
    Ok(StatusCode::OK)
}
