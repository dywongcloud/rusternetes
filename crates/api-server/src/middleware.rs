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

        // Handle protobuf Content-Type: extract JSON from K8s protobuf envelope.
        // The K8s protobuf format wraps JSON in a simple envelope:
        //   magic: "k8s\0" (4 bytes)
        //   protobuf Unknown message with `raw` field containing JSON
        if content_type.starts_with("application/vnd.kubernetes.protobuf") {
            debug!(
                "Converting protobuf to JSON for: {} {}",
                request.method(), request.uri()
            );

            // Read the body
            let (parts, body) = request.into_parts();
            let body_bytes = match axum::body::to_bytes(body, 10 * 1024 * 1024).await {
                Ok(b) => b,
                Err(_) => {
                    return Err(axum::response::Response::builder()
                        .status(axum::http::StatusCode::BAD_REQUEST)
                        .body(axum::body::Body::from("failed to read request body"))
                        .unwrap());
                }
            };

            let json_body = if body_bytes.starts_with(b"k8s\0") {
                // K8s protobuf envelope — extract the JSON from the `raw` field.
                let extracted = extract_json_from_k8s_protobuf(&body_bytes);
                if let Some(json) = extracted {
                    json
                } else {
                    // Extraction failed — scan for JSON object with balanced braces
                    let mut found_json = None;
                    for i in 0..body_bytes.len() {
                        if body_bytes[i] == b'{' {
                            let mut depth = 0i32;
                            let mut in_string = false;
                            let mut escape = false;
                            for j in i..body_bytes.len() {
                                if escape { escape = false; continue; }
                                match body_bytes[j] {
                                    b'\\' if in_string => escape = true,
                                    b'"' => in_string = !in_string,
                                    b'{' if !in_string => depth += 1,
                                    b'}' if !in_string => {
                                        depth -= 1;
                                        if depth == 0 {
                                            found_json = Some(body_bytes[i..=j].to_vec());
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            if found_json.is_some() { break; }
                            // Unbalanced — use from { to end
                            found_json = Some(body_bytes[i..].to_vec());
                            break;
                        }
                    }
                    if let Some(json) = found_json {
                        json
                    } else {
                        // No JSON found — return 415 so client retries with JSON
                        warn!("Protobuf body has no JSON payload ({} bytes), returning 415", body_bytes.len());
                        return Err(axum::response::Response::builder()
                            .status(axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE)
                            .header(axum::http::header::CONTENT_TYPE, "application/json")
                            .body(axum::body::Body::from(
                                r#"{"kind":"Status","apiVersion":"v1","metadata":{},"status":"Failure","message":"the body content type is not supported: application/vnd.kubernetes.protobuf","reason":"UnsupportedMediaType","code":415}"#
                            ))
                            .unwrap());
                    }
                }
            } else if body_bytes.starts_with(b"{") || body_bytes.starts_with(b"[") {
                // Already JSON despite protobuf Content-Type
                body_bytes.to_vec()
            } else {
                // Unknown binary format — try to find JSON, else empty object
                let mut found = None;
                for i in 0..body_bytes.len() {
                    if body_bytes[i] == b'{' {
                        found = Some(body_bytes[i..].to_vec());
                        break;
                    }
                }
                found.unwrap_or_else(|| b"{}".to_vec())
            };

            let mut new_request = Request::from_parts(parts, axum::body::Body::from(json_body));
            new_request.headers_mut().insert(
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static("application/json"),
            );
            request = new_request;
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

/// Extract JSON from a Kubernetes protobuf envelope.
/// K8s protobuf format: "k8s\0" + protobuf Unknown { raw: bytes, contentType: string }
/// The `raw` field (protobuf field 2, wire type 2 = length-delimited) contains the JSON.
fn extract_json_from_k8s_protobuf(data: &[u8]) -> Option<Vec<u8>> {
    // Skip the 4-byte magic "k8s\0"
    if data.len() < 5 || &data[0..4] != b"k8s\0" {
        return None;
    }
    let data = &data[4..];

    // Parse the protobuf Unknown message looking for field 2 (raw bytes)
    // Field 2, wire type 2 (length-delimited) = tag byte 0x12
    let mut pos = 0;
    while pos < data.len() {
        // Read tag as varint (supports field numbers > 15)
        let mut tag: u64 = 0;
        let mut shift = 0;
        while pos < data.len() {
            let b = data[pos] as u64;
            pos += 1;
            tag |= (b & 0x7f) << shift;
            if b & 0x80 == 0 { break; }
            shift += 7;
        }
        let field_number = tag >> 3;
        let wire_type = tag & 0x07;

        match wire_type {
            0 => {
                // Varint — skip
                while pos < data.len() && data[pos] & 0x80 != 0 { pos += 1; }
                if pos < data.len() { pos += 1; }
            }
            1 => {
                // 64-bit fixed — skip 8 bytes
                pos += 8;
            }
            2 => {
                // Length-delimited — read length then data
                let mut len: usize = 0;
                let mut shift = 0;
                while pos < data.len() {
                    let b = data[pos] as usize;
                    pos += 1;
                    len |= (b & 0x7f) << shift;
                    if b & 0x80 == 0 { break; }
                    shift += 7;
                }
                if (field_number == 2 || field_number == 3) && pos + len <= data.len() {
                    // Field 2 or 3 may contain the raw JSON bytes
                    let raw = &data[pos..pos + len];
                    if !raw.is_empty() && (raw[0] == b'{' || raw[0] == b'[') {
                        return Some(raw.to_vec());
                    }
                }
                if pos + len > data.len() { return None; }
                pos += len;
            }
            5 => {
                // 32-bit fixed — skip 4 bytes
                pos += 4;
            }
            _ => {
                // Unknown wire type — can't parse further, try fallback
                break;
            }
        }
    }

    // Fallback: scan for the first JSON object in the data
    for i in 0..data.len() {
        if data[i] == b'{' {
            // Try to find matching closing brace
            let mut depth = 0i32;
            let mut in_string = false;
            let mut escape = false;
            for j in i..data.len() {
                if escape {
                    escape = false;
                    continue;
                }
                match data[j] {
                    b'\\' if in_string => escape = true,
                    b'"' => in_string = !in_string,
                    b'{' if !in_string => depth += 1,
                    b'}' if !in_string => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(data[i..=j].to_vec());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    None
}
