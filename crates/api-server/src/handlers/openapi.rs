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

/// GET /openapi/v2 and /swagger.json
/// Returns an OpenAPI v2 (Swagger) specification.
/// kubectl sends Accept: application/com.github.proto-openapi.spec.v2@v1.0+protobuf
/// first, then falls back to JSON on 406. We return 406 for protobuf requests
/// to force kubectl to use JSON, which we can serve.
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

    // Check if client requests protobuf OpenAPI
    let accept = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if accept.contains("protobuf") {
        // Encode as K8s protobuf envelope (runtime.Unknown wrapper).
        // Format: 4-byte magic "k8s\0" + protobuf Unknown message containing JSON.
        // Field 2 (content_type) = "application/json"
        // Field 3 (content_encoding) = "" (no encoding)
        // Field 1 (raw) = JSON bytes
        let content_type = b"application/json";
        let mut proto = Vec::new();
        // Magic header
        proto.extend_from_slice(&[0x6b, 0x38, 0x73, 0x00]);
        // Protobuf Unknown message fields:
        // Field 2 (content_type, string, wire type 2): tag = (2 << 3) | 2 = 18
        proto.push(18);
        encode_varint(&mut proto, content_type.len() as u64);
        proto.extend_from_slice(content_type);
        // Field 3 (content_encoding, string, wire type 2): tag = (3 << 3) | 2 = 26
        // Empty string, skip
        // Field 1 (raw, bytes, wire type 2): tag = (1 << 3) | 2 = 10
        proto.push(10);
        encode_varint(&mut proto, json_bytes.len() as u64);
        proto.extend_from_slice(&json_bytes);

        return Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                "application/octet-stream",
            )
            .body(Body::from(proto))
            .unwrap();
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json_bytes))
        .unwrap()
}
