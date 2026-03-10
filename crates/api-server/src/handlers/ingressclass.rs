use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::IngressClass,
    List,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

pub async fn create_ingressclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut ingress_class): Json<IngressClass>,
) -> Result<(StatusCode, Json<IngressClass>)> {
    info!("Creating IngressClass: {}", ingress_class.metadata.name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "ingressclasses")
        .with_api_group("networking.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Enrich metadata with system fields
    ingress_class.metadata.ensure_uid();
    ingress_class.metadata.ensure_creation_timestamp();

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: IngressClass validated successfully (not created)");
        return Ok((StatusCode::CREATED, Json(ingress_class)));
    }

    let key = build_key("ingressclasses", None, &ingress_class.metadata.name);
    let created = state.storage.create(&key, &ingress_class).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_ingressclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<IngressClass>> {
    info!("Getting IngressClass: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "ingressclasses")
        .with_api_group("networking.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("ingressclasses", None, &name);
    let ingress_class = state.storage.get(&key).await?;

    Ok(Json(ingress_class))
}

pub async fn update_ingressclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut ingress_class): Json<IngressClass>,
) -> Result<Json<IngressClass>> {
    info!("Updating IngressClass: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "ingressclasses")
        .with_api_group("networking.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    ingress_class.metadata.name = name.clone();

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: IngressClass validated successfully (not updated)");
        return Ok(Json(ingress_class));
    }

    let key = build_key("ingressclasses", None, &name);

    // Try to update first, if not found then create (upsert behavior)
    let result = match state.storage.update(&key, &ingress_class).await {
        Ok(updated) => updated,
        Err(rusternetes_common::Error::NotFound(_)) => {
            state.storage.create(&key, &ingress_class).await?
        }
        Err(e) => return Err(e),
    };

    Ok(Json(result))
}

pub async fn delete_ingressclass(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("Deleting IngressClass: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "ingressclasses")
        .with_api_group("networking.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("ingressclasses", None, &name);

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: IngressClass validated successfully (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get the resource for finalizer handling
    let ingress_class: IngressClass = state.storage.get(&key).await?;

    // Handle deletion with finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &ingress_class,
    )
    .await?;

    if deleted_immediately {
        Ok(StatusCode::NO_CONTENT)
    } else {
        info!(
            "IngressClass marked for deletion (has finalizers: {:?})",
            ingress_class.metadata.finalizers
        );
        Ok(StatusCode::OK)
    }
}

pub async fn list_ingressclasses(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<List<IngressClass>>> {
    info!("Listing IngressClasses");

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "ingressclasses")
        .with_api_group("networking.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("ingressclasses", None);
    let ingress_classes = state.storage.list(&prefix).await?;

    let list = List::new("IngressClassList", "networking.k8s.io/v1", ingress_classes);
    Ok(Json(list))
}

// Use the macro to create a PATCH handler for cluster-scoped IngressClass
crate::patch_handler_cluster!(patch_ingressclass, IngressClass, "ingressclasses", "networking.k8s.io");
