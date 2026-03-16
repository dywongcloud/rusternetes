use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::PodTemplate,
    List, Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

pub async fn create_podtemplate(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut podtemplate): Json<PodTemplate>,
) -> Result<(StatusCode, Json<PodTemplate>)> {
    info!(
        "Creating podtemplate: {} in namespace: {}",
        podtemplate.metadata.name, namespace
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "podtemplates")
        .with_api_group("")
        .with_namespace(&namespace);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Enrich metadata with system fields
    podtemplate.metadata.ensure_uid();
    podtemplate.metadata.ensure_creation_timestamp();

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: PodTemplate validated successfully (not created)");
        return Ok((StatusCode::CREATED, Json(podtemplate)));
    }

    let key = build_key("podtemplates", Some(&namespace), &podtemplate.metadata.name);
    let created = state.storage.create(&key, &podtemplate).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_podtemplate(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<PodTemplate>> {
    info!("Getting podtemplate: {} in namespace: {}", name, namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "podtemplates")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("podtemplates", Some(&namespace), &name);
    let podtemplate = state.storage.get(&key).await?;

    Ok(Json(podtemplate))
}

pub async fn update_podtemplate(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut podtemplate): Json<PodTemplate>,
) -> Result<Json<PodTemplate>> {
    info!("Updating podtemplate: {} in namespace: {}", name, namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "podtemplates")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    podtemplate.metadata.name = name.clone();
    podtemplate.metadata.namespace = Some(namespace.clone());

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: PodTemplate validated successfully (not updated)");
        return Ok(Json(podtemplate));
    }

    let key = build_key("podtemplates", Some(&namespace), &name);

    // Try to update first, if not found then create (upsert behavior)
    let result = match state.storage.update(&key, &podtemplate).await {
        Ok(updated) => updated,
        Err(rusternetes_common::Error::NotFound(_)) => {
            state.storage.create(&key, &podtemplate).await?
        }
        Err(e) => return Err(e),
    };

    Ok(Json(result))
}

pub async fn delete_podtemplate(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("Deleting podtemplate: {} in namespace: {}", name, namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "podtemplates")
        .with_api_group("")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("podtemplates", Some(&namespace), &name);

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: PodTemplate validated successfully (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get the resource for finalizer handling
    let podtemplate: PodTemplate = state.storage.get(&key).await?;

    // Handle deletion with finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &podtemplate,
    )
    .await?;

    if deleted_immediately {
        Ok(StatusCode::NO_CONTENT)
    } else {
        info!(
            "PodTemplate marked for deletion (has finalizers: {:?})",
            podtemplate.metadata.finalizers
        );
        Ok(StatusCode::OK)
    }
}

pub async fn list_podtemplates(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
) -> Result<Json<List<PodTemplate>>> {
    info!("Listing podtemplates in namespace: {}", namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "podtemplates")
        .with_api_group("")
        .with_namespace(&namespace);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("podtemplates", Some(&namespace));
    let podtemplates = state.storage.list(&prefix).await?;

    let list = List::new("PodTemplateList", "v1", podtemplates);
    Ok(Json(list))
}

/// List all podtemplates across all namespaces
pub async fn list_all_podtemplates(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<List<PodTemplate>>> {
    info!("Listing all podtemplates");

    // Check authorization (cluster-wide list)
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "podtemplates").with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("podtemplates", None);
    let podtemplates = state.storage.list::<PodTemplate>(&prefix).await?;

    let list = List::new("PodTemplateList", "v1", podtemplates);
    Ok(Json(list))
}

// Use the macro to create a PATCH handler
crate::patch_handler_namespaced!(patch_podtemplate, PodTemplate, "podtemplates", "");

#[cfg(test)]
#[cfg(feature = "integration-tests")] // Disable tests that require full setup
mod tests {
    use super::*;
    use crate::state::ApiServerState;
    use rusternetes_common::{
        authz::Authorizer,
        resources::{PodSpec, PodTemplateSpec},
        types::ObjectMeta,
        User,
    };
    use rusternetes_storage::memory::MemoryStorage;

    #[tokio::test]
    async fn test_podtemplate_create() {
        use rusternetes_common::auth::{BootstrapTokenManager, TokenManager};
        use rusternetes_common::authz::AlwaysAllowAuthorizer;
        use rusternetes_common::observability::MetricsRegistry;

        let storage = Arc::new(MemoryStorage::new());
        let token_manager = Arc::new(TokenManager::new(b"test-key"));
        let bootstrap_token_manager = Arc::new(BootstrapTokenManager::new());
        let authorizer = Arc::new(AlwaysAllowAuthorizer);
        let metrics = Arc::new(MetricsRegistry::new());

        let state = Arc::new(ApiServerState::new(
            storage,
            token_manager,
            bootstrap_token_manager,
            authorizer,
            metrics,
            true, // skip_auth for tests
            None, // ca_cert_pem
        ));

        let pod_template_spec = PodTemplateSpec {
            metadata: Some(ObjectMeta::new("test-pod")),
            spec: PodSpec {
                containers: vec![],
                init_containers: None,
                volumes: None,
                restart_policy: Some("Always".to_string()),
                node_name: None,
                node_selector: None,
                service_account_name: None,
                hostname: None,
                subdomain: None,
                host_network: None,
                host_pid: None,
                host_ipc: None,
                affinity: None,
                tolerations: None,
                priority: None,
                priority_class_name: None,
                automount_service_account_token: None,
                ephemeral_containers: None,
                overhead: None,
                scheduler_name: None,
                topology_spread_constraints: None,
                resource_claims: None,
            },
        };

        let podtemplate = PodTemplate::new("test-template", "default", pod_template_spec);

        let auth_ctx = AuthContext {
            user: User::system(),
        };

        let result = create_podtemplate(
            State(state),
            Extension(auth_ctx),
            Path("default".to_string()),
            Json(podtemplate.clone()),
        )
        .await;

        assert!(result.is_ok());
        let (status, created) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(created.0.metadata.name, "test-template");
    }
}

pub async fn deletecollection_podtemplates(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<StatusCode> {
    info!(
        "DeleteCollection podtemplates in namespace: {} with params: {:?}",
        namespace, params
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "deletecollection", "podtemplates")
        .with_namespace(&namespace)
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: PodTemplate collection would be deleted (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get all podtemplates in the namespace
    let prefix = build_prefix("podtemplates", Some(&namespace));
    let mut items = state.storage.list::<PodTemplate>(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut items, &params)?;

    // Delete each matching resource
    let mut deleted_count = 0;
    for item in items {
        let key = build_key("podtemplates", Some(&namespace), &item.metadata.name);

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
        "DeleteCollection completed: {} podtemplates deleted",
        deleted_count
    );
    Ok(StatusCode::OK)
}
