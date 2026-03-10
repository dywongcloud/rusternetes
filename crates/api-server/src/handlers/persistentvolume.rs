use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::PersistentVolume,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::info;

pub async fn create_pv(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Json(mut pv): Json<PersistentVolume>,
) -> Result<(StatusCode, Json<PersistentVolume>)> {
    info!("Creating PersistentVolume: {}", pv.metadata.name);

    // Check authorization (cluster-scoped)
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "persistentvolumes")
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    pv.metadata.ensure_uid();
    pv.metadata.ensure_creation_timestamp();

    let key = build_key("persistentvolumes", None, &pv.metadata.name);
    let created = state.storage.create(&key, &pv).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_pv(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<PersistentVolume>> {
    info!("Getting PersistentVolume: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "get", "persistentvolumes")
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("persistentvolumes", None, &name);
    let pv = state.storage.get(&key).await?;

    Ok(Json(pv))
}

pub async fn list_pvs(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<Vec<PersistentVolume>>> {
    info!("Listing all PersistentVolumes");

    let attrs = RequestAttributes::new(auth_ctx.user, "list", "persistentvolumes")
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("persistentvolumes", None);
    let pvs = state.storage.list(&prefix).await?;

    Ok(Json(pvs))
}

pub async fn update_pv(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Json(mut pv): Json<PersistentVolume>,
) -> Result<Json<PersistentVolume>> {
    info!("Updating PersistentVolume: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "update", "persistentvolumes")
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    pv.metadata.name = name.clone();

    let key = build_key("persistentvolumes", None, &name);
    let updated = state.storage.update(&key, &pv).await?;

    Ok(Json(updated))
}

pub async fn delete_pv(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<StatusCode> {
    info!("Deleting PersistentVolume: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "persistentvolumes")
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("persistentvolumes", None, &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

// Use the macro to create a PATCH handler
crate::patch_handler_cluster!(patch_pv, PersistentVolume, "persistentvolumes", "");
