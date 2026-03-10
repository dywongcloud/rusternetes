use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::StatefulSet,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::info;

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Json(mut statefulset): Json<StatefulSet>,
) -> Result<(StatusCode, Json<StatefulSet>)> {
    info!(
        "Creating statefulset: {}/{}",
        namespace, statefulset.metadata.name
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "statefulsets")
        .with_namespace(&namespace)
        .with_api_group("apps");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    statefulset.metadata.namespace = Some(namespace.clone());
    statefulset.metadata.ensure_uid();
    statefulset.metadata.ensure_creation_timestamp();

    let key = build_key("statefulsets", Some(&namespace), &statefulset.metadata.name);
    let created = state.storage.create(&key, &statefulset).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<StatefulSet>> {
    info!("Getting statefulset: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "statefulsets")
        .with_namespace(&namespace)
        .with_api_group("apps")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("statefulsets", Some(&namespace), &name);
    let statefulset = state.storage.get(&key).await?;

    Ok(Json(statefulset))
}

pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Json(mut statefulset): Json<StatefulSet>,
) -> Result<Json<StatefulSet>> {
    info!("Updating statefulset: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "statefulsets")
        .with_namespace(&namespace)
        .with_api_group("apps")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    statefulset.metadata.name = name.clone();
    statefulset.metadata.namespace = Some(namespace.clone());

    let key = build_key("statefulsets", Some(&namespace), &name);

    // Try to update first, if not found then create (upsert behavior)
    let result = match state.storage.update(&key, &statefulset).await {
        Ok(updated) => updated,
        Err(rusternetes_common::Error::NotFound(_)) => {
            state.storage.create(&key, &statefulset).await?
        }
        Err(e) => return Err(e),
    };

    Ok(Json(result))
}

pub async fn delete_statefulset(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<StatusCode> {
    info!("Deleting statefulset: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "statefulsets")
        .with_namespace(&namespace)
        .with_api_group("apps")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("statefulsets", Some(&namespace), &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
) -> Result<Json<Vec<StatefulSet>>> {
    info!("Listing statefulsets in namespace: {}", namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "statefulsets")
        .with_namespace(&namespace)
        .with_api_group("apps");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("statefulsets", Some(&namespace));
    let statefulsets = state.storage.list(&prefix).await?;

    Ok(Json(statefulsets))
}
