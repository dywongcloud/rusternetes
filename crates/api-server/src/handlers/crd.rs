// CustomResourceDefinition API handlers

use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::CustomResourceDefinition,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::{info, warn};

/// Create a new CustomResourceDefinition
pub async fn create_crd(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Json(mut crd): Json<CustomResourceDefinition>,
) -> Result<(StatusCode, Json<CustomResourceDefinition>)> {
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
        crd.status = Some(rusternetes_common::resources::CustomResourceDefinitionStatus {
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
        });
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
) -> Result<Json<Vec<CustomResourceDefinition>>> {
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
    let crds = state.storage.list(&prefix).await?;

    Ok(Json(crds))
}

/// Update a CustomResourceDefinition
pub async fn update_crd(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Json(mut crd): Json<CustomResourceDefinition>,
) -> Result<Json<CustomResourceDefinition>> {
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
) -> Result<StatusCode> {
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

    // TODO: Check if there are any custom resources of this type
    // and optionally delete them based on finalizers

    state.storage.delete(&key).await?;

    // Notify dynamic route manager to remove routes for this CRD
    info!("CRD deleted: {}", name);

    Ok(StatusCode::NO_CONTENT)
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
    let storage_versions: Vec<_> = crd
        .spec
        .versions
        .iter()
        .filter(|v| v.storage)
        .collect();

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

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::resources::{
        CustomResourceDefinition, CustomResourceDefinitionNames,
        CustomResourceDefinitionSpec, CustomResourceDefinitionVersion, ResourceScope,
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
