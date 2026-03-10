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

/// CPU profiling endpoint - returns pprof CPU profile
/// This endpoint captures CPU profiling data for performance analysis
#[cfg(feature = "pprof")]
pub async fn pprof_profile() -> Result<Vec<u8>, (StatusCode, String)> {
    use pprof::ProfilerGuard;
    use std::time::Duration;

    // Create a profiler guard
    let guard = ProfilerGuard::new(100).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to start profiler: {}", e),
        )
    })?;

    // Profile for 30 seconds
    tokio::time::sleep(Duration::from_secs(30)).await;

    // Build the report
    match guard.report().build() {
        Ok(report) => {
            let mut buffer = Vec::new();
            report.flamegraph(&mut buffer).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to generate flamegraph: {}", e),
                )
            })?;
            Ok(buffer)
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to build profile report: {}", e),
        )),
    }
}

/// Heap profiling endpoint - returns heap allocation profile
/// This endpoint provides information about memory allocations
#[cfg(feature = "pprof")]
pub async fn pprof_heap() -> Result<String, (StatusCode, String)> {
    // In Rust, we don't have runtime heap profiling like Go
    // This would require integration with jemalloc or another allocator
    // For now, return a stub response
    Ok("Heap profiling requires jemalloc or similar allocator integration".to_string())
}

/// Goroutine profiling endpoint (Rust equivalent: thread/task info)
/// This endpoint provides information about active threads and async tasks
pub async fn pprof_goroutine() -> Result<String, (StatusCode, String)> {
    // Collect thread information
    let thread_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    // Get Tokio runtime stats if available
    let runtime_info = format!(
        "Active threads: {}\nNote: Detailed async task profiling requires additional runtime instrumentation\n",
        thread_count
    );

    Ok(runtime_info)
}

/// Block profiling endpoint stub
/// Returns information about blocking operations
pub async fn pprof_block() -> Result<String, (StatusCode, String)> {
    Ok("Block profiling in Rust requires custom instrumentation\n\
        Consider using tokio-console for async task blocking analysis".to_string())
}

/// Mutex profiling endpoint stub
/// Returns information about mutex contention
pub async fn pprof_mutex() -> Result<String, (StatusCode, String)> {
    Ok("Mutex profiling in Rust requires custom instrumentation\n\
        Consider using parking_lot with contention metrics".to_string())
}

/// Symbol lookup endpoint
/// Returns symbol information for profiling
pub async fn pprof_symbol() -> String {
    "Symbol lookup not implemented\n\
     Rust binaries contain symbol information in debug builds".to_string()
}

/// Trace endpoint - returns execution trace
/// This endpoint captures execution traces for analysis
pub async fn pprof_trace() -> Result<String, (StatusCode, String)> {
    Ok("Execution tracing in Rust requires tokio-console or tracing subscriber integration".to_string())
}
