/// OpenAPI specification handler
use crate::openapi::generate_openapi_spec;
use axum::Json;

/// GET /openapi/v3
/// Get the OpenAPI v3 specification
pub async fn get_openapi_spec() -> Json<openapiv3::OpenAPI> {
    Json(generate_openapi_spec())
}

/// GET /swagger.json (v2 compatibility)
/// Get the OpenAPI v2 (Swagger) specification
/// This is a placeholder - Kubernetes still supports v2 for some clients
pub async fn get_swagger_spec() -> Json<serde_json::Value> {
    // For now, return a minimal v2 spec
    // In a complete implementation, this would convert the v3 spec to v2
    Json(serde_json::json!({
        "swagger": "2.0",
        "info": {
            "title": "Rusternetes Kubernetes API",
            "version": "v1.35.0"
        },
        "paths": {}
    }))
}
