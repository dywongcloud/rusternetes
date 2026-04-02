/// OpenAPI specification handler
use crate::openapi::generate_openapi_spec;
use axum::{
    body::Body,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

/// Encode a u64 as a protobuf varint
fn encode_varint(buf: &mut Vec<u8>, mut value: u64) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            buf.push(byte);
            break;
        } else {
            buf.push(byte | 0x80);
        }
    }
}

/// GET /openapi/v3
/// Get the OpenAPI v3 root document listing available paths
pub async fn get_openapi_spec() -> Response {
    // Return the root document that lists all available OpenAPI paths
    // In real K8s, this returns {"paths": {"/apis/apps/v1": {...}, ...}}
    let mut paths = serde_json::Map::new();
    let path_entry =
        |gv: &str| serde_json::json!({"serverRelativeURL": format!("/openapi/v3/{}", gv)});
    paths.insert("api/v1".into(), path_entry("api/v1"));
    for (group, version) in &[
        ("apps", "v1"),
        ("batch", "v1"),
        ("networking.k8s.io", "v1"),
        ("rbac.authorization.k8s.io", "v1"),
        ("storage.k8s.io", "v1"),
        ("scheduling.k8s.io", "v1"),
        ("apiextensions.k8s.io", "v1"),
        ("admissionregistration.k8s.io", "v1"),
        ("coordination.k8s.io", "v1"),
        ("flowcontrol.apiserver.k8s.io", "v1"),
        ("certificates.k8s.io", "v1"),
        ("discovery.k8s.io", "v1"),
        ("node.k8s.io", "v1"),
        ("autoscaling", "v1"),
        ("autoscaling", "v2"),
        ("policy", "v1"),
        ("resource.k8s.io", "v1"),
        ("events.k8s.io", "v1"),
    ] {
        paths.insert(
            format!("apis/{}/{}", group, version),
            path_entry(&format!("apis/{}/{}", group, version)),
        );
    }
    let root = serde_json::json!({"paths": paths});
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&root).unwrap_or_default()))
        .unwrap()
}

/// GET /openapi/v3/*path
/// Returns the OpenAPI v3 spec for a specific group version
pub async fn get_openapi_spec_path() -> Response {
    let spec = generate_openapi_spec();
    let json_bytes = serde_json::to_vec(&spec).unwrap_or_default();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json_bytes))
        .unwrap()
}

/// Wrap JSON bytes in the Kubernetes protobuf wire format.
///
/// The K8s protobuf format for OpenAPI is:
/// - 4 bytes magic: "k8s\0"
/// - Protobuf message with:
///   - field 1 (string): content type
///   - field 2 (bytes): encoding (empty)
///   - field 3 (bytes): the raw data (JSON spec)
///   - field 4 (string): content encoding (empty)
fn wrap_in_k8s_protobuf(content_type: &str, data: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(data.len() + 100);
    // Magic prefix
    buf.extend_from_slice(b"k8s\0");

    // Build the protobuf message body first to compute its length
    let mut msg = Vec::with_capacity(data.len() + 80);
    // Field 1: content type (wire type 2 = length-delimited, field number 1)
    msg.push((1 << 3) | 2); // tag
    encode_varint(&mut msg, content_type.len() as u64);
    msg.extend_from_slice(content_type.as_bytes());
    // Field 2: encoding (empty string)
    msg.push((2 << 3) | 2);
    encode_varint(&mut msg, 0);
    // Field 3: raw data (the JSON spec bytes)
    msg.push((3 << 3) | 2);
    encode_varint(&mut msg, data.len() as u64);
    msg.extend_from_slice(data);
    // Field 4: content encoding (empty)
    msg.push((4 << 3) | 2);
    encode_varint(&mut msg, 0);

    buf.extend_from_slice(&msg);
    buf
}

/// GET /openapi/v2 and /swagger.json
/// Returns an OpenAPI v2 (Swagger) specification.
///
/// Supports both protobuf and JSON Accept headers:
/// - application/com.github.proto-openapi.spec.v2@v1.0+protobuf → protobuf-wrapped JSON
/// - application/json → raw JSON
pub async fn get_swagger_spec(headers: HeaderMap) -> Response {
    let spec = serde_json::json!({
        "swagger": "2.0",
        "info": {
            "title": "Rusternetes Kubernetes API",
            "version": "v1.35.0"
        },
        "paths": {},
        "definitions": {}
    });
    let json_bytes = serde_json::to_vec(&spec).unwrap_or_default();

    let accept = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if accept.contains("proto-openapi") || accept.contains("protobuf") {
        // Wrap JSON in K8s protobuf envelope. The internal content type inside the
        // protobuf wrapper uses the full proto-openapi identifier.
        let internal_ct = "application/com.github.proto-openapi.spec.v2@v1.0+protobuf";
        let pb_bytes = wrap_in_k8s_protobuf(internal_ct, &json_bytes);
        // Use application/vnd.kubernetes.protobuf as the HTTP Content-Type header.
        // The full proto-openapi content type contains '@' which Go's
        // mime.ParseMediaType rejects with "unexpected content after media subtype".
        Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                "application/vnd.kubernetes.protobuf",
            )
            .body(Body::from(pb_bytes))
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json_bytes))
            .unwrap()
    }
}
