use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::Node,
    List,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut node): Json<Node>,
) -> Result<(StatusCode, Json<Node>)> {
    info!("Creating node: {}", node.metadata.name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "nodes")
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Enrich metadata with system fields
    node.metadata.ensure_uid();
    node.metadata.ensure_creation_timestamp();

    let key = build_key("nodes", None, &node.metadata.name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!("Dry-run: Node {} validated successfully (not created)", node.metadata.name);
        return Ok((StatusCode::CREATED, Json(node)));
    }

    let created = state.storage.create(&key, &node).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<Node>> {
    info!("Getting node: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "nodes")
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("nodes", None, &name);
    let node = state.storage.get(&key).await?;

    Ok(Json(node))
}

pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut node): Json<Node>,
) -> Result<Json<Node>> {
    info!("Updating node: {}", name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "nodes")
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    node.metadata.name = name.clone();

    let key = build_key("nodes", None, &name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!("Dry-run: Node {} validated successfully (not updated)", name);
        return Ok(Json(node));
    }

    let updated = state.storage.update(&key, &node).await?;

    Ok(Json(updated))
}

pub async fn delete_node(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("Deleting node: {}", name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "nodes")
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("nodes", None, &name);

    // If dry-run, skip delete operation
    if is_dry_run {
        info!("Dry-run: Node {} validated successfully (not deleted)", name);
        return Ok(StatusCode::OK);
    }

    // Get the node for finalizer handling
    let node: Node = state.storage.get(&key).await?;

    // Handle deletion with finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &node,
    )
    .await?;

    if deleted_immediately {
        Ok(StatusCode::NO_CONTENT)
    } else {
        info!(
            "Node {} marked for deletion (has finalizers: {:?})",
            name,
            node.metadata.finalizers
        );
        Ok(StatusCode::OK)
    }
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<List<Node>>> {
    info!("Listing nodes");

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "nodes")
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("nodes", None);
    let mut nodes = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut nodes, &params)?;

    let list = List::new("NodeList", "v1", nodes);
    Ok(Json(list))
}

// Use the macro to create a PATCH handler
crate::patch_handler_cluster!(patch, Node, "nodes", "");
