/// OpenAPI specification handler
use crate::openapi::generate_openapi_spec;
use axum::{
    body::Body,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};

/// GET /openapi/v3
/// Get the OpenAPI v3 root document listing available paths
pub async fn get_openapi_spec() -> Response {
    // Return the root document that lists all available OpenAPI paths
    // In real K8s, this returns {"paths": {"/apis/apps/v1": {...}, ...}}
    let mut paths = serde_json::Map::new();
    let path_entry = |gv: &str| serde_json::json!({"serverRelativeURL": format!("/openapi/v3/{}", gv)});
    paths.insert("api/v1".into(), path_entry("api/v1"));
    for (group, version) in &[
        ("apps", "v1"), ("batch", "v1"), ("networking.k8s.io", "v1"),
        ("rbac.authorization.k8s.io", "v1"), ("storage.k8s.io", "v1"),
        ("scheduling.k8s.io", "v1"), ("apiextensions.k8s.io", "v1"),
        ("admissionregistration.k8s.io", "v1"), ("coordination.k8s.io", "v1"),
        ("flowcontrol.apiserver.k8s.io", "v1"), ("certificates.k8s.io", "v1"),
        ("discovery.k8s.io", "v1"), ("node.k8s.io", "v1"),
        ("autoscaling", "v1"), ("autoscaling", "v2"), ("policy", "v1"),
        ("resource.k8s.io", "v1"), ("events.k8s.io", "v1"),
    ] {
        paths.insert(format!("apis/{}/{}", group, version), path_entry(&format!("apis/{}/{}", group, version)));
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
/// kubectl --validate fetches this to validate resources; the response MUST be
/// valid JSON with an explicit Content-Type so kubectl does not fall back to protobuf.
/// kubectl may send Accept: application/com.github.proto-openapi.spec.v2@v1.0+protobuf
/// but we always respond with JSON.
pub async fn get_swagger_spec(_headers: HeaderMap) -> Response {
    // Always return JSON OpenAPI spec regardless of Accept header.
    // kubectl sends Accept: application/com.github.proto-openapi.spec.v2@v1.0+protobuf
    // but can parse JSON just fine. Returning 406 breaks kubectl --validate.
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
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json_bytes))
        .unwrap()
}
