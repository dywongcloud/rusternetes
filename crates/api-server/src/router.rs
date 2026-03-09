use crate::{handlers, middleware, state::ApiServerState};
use axum::{
    middleware as axum_middleware, routing::{get, post}, Extension, Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub fn build_router(state: Arc<ApiServerState>) -> Router {
    let skip_auth = state.skip_auth;

    // Routes that don't require authentication
    let public_routes = Router::new()
        .route("/healthz", get(handlers::health::healthz))
        .route("/readyz", get(handlers::health::readyz))
        .route("/metrics", get(handlers::health::metrics));

    // Routes that require authentication (unless skip_auth is enabled)
    let mut protected_routes = Router::new()
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
        // Batch v1 API - Jobs
        .route(
            "/apis/batch/v1/namespaces/:namespace/jobs",
            get(handlers::job::list).post(handlers::job::create),
        )
        .route(
            "/apis/batch/v1/namespaces/:namespace/jobs/:name",
            get(handlers::job::get)
                .put(handlers::job::update)
                .delete(handlers::job::delete_job),
        )
        // Batch v1 API - CronJobs
        .route(
            "/apis/batch/v1/namespaces/:namespace/cronjobs",
            get(handlers::cronjob::list).post(handlers::cronjob::create),
        )
        .route(
            "/apis/batch/v1/namespaces/:namespace/cronjobs/:name",
            get(handlers::cronjob::get)
                .put(handlers::cronjob::update)
                .delete(handlers::cronjob::delete_cronjob),
        )
        // ServiceAccounts
        .route(
            "/api/v1/namespaces/:namespace/serviceaccounts",
            get(handlers::service_account::list).post(handlers::service_account::create),
        )
        .route(
            "/api/v1/namespaces/:namespace/serviceaccounts/:name",
            get(handlers::service_account::get)
                .put(handlers::service_account::update)
                .delete(handlers::service_account::delete_service_account),
        )
        // RBAC - Roles
        .route(
            "/apis/rbac.authorization.k8s.io/v1/namespaces/:namespace/roles",
            get(handlers::rbac::list_roles).post(handlers::rbac::create_role),
        )
        .route(
            "/apis/rbac.authorization.k8s.io/v1/namespaces/:namespace/roles/:name",
            get(handlers::rbac::get_role)
                .put(handlers::rbac::update_role)
                .delete(handlers::rbac::delete_role),
        )
        // RBAC - RoleBindings
        .route(
            "/apis/rbac.authorization.k8s.io/v1/namespaces/:namespace/rolebindings",
            get(handlers::rbac::list_rolebindings).post(handlers::rbac::create_rolebinding),
        )
        .route(
            "/apis/rbac.authorization.k8s.io/v1/namespaces/:namespace/rolebindings/:name",
            get(handlers::rbac::get_rolebinding)
                .put(handlers::rbac::update_rolebinding)
                .delete(handlers::rbac::delete_rolebinding),
        )
        // RBAC - ClusterRoles
        .route(
            "/apis/rbac.authorization.k8s.io/v1/clusterroles",
            get(handlers::rbac::list_clusterroles).post(handlers::rbac::create_clusterrole),
        )
        .route(
            "/apis/rbac.authorization.k8s.io/v1/clusterroles/:name",
            get(handlers::rbac::get_clusterrole)
                .put(handlers::rbac::update_clusterrole)
                .delete(handlers::rbac::delete_clusterrole),
        )
        // RBAC - ClusterRoleBindings
        .route(
            "/apis/rbac.authorization.k8s.io/v1/clusterrolebindings",
            get(handlers::rbac::list_clusterrolebindings).post(handlers::rbac::create_clusterrolebinding),
        )
        .route(
            "/apis/rbac.authorization.k8s.io/v1/clusterrolebindings/:name",
            get(handlers::rbac::get_clusterrolebinding)
                .put(handlers::rbac::update_clusterrolebinding)
                .delete(handlers::rbac::delete_clusterrolebinding),
        );

    // Conditionally apply authentication middleware
    if skip_auth {
        // In skip-auth mode, inject a default admin user context
        protected_routes = protected_routes
            .layer(axum_middleware::from_fn(middleware::skip_auth_middleware));
    } else {
        // In normal mode, apply full authentication
        protected_routes = protected_routes
            .layer(axum_middleware::from_fn(middleware::auth_middleware))
            .layer(Extension(state.token_manager.clone()));
    }

    // Combine routes and add shared state
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
