use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::VolumeSnapshotContent,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::info;

pub async fn create_volumesnapshotcontent(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Json(mut vsc): Json<VolumeSnapshotContent>,
) -> Result<(StatusCode, Json<VolumeSnapshotContent>)> {
    info!("Creating VolumeSnapshotContent: {}", vsc.metadata.name);

    // Check authorization (cluster-scoped)
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "volumesnapshotcontents")
        .with_api_group("snapshot.storage.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    vsc.metadata.ensure_uid();
    vsc.metadata.ensure_creation_timestamp();

    let key = build_key("volumesnapshotcontents", None, &vsc.metadata.name);
    let created = state.storage.create(&key, &vsc).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_volumesnapshotcontent(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<VolumeSnapshotContent>> {
    info!("Getting VolumeSnapshotContent: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "get", "volumesnapshotcontents")
        .with_api_group("snapshot.storage.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("volumesnapshotcontents", None, &name);
    let vsc = state.storage.get(&key).await?;

    Ok(Json(vsc))
}

pub async fn list_volumesnapshotcontents(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<Vec<VolumeSnapshotContent>>> {
    info!("Listing all VolumeSnapshotContents");

    let attrs = RequestAttributes::new(auth_ctx.user, "list", "volumesnapshotcontents")
        .with_api_group("snapshot.storage.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("volumesnapshotcontents", None);
    let vscs = state.storage.list(&prefix).await?;

    Ok(Json(vscs))
}

pub async fn update_volumesnapshotcontent(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Json(mut vsc): Json<VolumeSnapshotContent>,
) -> Result<Json<VolumeSnapshotContent>> {
    info!("Updating VolumeSnapshotContent: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "update", "volumesnapshotcontents")
        .with_api_group("snapshot.storage.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    vsc.metadata.name = name.clone();

    let key = build_key("volumesnapshotcontents", None, &name);
    let updated = state.storage.update(&key, &vsc).await?;

    Ok(Json(updated))
}

pub async fn delete_volumesnapshotcontent(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<StatusCode> {
    info!("Deleting VolumeSnapshotContent: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "volumesnapshotcontents")
        .with_api_group("snapshot.storage.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("volumesnapshotcontents", None, &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}
