use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::Deployment,
    List,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut deployment): Json<Deployment>,
) -> Result<(StatusCode, Json<Deployment>)> {
    info!(
        "Creating deployment: {}/{}",
        namespace, deployment.metadata.name
    );

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "deployments")
        .with_namespace(&namespace)
        .with_api_group("apps");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    deployment.metadata.namespace = Some(namespace.clone());
    deployment.metadata.ensure_uid();
    deployment.metadata.ensure_creation_timestamp();

    let key = build_key("deployments", Some(&namespace), &deployment.metadata.name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!("Dry-run: Deployment {}/{} validated successfully (not created)", namespace, deployment.metadata.name);
        return Ok((StatusCode::CREATED, Json(deployment)));
    }

    let created = state.storage.create(&key, &deployment).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<Deployment>> {
    info!("Getting deployment: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "deployments")
        .with_namespace(&namespace)
        .with_api_group("apps")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("deployments", Some(&namespace), &name);
    let deployment = state.storage.get(&key).await?;

    Ok(Json(deployment))
}

pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut deployment): Json<Deployment>,
) -> Result<Json<Deployment>> {
    info!("Updating deployment: {}/{}", namespace, name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "deployments")
        .with_namespace(&namespace)
        .with_api_group("apps")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    deployment.metadata.name = name.clone();
    deployment.metadata.namespace = Some(namespace.clone());

    let key = build_key("deployments", Some(&namespace), &name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!("Dry-run: Deployment {}/{} validated successfully (not updated)", namespace, name);
        return Ok(Json(deployment));
    }

    // Try to update first, if not found then create (upsert behavior)
    let result = match state.storage.update(&key, &deployment).await {
        Ok(updated) => updated,
        Err(rusternetes_common::Error::NotFound(_)) => {
            state.storage.create(&key, &deployment).await?
        }
        Err(e) => return Err(e),
    };

    Ok(Json(result))
}

pub async fn delete_deployment(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("Deleting deployment: {}/{}", namespace, name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "deployments")
        .with_namespace(&namespace)
        .with_api_group("apps")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("deployments", Some(&namespace), &name);

    // Get the deployment to check for finalizers
    let deployment: Deployment = state.storage.get(&key).await?;

    // If dry-run, skip delete operation
    if is_dry_run {
        info!("Dry-run: Deployment {}/{} validated successfully (not deleted)", namespace, name);
        return Ok(StatusCode::OK);
    }

    // Handle deletion with finalizers
    // If the deployment has finalizers, it will be marked for deletion (deletionTimestamp set)
    // and remain in storage until controllers remove the finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &deployment,
    )
    .await?;

    if deleted_immediately {
        // Deployment had no finalizers and was deleted immediately
        Ok(StatusCode::NO_CONTENT)
    } else {
        // Deployment has finalizers and was marked for deletion
        info!(
            "Deployment {}/{} marked for deletion (has finalizers: {:?})",
            namespace,
            name,
            deployment.metadata.finalizers
        );
        Ok(StatusCode::OK)
    }
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Result<axum::response::Response> {
    info!("Listing deployments in namespace: {}", namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "deployments")
        .with_namespace(&namespace)
        .with_api_group("apps");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("deployments", Some(&namespace));
    let mut deployments: Vec<Deployment> = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut deployments, &params)?;

    // Get a resource version for consistency
    let resource_version = "1"; // Simplified for now

    // Check if table format is requested
    let accept = headers.get("accept").and_then(|v| v.to_str().ok());
    if crate::handlers::table::wants_table(accept) {
        let table = crate::handlers::table::generic_table(
            deployments,
            Some(resource_version.to_string()),
            "Deployment",
        );
        return Ok(axum::Json(table).into_response());
    }

    let list = List::new("DeploymentList", "apps/v1", deployments);
    Ok(Json(list).into_response())
}

/// List all deployments across all namespaces
pub async fn list_all_deployments(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Result<axum::response::Response> {
    info!("Listing all deployments");

    // Check authorization (cluster-wide list)
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "deployments")
        .with_api_group("apps");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("deployments", None);
    let mut deployments = state.storage.list::<Deployment>(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut deployments, &params)?;

    // Get a resource version for consistency
    let resource_version = "1"; // Simplified for now

    // Check if table format is requested
    let accept = headers.get("accept").and_then(|v| v.to_str().ok());
    if crate::handlers::table::wants_table(accept) {
        let table = crate::handlers::table::generic_table(
            deployments,
            Some(resource_version.to_string()),
            "Deployment",
        );
        return Ok(axum::Json(table).into_response());
    }

    let list = List::new("DeploymentList", "apps/v1", deployments);
    Ok(Json(list).into_response())
}

// Use the macro to create a PATCH handler
crate::patch_handler_namespaced!(patch, Deployment, "deployments", "apps");

pub async fn deletecollection_deployments(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("DeleteCollection deployments in namespace: {} with params: {:?}", namespace, params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "deletecollection", "deployments")
        .with_namespace(&namespace)
        .with_api_group("apps");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: Deployment collection would be deleted (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get all deployments in the namespace
    let prefix = build_prefix("deployments", Some(&namespace));
    let mut items = state.storage.list::<Deployment>(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut items, &params)?;

    // Delete each matching resource
    let mut deleted_count = 0;
    for item in items {
        let key = build_key("deployments", Some(&namespace), &item.metadata.name);

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

    info!("DeleteCollection completed: {} deployments deleted", deleted_count);
    Ok(StatusCode::OK)
}
