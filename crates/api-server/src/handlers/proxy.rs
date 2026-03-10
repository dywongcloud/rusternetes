/// HTTP proxy handlers for nodes, services, and pods
///
/// These handlers proxy HTTP requests to the target resource for debugging and monitoring.
///
/// Kubernetes proxy subresources:
/// - `/api/v1/nodes/{name}/proxy/{path}` - Proxy to node
/// - `/api/v1/namespaces/{ns}/services/{name}/proxy/{path}` - Proxy to service
/// - `/api/v1/namespaces/{ns}/pods/{name}/proxy/{path}` - Proxy to pod

use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, Method, StatusCode},
    response::Response,
    Extension,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    Result,
};
use rusternetes_storage::Storage;
use std::{collections::HashMap, sync::Arc};
use tracing::{info, warn};

/// Proxy HTTP requests to a node
///
/// Proxies requests to the kubelet API on the specified node for debugging and metrics.
pub async fn proxy_node(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((node_name, path)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
    body: Body,
) -> Result<Response> {
    info!("Proxying request to node: {}, path: {}", node_name, path);

    // Check authorization - requires permission to proxy to nodes
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "nodes/proxy")
        .with_api_group("")
        .with_name(&node_name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Get the node to find its address
    let node_key = format!("/api/v1/nodes/{}", node_name);
    let node: rusternetes_common::resources::Node = state.storage.get(&node_key).await?;

    // Extract node address (prefer InternalIP, fallback to ExternalIP)
    let node_address = node
        .status
        .as_ref()
        .and_then(|s| s.addresses.as_ref())
        .and_then(|addrs| {
            addrs
                .iter()
                .find(|a| a.address_type == "InternalIP")
                .or_else(|| addrs.iter().find(|a| a.address_type == "ExternalIP"))
        })
        .map(|a| a.address.clone())
        .ok_or_else(|| {
            rusternetes_common::Error::NotFound(format!(
                "No address found for node {}",
                node_name
            ))
        })?;

    // Build target URL (kubelet typically runs on port 10250)
    let kubelet_port = 10250;
    let target_url = format!("https://{}:{}/{}", node_address, kubelet_port, path);

    // Forward the request to the kubelet
    proxy_request(target_url, method, headers, params, body).await
}

/// Proxy HTTP requests to a service
///
/// Proxies requests to a service endpoint for debugging.
pub async fn proxy_service(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, service_name, path)): Path<(String, String, String)>,
    method: Method,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
    body: Body,
) -> Result<Response> {
    info!(
        "Proxying request to service: {}/{}, path: {}",
        namespace, service_name, path
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "services/proxy")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(&service_name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Get the service to find its ClusterIP and port
    let service_key = format!("/api/v1/namespaces/{}/services/{}", namespace, service_name);
    let service: rusternetes_common::resources::Service = state.storage.get(&service_key).await?;

    // Get ClusterIP and first port
    let cluster_ip = service
        .spec
        .cluster_ip
        .ok_or_else(|| {
            rusternetes_common::Error::InvalidResource(format!(
                "Service {}/{} has no ClusterIP",
                namespace, service_name
            ))
        })?;

    let port = service
        .spec
        .ports
        .first()
        .map(|p| p.port)
        .unwrap_or(80);

    // Build target URL
    let target_url = format!("http://{}:{}/{}", cluster_ip, port, path);

    // Forward the request
    proxy_request(target_url, method, headers, params, body).await
}

/// Proxy HTTP requests to a pod
///
/// Proxies requests to a specific pod for debugging.
pub async fn proxy_pod(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, pod_name, path)): Path<(String, String, String)>,
    method: Method,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
    body: Body,
) -> Result<Response> {
    info!(
        "Proxying request to pod: {}/{}, path: {}",
        namespace, pod_name, path
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "pods/proxy")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(&pod_name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Get the pod to find its IP
    let pod_key = format!("/api/v1/namespaces/{}/pods/{}", namespace, pod_name);
    let pod: rusternetes_common::resources::Pod = state.storage.get(&pod_key).await?;

    // Get pod IP
    let pod_ip = pod
        .status
        .as_ref()
        .and_then(|s| s.pod_ip.clone())
        .ok_or_else(|| {
            rusternetes_common::Error::NotFound(format!(
                "Pod {}/{} has no IP address yet",
                namespace, pod_name
            ))
        })?;

    // Default to port 80 or first container port
    let port = pod
        .spec
        .as_ref()
        .and_then(|spec| spec.containers.first())
        .and_then(|c| c.ports.as_ref())
        .and_then(|ports| ports.first())
        .map(|p| p.container_port)
        .unwrap_or(80);

    // Build target URL
    let target_url = format!("http://{}:{}/{}", pod_ip, port, path);

    // Forward the request
    proxy_request(target_url, method, headers, params, body).await
}

/// Helper function to proxy an HTTP request to a target URL
async fn proxy_request(
    target_url: String,
    method: Method,
    headers: HeaderMap,
    params: HashMap<String, String>,
    body: Body,
) -> Result<Response> {
    // Build query string from params
    let query_string = if !params.is_empty() {
        let pairs: Vec<String> = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        format!("?{}", pairs.join("&"))
    } else {
        String::new()
    };

    let full_url = format!("{}{}", target_url, query_string);

    info!("Forwarding {} request to {}", method, full_url);

    // Create HTTP client
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true) // For kubelet self-signed certs
        .build()
        .map_err(|e| {
            rusternetes_common::Error::Internal(format!("Failed to create HTTP client: {}", e))
        })?;

    // Convert axum body to bytes
    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|e| {
            rusternetes_common::Error::Internal(format!("Failed to read request body: {}", e))
        })?;

    // Build the request
    let mut request = client
        .request(
            method.as_str().parse().unwrap_or(reqwest::Method::GET),
            &full_url,
        )
        .body(body_bytes.to_vec());

    // Forward headers (filter out hop-by-hop headers)
    for (name, value) in headers.iter() {
        let name_str = name.as_str();
        if !is_hop_by_hop_header(name_str) {
            if let Ok(val_str) = value.to_str() {
                request = request.header(name_str, val_str);
            }
        }
    }

    // Execute the request
    let response = request.send().await.map_err(|e| {
        warn!("Proxy request failed: {}", e);
        rusternetes_common::Error::Internal(format!("Proxy request failed: {}", e))
    })?;

    // Convert response to axum response
    let status = response.status();
    let mut axum_response = Response::builder().status(
        StatusCode::from_u16(status.as_u16())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
    );

    // Copy response headers
    for (name, value) in response.headers().iter() {
        let name_str = name.as_str();
        if !is_hop_by_hop_header(name_str) {
            axum_response = axum_response.header(name_str, value);
        }
    }

    // Get response body
    let body_bytes = response.bytes().await.map_err(|e| {
        rusternetes_common::Error::Internal(format!("Failed to read response body: {}", e))
    })?;

    // Build final response
    axum_response
        .body(Body::from(body_bytes.to_vec()))
        .map_err(|e| {
            rusternetes_common::Error::Internal(format!("Failed to build response: {}", e))
        })
}

/// Check if a header is a hop-by-hop header that should not be forwarded
fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hop_by_hop_headers() {
        assert!(is_hop_by_hop_header("Connection"));
        assert!(is_hop_by_hop_header("Keep-Alive"));
        assert!(is_hop_by_hop_header("transfer-encoding"));
        assert!(!is_hop_by_hop_header("Content-Type"));
        assert!(!is_hop_by_hop_header("Authorization"));
    }
}
