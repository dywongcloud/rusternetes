// Custom Resource (CR) API handlers for dynamically created CRDs
//
// This module handles CRUD operations for custom resources defined by CRDs.

use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::{CustomResource, CustomResourceDefinition},
    schema_validation::SchemaValidator,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::sync::Arc;
use tracing::{info, warn};

/// Create a new custom resource instance
pub async fn create_custom_resource(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((group, version, plural, namespace)): Path<(String, String, String, Option<String>)>,
    Json(mut cr): Json<CustomResource>,
) -> Result<(StatusCode, Json<CustomResource>)> {
    let cr_name = cr.metadata.name.clone();
    info!(
        "Creating custom resource {}/{}/{}: {}",
        group, version, plural, cr_name
    );

    // Find the CRD for this resource type
    let crd_name = format!("{}.{}", plural, group);
    let crd = get_crd_for_resource(&state, &crd_name).await?;

    // Validate the resource against CRD schema
    validate_custom_resource(&crd, &version, &cr)?;

    // Check authorization
    let attrs = if let Some(ref ns) = namespace {
        RequestAttributes::new(auth_ctx.user.clone(), "create", &plural)
            .with_api_group(&group)
            .with_namespace(ns)
    } else {
        RequestAttributes::new(auth_ctx.user, "create", &plural).with_api_group(&group)
    };

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Ensure metadata fields
    cr.metadata.ensure_uid();
    cr.metadata.ensure_creation_timestamp();

    // Set API version and kind
    cr.api_version = format!("{}/{}", group, version);
    cr.kind = crd.spec.names.kind.clone();

    // Build storage key
    let resource_type = format!("{}_{}", group.replace('.', "_"), plural);
    let key = if let Some(ref ns) = namespace {
        build_key(&resource_type, Some(ns), &cr_name)
    } else {
        build_key(&resource_type, None, &cr_name)
    };

    let created = state.storage.create(&key, &cr).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

/// Get a specific custom resource instance
pub async fn get_custom_resource(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((group, version, plural, namespace, name)): Path<(
        String,
        String,
        String,
        Option<String>,
        String,
    )>,
) -> Result<Json<CustomResource>> {
    info!(
        "Getting custom resource {}/{}/{}: {}",
        group, version, plural, name
    );

    // Find the CRD for this resource type
    let crd_name = format!("{}.{}", plural, group);
    let _crd = get_crd_for_resource(&state, &crd_name).await?;

    // Check authorization
    let attrs = if let Some(ref ns) = namespace {
        RequestAttributes::new(auth_ctx.user.clone(), "get", &plural)
            .with_api_group(&group)
            .with_namespace(ns)
            .with_name(&name)
    } else {
        RequestAttributes::new(auth_ctx.user, "get", &plural)
            .with_api_group(&group)
            .with_name(&name)
    };

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Build storage key
    let resource_type = format!("{}_{}", group.replace('.', "_"), plural);
    let key = if let Some(ref ns) = namespace {
        build_key(&resource_type, Some(ns), &name)
    } else {
        build_key(&resource_type, None, &name)
    };

    let cr = state.storage.get(&key).await?;

    Ok(Json(cr))
}

/// List custom resource instances
pub async fn list_custom_resources(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((group, version, plural, namespace)): Path<(String, String, String, Option<String>)>,
) -> Result<Json<Vec<CustomResource>>> {
    info!(
        "Listing custom resources {}/{}/{}",
        group, version, plural
    );

    // Find the CRD for this resource type
    let crd_name = format!("{}.{}", plural, group);
    let _crd = get_crd_for_resource(&state, &crd_name).await?;

    // Check authorization
    let attrs = if let Some(ref ns) = namespace {
        RequestAttributes::new(auth_ctx.user.clone(), "list", &plural)
            .with_api_group(&group)
            .with_namespace(ns)
    } else {
        RequestAttributes::new(auth_ctx.user, "list", &plural).with_api_group(&group)
    };

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Build storage prefix
    let resource_type = format!("{}_{}", group.replace('.', "_"), plural);
    let prefix = if let Some(ref ns) = namespace {
        build_prefix(&resource_type, Some(ns))
    } else {
        build_prefix(&resource_type, None)
    };

    let crs = state.storage.list(&prefix).await?;

    Ok(Json(crs))
}

/// Update a custom resource instance
pub async fn update_custom_resource(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((group, version, plural, namespace, name)): Path<(
        String,
        String,
        String,
        Option<String>,
        String,
    )>,
    Json(mut cr): Json<CustomResource>,
) -> Result<Json<CustomResource>> {
    info!(
        "Updating custom resource {}/{}/{}: {}",
        group, version, plural, name
    );

    // Find the CRD for this resource type
    let crd_name = format!("{}.{}", plural, group);
    let crd = get_crd_for_resource(&state, &crd_name).await?;

    // Validate the resource against CRD schema
    validate_custom_resource(&crd, &version, &cr)?;

    // Check authorization
    let attrs = if let Some(ref ns) = namespace {
        RequestAttributes::new(auth_ctx.user.clone(), "update", &plural)
            .with_api_group(&group)
            .with_namespace(ns)
            .with_name(&name)
    } else {
        RequestAttributes::new(auth_ctx.user, "update", &plural)
            .with_api_group(&group)
            .with_name(&name)
    };

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Ensure name matches
    cr.metadata.name = name.clone();
    cr.api_version = format!("{}/{}", group, version);
    cr.kind = crd.spec.names.kind.clone();

    // Build storage key
    let resource_type = format!("{}_{}", group.replace('.', "_"), plural);
    let key = if let Some(ref ns) = namespace {
        build_key(&resource_type, Some(ns), &name)
    } else {
        build_key(&resource_type, None, &name)
    };

    let updated = state.storage.update(&key, &cr).await?;

    Ok(Json(updated))
}

/// Delete a custom resource instance
pub async fn delete_custom_resource(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((group, version, plural, namespace, name)): Path<(
        String,
        String,
        String,
        Option<String>,
        String,
    )>,
) -> Result<StatusCode> {
    info!(
        "Deleting custom resource {}/{}/{}: {}",
        group, version, plural, name
    );

    // Find the CRD for this resource type
    let crd_name = format!("{}.{}", plural, group);
    let _crd = get_crd_for_resource(&state, &crd_name).await?;

    // Check authorization
    let attrs = if let Some(ref ns) = namespace {
        RequestAttributes::new(auth_ctx.user.clone(), "delete", &plural)
            .with_api_group(&group)
            .with_namespace(ns)
            .with_name(&name)
    } else {
        RequestAttributes::new(auth_ctx.user, "delete", &plural)
            .with_api_group(&group)
            .with_name(&name)
    };

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Build storage key
    let resource_type = format!("{}_{}", group.replace('.', "_"), plural);
    let key = if let Some(ref ns) = namespace {
        build_key(&resource_type, Some(ns), &name)
    } else {
        build_key(&resource_type, None, &name)
    };

    state.storage.delete(&key).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Helper to get CRD from storage
async fn get_crd_for_resource(
    state: &ApiServerState,
    crd_name: &str,
) -> Result<CustomResourceDefinition> {
    let key = build_key("customresourcedefinitions", None, crd_name);
    state.storage.get(&key).await
}

/// Validate a custom resource against its CRD schema
fn validate_custom_resource(
    crd: &CustomResourceDefinition,
    version: &str,
    cr: &CustomResource,
) -> Result<()> {
    // Find the version in the CRD
    let crd_version = crd
        .spec
        .versions
        .iter()
        .find(|v| v.name == version)
        .ok_or_else(|| {
            rusternetes_common::Error::InvalidResource(format!(
                "Version {} not found in CRD",
                version
            ))
        })?;

    // Check if version is served
    if !crd_version.served {
        warn!("Version {} is not served", version);
        return Err(rusternetes_common::Error::InvalidResource(format!(
            "Version {} is not served",
            version
        )));
    }

    // Validate against schema if present
    if let Some(ref validation) = crd_version.schema {
        if let Some(ref spec) = cr.spec {
            SchemaValidator::validate(&validation.open_apiv3_schema, spec)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::resources::{
        CustomResourceDefinitionNames, CustomResourceDefinitionSpec,
        CustomResourceDefinitionVersion, ResourceScope,
    };
    use rusternetes_common::types::ObjectMeta;
    use serde_json::json;

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

    fn create_test_custom_resource() -> CustomResource {
        CustomResource {
            api_version: "stable.example.com/v1".to_string(),
            kind: "CronTab".to_string(),
            metadata: {
                let mut meta = ObjectMeta::new("my-crontab");
                meta.namespace = Some("default".to_string());
                meta
            },
            spec: Some(json!({
                "cronSpec": "* * * * */5",
                "image": "my-cron-image"
            })),
            status: None,
        }
    }

    #[test]
    fn test_validate_custom_resource_success() {
        let crd = create_test_crd();
        let cr = create_test_custom_resource();

        assert!(validate_custom_resource(&crd, "v1", &cr).is_ok());
    }

    #[test]
    fn test_validate_custom_resource_invalid_version() {
        let crd = create_test_crd();
        let cr = create_test_custom_resource();

        let result = validate_custom_resource(&crd, "v2", &cr);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_custom_resource_not_served() {
        let mut crd = create_test_crd();
        crd.spec.versions[0].served = false;
        let cr = create_test_custom_resource();

        let result = validate_custom_resource(&crd, "v1", &cr);
        assert!(result.is_err());
    }
}
