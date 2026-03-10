use crate::state::ApiServerState;
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    status: String,
    checks: Vec<ComponentHealth>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentHealth {
    name: String,
    status: String,
    message: Option<String>,
}

/// Health check endpoint - liveness probe
/// Returns OK if the server is running
pub async fn healthz() -> StatusCode {
    StatusCode::OK
}

/// Readiness check endpoint - readiness probe
/// Returns OK if the server is ready to accept requests
/// Checks:
/// - Storage (etcd) connectivity
pub async fn readyz(State(state): State<Arc<ApiServerState>>) -> (StatusCode, Json<HealthStatus>) {
    let mut checks = Vec::new();
    let mut all_healthy = true;

    // Check storage connectivity
    match check_storage(&state).await {
        Ok(_) => {
            checks.push(ComponentHealth {
                name: "storage".to_string(),
                status: "ok".to_string(),
                message: None,
            });
        }
        Err(e) => {
            all_healthy = false;
            checks.push(ComponentHealth {
                name: "storage".to_string(),
                status: "failed".to_string(),
                message: Some(e),
            });
        }
    }

    let status = if all_healthy {
        HealthStatus {
            status: "ok".to_string(),
            checks,
        }
    } else {
        HealthStatus {
            status: "degraded".to_string(),
            checks,
        }
    };

    let status_code = if all_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(status))
}

/// Detailed health check endpoint
/// Returns detailed status of all components
pub async fn healthz_verbose(State(state): State<Arc<ApiServerState>>) -> (StatusCode, Json<HealthStatus>) {
    let mut checks = Vec::new();
    let mut all_healthy = true;

    // Check storage
    match check_storage(&state).await {
        Ok(_) => {
            checks.push(ComponentHealth {
                name: "storage".to_string(),
                status: "ok".to_string(),
                message: Some("etcd connection healthy".to_string()),
            });
        }
        Err(e) => {
            all_healthy = false;
            checks.push(ComponentHealth {
                name: "storage".to_string(),
                status: "failed".to_string(),
                message: Some(e),
            });
        }
    }

    // Add metrics status
    checks.push(ComponentHealth {
        name: "metrics".to_string(),
        status: "ok".to_string(),
        message: Some("metrics collection active".to_string()),
    });

    let status = if all_healthy {
        HealthStatus {
            status: "healthy".to_string(),
            checks,
        }
    } else {
        HealthStatus {
            status: "unhealthy".to_string(),
            checks,
        }
    };

    let status_code = if all_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(status))
}

/// Check storage connectivity by listing namespaces
async fn check_storage(state: &Arc<ApiServerState>) -> Result<(), String> {
    use rusternetes_common::resources::namespace::Namespace;
    use rusternetes_storage::Storage;

    // Try to list namespaces as a health check
    state
        .storage
        .list::<Namespace>("/registry/namespaces/")
        .await
        .map_err(|e| format!("Storage check failed: {}", e))?;

    Ok(())
}

/// Metrics endpoint - returns Prometheus metrics
pub async fn metrics(State(state): State<Arc<ApiServerState>>) -> String {
    state.metrics.gather()
}
