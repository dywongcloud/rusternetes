use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::StorageClass,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::info;

pub async fn create_storageclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
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
) -> Result<Json<Vec<StorageClass>>> {
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
    let scs = state.storage.list(&prefix).await?;

    Ok(Json(scs))
}

pub async fn update_storageclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
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

    let key = build_key("storageclasses", None, &name);
    let updated = state.storage.update(&key, &sc).await?;

    Ok(Json(updated))
}

pub async fn delete_storageclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
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
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}
