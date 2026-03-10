use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::ServiceAccount,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::info;

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Json(mut service_account): Json<ServiceAccount>,
) -> Result<(StatusCode, Json<ServiceAccount>)> {
    info!(
        "Creating service account: {}/{}",
        namespace, service_account.metadata.name
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "serviceaccounts")
        .with_namespace(&namespace)
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    service_account.metadata.namespace = Some(namespace.clone());

    let key = build_key("serviceaccounts", Some(&namespace), &service_account.metadata.name);
    let created = state.storage.create(&key, &service_account).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<ServiceAccount>> {
    info!("Getting service account: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "serviceaccounts")
        .with_namespace(&namespace)
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("serviceaccounts", Some(&namespace), &name);
    let service_account = state.storage.get(&key).await?;

    Ok(Json(service_account))
}

pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Json(mut service_account): Json<ServiceAccount>,
) -> Result<Json<ServiceAccount>> {
    info!("Updating service account: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "serviceaccounts")
        .with_namespace(&namespace)
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    service_account.metadata.name = name.clone();
    service_account.metadata.namespace = Some(namespace.clone());

    let key = build_key("serviceaccounts", Some(&namespace), &name);
    let updated = state.storage.update(&key, &service_account).await?;

    Ok(Json(updated))
}

pub async fn delete_service_account(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<StatusCode> {
    info!("Deleting service account: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "serviceaccounts")
        .with_namespace(&namespace)
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("serviceaccounts", Some(&namespace), &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
) -> Result<Json<Vec<ServiceAccount>>> {
    info!("Listing service accounts in namespace: {}", namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "serviceaccounts")
        .with_namespace(&namespace)
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("serviceaccounts", Some(&namespace));
    let service_accounts = state.storage.list(&prefix).await?;

    Ok(Json(service_accounts))
}

// Use the macro to create a PATCH handler
crate::patch_handler_namespaced!(patch, ServiceAccount, "serviceaccounts", "");
