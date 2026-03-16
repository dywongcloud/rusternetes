use crate::{handlers::watch::WatchParams, middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::{Service, ServiceType},
    List, Result,
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
    Json(mut service): Json<Service>,
) -> Result<(StatusCode, Json<Service>)> {
    info!("Creating service: {}/{}", namespace, service.metadata.name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "services")
        .with_namespace(&namespace)
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    service.metadata.namespace = Some(namespace.clone());

    // Enrich metadata with system fields
    service.metadata.ensure_uid();
    service.metadata.ensure_creation_timestamp();

    // Allocate ClusterIP if needed
    let service_type = service
        .spec
        .service_type
        .as_ref()
        .unwrap_or(&ServiceType::ClusterIP);

    // Validate ExternalName services
    if matches!(service_type, ServiceType::ExternalName) {
        // ExternalName services must have an externalName field
        if service.spec.external_name.is_none() {
            return Err(rusternetes_common::Error::InvalidResource(
                "ExternalName service must have spec.externalName set".to_string(),
            ));
        }
        // ExternalName services cannot have a ClusterIP
        if service.spec.cluster_ip.is_some() && service.spec.cluster_ip.as_deref() != Some("None") {
            return Err(rusternetes_common::Error::InvalidResource(
                "ExternalName service cannot have a ClusterIP".to_string(),
            ));
        }
        // ExternalName services don't need ClusterIP allocation
        service.spec.cluster_ip = Some("None".to_string());
    } else {
        // Only allocate ClusterIP for ClusterIP, NodePort, and LoadBalancer services
        if matches!(
            service_type,
            ServiceType::ClusterIP | ServiceType::NodePort | ServiceType::LoadBalancer
        ) {
            // If ClusterIP is not specified or is "None", allocate one
            if service.spec.cluster_ip.is_none()
                || service.spec.cluster_ip.as_deref() == Some("None")
            {
                if let Some(allocated_ip) = state.ip_allocator.allocate() {
                    info!(
                        "Allocated ClusterIP {} for service {}/{}",
                        allocated_ip, namespace, service.metadata.name
                    );
                    service.spec.cluster_ip = Some(allocated_ip);
                } else {
                    return Err(rusternetes_common::Error::Internal(
                        "Failed to allocate ClusterIP: no IPs available".to_string(),
                    ));
                }
            } else {
                // User specified a ClusterIP, try to allocate it
                let requested_ip = service.spec.cluster_ip.clone().unwrap();
                if !state.ip_allocator.allocate_specific(requested_ip.clone()) {
                    return Err(rusternetes_common::Error::InvalidResource(format!(
                        "ClusterIP {} is already allocated or invalid",
                        requested_ip
                    )));
                }
                info!(
                    "Allocated specific ClusterIP {} for service {}/{}",
                    requested_ip, namespace, service.metadata.name
                );
            }
        }
    }

    let key = build_key("services", Some(&namespace), &service.metadata.name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!(
            "Dry-run: Service {}/{} validated successfully (not created)",
            namespace, service.metadata.name
        );
        return Ok((StatusCode::CREATED, Json(service)));
    }

    let created = state.storage.create(&key, &service).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<Service>> {
    info!("Getting service: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "services")
        .with_namespace(&namespace)
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("services", Some(&namespace), &name);
    let service = state.storage.get(&key).await?;

    Ok(Json(service))
}

pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut service): Json<Service>,
) -> Result<Json<Service>> {
    info!("Updating service: {}/{}", namespace, name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "services")
        .with_namespace(&namespace)
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    service.metadata.name = name.clone();
    service.metadata.namespace = Some(namespace.clone());

    let key = build_key("services", Some(&namespace), &name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!(
            "Dry-run: Service {}/{} validated successfully (not updated)",
            namespace, name
        );
        return Ok(Json(service));
    }

    let updated = state.storage.update(&key, &service).await?;

    Ok(Json(updated))
}

