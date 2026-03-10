use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::Ingress,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::info;

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Json(mut ingress): Json<Ingress>,
) -> Result<(StatusCode, Json<Ingress>)> {
    info!(
        "Creating ingress: {}/{}",
        namespace, ingress.metadata.name
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "ingresses")
        .with_namespace(&namespace)
        .with_api_group("networking.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    ingress.metadata.namespace = Some(namespace.clone());
    ingress.metadata.ensure_uid();
    ingress.metadata.ensure_creation_timestamp();

    let key = build_key("ingresses", Some(&namespace), &ingress.metadata.name);
    let created = state.storage.create(&key, &ingress).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<Ingress>> {
    info!("Getting ingress: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "ingresses")
        .with_namespace(&namespace)
        .with_api_group("networking.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("ingresses", Some(&namespace), &name);
    let ingress = state.storage.get(&key).await?;

    Ok(Json(ingress))
}

pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Json(mut ingress): Json<Ingress>,
) -> Result<Json<Ingress>> {
    info!("Updating ingress: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "ingresses")
        .with_namespace(&namespace)
        .with_api_group("networking.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    ingress.metadata.name = name.clone();
    ingress.metadata.namespace = Some(namespace.clone());

    let key = build_key("ingresses", Some(&namespace), &name);

    // Try to update first, if not found then create (upsert behavior)
    let result = match state.storage.update(&key, &ingress).await {
        Ok(updated) => updated,
        Err(rusternetes_common::Error::NotFound(_)) => {
            state.storage.create(&key, &ingress).await?
        }
        Err(e) => return Err(e),
    };

    Ok(Json(result))
}

pub async fn delete_ingress(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<StatusCode> {
    info!("Deleting ingress: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "ingresses")
        .with_namespace(&namespace)
        .with_api_group("networking.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("ingresses", Some(&namespace), &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
) -> Result<Json<Vec<Ingress>>> {
    info!("Listing ingresses in namespace: {}", namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "ingresses")
        .with_namespace(&namespace)
        .with_api_group("networking.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("ingresses", Some(&namespace));
    let ingresses = state.storage.list(&prefix).await?;

    Ok(Json(ingresses))
}
