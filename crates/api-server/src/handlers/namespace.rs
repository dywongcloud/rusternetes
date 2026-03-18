use crate::{handlers::watch, middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, Json,
};
use rusternetes_common::{
    auth::ServiceAccountClaims,
    authz::{Decision, RequestAttributes},
    resources::{Namespace, Secret, ServiceAccount},
    List, Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

// Removed - using HashMap<String, String> for query params

pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut namespace): Json<Namespace>,
) -> Result<(StatusCode, Json<Namespace>)> {
    info!("Creating namespace: {}", namespace.metadata.name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "namespaces").with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Enrich metadata with system fields
    namespace.metadata.ensure_uid();
    namespace.metadata.ensure_creation_timestamp();

    let key = build_key("namespaces", None, &namespace.metadata.name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!(
            "Dry-run: Namespace {} validated successfully (not created)",
            namespace.metadata.name
        );
        return Ok((StatusCode::CREATED, Json(namespace)));
    }

    let created = state.storage.create(&key, &namespace).await?;

    // Automatically create default ServiceAccount in the new namespace
    let ns_name = created.metadata.name.clone();
    info!("Creating default ServiceAccount for namespace: {}", ns_name);

    match create_default_service_account(&state, &ns_name).await {
        Ok(_) => {
            info!(
                "Successfully created default ServiceAccount for namespace: {}",
                ns_name
            );
        }
        Err(e) => {
            tracing::warn!(
                "Failed to create default ServiceAccount for namespace {}: {}",
                ns_name,
                e
            );
            // Don't fail namespace creation if default SA creation fails
        }
    }

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<Namespace>> {
    info!("Getting namespace: {}", name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "namespaces")
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("namespaces", None, &name);
    let namespace = state.storage.get(&key).await?;

    Ok(Json(namespace))
}

pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut namespace): Json<Namespace>,
) -> Result<Json<Namespace>> {
    info!("Updating namespace: {}", name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "namespaces")
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    namespace.metadata.name = name.clone();

    let key = build_key("namespaces", None, &name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!(
            "Dry-run: Namespace {} validated successfully (not updated)",
            name
        );
        return Ok(Json(namespace));
    }

    // Try to update first, if not found then create (upsert behavior)
    let result = match state.storage.update(&key, &namespace).await {
        Ok(updated) => updated,
        Err(rusternetes_common::Error::NotFound(_)) => {
            state.storage.create(&key, &namespace).await?
        }
        Err(e) => return Err(e),
    };

    Ok(Json(result))
}

pub async fn delete_ns(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Namespace>> {
    info!("Deleting namespace: {}", name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "namespaces")
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("namespaces", None, &name);

    // Get the namespace to check for finalizers
    let namespace: Namespace = state.storage.get(&key).await?;

    // If dry-run, skip delete operation
    if is_dry_run {
        info!(
            "Dry-run: Namespace {} validated successfully (not deleted)",
            name
        );
        return Ok(Json(namespace));
    }

    // CASCADE DELETE: Delete all resources in this namespace before deleting the namespace
    info!("Cascade deleting all resources in namespace: {}", name);
    cascade_delete_namespace_resources(&*state.storage, &name).await?;

    // Handle deletion with finalizers
    // If the namespace has finalizers, it will be marked for deletion (deletionTimestamp set)
    // and remain in storage until controllers remove the finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &namespace,
    )
    .await?;

    if deleted_immediately {
        info!("Namespace {} deleted successfully (no finalizers)", name);
        Ok(Json(namespace))
    } else {
        // Resource has finalizers, re-read to get updated version with deletionTimestamp
        let updated: Namespace = state.storage.get(&key).await?;
        Ok(Json(updated))
    }
}

/// Cascade delete all resources in a namespace
/// This ensures proper cleanup when a namespace is deleted
async fn cascade_delete_namespace_resources(
    storage: &rusternetes_storage::etcd::EtcdStorage,
    namespace: &str,
) -> Result<()> {
    use serde_json::Value;
    use tracing::warn;

    // List of resource types that are namespace-scoped and should be deleted
    // Order matters: delete child resources before parent resources
    let resource_types = vec![
        "events",
        "endpoints",
        "endpointslices",
        "configmaps",
        "secrets",
        "serviceaccounts",
        "persistentvolumeclaims",
        "pods",
        "replicationcontrollers",
        "services",
        "daemonsets",
        "deployments",
        "replicasets",
        "statefulsets",
        "jobs",
        "cronjobs",
        "ingresses",
        "networkpolicies",
        "poddisruptionbudgets",
        "resourcequotas",
        "limitranges",
        "horizontalpodautoscalers",
        "volumesnapshots",
        "leases",
        "rolebindings",
        "roles",
    ];

    let mut total_deleted = 0;
    for resource_type in resource_types {
        let prefix = build_prefix(resource_type, Some(namespace));

        // List all resources with this prefix, then delete each one
        match storage.list::<Value>(&prefix).await {
            Ok(resources) => {
                let count = resources.len();
                for resource in resources {
                    // Extract the resource name from the metadata
                    if let Some(metadata) = resource.get("metadata") {
                        if let Some(name) = metadata.get("name").and_then(|n| n.as_str()) {
                            let key = build_key(resource_type, Some(namespace), name);
                            match storage.delete(&key).await {
                                Ok(_) => {
                                    total_deleted += 1;
                                }
                                Err(e) => {
                                    warn!(
                                        "Failed to delete {} {}/{}: {}",
                                        resource_type, namespace, name, e
                                    );
                                }
                            }
                        }
                    }
                }
                if count > 0 {
                    info!(
                        "Deleted {} {} resources in namespace {}",
                        count, resource_type, namespace
                    );
                }
            }
            Err(e) => {
                // Log warning but continue - resource type might not exist or have no resources
                warn!(
                    "Failed to list {} in namespace {}: {}",
                    resource_type, namespace, e
                );
            }
        }
    }

    info!(
        "Cascade deletion completed for namespace {}: {} resources deleted",
        namespace, total_deleted
    );
    Ok(())
}

pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response> {
    // Check if this is a watch request
    if params
        .get("watch")
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false)
    {
        info!("Watching namespaces");
        // Parse WatchParams from the query parameters
        let watch_params = watch::WatchParams {
            resource_version: params.get("resourceVersion").map(|s| s.clone()),
            timeout_seconds: params
                .get("timeoutSeconds")
                .and_then(|v| v.parse::<u64>().ok()),
            label_selector: params.get("labelSelector").map(|s| s.clone()),
            field_selector: params.get("fieldSelector").map(|s| s.clone()),
            watch: Some(true),
            allow_watch_bookmarks: params
                .get("allowWatchBookmarks")
                .and_then(|v| v.parse::<bool>().ok()),
        };
        return watch::watch_cluster_scoped::<Namespace>(
            state,
            auth_ctx,
            "namespaces",
            "",
            watch_params,
        )
        .await;
    }

    info!("Listing namespaces");

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "namespaces").with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("namespaces", None);
    let mut namespaces = state.storage.list::<Namespace>(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut namespaces, &params)?;

    let list = List::new("NamespaceList", "v1", namespaces);
    Ok(Json(list).into_response())
}

/// Helper function to create the default ServiceAccount in a namespace
/// This replicates what Kubernetes does automatically when a namespace is created
async fn create_default_service_account(
    state: &Arc<ApiServerState>,
    namespace: &str,
) -> Result<()> {
    let sa_name = "default";

    // Create default ServiceAccount
    let mut service_account = ServiceAccount::new(sa_name, namespace);
    service_account.metadata.ensure_uid();
    service_account.metadata.ensure_creation_timestamp();

    let sa_key = build_key("serviceaccounts", Some(namespace), sa_name);
    let created_sa = state.storage.create(&sa_key, &service_account).await?;

    // Generate ServiceAccount token and store it in a Secret
    let sa_uid = created_sa.metadata.uid.clone();

    // Generate JWT token (valid for 10 years - Kubernetes default for static tokens)
    let claims = ServiceAccountClaims::new(
        sa_name.to_string(),
        namespace.to_string(),
        sa_uid.clone(),
        87600, // 10 years in hours
    );

    let token = state.token_manager.generate_token(claims)?;

    // Create Secret to store the token
    let secret_name = format!("{}-token", sa_name);
    let mut string_data = HashMap::new();
    string_data.insert("token".to_string(), token);
    string_data.insert("namespace".to_string(), namespace.to_string());

    let mut secret =
        Secret::new(&secret_name, namespace).with_type("kubernetes.io/service-account-token");

    // Add labels and annotations
    secret.metadata.labels = Some({
        let mut labels = HashMap::new();
        labels.insert(
            "kubernetes.io/service-account.name".to_string(),
            sa_name.to_string(),
        );
        labels
    });
    secret.metadata.annotations = Some({
        let mut annotations = HashMap::new();
        annotations.insert("kubernetes.io/service-account.uid".to_string(), sa_uid);
        annotations
    });
    secret.string_data = Some(string_data);

    // Normalize: convert stringData to base64-encoded data before storing
    secret.normalize();

    // Store the secret
    let secret_key = build_key("secrets", Some(namespace), &secret_name);
    state.storage.create(&secret_key, &secret).await?;

    info!(
        "Created default ServiceAccount and token secret in namespace: {}",
        namespace
    );
    Ok(())
}

// Use the macro to create a PATCH handler for cluster-scoped namespace
crate::patch_handler_cluster!(patch, Namespace, "namespaces", "");

pub async fn deletecollection_namespaces(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("DeleteCollection namespaces with params: {:?}", params);

    // Check authorization
    let attrs =
        RequestAttributes::new(auth_ctx.user, "deletecollection", "namespaces").with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: Namespace collection would be deleted (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get all namespaces
    let prefix = build_prefix("namespaces", None);
    let mut items = state.storage.list::<Namespace>(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut items, &params)?;

    // Delete each matching resource
    let mut deleted_count = 0;
    for item in items {
        let key = build_key("namespaces", None, &item.metadata.name);

        // Handle deletion with finalizers
        let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
            &state.storage,
            &key,
            &item,
        )
        .await?;

        if deleted_immediately {
            deleted_count += 1;
        }
    }

    info!(
        "DeleteCollection completed: {} namespaces deleted",
        deleted_count
    );
    Ok(StatusCode::OK)
}
