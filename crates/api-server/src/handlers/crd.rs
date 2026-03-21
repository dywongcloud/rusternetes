// CustomResourceDefinition API handlers

use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::CustomResourceDefinition,
    List, Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

/// Create a new CustomResourceDefinition
pub async fn create_crd(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
    body: Bytes,
) -> Result<(StatusCode, Json<CustomResourceDefinition>)> {
    // Parse the body manually for better error handling — axum's Json extractor
    // returns 422 Unprocessable Entity on failure, but Kubernetes expects a proper
    // Status object. Manual parsing also tolerates unknown fields gracefully.
    let mut crd: CustomResourceDefinition = serde_json::from_slice(&body).map_err(|e| {
        rusternetes_common::Error::InvalidResource(format!("failed to decode: {}", e))
    })?;
    let crd_name = crd.metadata.name.clone();
    info!("Creating CustomResourceDefinition: {}", crd_name);

    // Check authorization (CRDs are cluster-scoped)
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "customresourcedefinitions")
        .with_api_group("apiextensions.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Validate CRD spec
    validate_crd(&crd)?;

    // Ensure metadata fields
    crd.metadata.ensure_uid();
    crd.metadata.ensure_creation_timestamp();

    // Initialize status
    if crd.status.is_none() {
        crd.status = Some(
            rusternetes_common::resources::CustomResourceDefinitionStatus {
                conditions: Some(vec![]),
                accepted_names: Some(crd.spec.names.clone()),
                stored_versions: Some(
                    crd.spec
                        .versions
                        .iter()
                        .filter(|v| v.storage)
                        .map(|v| v.name.clone())
                        .collect(),
                ),
            },
        );
    }

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: CustomResourceDefinition validated successfully (not created)");
        return Ok((StatusCode::CREATED, Json(crd)));
    }

    let key = build_key("customresourcedefinitions", None, &crd_name);
    let created = state.storage.create(&key, &crd).await?;

    // Notify the dynamic route manager about new CRD
    // This will be implemented in the dynamic routing section
    info!("CRD created: {}", crd_name);

    Ok((StatusCode::CREATED, Json(created)))
}

