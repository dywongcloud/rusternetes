use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::{Service, ServiceType},
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::info;

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Json(mut service): Json<Service>,
) -> Result<(StatusCode, Json<Service>)> {
    info!("Creating service: {}/{}", namespace, service.metadata.name);

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

    // Allocate ClusterIP if needed
    let service_type = service.spec.service_type.as_ref()
        .unwrap_or(&ServiceType::ClusterIP);

    // Only allocate ClusterIP for ClusterIP, NodePort, and LoadBalancer services
    if matches!(service_type, ServiceType::ClusterIP | ServiceType::NodePort | ServiceType::LoadBalancer) {
        // If ClusterIP is not specified or is "None", allocate one
        if service.spec.cluster_ip.is_none() || service.spec.cluster_ip.as_deref() == Some("None") {
            if let Some(allocated_ip) = state.ip_allocator.allocate() {
                info!("Allocated ClusterIP {} for service {}/{}", allocated_ip, namespace, service.metadata.name);
                service.spec.cluster_ip = Some(allocated_ip);
            } else {
                return Err(rusternetes_common::Error::Internal(
                    "Failed to allocate ClusterIP: no IPs available".to_string()
                ));
            }
        } else {
            // User specified a ClusterIP, try to allocate it
            let requested_ip = service.spec.cluster_ip.clone().unwrap();
            if !state.ip_allocator.allocate_specific(requested_ip.clone()) {
                return Err(rusternetes_common::Error::InvalidResource(
                    format!("ClusterIP {} is already allocated or invalid", requested_ip)
                ));
            }
            info!("Allocated specific ClusterIP {} for service {}/{}", requested_ip, namespace, service.metadata.name);
        }
    }

    let key = build_key("services", Some(&namespace), &service.metadata.name);
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
    Json(mut service): Json<Service>,
) -> Result<Json<Service>> {
    info!("Updating service: {}/{}", namespace, name);

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
    let updated = state.storage.update(&key, &service).await?;

    Ok(Json(updated))
}

pub async fn delete_service(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<StatusCode> {
    info!("Deleting service: {}/{}", namespace, name);

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

    // Get the service to release its ClusterIP
    let key = build_key("services", Some(&namespace), &name);
    if let Ok(service) = state.storage.get::<Service>(&key).await {
        if let Some(cluster_ip) = &service.spec.cluster_ip {
            // Release the ClusterIP back to the pool
            state.ip_allocator.release(cluster_ip);
            info!("Released ClusterIP {} from service {}/{}", cluster_ip, namespace, name);
        }
    }

    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
) -> Result<Json<Vec<Service>>> {
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
    let services = state.storage.list(&prefix).await?;

    Ok(Json(services))
}

// Use the macro to create a PATCH handler
crate::patch_handler_namespaced!(patch, Service, "services", "");
