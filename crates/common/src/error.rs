use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Resource already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid resource: {0}")]
    InvalidResource(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(feature = "axum-support")]
impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;
        use serde_json::json;

        let (status, message) = match self {
            Error::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Error::AlreadyExists(msg) => (StatusCode::CONFLICT, msg),
            Error::InvalidResource(msg) => (StatusCode::BAD_REQUEST, msg),
            Error::Authentication(msg) => (StatusCode::UNAUTHORIZED, msg),
            Error::Authorization(msg) => (StatusCode::FORBIDDEN, msg),
            Error::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            Error::Conflict(msg) => (StatusCode::CONFLICT, msg),
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
