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

    #[error("Too many requests: {0}")]
    TooManyRequests(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Returns the machine-readable reason string matching Kubernetes StatusReason values
    pub fn reason(&self) -> &str {
        match self {
            Error::NotFound(_) => "NotFound",
            Error::AlreadyExists(_) => "AlreadyExists",
            Error::InvalidResource(_) => "Invalid",
            Error::Serialization(_) => "BadRequest",
            Error::Storage(_) => "InternalError",
            Error::Network(_) => "ServiceUnavailable",
            Error::Authentication(_) => "Unauthorized",
            Error::Authorization(_) => "Forbidden",
            Error::Forbidden(_) => "Forbidden",
            Error::Conflict(_) => "Conflict",
            Error::TooManyRequests(_) => "TooManyRequests",
            Error::Internal(_) => "InternalError",
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(feature = "axum-support")]
impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;

        let (status, message) = match self {
            Error::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Error::AlreadyExists(msg) => (StatusCode::CONFLICT, msg),
            Error::InvalidResource(msg) => (StatusCode::BAD_REQUEST, msg),
            Error::Authentication(msg) => (StatusCode::UNAUTHORIZED, msg),
            Error::Authorization(msg) => (StatusCode::FORBIDDEN, msg),
            Error::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            Error::Conflict(msg) => (StatusCode::CONFLICT, msg),
            Error::TooManyRequests(msg) => (StatusCode::TOO_MANY_REQUESTS, msg),
            Error::Storage(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Error::Network(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            Error::Serialization(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            Error::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        // Reconstruct reason from the status code (we consumed self above)
        let reason = match status {
            StatusCode::NOT_FOUND => "NotFound",
            StatusCode::CONFLICT => "Conflict",
            StatusCode::BAD_REQUEST => "Invalid",
            StatusCode::UNAUTHORIZED => "Unauthorized",
            StatusCode::FORBIDDEN => "Forbidden",
            StatusCode::TOO_MANY_REQUESTS => "TooManyRequests",
            StatusCode::SERVICE_UNAVAILABLE => "ServiceUnavailable",
            _ => "InternalError",
        };

        let status_obj = crate::types::Status::failure(&message, reason, status.as_u16());

        (status, Json(status_obj)).into_response()
    }
}
