use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::{MutatingWebhookConfiguration, ValidatingWebhookConfiguration},
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::info;

// ===== ValidatingWebhookConfiguration Handlers =====

pub async fn create_validating_webhook(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Json(config): Json<ValidatingWebhookConfiguration>,
) -> Result<(StatusCode, Json<ValidatingWebhookConfiguration>)> {
    info!("Creating ValidatingWebhookConfiguration: {}", config.metadata.name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "validatingwebhookconfigurations")
        .with_api_group("admissionregistration.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("validatingwebhookconfigurations", None, &config.metadata.name);
    let created = state.storage.create(&key, &config).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_validating_webhook(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<ValidatingWebhookConfiguration>> {
    info!("Getting ValidatingWebhookConfiguration: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "validatingwebhookconfigurations")
        .with_api_group("admissionregistration.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("validatingwebhookconfigurations", None, &name);
    let config = state.storage.get(&key).await?;

    Ok(Json(config))
}

pub async fn update_validating_webhook(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Json(mut config): Json<ValidatingWebhookConfiguration>,
) -> Result<Json<ValidatingWebhookConfiguration>> {
    info!("Updating ValidatingWebhookConfiguration: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "validatingwebhookconfigurations")
        .with_api_group("admissionregistration.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    config.metadata.name = name.clone();

    let key = build_key("validatingwebhookconfigurations", None, &name);

    let result = match state.storage.update(&key, &config).await {
        Ok(updated) => updated,
        Err(rusternetes_common::Error::NotFound(_)) => {
            state.storage.create(&key, &config).await?
        }
        Err(e) => return Err(e),
    };

    Ok(Json(result))
}

pub async fn delete_validating_webhook(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<StatusCode> {
    info!("Deleting ValidatingWebhookConfiguration: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "validatingwebhookconfigurations")
        .with_api_group("admissionregistration.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("validatingwebhookconfigurations", None, &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_validating_webhooks(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<Vec<ValidatingWebhookConfiguration>>> {
    info!("Listing ValidatingWebhookConfigurations");

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "validatingwebhookconfigurations")
        .with_api_group("admissionregistration.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("validatingwebhookconfigurations", None);
    let configs = state.storage.list(&prefix).await?;

    Ok(Json(configs))
}

// Use the macro to create a PATCH handler
crate::patch_handler_cluster!(patch_validating_webhook, ValidatingWebhookConfiguration, "validatingwebhookconfigurations", "admissionregistration.k8s.io");

// ===== MutatingWebhookConfiguration Handlers =====

pub async fn create_mutating_webhook(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Json(config): Json<MutatingWebhookConfiguration>,
) -> Result<(StatusCode, Json<MutatingWebhookConfiguration>)> {
    info!("Creating MutatingWebhookConfiguration: {}", config.metadata.name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "mutatingwebhookconfigurations")
        .with_api_group("admissionregistration.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("mutatingwebhookconfigurations", None, &config.metadata.name);
    let created = state.storage.create(&key, &config).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_mutating_webhook(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<MutatingWebhookConfiguration>> {
    info!("Getting MutatingWebhookConfiguration: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "mutatingwebhookconfigurations")
        .with_api_group("admissionregistration.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("mutatingwebhookconfigurations", None, &name);
    let config = state.storage.get(&key).await?;

    Ok(Json(config))
}

pub async fn update_mutating_webhook(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Json(mut config): Json<MutatingWebhookConfiguration>,
) -> Result<Json<MutatingWebhookConfiguration>> {
    info!("Updating MutatingWebhookConfiguration: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "mutatingwebhookconfigurations")
        .with_api_group("admissionregistration.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    config.metadata.name = name.clone();

    let key = build_key("mutatingwebhookconfigurations", None, &name);

    let result = match state.storage.update(&key, &config).await {
        Ok(updated) => updated,
        Err(rusternetes_common::Error::NotFound(_)) => {
            state.storage.create(&key, &config).await?
        }
        Err(e) => return Err(e),
    };

    Ok(Json(result))
}

pub async fn delete_mutating_webhook(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<StatusCode> {
    info!("Deleting MutatingWebhookConfiguration: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "mutatingwebhookconfigurations")
        .with_api_group("admissionregistration.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("mutatingwebhookconfigurations", None, &name);
    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_mutating_webhooks(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<Vec<MutatingWebhookConfiguration>>> {
    info!("Listing MutatingWebhookConfigurations");

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "mutatingwebhookconfigurations")
        .with_api_group("admissionregistration.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("mutatingwebhookconfigurations", None);
    let configs = state.storage.list(&prefix).await?;

    Ok(Json(configs))
}

// Use the macro to create a PATCH handler
crate::patch_handler_cluster!(patch_mutating_webhook, MutatingWebhookConfiguration, "mutatingwebhookconfigurations", "admissionregistration.k8s.io");
