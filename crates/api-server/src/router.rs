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
        .route("/healthz/verbose", get(handlers::health::healthz_verbose))
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
                .patch(handlers::namespace::patch)
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
                .patch(handlers::pod::patch)
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
                .patch(handlers::service::patch)
                .delete(handlers::service::delete_service),
        )
        // Endpoints
        .route(
            "/api/v1/namespaces/:namespace/endpoints",
            get(handlers::endpoints::list_endpoints).post(handlers::endpoints::create_endpoints),
        )
        .route(
            "/api/v1/namespaces/:namespace/endpoints/:name",
            get(handlers::endpoints::get_endpoints)
                .put(handlers::endpoints::update_endpoints)
                .patch(handlers::endpoints::patch_endpoints)
                .delete(handlers::endpoints::delete_endpoints),
        )
        .route(
            "/api/v1/endpoints",
            get(handlers::endpoints::list_all_endpoints),
        )
        // ConfigMaps
        .route(
            "/api/v1/namespaces/:namespace/configmaps",
            get(handlers::configmap::list).post(handlers::configmap::create),
        )
        .route(
            "/api/v1/namespaces/:namespace/configmaps/:name",
            get(handlers::configmap::get)
                .put(handlers::configmap::update)
                .patch(handlers::configmap::patch)
                .delete(handlers::configmap::delete_configmap),
        )
        // Secrets
        .route(
            "/api/v1/namespaces/:namespace/secrets",
            get(handlers::secret::list).post(handlers::secret::create),
        )
        .route(
            "/api/v1/namespaces/:namespace/secrets/:name",
            get(handlers::secret::get)
                .put(handlers::secret::update)
                .patch(handlers::secret::patch)
                .delete(handlers::secret::delete_secret),
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
                .patch(handlers::node::patch)
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
                .patch(handlers::deployment::patch)
                .delete(handlers::deployment::delete_deployment),
        )
        // Apps v1 API - StatefulSets
        .route(
            "/apis/apps/v1/namespaces/:namespace/statefulsets",
            get(handlers::statefulset::list).post(handlers::statefulset::create),
        )
        .route(
            "/apis/apps/v1/namespaces/:namespace/statefulsets/:name",
            get(handlers::statefulset::get)
                .put(handlers::statefulset::update)
                .patch(handlers::statefulset::patch)
                .delete(handlers::statefulset::delete_statefulset),
        )
        // Apps v1 API - DaemonSets
        .route(
            "/apis/apps/v1/namespaces/:namespace/daemonsets",
            get(handlers::daemonset::list).post(handlers::daemonset::create),
        )
        .route(
            "/apis/apps/v1/namespaces/:namespace/daemonsets/:name",
            get(handlers::daemonset::get)
                .put(handlers::daemonset::update)
                .patch(handlers::daemonset::patch)
                .delete(handlers::daemonset::delete_daemonset),
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
                .patch(handlers::job::patch)
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
                .patch(handlers::cronjob::patch)
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
                .patch(handlers::service_account::patch)
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
                .patch(handlers::rbac::patch_role)
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
                .patch(handlers::rbac::patch_rolebinding)
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
                .patch(handlers::rbac::patch_clusterrole)
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
                .patch(handlers::rbac::patch_clusterrolebinding)
                .delete(handlers::rbac::delete_clusterrolebinding),
        )
        // Storage v1 API - PersistentVolumes (cluster-scoped)
        .route(
            "/api/v1/persistentvolumes",
            get(handlers::persistentvolume::list_pvs).post(handlers::persistentvolume::create_pv),
        )
        .route(
            "/api/v1/persistentvolumes/:name",
            get(handlers::persistentvolume::get_pv)
                .put(handlers::persistentvolume::update_pv)
                .patch(handlers::persistentvolume::patch_pv)
                .delete(handlers::persistentvolume::delete_pv),
        )
        // PersistentVolumeClaims (namespace-scoped)
        .route(
            "/api/v1/namespaces/:namespace/persistentvolumeclaims",
            get(handlers::persistentvolumeclaim::list_pvcs).post(handlers::persistentvolumeclaim::create_pvc),
        )
        .route(
            "/api/v1/namespaces/:namespace/persistentvolumeclaims/:name",
            get(handlers::persistentvolumeclaim::get_pvc)
                .put(handlers::persistentvolumeclaim::update_pvc)
                .patch(handlers::persistentvolumeclaim::patch_pvc)
                .delete(handlers::persistentvolumeclaim::delete_pvc),
        )
        // StorageClasses (cluster-scoped)
        .route(
            "/apis/storage.k8s.io/v1/storageclasses",
            get(handlers::storageclass::list_storageclasses).post(handlers::storageclass::create_storageclass),
        )
        .route(
            "/apis/storage.k8s.io/v1/storageclasses/:name",
            get(handlers::storageclass::get_storageclass)
                .put(handlers::storageclass::update_storageclass)
                .patch(handlers::storageclass::patch_storageclass)
                .delete(handlers::storageclass::delete_storageclass),
        )
        // Networking v1 API - Ingresses
        .route(
            "/apis/networking.k8s.io/v1/namespaces/:namespace/ingresses",
            get(handlers::ingress::list).post(handlers::ingress::create),
        )
        .route(
            "/apis/networking.k8s.io/v1/namespaces/:namespace/ingresses/:name",
            get(handlers::ingress::get)
                .put(handlers::ingress::update)
                .patch(handlers::ingress::patch)
                .delete(handlers::ingress::delete_ingress),
        )
        // Snapshot storage API - VolumeSnapshotClasses (cluster-scoped)
        .route(
            "/apis/snapshot.storage.k8s.io/v1/volumesnapshotclasses",
            get(handlers::volumesnapshotclass::list_volumesnapshotclasses)
                .post(handlers::volumesnapshotclass::create_volumesnapshotclass),
        )
        .route(
            "/apis/snapshot.storage.k8s.io/v1/volumesnapshotclasses/:name",
            get(handlers::volumesnapshotclass::get_volumesnapshotclass)
                .put(handlers::volumesnapshotclass::update_volumesnapshotclass)
                .patch(handlers::volumesnapshotclass::patch_volumesnapshotclass)
                .delete(handlers::volumesnapshotclass::delete_volumesnapshotclass),
        )
        // VolumeSnapshots (namespace-scoped)
        .route(
            "/apis/snapshot.storage.k8s.io/v1/namespaces/:namespace/volumesnapshots",
            get(handlers::volumesnapshot::list_volumesnapshots)
                .post(handlers::volumesnapshot::create_volumesnapshot),
        )
        .route(
            "/apis/snapshot.storage.k8s.io/v1/namespaces/:namespace/volumesnapshots/:name",
            get(handlers::volumesnapshot::get_volumesnapshot)
                .put(handlers::volumesnapshot::update_volumesnapshot)
                .patch(handlers::volumesnapshot::patch_volumesnapshot)
                .delete(handlers::volumesnapshot::delete_volumesnapshot),
        )
        // VolumeSnapshots (all namespaces)
        .route(
            "/apis/snapshot.storage.k8s.io/v1/volumesnapshots",
            get(handlers::volumesnapshot::list_all_volumesnapshots),
        )
        // VolumeSnapshotContents (cluster-scoped)
        .route(
            "/apis/snapshot.storage.k8s.io/v1/volumesnapshotcontents",
            get(handlers::volumesnapshotcontent::list_volumesnapshotcontents)
                .post(handlers::volumesnapshotcontent::create_volumesnapshotcontent),
        )
        .route(
            "/apis/snapshot.storage.k8s.io/v1/volumesnapshotcontents/:name",
            get(handlers::volumesnapshotcontent::get_volumesnapshotcontent)
                .put(handlers::volumesnapshotcontent::update_volumesnapshotcontent)
                .patch(handlers::volumesnapshotcontent::patch_volumesnapshotcontent)
                .delete(handlers::volumesnapshotcontent::delete_volumesnapshotcontent),
        )
        // Events (namespace-scoped)
        .route(
            "/api/v1/namespaces/:namespace/events",
            get(handlers::event::list).post(handlers::event::create),
        )
        .route(
            "/api/v1/namespaces/:namespace/events/:name",
            get(handlers::event::get)
                .put(handlers::event::update)
                .patch(handlers::event::patch)
                .delete(handlers::event::delete),
        )
        // Events (all namespaces)
        .route(
            "/api/v1/events",
            get(handlers::event::list_all),
        )
        // ResourceQuotas (namespace-scoped)
        .route(
            "/api/v1/namespaces/:namespace/resourcequotas",
            get(handlers::resourcequota::list).post(handlers::resourcequota::create),
        )
        .route(
            "/api/v1/namespaces/:namespace/resourcequotas/:name",
            get(handlers::resourcequota::get)
                .put(handlers::resourcequota::update)
                .patch(handlers::resourcequota::patch)
                .delete(handlers::resourcequota::delete),
        )
        // ResourceQuotas (all namespaces)
        .route(
            "/api/v1/resourcequotas",
            get(handlers::resourcequota::list_all),
        )
        // LimitRanges (namespace-scoped)
        .route(
            "/api/v1/namespaces/:namespace/limitranges",
            get(handlers::limitrange::list).post(handlers::limitrange::create),
        )
        .route(
            "/api/v1/namespaces/:namespace/limitranges/:name",
            get(handlers::limitrange::get)
                .put(handlers::limitrange::update)
                .patch(handlers::limitrange::patch)
                .delete(handlers::limitrange::delete),
        )
        // LimitRanges (all namespaces)
        .route(
            "/api/v1/limitranges",
            get(handlers::limitrange::list_all),
        )
        // PriorityClasses (cluster-scoped)
        .route(
            "/apis/scheduling.k8s.io/v1/priorityclasses",
            get(handlers::priorityclass::list).post(handlers::priorityclass::create),
        )
        .route(
            "/apis/scheduling.k8s.io/v1/priorityclasses/:name",
            get(handlers::priorityclass::get)
                .put(handlers::priorityclass::update)
                .patch(handlers::priorityclass::patch)
                .delete(handlers::priorityclass::delete),
        )
        // CustomResourceDefinitions (cluster-scoped)
        .route(
            "/apis/apiextensions.k8s.io/v1/customresourcedefinitions",
            get(handlers::crd::list_crds).post(handlers::crd::create_crd),
        )
        .route(
            "/apis/apiextensions.k8s.io/v1/customresourcedefinitions/:name",
            get(handlers::crd::get_crd)
                .put(handlers::crd::update_crd)
                .delete(handlers::crd::delete_crd),
        )
        // ValidatingWebhookConfiguration (cluster-scoped)
        .route(
            "/apis/admissionregistration.k8s.io/v1/validatingwebhookconfigurations",
            get(handlers::admission_webhook::list_validating_webhooks)
                .post(handlers::admission_webhook::create_validating_webhook),
        )
        .route(
            "/apis/admissionregistration.k8s.io/v1/validatingwebhookconfigurations/:name",
            get(handlers::admission_webhook::get_validating_webhook)
                .put(handlers::admission_webhook::update_validating_webhook)
                .patch(handlers::admission_webhook::patch_validating_webhook)
                .delete(handlers::admission_webhook::delete_validating_webhook),
        )
        // MutatingWebhookConfiguration (cluster-scoped)
        .route(
            "/apis/admissionregistration.k8s.io/v1/mutatingwebhookconfigurations",
            get(handlers::admission_webhook::list_mutating_webhooks)
                .post(handlers::admission_webhook::create_mutating_webhook),
        )
        .route(
            "/apis/admissionregistration.k8s.io/v1/mutatingwebhookconfigurations/:name",
            get(handlers::admission_webhook::get_mutating_webhook)
                .put(handlers::admission_webhook::update_mutating_webhook)
                .patch(handlers::admission_webhook::patch_mutating_webhook)
                .delete(handlers::admission_webhook::delete_mutating_webhook),
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
