use crate::state::ApiServerState;
use axum::{extract::State, http::StatusCode};
use std::sync::Arc;

/// Health check endpoint
pub async fn healthz() -> StatusCode {
    StatusCode::OK
}

/// Readiness check endpoint
pub async fn readyz() -> StatusCode {
    StatusCode::OK
}

/// Metrics endpoint - returns Prometheus metrics
pub async fn metrics(State(state): State<Arc<ApiServerState>>) -> String {
    state.metrics.gather()
}
