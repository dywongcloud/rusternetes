use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::CSIStorageCapacity,
    List, Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

pub async fn create_csistoragecapacity(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut csc): Json<CSIStorageCapacity>,
) -> Result<(StatusCode, Json<CSIStorageCapacity>)> {
    info!(
        "Creating CSIStorageCapacity: {} in namespace: {}",
        csc.metadata.name, namespace
    );

    // Check authorization (namespaced)
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "csistoragecapacities")
        .with_api_group("storage.k8s.io")
        .with_namespace(&namespace);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    csc.metadata.namespace = Some(namespace.clone());
    csc.metadata.ensure_uid();
    csc.metadata.ensure_creation_timestamp();

    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: CSIStorageCapacity validated successfully (not created)");
        return Ok((StatusCode::CREATED, Json(csc)));
    }

    let key = build_key("csistoragecapacities", Some(&namespace), &csc.metadata.name);
    let created = state.storage.create(&key, &csc).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_csistoragecapacity(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<CSIStorageCapacity>> {
    info!(
        "Getting CSIStorageCapacity: {} in namespace: {}",
        name, namespace
    );

    let attrs = RequestAttributes::new(auth_ctx.user, "get", "csistoragecapacities")
        .with_api_group("storage.k8s.io")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("csistoragecapacities", Some(&namespace), &name);
    let csc = state.storage.get(&key).await?;

    Ok(Json(csc))
}

pub async fn list_csistoragecapacities(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<List<CSIStorageCapacity>>> {
    info!("Listing CSIStorageCapacities in namespace: {}", namespace);

    let attrs = RequestAttributes::new(auth_ctx.user, "list", "csistoragecapacities")
        .with_api_group("storage.k8s.io")
        .with_namespace(&namespace);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("csistoragecapacities", Some(&namespace));
    let mut cscs = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut cscs, &params)?;

    let list = List::new("CSIStorageCapacityList", "storage.k8s.io/v1", cscs);
    Ok(Json(list))
}

pub async fn list_all_csistoragecapacities(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<List<CSIStorageCapacity>>> {
    info!("Listing all CSIStorageCapacities across all namespaces");

    let attrs = RequestAttributes::new(auth_ctx.user, "list", "csistoragecapacities")
        .with_api_group("storage.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("csistoragecapacities", None);
    let mut cscs = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut cscs, &params)?;

    let list = List::new("CSIStorageCapacityList", "storage.k8s.io/v1", cscs);
    Ok(Json(list))
}

pub async fn update_csistoragecapacity(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut csc): Json<CSIStorageCapacity>,
) -> Result<Json<CSIStorageCapacity>> {
    info!(
        "Updating CSIStorageCapacity: {} in namespace: {}",
        name, namespace
    );

    let attrs = RequestAttributes::new(auth_ctx.user, "update", "csistoragecapacities")
        .with_api_group("storage.k8s.io")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    csc.metadata.name = name.clone();
    csc.metadata.namespace = Some(namespace.clone());

    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: CSIStorageCapacity validated successfully (not updated)");
        return Ok(Json(csc));
    }

    let key = build_key("csistoragecapacities", Some(&namespace), &name);
    let updated = state.storage.update(&key, &csc).await?;

    Ok(Json(updated))
}

pub async fn delete_csistoragecapacity(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<CSIStorageCapacity>> {
    info!(
        "Deleting CSIStorageCapacity: {} in namespace: {}",
        name, namespace
    );

    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "csistoragecapacities")
        .with_api_group("storage.k8s.io")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("csistoragecapacities", Some(&namespace), &name);

    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Get the resource for finalizer handling
    let resource: CSIStorageCapacity = state.storage.get(&key).await?;

    if is_dry_run {
        info!("Dry-run: CSIStorageCapacity validated successfully (not deleted)");
        return Ok(Json(resource));
    }

    // Handle deletion with finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &resource,
    )
    .await?;

    if deleted_immediately {
        Ok(Json(resource))
    } else {
        // Resource has finalizers, re-read to get updated version with deletionTimestamp
        let updated: CSIStorageCapacity = state.storage.get(&key).await?;
        Ok(Json(updated))
    }
}

// Use the macro to create a PATCH handler
crate::patch_handler_namespaced!(
    patch_csistoragecapacity,
    CSIStorageCapacity,
    "csistoragecapacities",
    "storage.k8s.io"
);

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::types::{ObjectMeta, TypeMeta};

    fn create_test_capacity(name: &str) -> CSIStorageCapacity {
        CSIStorageCapacity {
            type_meta: TypeMeta {
                kind: "CSIStorageCapacity".to_string(),
                api_version: "storage.k8s.io/v1".to_string(),
            },
            metadata: ObjectMeta::new(name).with_namespace("default"),
            storage_class_name: "fast-ssd".to_string(),
            capacity: Some("100Gi".to_string()),
            maximum_volume_size: Some("10Gi".to_string()),
            node_topology: None,
        }
    }

    #[tokio::test]
    async fn test_csistoragecapacity_serialization() {
        let csc = create_test_capacity("test-csc");
        let json = serde_json::to_string(&csc).unwrap();
        let deserialized: CSIStorageCapacity = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.metadata.name, "test-csc");
        assert_eq!(deserialized.storage_class_name, "fast-ssd");
    }
}

pub async fn deletecollection_csistoragecapacities(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<StatusCode> {
    info!(
        "DeleteCollection csistoragecapacities in namespace: {} with params: {:?}",
        namespace, params
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "deletecollection", "csistoragecapacities")
        .with_namespace(&namespace)
        .with_api_group("storage.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: CSIStorageCapacity collection would be deleted (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get all csistoragecapacities in the namespace
    let prefix = build_prefix("csistoragecapacities", Some(&namespace));
    let mut items = state.storage.list::<CSIStorageCapacity>(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut items, &params)?;

    // Delete each matching resource
    let mut deleted_count = 0;
    for item in items {
        let key = build_key(
            "csistoragecapacities",
            Some(&namespace),
            &item.metadata.name,
        );

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
        "DeleteCollection completed: {} csistoragecapacities deleted",
        deleted_count
    );
    Ok(StatusCode::OK)
}
