use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::Node,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::info;

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Json(node): Json<Node>,
) -> Result<(StatusCode, Json<Node>)> {
    info!("Creating node: {}", node.metadata.name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "nodes")
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("nodes", None, &node.metadata.name);
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
    Json(mut node): Json<Node>,
) -> Result<Json<Node>> {
    info!("Updating node: {}", name);

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
    let updated = state.storage.update(&key, &node).await?;

    Ok(Json(updated))
}

pub async fn delete_node(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<StatusCode> {
    info!("Deleting node: {}", name);

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
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<Vec<Node>>> {
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
    let nodes = state.storage.list(&prefix).await?;

    Ok(Json(nodes))
}

// Use the macro to create a PATCH handler
crate::patch_handler_cluster!(patch, Node, "nodes", "");
