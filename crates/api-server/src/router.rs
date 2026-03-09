use crate::{handlers, state::ApiServerState};
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub fn build_router(state: Arc<ApiServerState>) -> Router {
    Router::new()
        // Health check
        .route("/healthz", get(handlers::health::healthz))
        .route("/readyz", get(handlers::health::readyz))
        // Core v1 API
        .route("/api/v1/namespaces", get(handlers::namespace::list))
        .route("/api/v1/namespaces", post(handlers::namespace::create))
        .route(
            "/api/v1/namespaces/:name",
            get(handlers::namespace::get)
                .put(handlers::namespace::update)
                .delete(handlers::namespace::delete_ns),
        )
        // Pods
        .route(
            "/api/v1/namespaces/:namespace/pods",
            get(handlers::pod::list).post(handlers::pod::create),
        )
        .route(
            "/api/v1/namespaces/:namespace/pods/:name",
            get(handlers::pod::get)
                .put(handlers::pod::update)
                .delete(handlers::pod::delete_pod),
        )
        // Services
        .route(
            "/api/v1/namespaces/:namespace/services",
            get(handlers::service::list).post(handlers::service::create),
        )
        .route(
            "/api/v1/namespaces/:namespace/services/:name",
            get(handlers::service::get)
                .put(handlers::service::update)
                .delete(handlers::service::delete_service),
        )
        // Nodes
        .route(
            "/api/v1/nodes",
            get(handlers::node::list).post(handlers::node::create),
        )
        .route(
            "/api/v1/nodes/:name",
            get(handlers::node::get)
                .put(handlers::node::update)
                .delete(handlers::node::delete_node),
        )
        // Apps v1 API - Deployments
        .route(
            "/apis/apps/v1/namespaces/:namespace/deployments",
            get(handlers::deployment::list).post(handlers::deployment::create),
        )
        .route(
            "/apis/apps/v1/namespaces/:namespace/deployments/:name",
            get(handlers::deployment::get)
                .put(handlers::deployment::update)
                .delete(handlers::deployment::delete_deployment),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
