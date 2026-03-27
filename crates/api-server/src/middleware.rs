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
use tracing::{debug, error, info, warn};

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
        || request.method() == axum::http::Method::DELETE
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
                        // No JSON found in protobuf body. Try to construct minimal JSON
                        // from the TypeMeta fields embedded in the protobuf envelope.
                        // The K8s Unknown message has field 1 = TypeMeta (apiVersion, kind).
                        // Log details about the protobuf body for debugging
                        let hex_preview: String = body_bytes.iter().skip(4).take(80)
                            .map(|b| format!("{:02x}", b))
                            .collect::<Vec<_>>()
                            .join(" ");
                        warn!("Protobuf body has no JSON payload ({} bytes). Hex after k8s\\0: {}", body_bytes.len(), hex_preview);
                        // Try to decode native K8s protobuf into JSON.
                        // The K8s protobuf Unknown message has:
                        //   field 1 = TypeMeta (apiVersion, kind)
                        //   field 2 = raw object bytes (also protobuf-encoded)
                        // We extract string fields from the raw bytes to construct JSON.
                        if let Some(json_bytes) = decode_k8s_protobuf_to_json(&body_bytes) {
                            info!("Decoded K8s protobuf to JSON ({} bytes)", json_bytes.len());
                            json_bytes
                        } else {
                            // Last resort: extract TypeMeta only
                            let type_meta = extract_type_meta_from_protobuf(&body_bytes);
                            if let Some((api_version, kind)) = type_meta {
                                let minimal = format!(
                                    r#"{{"apiVersion":"{}","kind":"{}","metadata":{{}}}}"#,
                                    api_version, kind
                                );
                                info!("Extracted TypeMeta from protobuf: apiVersion={}, kind={}", api_version, kind);
                                minimal.into_bytes()
                            } else {
                                warn!("Could not extract TypeMeta from protobuf, using empty object");
                                b"{}".to_vec()
                            }
                        }
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
                    // Log what field 2 contains if it's not JSON
                    if field_number == 2 && !raw.is_empty() {
                        let preview: String = raw.iter().take(40)
                            .map(|b| format!("{:02x}", b))
                            .collect::<Vec<_>>()
                            .join(" ");
                        tracing::debug!("Protobuf field 2 ({} bytes, not JSON): first bytes: {}", len, preview);
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

/// Extract TypeMeta (apiVersion, kind) from a K8s protobuf envelope.
/// The Unknown message structure: field 1 = TypeMeta { field 1 = apiVersion, field 2 = kind }
fn extract_type_meta_from_protobuf(data: &[u8]) -> Option<(String, String)> {
    if data.len() < 5 || &data[0..4] != b"k8s\0" {
        return None;
    }
    let data = &data[4..];

    // Read field 1 (TypeMeta) — tag 0x0a, wire type 2
    let mut pos = 0;
    if pos >= data.len() || data[pos] != 0x0a { return None; }
    pos += 1;

    // Read length varint
    let mut type_meta_len: usize = 0;
    let mut shift = 0;
    while pos < data.len() {
        let b = data[pos] as usize;
        pos += 1;
        type_meta_len |= (b & 0x7f) << shift;
        if b & 0x80 == 0 { break; }
        shift += 7;
    }

    if pos + type_meta_len > data.len() { return None; }
    let type_meta = &data[pos..pos + type_meta_len];

    // Parse TypeMeta: field 1 = apiVersion, field 2 = kind
    let mut api_version = String::new();
    let mut kind = String::new();
    let mut tpos = 0;
    while tpos < type_meta.len() {
        let tag = type_meta[tpos];
        tpos += 1;
        let field_num = tag >> 3;
        let wire_type = tag & 0x07;

        if wire_type == 2 {
            // Length-delimited string
            let mut slen: usize = 0;
            let mut sshift = 0;
            while tpos < type_meta.len() {
                let b = type_meta[tpos] as usize;
                tpos += 1;
                slen |= (b & 0x7f) << sshift;
                if b & 0x80 == 0 { break; }
                sshift += 7;
            }
            if tpos + slen <= type_meta.len() {
                if let Ok(s) = std::str::from_utf8(&type_meta[tpos..tpos + slen]) {
                    match field_num {
                        1 => api_version = s.to_string(),
                        2 => kind = s.to_string(),
                        _ => {}
                    }
                }
            }
            tpos += slen;
        } else {
            break; // Unknown wire type, stop
        }
    }

    if !api_version.is_empty() && !kind.is_empty() {
        Some((api_version, kind))
    } else {
        None
    }
}

/// Attempt to decode a K8s protobuf body into JSON.
/// The K8s Unknown message wraps: field 1 = TypeMeta, field 2 = raw object (protobuf).
/// We extract TypeMeta and the raw object, then recursively decode protobuf string fields
/// into a JSON structure. This is a best-effort decoder for CRDs and other resources
/// where the client hardcodes protobuf encoding.
fn decode_k8s_protobuf_to_json(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 5 || &data[0..4] != b"k8s\0" {
        return None;
    }
    let data = &data[4..];

    let mut api_version = String::new();
    let mut kind = String::new();
    let mut raw_bytes: Option<&[u8]> = None;

    let mut pos = 0;
    while pos < data.len() {
        let tag = data[pos];
        pos += 1;
        let field_num = tag >> 3;
        let wire_type = tag & 0x07;

        if wire_type == 2 {
            // Length-delimited
            let mut len: usize = 0;
            let mut shift = 0;
            while pos < data.len() {
                let b = data[pos] as usize;
                pos += 1;
                len |= (b & 0x7f) << shift;
                if b & 0x80 == 0 { break; }
                shift += 7;
            }
            if pos + len > data.len() { break; }
            let field_data = &data[pos..pos + len];
            pos += len;

            match field_num {
                1 => {
                    // TypeMeta — parse apiVersion and kind
                    let mut tpos = 0;
                    while tpos < field_data.len() {
                        let t = field_data[tpos];
                        tpos += 1;
                        let fnum = t >> 3;
                        if (t & 0x07) != 2 { break; }
                        let mut slen: usize = 0;
                        let mut ss = 0;
                        while tpos < field_data.len() {
                            let b = field_data[tpos] as usize;
                            tpos += 1;
                            slen |= (b & 0x7f) << ss;
                            if b & 0x80 == 0 { break; }
                            ss += 7;
                        }
                        if tpos + slen <= field_data.len() {
                            if let Ok(s) = std::str::from_utf8(&field_data[tpos..tpos + slen]) {
                                match fnum {
                                    1 => api_version = s.to_string(),
                                    2 => kind = s.to_string(),
                                    _ => {}
                                }
                            }
                        }
                        tpos += slen;
                    }
                }
                2 => raw_bytes = Some(field_data),
                _ => {}
            }
        } else if wire_type == 0 {
            // Varint — skip
            while pos < data.len() && data[pos] & 0x80 != 0 { pos += 1; }
            if pos < data.len() { pos += 1; }
        } else {
            break;
        }
    }

    if api_version.is_empty() || kind.is_empty() {
        return None;
    }

    // For CRDs specifically, try to decode the raw protobuf into a JSON CRD.
    // The CRD protobuf schema has known field numbers:
    //   ObjectMeta = field 1, CRDSpec = field 2
    // ObjectMeta fields: name=1, namespace=3, uid=5, resourceVersion=6
    // CRDSpec fields: group=1, names=3, scope=4, versions=7
    // This is best-effort — we extract what we can.
    let raw = raw_bytes?;

    // Extract ObjectMeta.name and CRD spec fields from the raw protobuf
    let mut metadata_name = String::new();
    let mut metadata_namespace = String::new();
    let mut spec_group = String::new();
    let mut spec_scope = String::new();
    let mut spec_names_plural = String::new();
    let mut spec_names_singular = String::new();
    let mut spec_names_kind = String::new();
    let mut spec_names_list_kind = String::new();

    let mut rpos = 0;
    while rpos < raw.len() {
        let tag = raw[rpos];
        rpos += 1;
        let field_num = tag >> 3;
        let wire_type = tag & 0x07;

        if wire_type == 2 {
            let mut len: usize = 0;
            let mut shift = 0;
            while rpos < raw.len() {
                let b = raw[rpos] as usize;
                rpos += 1;
                len |= (b & 0x7f) << shift;
                if b & 0x80 == 0 { break; }
                shift += 7;
            }
            if rpos + len > raw.len() { break; }
            let field_data = &raw[rpos..rpos + len];
            rpos += len;

            match field_num {
                1 => {
                    // ObjectMeta — parse name (field 1) and namespace (field 3)
                    let mut mpos = 0;
                    while mpos < field_data.len() {
                        let mt = field_data[mpos];
                        mpos += 1;
                        let mfnum = mt >> 3;
                        let mwire = mt & 0x07;
                        if mwire == 2 {
                            let mut mlen: usize = 0;
                            let mut ms = 0;
                            while mpos < field_data.len() {
                                let b = field_data[mpos] as usize;
                                mpos += 1;
                                mlen |= (b & 0x7f) << ms;
                                if b & 0x80 == 0 { break; }
                                ms += 7;
                            }
                            if mpos + mlen <= field_data.len() {
                                if let Ok(s) = std::str::from_utf8(&field_data[mpos..mpos + mlen]) {
                                    match mfnum {
                                        1 => metadata_name = s.to_string(),
                                        3 => metadata_namespace = s.to_string(),
                                        _ => {}
                                    }
                                }
                            }
                            mpos += mlen;
                        } else if mwire == 0 {
                            while mpos < field_data.len() && field_data[mpos] & 0x80 != 0 { mpos += 1; }
                            if mpos < field_data.len() { mpos += 1; }
                        } else {
                            break;
                        }
                    }
                }
                2 => {
                    // CRDSpec — parse group, names, scope, versions
                    let mut spos = 0;
                    while spos < field_data.len() {
                        let st = field_data[spos];
                        spos += 1;
                        let sfnum = st >> 3;
                        let swire = st & 0x07;
                        if swire == 2 {
                            let mut slen: usize = 0;
                            let mut ss = 0;
                            while spos < field_data.len() {
                                let b = field_data[spos] as usize;
                                spos += 1;
                                slen |= (b & 0x7f) << ss;
                                if b & 0x80 == 0 { break; }
                                ss += 7;
                            }
                            if spos + slen <= field_data.len() {
                                match sfnum {
                                    1 => { spec_group = String::from_utf8_lossy(&field_data[spos..spos + slen]).to_string(); }
                                    3 => {
                                        // Names submessage — parse plural, singular, kind, listKind
                                        let names = &field_data[spos..spos + slen];
                                        let mut npos = 0;
                                        while npos < names.len() {
                                            let nt = names[npos];
                                            npos += 1;
                                            let nfnum = nt >> 3;
                                            if (nt & 0x07) != 2 { break; }
                                            let mut nlen: usize = 0;
                                            let mut ns = 0;
                                            while npos < names.len() {
                                                let b = names[npos] as usize;
                                                npos += 1;
                                                nlen |= (b & 0x7f) << ns;
                                                if b & 0x80 == 0 { break; }
                                                ns += 7;
                                            }
                                            if npos + nlen <= names.len() {
                                                if let Ok(s) = std::str::from_utf8(&names[npos..npos + nlen]) {
                                                    match nfnum {
                                                        1 => spec_names_plural = s.to_string(),
                                                        2 => spec_names_singular = s.to_string(),
                                                        // field 3 = shortNames (repeated, skip)
                                                        4 => spec_names_kind = s.to_string(),
                                                        5 => spec_names_list_kind = s.to_string(),
                                                        _ => {}
                                                    }
                                                }
                                            }
                                            npos += nlen;
                                        }
                                    }
                                    4 => { spec_scope = String::from_utf8_lossy(&field_data[spos..spos + slen]).to_string(); }
                                    _ => {}
                                }
                            }
                            spos += slen;
                        } else if swire == 0 {
                            while spos < field_data.len() && field_data[spos] & 0x80 != 0 { spos += 1; }
                            if spos < field_data.len() { spos += 1; }
                        } else {
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else if wire_type == 0 {
            while rpos < raw.len() && raw[rpos] & 0x80 != 0 { rpos += 1; }
            if rpos < raw.len() { rpos += 1; }
        } else {
            break;
        }
    }

    if metadata_name.is_empty() {
        return None;
    }

    // Construct a JSON CRD with the extracted fields
    let scope = if spec_scope.is_empty() { "Namespaced" } else { &spec_scope };
    let json = serde_json::json!({
        "apiVersion": api_version,
        "kind": kind,
        "metadata": {
            "name": metadata_name,
            "namespace": if metadata_namespace.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(metadata_namespace) },
        },
        "spec": {
            "group": spec_group,
            "scope": scope,
            "names": {
                "plural": spec_names_plural,
                "singular": spec_names_singular,
                "kind": spec_names_kind,
                "listKind": if spec_names_list_kind.is_empty() { format!("{}List", spec_names_kind) } else { spec_names_list_kind },
            },
            "versions": [{
                "name": "v1",
                "served": true,
                "storage": true,
                "schema": {
                    "openAPIV3Schema": {
                        "type": "object",
                        "x-kubernetes-preserve-unknown-fields": true,
                    }
                }
            }],
        }
    });

    Some(serde_json::to_vec(&json).ok()?)
}
