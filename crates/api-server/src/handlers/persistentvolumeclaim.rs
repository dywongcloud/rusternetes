use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::PersistentVolumeClaim,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::info;

pub async fn create_pvc(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Json(mut pvc): Json<PersistentVolumeClaim>,
) -> Result<(StatusCode, Json<PersistentVolumeClaim>)> {
    info!("Creating PersistentVolumeClaim: {}/{}", namespace, pvc.metadata.name);

    let attrs = RequestAttributes::new(auth_ctx.user, "create", "persistentvolumeclaims")
        .with_namespace(&namespace)
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    pvc.metadata.namespace = Some(namespace.clone());
    pvc.metadata.ensure_uid();
    pvc.metadata.ensure_creation_timestamp();

    let key = build_key("persistentvolumeclaims", Some(&namespace), &pvc.metadata.name);
    let created = state.storage.create(&key, &pvc).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_pvc(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<PersistentVolumeClaim>> {
    info!("Getting PersistentVolumeClaim: {}/{}", namespace, name);

    let attrs = RequestAttributes::new(auth_ctx.user, "get", "persistentvolumeclaims")
        .with_namespace(&namespace)
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("persistentvolumeclaims", Some(&namespace), &name);
    let pvc = state.storage.get(&key).await?;

    Ok(Json(pvc))
}

pub async fn list_pvcs(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
) -> Result<Json<Vec<PersistentVolumeClaim>>> {
    info!("Listing PersistentVolumeClaims in namespace: {}", namespace);

    let attrs = RequestAttributes::new(auth_ctx.user, "list", "persistentvolumeclaims")
        .with_namespace(&namespace)
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("persistentvolumeclaims", Some(&namespace));
    let pvcs = state.storage.list(&prefix).await?;

    Ok(Json(pvcs))
}

pub async fn update_pvc(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Json(mut pvc): Json<PersistentVolumeClaim>,
) -> Result<Json<PersistentVolumeClaim>> {
    info!("Updating PersistentVolumeClaim: {}/{}", namespace, name);

    let attrs = RequestAttributes::new(auth_ctx.user, "update", "persistentvolumeclaims")
        .with_namespace(&namespace)
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    pvc.metadata.name = name.clone();
    pvc.metadata.namespace = Some(namespace.clone());

    let key = build_key("persistentvolumeclaims", Some(&namespace), &name);
    let updated = state.storage.update(&key, &pvc).await?;

    Ok(Json(updated))
}

pub async fn delete_pvc(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<StatusCode> {
    info!("Deleting PersistentVolumeClaim: {}/{}", namespace, name);

    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "persistentvolumeclaims")
        .with_namespace(&namespace)
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("persistentvolumeclaims", Some(&namespace), &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}
