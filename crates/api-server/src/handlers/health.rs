use axum::http::StatusCode;

/// Health check endpoint
pub async fn healthz() -> StatusCode {
    StatusCode::OK
}

/// Readiness check endpoint
pub async fn readyz() -> StatusCode {
    StatusCode::OK
}
