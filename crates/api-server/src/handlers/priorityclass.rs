use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::PriorityClass,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::info;

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Json(priority_class): Json<PriorityClass>,
) -> Result<(StatusCode, Json<PriorityClass>)> {
    info!("Creating PriorityClass: {}", priority_class.metadata.name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "priorityclasses")
        .with_api_group("scheduling.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("priorityclasses", None, &priority_class.metadata.name);
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
    Json(mut priority_class): Json<PriorityClass>,
) -> Result<Json<PriorityClass>> {
    info!("Updating PriorityClass: {}", name);

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
) -> Result<StatusCode> {
    info!("Deleting PriorityClass: {}", name);

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
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<Vec<PriorityClass>>> {
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
    let priority_classes = state.storage.list(&prefix).await?;

    Ok(Json(priority_classes))
}

// Use the macro to create a PATCH handler
crate::patch_handler_cluster!(patch, PriorityClass, "priorityclasses", "scheduling.k8s.io");
