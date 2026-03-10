use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::VolumeSnapshotClass,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::info;

pub async fn create_volumesnapshotclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Json(mut vsc): Json<VolumeSnapshotClass>,
) -> Result<(StatusCode, Json<VolumeSnapshotClass>)> {
    info!("Creating VolumeSnapshotClass: {}", vsc.metadata.name);

    // Check authorization (cluster-scoped)
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "volumesnapshotclasses")
        .with_api_group("snapshot.storage.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    vsc.metadata.ensure_uid();
    vsc.metadata.ensure_creation_timestamp();

    let key = build_key("volumesnapshotclasses", None, &vsc.metadata.name);
    let created = state.storage.create(&key, &vsc).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_volumesnapshotclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<VolumeSnapshotClass>> {
    info!("Getting VolumeSnapshotClass: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "get", "volumesnapshotclasses")
        .with_api_group("snapshot.storage.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("volumesnapshotclasses", None, &name);
    let vsc = state.storage.get(&key).await?;

    Ok(Json(vsc))
}

pub async fn list_volumesnapshotclasses(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<Vec<VolumeSnapshotClass>>> {
    info!("Listing all VolumeSnapshotClasses");

    let attrs = RequestAttributes::new(auth_ctx.user, "list", "volumesnapshotclasses")
        .with_api_group("snapshot.storage.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("volumesnapshotclasses", None);
    let vscs = state.storage.list(&prefix).await?;

    Ok(Json(vscs))
}

pub async fn update_volumesnapshotclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Json(mut vsc): Json<VolumeSnapshotClass>,
) -> Result<Json<VolumeSnapshotClass>> {
    info!("Updating VolumeSnapshotClass: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "update", "volumesnapshotclasses")
        .with_api_group("snapshot.storage.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    vsc.metadata.name = name.clone();

    let key = build_key("volumesnapshotclasses", None, &name);
    let updated = state.storage.update(&key, &vsc).await?;

    Ok(Json(updated))
}

pub async fn delete_volumesnapshotclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<StatusCode> {
    info!("Deleting VolumeSnapshotClass: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "volumesnapshotclasses")
        .with_api_group("snapshot.storage.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("volumesnapshotclasses", None, &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}
