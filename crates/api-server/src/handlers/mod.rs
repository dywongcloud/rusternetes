pub mod health;
pub mod namespace;
pub mod pod;
pub mod service;
pub mod node;
pub mod deployment;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rusternetes_common::Error;
use serde_json::json;

/// Convert our Error type into an HTTP response
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Error::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Error::AlreadyExists(msg) => (StatusCode::CONFLICT, msg),
            Error::InvalidResource(msg) => (StatusCode::BAD_REQUEST, msg),
            Error::Authentication(msg) => (StatusCode::UNAUTHORIZED, msg),
            Error::Authorization(msg) => (StatusCode::FORBIDDEN, msg),
            Error::Storage(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Error::Network(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            Error::Serialization(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            Error::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "kind": "Status",
            "apiVersion": "v1",
            "status": "Failure",
            "message": message,
            "code": status.as_u16(),
        }));

        (status, body).into_response()
    }
}
