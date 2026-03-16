use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::{MutatingWebhookConfiguration, ValidatingWebhookConfiguration},
    List, Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

// ===== ValidatingWebhookConfiguration Handlers =====

pub async fn create_validating_webhook(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut config): Json<ValidatingWebhookConfiguration>,
) -> Result<(StatusCode, Json<ValidatingWebhookConfiguration>)> {
    info!(
        "Creating ValidatingWebhookConfiguration: {}",
        config.metadata.name
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "validatingwebhookconfigurations")
        .with_api_group("admissionregistration.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Enrich metadata with system fields
    config.metadata.ensure_uid();
    config.metadata.ensure_creation_timestamp();

    // Check for dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: ValidatingWebhookConfiguration validated successfully (not created)");
        return Ok((StatusCode::CREATED, Json(config)));
    }

    let key = build_key(
        "validatingwebhookconfigurations",
        None,
        &config.metadata.name,
    );
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
    Query(params): Query<HashMap<String, String>>,
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

    // Check for dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: ValidatingWebhookConfiguration validated successfully (not updated)");
        return Ok(Json(config));
    }

    let key = build_key("validatingwebhookconfigurations", None, &name);

    let result = match state.storage.update(&key, &config).await {
        Ok(updated) => updated,
        Err(rusternetes_common::Error::NotFound(_)) => state.storage.create(&key, &config).await?,
        Err(e) => return Err(e),
    };

    Ok(Json(result))
}

pub async fn delete_validating_webhook(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
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

    // Check for dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: ValidatingWebhookConfiguration validated successfully (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get the resource for finalizer handling
    let resource: ValidatingWebhookConfiguration = state.storage.get(&key).await?;

    // Handle deletion with finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &resource,
    )
    .await?;

    if deleted_immediately {
        Ok(StatusCode::NO_CONTENT)
    } else {
        info!(
            "ValidatingWebhookConfiguration marked for deletion (has finalizers: {:?})",
            resource.metadata.finalizers
        );
        Ok(StatusCode::OK)
    }
}

pub async fn list_validating_webhooks(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<List<ValidatingWebhookConfiguration>>> {
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
    let mut configs = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut configs, &params)?;

    let list = List::new(
        "ValidatingWebhookConfigurationList",
        "admissionregistration.k8s.io/v1",
        configs,
    );
    Ok(Json(list))
}

// Use the macro to create a PATCH handler
crate::patch_handler_cluster!(
    patch_validating_webhook,
    ValidatingWebhookConfiguration,
    "validatingwebhookconfigurations",
    "admissionregistration.k8s.io"
);

// ===== MutatingWebhookConfiguration Handlers =====

pub async fn create_mutating_webhook(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut config): Json<MutatingWebhookConfiguration>,
) -> Result<(StatusCode, Json<MutatingWebhookConfiguration>)> {
    info!(
        "Creating MutatingWebhookConfiguration: {}",
        config.metadata.name
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "mutatingwebhookconfigurations")
        .with_api_group("admissionregistration.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Enrich metadata with system fields
    config.metadata.ensure_uid();
    config.metadata.ensure_creation_timestamp();

    // Check for dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: MutatingWebhookConfiguration validated successfully (not created)");
        return Ok((StatusCode::CREATED, Json(config)));
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
    Query(params): Query<HashMap<String, String>>,
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

    // Check for dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: MutatingWebhookConfiguration validated successfully (not updated)");
        return Ok(Json(config));
    }

    let key = build_key("mutatingwebhookconfigurations", None, &name);

    let result = match state.storage.update(&key, &config).await {
        Ok(updated) => updated,
        Err(rusternetes_common::Error::NotFound(_)) => state.storage.create(&key, &config).await?,
        Err(e) => return Err(e),
    };

    Ok(Json(result))
}

pub async fn delete_mutating_webhook(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
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

    // Check for dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: MutatingWebhookConfiguration validated successfully (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get the resource for finalizer handling
    let resource: MutatingWebhookConfiguration = state.storage.get(&key).await?;

    // Handle deletion with finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &resource,
    )
    .await?;

    if deleted_immediately {
        Ok(StatusCode::NO_CONTENT)
    } else {
        info!(
            "MutatingWebhookConfiguration marked for deletion (has finalizers: {:?})",
            resource.metadata.finalizers
        );
        Ok(StatusCode::OK)
    }
}

pub async fn list_mutating_webhooks(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<List<MutatingWebhookConfiguration>>> {
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
    let mut configs = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut configs, &params)?;

    let list = List::new(
        "MutatingWebhookConfigurationList",
        "admissionregistration.k8s.io/v1",
        configs,
    );
    Ok(Json(list))
}

// Use the macro to create a PATCH handler
crate::patch_handler_cluster!(
    patch_mutating_webhook,
    MutatingWebhookConfiguration,
    "mutatingwebhookconfigurations",
    "admissionregistration.k8s.io"
);

pub async fn deletecollection_validatingwebhookconfigurations(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<StatusCode> {
    info!(
        "DeleteCollection validatingwebhookconfigurations with params: {:?}",
        params
    );

    // Check authorization
    let attrs = RequestAttributes::new(
        auth_ctx.user,
        "deletecollection",
        "validatingwebhookconfigurations",
    )
    .with_api_group("admissionregistration.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: ValidatingWebhookConfiguration collection would be deleted (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get all validatingwebhookconfigurations
    let prefix = build_prefix("validatingwebhookconfigurations", None);
    let mut items = state
        .storage
        .list::<ValidatingWebhookConfiguration>(&prefix)
        .await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut items, &params)?;

    // Delete each matching resource
    let mut deleted_count = 0;
    for item in items {
        let key = build_key("validatingwebhookconfigurations", None, &item.metadata.name);

        // Handle deletion with finalizers
        let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
            &state.storage,
            &key,
            &item,
        )
        .await?;

        if deleted_immediately {
            deleted_count += 1;
        }
    }

    info!(
        "DeleteCollection completed: {} validatingwebhookconfigurations deleted",
        deleted_count
    );
    Ok(StatusCode::OK)
}

pub async fn deletecollection_mutatingwebhookconfigurations(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<StatusCode> {
    info!(
        "DeleteCollection mutatingwebhookconfigurations with params: {:?}",
        params
    );

    // Check authorization
    let attrs = RequestAttributes::new(
        auth_ctx.user,
        "deletecollection",
        "mutatingwebhookconfigurations",
    )
    .with_api_group("admissionregistration.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: MutatingWebhookConfiguration collection would be deleted (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get all mutatingwebhookconfigurations
    let prefix = build_prefix("mutatingwebhookconfigurations", None);
    let mut items = state
        .storage
        .list::<MutatingWebhookConfiguration>(&prefix)
        .await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut items, &params)?;

    // Delete each matching resource
    let mut deleted_count = 0;
    for item in items {
        let key = build_key("mutatingwebhookconfigurations", None, &item.metadata.name);

        // Handle deletion with finalizers
        let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
            &state.storage,
            &key,
            &item,
        )
        .await?;

        if deleted_immediately {
            deleted_count += 1;
        }
    }

    info!(
        "DeleteCollection completed: {} mutatingwebhookconfigurations deleted",
        deleted_count
    );
    Ok(StatusCode::OK)
}
