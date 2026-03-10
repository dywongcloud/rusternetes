use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::VolumeSnapshot,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::info;

pub async fn create_volumesnapshot(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Json(mut vs): Json<VolumeSnapshot>,
) -> Result<(StatusCode, Json<VolumeSnapshot>)> {
    info!(
        "Creating VolumeSnapshot: {}/{}",
        namespace, vs.metadata.name
    );

    // Check authorization (namespace-scoped)
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "volumesnapshots")
        .with_api_group("snapshot.storage.k8s.io")
        .with_namespace(&namespace);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    vs.metadata.namespace = Some(namespace.clone());
    vs.metadata.ensure_uid();
    vs.metadata.ensure_creation_timestamp();

    let key = build_key("volumesnapshots", Some(&namespace), &vs.metadata.name);
    let created = state.storage.create(&key, &vs).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_volumesnapshot(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<VolumeSnapshot>> {
    info!("Getting VolumeSnapshot: {}/{}", namespace, name);

    let attrs = RequestAttributes::new(auth_ctx.user, "get", "volumesnapshots")
        .with_api_group("snapshot.storage.k8s.io")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("volumesnapshots", Some(&namespace), &name);
    let vs = state.storage.get(&key).await?;

    Ok(Json(vs))
}

pub async fn list_volumesnapshots(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
) -> Result<Json<Vec<VolumeSnapshot>>> {
    info!("Listing VolumeSnapshots in namespace: {}", namespace);

    let attrs = RequestAttributes::new(auth_ctx.user, "list", "volumesnapshots")
        .with_api_group("snapshot.storage.k8s.io")
        .with_namespace(&namespace);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("volumesnapshots", Some(&namespace));
    let vss = state.storage.list(&prefix).await?;

    Ok(Json(vss))
}

pub async fn list_all_volumesnapshots(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<Vec<VolumeSnapshot>>> {
    info!("Listing all VolumeSnapshots");

    let attrs = RequestAttributes::new(auth_ctx.user, "list", "volumesnapshots")
        .with_api_group("snapshot.storage.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("volumesnapshots", None);
    let vss = state.storage.list(&prefix).await?;

    Ok(Json(vss))
}

pub async fn update_volumesnapshot(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Json(mut vs): Json<VolumeSnapshot>,
) -> Result<Json<VolumeSnapshot>> {
    info!("Updating VolumeSnapshot: {}/{}", namespace, name);

    let attrs = RequestAttributes::new(auth_ctx.user, "update", "volumesnapshots")
        .with_api_group("snapshot.storage.k8s.io")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    vs.metadata.name = name.clone();
    vs.metadata.namespace = Some(namespace.clone());

    let key = build_key("volumesnapshots", Some(&namespace), &name);
    let updated = state.storage.update(&key, &vs).await?;

    Ok(Json(updated))
}

pub async fn delete_volumesnapshot(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<StatusCode> {
    info!("Deleting VolumeSnapshot: {}/{}", namespace, name);

    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "volumesnapshots")
        .with_api_group("snapshot.storage.k8s.io")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("volumesnapshots", Some(&namespace), &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

// Use the macro to create a PATCH handler
crate::patch_handler_namespaced!(patch_volumesnapshot, VolumeSnapshot, "volumesnapshots", "snapshot.storage.k8s.io");
