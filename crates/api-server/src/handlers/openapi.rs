/// OpenAPI specification handler
use crate::openapi::generate_openapi_spec;
use axum::{
    body::Body,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};

/// GET /openapi/v3
/// Get the OpenAPI v3 specification
/// Explicitly returns application/json to prevent kubectl from attempting protobuf decode.
pub async fn get_openapi_spec() -> Response {
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
pub async fn get_swagger_spec(headers: HeaderMap) -> Response {
    // Check if client ONLY accepts protobuf (no JSON fallback)
    if let Some(accept) = headers.get(header::ACCEPT).and_then(|v| v.to_str().ok()) {
        if accept.contains("protobuf") && !accept.contains("json") && !accept.contains("*/*") {
            // Return 406 — we don't support protobuf OpenAPI encoding
            return Response::builder()
                .status(StatusCode::NOT_ACCEPTABLE)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"kind":"Status","apiVersion":"v1","status":"Failure","message":"only JSON OpenAPI spec is available","reason":"NotAcceptable","code":406}"#))
                .unwrap();
        }
    }

    // Return JSON OpenAPI spec
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