pub async fn delete_service(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("Deleting service: {}/{}", namespace, name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "services")
        .with_namespace(&namespace)
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Get the service to validate it exists and potentially release its ClusterIP
    let key = build_key("services", Some(&namespace), &name);
    let service = state.storage.get::<Service>(&key).await?;

    // If dry-run, skip delete operation
    if is_dry_run {
        info!(
            "Dry-run: Service {}/{} validated successfully (not deleted)",
            namespace, name
        );
        return Ok(StatusCode::OK);
    }

    // Handle deletion with finalizers
    let deleted_immediately =
        !crate::handlers::finalizers::handle_delete_with_finalizers(&state.storage, &key, &service)
            .await?;

    if deleted_immediately {
        // Resource had no finalizers and was deleted immediately
        // Release the ClusterIP back to the pool
        if let Some(cluster_ip) = &service.spec.cluster_ip {
            state.ip_allocator.release(cluster_ip);
            info!(
                "Released ClusterIP {} from service {}/{}",
                cluster_ip, namespace, name
            );
        }
        Ok(StatusCode::NO_CONTENT)
    } else {
        // Resource has finalizers and was marked for deletion
        info!(
            "Service {}/{} marked for deletion (has finalizers: {:?})",
            namespace, name, service.metadata.finalizers
        );
        Ok(StatusCode::OK)
    }
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    headers: HeaderMap,
    Query(params): Query<WatchParams>,
) -> Result<Response> {
    // Check if this is a watch request
    if params.watch.unwrap_or(false) {
        info!("Watch request for services in namespace: {}", namespace);
        return crate::handlers::watch::watch_services(
            State(state),
            Extension(auth_ctx),
            Path(namespace),
            Query(params),
        )
        .await;
    }

    info!("Listing services in namespace: {}", namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "services")
        .with_namespace(&namespace)
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("services", Some(&namespace));
    let mut services = state.storage.list::<Service>(&prefix).await?;

    // Apply field and label selector filtering
    let mut params_map = HashMap::new();
    if let Some(fs) = params.field_selector {
        params_map.insert("fieldSelector".to_string(), fs);
    }
    if let Some(ls) = params.label_selector {
        params_map.insert("labelSelector".to_string(), ls);
    }
    crate::handlers::filtering::apply_selectors(&mut services, &params_map)?;

    // Get a resource version for consistency
    let resource_version = "1"; // Simplified for now

    // Check if table format is requested
    let accept = headers.get("accept").and_then(|v| v.to_str().ok());
    if crate::handlers::table::wants_table(accept) {
        let table = crate::handlers::table::generic_table(
            services,
            Some(resource_version.to_string()),
            "Service",
        );
        return Ok(Json(table).into_response());
    }

    // Wrap in proper List object
    let list = List::new("ServiceList", "v1", services);
    Ok(Json(list).into_response())
}

/// List all services across all namespaces
pub async fn list_all_services(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    headers: HeaderMap,
    Query(params): Query<WatchParams>,
) -> Result<Response> {
    // Debug log the params
    info!("list_all_services called with watch={:?}", params.watch);

    // Check if this is a watch request
    if params.watch.unwrap_or(false) {
        info!("Watch request for all services");
        return crate::handlers::watch::watch_cluster_scoped::<Service>(
            state, auth_ctx, "services", "", params,
        )
        .await;
    }

    info!("Listing all services");

    // Check authorization (cluster-wide list)
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "services").with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("services", None);
    let mut services = state.storage.list::<Service>(&prefix).await?;

    // Apply field and label selector filtering
    let mut params_map = HashMap::new();
    if let Some(fs) = params.field_selector {
        params_map.insert("fieldSelector".to_string(), fs);
    }
    if let Some(ls) = params.label_selector {
        params_map.insert("labelSelector".to_string(), ls);
    }
    crate::handlers::filtering::apply_selectors(&mut services, &params_map)?;

    // Get a resource version for consistency
    let resource_version = "1"; // Simplified for now

    // Check if table format is requested
    let accept = headers.get("accept").and_then(|v| v.to_str().ok());
    if crate::handlers::table::wants_table(accept) {
        let table = crate::handlers::table::generic_table(
            services,
            Some(resource_version.to_string()),
            "Service",
        );
        return Ok(Json(table).into_response());
    }

    let list = List::new("ServiceList", "v1", services);
    Ok(Json(list).into_response())
}

// Use the macro to create a PATCH handler
crate::patch_handler_namespaced!(patch, Service, "services", "");

pub async fn deletecollection_services(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<StatusCode> {
    info!(
        "DeleteCollection services in namespace: {} with params: {:?}",
        namespace, params
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "deletecollection", "services")
        .with_namespace(&namespace)
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: Service collection would be deleted (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get all services in the namespace
    let prefix = build_prefix("services", Some(&namespace));
    let mut items = state.storage.list::<Service>(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut items, &params)?;

    // Delete each matching resource
    let mut deleted_count = 0;
    for item in items {
        let key = build_key("services", Some(&namespace), &item.metadata.name);

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
        "DeleteCollection completed: {} services deleted",
        deleted_count
    );
    Ok(StatusCode::OK)
}
