use axum::body::to_bytes;
use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Extension,
};
use rusternetes_common::auth::{BootstrapTokenManager, TokenManager, UserInfo};
use std::sync::Arc;
use tracing::{debug, error, warn};

/// Extension type to carry UserInfo through the request
#[derive(Clone, Debug)]
pub struct AuthContext {
    pub user: UserInfo,
}

/// Middleware that adds a default admin AuthContext when skip_auth is enabled
pub async fn skip_auth_middleware(mut request: Request, next: Next) -> Result<Response, Response> {
    debug!(
        "skip_auth_middleware called for: {} {}",
        request.method(),
        request.uri()
    );

    // Create an admin user context
    let admin_user = UserInfo {
        username: "admin".to_string(),
        uid: "system:admin".to_string(),
        groups: vec!["system:masters".to_string()],
        extra: std::collections::HashMap::new(),
    };

    // Insert AuthContext into request extensions
    request
        .extensions_mut()
        .insert(AuthContext { user: admin_user });

    debug!("AuthContext inserted into request extensions");

    Ok(next.run(request).await)
}

/// Authentication middleware that extracts and validates JWT tokens
pub async fn auth_middleware(
    Extension(token_manager): Extension<Arc<TokenManager>>,
    Extension(bootstrap_token_manager): Extension<Arc<BootstrapTokenManager>>,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Extract Bearer token from Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let user = if auth_header.starts_with("Bearer ") {
        let token = &auth_header[7..]; // Skip "Bearer "

        // Try to validate as a service account token first
        if let Ok(claims) = token_manager.validate_token(token) {
            let user_info = UserInfo::from_service_account_claims(&claims);
            debug!(
                "Authenticated user (service account): {}",
                user_info.username
            );
            user_info
        }
        // Try to validate as a bootstrap token
        else if let Ok(bootstrap_token) = bootstrap_token_manager.validate_token(token) {
            let user_info = UserInfo::from_bootstrap_token(&bootstrap_token);
            debug!(
                "Authenticated user (bootstrap token): {}",
                user_info.username
            );
            user_info
        }
        // Invalid token
        else {
            warn!("Invalid token");
            return Err((StatusCode::UNAUTHORIZED, "Invalid token").into_response());
        }
    } else {
        // Anonymous user
        debug!("Anonymous request");
        UserInfo::anonymous()
    };

    // Insert UserInfo into request extensions
    request.extensions_mut().insert(AuthContext { user });

    Ok(next.run(request).await)
}

/// Middleware that normalizes Content-Type to application/json for write requests.
/// The Kubernetes client defaults to application/vnd.kubernetes.protobuf, but we only
/// support JSON. Axum's Json extractor rejects non-application/json content types with
/// HTTP 415, so we rewrite the header before the request reaches the handler.
pub async fn normalize_content_type_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    if request.method() == axum::http::Method::POST
        || request.method() == axum::http::Method::PUT
        || request.method() == axum::http::Method::PATCH
    {
        let content_type = request
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        // Reject protobuf — rusternetes only supports JSON encoding.
        // Return 415 Unsupported Media Type with a proper Status body.
        if content_type.starts_with("application/vnd.kubernetes.protobuf") {
            warn!(
                "Rejecting protobuf request: {} {} (use --kube-api-content-type=application/json)",
                request.method(), request.uri()
            );
            let status_body = serde_json::json!({
                "kind": "Status",
                "apiVersion": "v1",
                "status": "Failure",
                "message": "the body of the request was in an unknown format - accepted media types include: application/json",
                "reason": "UnsupportedMediaType",
                "code": 415
            });
            return Ok(axum::response::Response::builder()
                .status(StatusCode::UNSUPPORTED_MEDIA_TYPE)
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&status_body).unwrap()))
                .unwrap());
        }

        // If not already a JSON content type, normalize to application/json
        if !content_type.starts_with("application/json")
            && !content_type.starts_with("application/strategic-merge-patch+json")
            && !content_type.starts_with("application/merge-patch+json")
            && !content_type.starts_with("application/json-patch+json")
            && !content_type.starts_with("application/apply-patch+yaml")
        {
            request.headers_mut().insert(
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static("application/json"),
            );
        }
    }

    Ok(next.run(request).await)
}

/// Middleware to log request bodies for debugging JSON deserialization errors
pub async fn log_request_body_middleware(
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let (parts, body) = request.into_parts();
    let method_str = parts.method.to_string();
    let uri_str = parts.uri.to_string();

    // Only log POST/PUT/PATCH requests
    if parts.method == axum::http::Method::POST
        || parts.method == axum::http::Method::PUT
        || parts.method == axum::http::Method::PATCH
    {
        // Read the body
        let bytes = match to_bytes(body, usize::MAX).await {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to read request body: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to read request body",
                )
                    .into_response());
            }
        };

        // Log the body if it's not too large
        if bytes.len() < 10000 {
            if let Ok(body_str) = String::from_utf8(bytes.to_vec()) {
                debug!(
                    "Request body for {} {}: {}",
                    parts.method, parts.uri, body_str
                );
            } else {
                warn!(
                    "Request body for {} {} is binary ({} bytes, first 20: {:?}), content-type: {:?}",
                    parts.method, parts.uri, bytes.len(),
                    &bytes[..std::cmp::min(20, bytes.len())],
                    parts.headers.get(axum::http::header::CONTENT_TYPE)
                );
            }
        }

        // Reconstruct the request with the body
        let request = Request::from_parts(parts, Body::from(bytes));
        let response = next.run(request).await;

        // Log if the response is a client error
        if response.status().is_client_error() {
            warn!(
                "HTTP {} returned for {} {} - check request body above",
                response.status().as_u16(),
                method_str,
                uri_str
            );
        }

        Ok(response)
    } else {
        // Pass through for GET/DELETE/etc
        let request = Request::from_parts(parts, body);
        Ok(next.run(request).await)
    }
}
