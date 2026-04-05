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
    let node_key = rusternetes_storage::build_key("nodes", None, &node_name);
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
            rusternetes_common::Error::NotFound(format!("No address found for node {}", node_name))
        })?;

    // Build target URL (kubelet typically runs on port 10250)
    let kubelet_port = 10250;
    let path = path.trim_start_matches('/');
    let target_url = format!("https://{}:{}/{}", node_address, kubelet_port, path);

    // Forward the request to the kubelet
    proxy_request(target_url, method, headers, params, body).await
}

/// Proxy HTTP requests to a service (root path — no sub-path)
pub async fn proxy_service_root(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, service_name)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
    body: Body,
) -> Result<Response> {
    proxy_service(
        State(state),
        Extension(auth_ctx),
        Path((namespace, service_name, String::new())),
        method,
        headers,
        Query(params),
        body,
    )
    .await
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

    // Parse service name — format may be "name" or "name:portname"
    let (actual_service_name, port_name) = if let Some(idx) = service_name.find(':') {
        (&service_name[..idx], Some(&service_name[idx + 1..]))
    } else {
        (service_name.as_str(), None)
    };

    // Get the service to find its ClusterIP and port
    let service_key =
        rusternetes_storage::build_key("services", Some(&namespace), actual_service_name);
    let service: rusternetes_common::resources::Service = state.storage.get(&service_key).await?;

    // Find the port — by name if specified, otherwise first port
    let port = if let Some(pn) = port_name {
        service
            .spec
            .ports
            .iter()
            .find(|p| p.name.as_deref() == Some(pn))
            .map(|p| p.port)
            .or_else(|| pn.parse::<u16>().ok())
            .unwrap_or_else(|| service.spec.ports.first().map(|p| p.port).unwrap_or(80))
    } else {
        service.spec.ports.first().map(|p| p.port).unwrap_or(80)
    };

    // Get ClusterIP as fallback
    let cluster_ip = service
        .spec
        .cluster_ip
        .clone()
        .unwrap_or_else(|| "127.0.0.1".to_string());

    // Try to find a pod endpoint IP for direct proxying (more reliable than ClusterIP DNAT)
    let target_port = match service
        .spec
        .ports
        .iter()
        .find(|p| port_name.map_or(true, |pn| p.name.as_deref() == Some(pn)))
    {
        Some(sp) => match &sp.target_port {
            Some(rusternetes_common::resources::IntOrString::Int(p)) => *p as u16,
            Some(rusternetes_common::resources::IntOrString::String(s)) => {
                s.parse::<u16>().unwrap_or(port)
            }
            None => port,
        },
        None => port,
    };

    // Look up endpoint IPs from EndpointSlices
    let es_prefix = rusternetes_storage::build_prefix("endpointslices", Some(&namespace));
    let endpoint_slices: Vec<rusternetes_common::resources::EndpointSlice> =
        state.storage.list(&es_prefix).await.unwrap_or_default();
    let mut endpoint_ip = None;
    for es in &endpoint_slices {
        let svc = es
            .metadata
            .labels
            .as_ref()
            .and_then(|l| l.get("kubernetes.io/service-name"));
        if svc.map(|s| s == actual_service_name).unwrap_or(false) {
            for ep in &es.endpoints {
                if ep.conditions.as_ref().and_then(|c| c.ready).unwrap_or(true) {
                    if let Some(addr) = ep.addresses.first() {
                        endpoint_ip = Some(addr.clone());
                        break;
                    }
                }
            }
            if endpoint_ip.is_some() {
                break;
            }
        }
    }

    // Use endpoint IP if available, otherwise fall back to ClusterIP
    let path = path.trim_start_matches('/');
    let target_url = if let Some(ep_ip) = endpoint_ip {
        format!("http://{}:{}/{}", ep_ip, target_port, path)
    } else {
        format!("http://{}:{}/{}", cluster_ip, port, path)
    };

    // Forward the request
    proxy_request(target_url, method, headers, params, body).await
}

/// Proxy HTTP requests to a pod (root path, no trailing path component)
pub async fn proxy_pod_root(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, pod_name)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
    body: Body,
) -> Result<Response> {
    proxy_pod(
        State(state),
        Extension(auth_ctx),
        Path((namespace, pod_name, String::new())),
        method,
        headers,
        Query(params),
        body,
    )
    .await
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

    // Parse pod name — format may be "name" or "name:port" (Kubernetes proxy subresource convention)
    let (actual_pod_name, explicit_port) = if let Some(idx) = pod_name.rfind(':') {
        let maybe_port = &pod_name[idx + 1..];
        if let Ok(p) = maybe_port.parse::<u16>() {
            (&pod_name[..idx], Some(p))
        } else {
            (pod_name.as_str(), None)
        }
    } else {
        (pod_name.as_str(), None)
    };

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "pods/proxy")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(actual_pod_name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Get the pod to find its IP
    let pod_key = rusternetes_storage::build_key("pods", Some(&namespace), actual_pod_name);
    let pod: rusternetes_common::resources::Pod = state.storage.get(&pod_key).await?;

    // Get pod IP
    let pod_ip = pod
        .status
        .as_ref()
        .and_then(|s| s.pod_ip.clone())
        .ok_or_else(|| {
            rusternetes_common::Error::NotFound(format!(
                "Pod {}/{} has no IP address yet",
                namespace, actual_pod_name
            ))
        })?;

    // Use explicit port from URL if provided, otherwise first container port, otherwise 80
    let port: u16 = if let Some(p) = explicit_port {
        p
    } else {
        pod.spec
            .as_ref()
            .and_then(|spec| spec.containers.first())
            .and_then(|c| c.ports.as_ref())
            .and_then(|ports| ports.first())
            .map(|p| p.container_port)
            .unwrap_or(80)
    };

    // Build target URL
    let path = path.trim_start_matches('/');
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
        let pairs: Vec<String> = params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        format!("?{}", pairs.join("&"))
    } else {
        String::new()
    };

    let full_url = format!("{}{}", target_url, query_string);

    info!("Forwarding {} request to {}", method, full_url);

    // Create HTTP client with timeouts
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true) // For kubelet self-signed certs
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| {
            rusternetes_common::Error::Internal(format!("Failed to create HTTP client: {}", e))
        })?;

    // Convert axum body to bytes
    let body_bytes = axum::body::to_bytes(body, usize::MAX).await.map_err(|e| {
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
    let mut axum_response = Response::builder()
        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));

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
            | "host"
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