/// Get a specific CustomResourceDefinition
pub async fn get_crd(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
) -> Result<Json<CustomResourceDefinition>> {
    info!("Getting CustomResourceDefinition: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "get", "customresourcedefinitions")
        .with_api_group("apiextensions.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("customresourcedefinitions", None, &name);
    let crd = state.storage.get(&key).await?;

    Ok(Json(crd))
}

/// List all CustomResourceDefinitions
pub async fn list_crds(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<List<CustomResourceDefinition>>> {
    info!("Listing all CustomResourceDefinitions");

    let attrs = RequestAttributes::new(auth_ctx.user, "list", "customresourcedefinitions")
        .with_api_group("apiextensions.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("customresourcedefinitions", None);
    let mut crds = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut crds, &params)?;

    let list = List::new(
        "CustomResourceDefinitionList",
        "apiextensions.k8s.io/v1",
        crds,
    );
    Ok(Json(list))
}

/// Update a CustomResourceDefinition
pub async fn update_crd(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    body: Bytes,
) -> Result<Json<CustomResourceDefinition>> {
    // Parse the body manually for better error handling
    let mut crd: CustomResourceDefinition = serde_json::from_slice(&body).map_err(|e| {
        rusternetes_common::Error::InvalidResource(format!("failed to decode: {}", e))
    })?;
    info!("Updating CustomResourceDefinition: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "update", "customresourcedefinitions")
        .with_api_group("apiextensions.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Validate CRD spec
    validate_crd(&crd)?;

    crd.metadata.name = name.clone();

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: CustomResourceDefinition validated successfully (not updated)");
        return Ok(Json(crd));
    }

    let key = build_key("customresourcedefinitions", None, &name);
    let updated = state.storage.update(&key, &crd).await?;

    // Notify dynamic route manager about CRD update
    info!("CRD updated: {}", name);

    Ok(Json(updated))
}

/// Delete a CustomResourceDefinition
pub async fn delete_crd(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<CustomResourceDefinition>> {
    info!("Deleting CustomResourceDefinition: {}", name);

    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "customresourcedefinitions")
        .with_api_group("apiextensions.k8s.io")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Get CRD to check for custom resources
    let key = build_key("customresourcedefinitions", None, &name);
    let crd: CustomResourceDefinition = state.storage.get(&key).await?;

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: CustomResourceDefinition validated successfully (not deleted)");
        return Ok(Json(crd));
    }

    // Check if there are any custom resources of this type
    // Custom resources are stored with keys like: /apis/{group}/{version}/{resource}/{namespace}/{name}
    // or /apis/{group}/{version}/{resource}/{name} for cluster-scoped
    for version in &crd.spec.versions {
        let resource_prefix =
            if crd.spec.scope == rusternetes_common::resources::ResourceScope::Namespaced {
                // For namespaced resources, check across all namespaces
                format!(
                    "/apis/{}/{}/{}/",
                    crd.spec.group, version.name, crd.spec.names.plural
                )
            } else {
                // For cluster-scoped resources
                format!(
                    "/apis/{}/{}/{}/",
                    crd.spec.group, version.name, crd.spec.names.plural
                )
            };

        // List all custom resources with this prefix
        let custom_resources: Vec<serde_json::Value> = state
            .storage
            .list(&resource_prefix)
            .await
            .unwrap_or_default();

        if !custom_resources.is_empty() {
            warn!(
                "CRD {} has {} existing custom resources of type {}/{}/{}",
                name,
                custom_resources.len(),
                crd.spec.group,
                version.name,
                crd.spec.names.plural
            );

            return Err(rusternetes_common::Error::InvalidResource(format!(
                "CustomResourceDefinition {} cannot be deleted because {} custom resource(s) still exist. Delete all custom resources first.",
                name,
                custom_resources.len()
            )));
        }
    }

    // Handle deletion with finalizers
    let deleted_immediately =
        !crate::handlers::finalizers::handle_delete_with_finalizers(&state.storage, &key, &crd)
            .await?;

    if deleted_immediately {
        info!("CRD deleted: {}", name);
        Ok(Json(crd))
    } else {
        // Resource has finalizers, re-read to get updated version with deletionTimestamp
        let updated: CustomResourceDefinition = state.storage.get(&key).await?;
        Ok(Json(updated))
    }
}

/// Validate a CustomResourceDefinition
fn validate_crd(crd: &CustomResourceDefinition) -> Result<()> {
    // Validate that at least one version is defined
    if crd.spec.versions.is_empty() {
        return Err(rusternetes_common::Error::InvalidResource(
            "CRD must have at least one version".to_string(),
        ));
    }

    // Validate that exactly one version is marked as storage
    let storage_versions: Vec<_> = crd.spec.versions.iter().filter(|v| v.storage).collect();

    if storage_versions.is_empty() {
        return Err(rusternetes_common::Error::InvalidResource(
            "CRD must have exactly one storage version".to_string(),
        ));
    }

    if storage_versions.len() > 1 {
        return Err(rusternetes_common::Error::InvalidResource(
            "CRD can only have one storage version".to_string(),
        ));
    }

    // Validate that all served versions either have storage=true or are marked as served
    for version in &crd.spec.versions {
        if version.storage && !version.served {
            warn!(
                "Version {} is marked as storage but not served, this is unusual",
                version.name
            );
        }
    }

    // Validate group name is not empty
    if crd.spec.group.is_empty() {
        return Err(rusternetes_common::Error::InvalidResource(
            "CRD group cannot be empty".to_string(),
        ));
    }

    // Validate names
    if crd.spec.names.plural.is_empty() {
        return Err(rusternetes_common::Error::InvalidResource(
            "CRD plural name cannot be empty".to_string(),
        ));
    }

    if crd.spec.names.kind.is_empty() {
        return Err(rusternetes_common::Error::InvalidResource(
            "CRD kind cannot be empty".to_string(),
        ));
    }

    // Validate that the CRD name follows the convention: <plural>.<group>
    let expected_name = format!("{}.{}", crd.spec.names.plural, crd.spec.group);
    if crd.metadata.name != expected_name {
        return Err(rusternetes_common::Error::InvalidResource(format!(
            "CRD name must be '{}'",
            expected_name
        )));
    }

    Ok(())
}

// Use the macro to create a PATCH handler for cluster-scoped CustomResourceDefinition
crate::patch_handler_cluster!(
    patch_crd,
    CustomResourceDefinition,
    "customresourcedefinitions",
    "apiextensions.k8s.io"
);

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::resources::{
        CustomResourceDefinition, CustomResourceDefinitionNames, CustomResourceDefinitionSpec,
        CustomResourceDefinitionVersion, ResourceScope,
    };
    use rusternetes_common::types::ObjectMeta;

    fn create_test_crd() -> CustomResourceDefinition {
        CustomResourceDefinition {
            api_version: "apiextensions.k8s.io/v1".to_string(),
            kind: "CustomResourceDefinition".to_string(),
            metadata: ObjectMeta::new("crontabs.stable.example.com"),
            spec: CustomResourceDefinitionSpec {
                group: "stable.example.com".to_string(),
                names: CustomResourceDefinitionNames {
                    plural: "crontabs".to_string(),
                    singular: Some("crontab".to_string()),
                    kind: "CronTab".to_string(),
                    short_names: Some(vec!["ct".to_string()]),
                    categories: None,
                    list_kind: Some("CronTabList".to_string()),
                },
                scope: ResourceScope::Namespaced,
                versions: vec![CustomResourceDefinitionVersion {
                    name: "v1".to_string(),
                    served: true,
                    storage: true,
                    deprecated: None,
                    deprecation_warning: None,
                    schema: None,
                    subresources: None,
                    additional_printer_columns: None,
                }],
                conversion: None,
                preserve_unknown_fields: None,
            },
            status: None,
        }
    }

    #[test]
    fn test_validate_crd_success() {
        let crd = create_test_crd();
        assert!(validate_crd(&crd).is_ok());
    }

    #[test]
    fn test_validate_crd_no_versions() {
        let mut crd = create_test_crd();
        crd.spec.versions.clear();
        assert!(validate_crd(&crd).is_err());
    }

    #[test]
    fn test_validate_crd_no_storage_version() {
        let mut crd = create_test_crd();
        crd.spec.versions[0].storage = false;
        assert!(validate_crd(&crd).is_err());
    }

    #[test]
    fn test_validate_crd_multiple_storage_versions() {
        let mut crd = create_test_crd();
        crd.spec.versions.push(CustomResourceDefinitionVersion {
            name: "v2".to_string(),
            served: true,
            storage: true,
            deprecated: None,
            deprecation_warning: None,
            schema: None,
            subresources: None,
            additional_printer_columns: None,
        });
        assert!(validate_crd(&crd).is_err());
    }

    #[test]
    fn test_validate_crd_empty_group() {
        let mut crd = create_test_crd();
        crd.spec.group = String::new();
        assert!(validate_crd(&crd).is_err());
    }

    #[test]
    fn test_validate_crd_wrong_name() {
        let mut crd = create_test_crd();
        crd.metadata.name = "wrong-name".to_string();
        assert!(validate_crd(&crd).is_err());
    }
}

pub async fn deletecollection_customresourcedefinitions(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<StatusCode> {
    info!(
        "DeleteCollection customresourcedefinitions with params: {:?}",
        params
    );

    // Check authorization
    let attrs = RequestAttributes::new(
        auth_ctx.user,
        "deletecollection",
        "customresourcedefinitions",
    )
    .with_api_group("apiextensions.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Handle dry-run
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: CustomResourceDefinition collection would be deleted (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get all customresourcedefinitions
    let prefix = build_prefix("customresourcedefinitions", None);
    let mut items = state
        .storage
        .list::<CustomResourceDefinition>(&prefix)
        .await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut items, &params)?;

    // Delete each matching resource
    let mut deleted_count = 0;
    for item in items {
        let key = build_key("customresourcedefinitions", None, &item.metadata.name);

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
        "DeleteCollection completed: {} customresourcedefinitions deleted",
        deleted_count
    );
    Ok(StatusCode::OK)
}
