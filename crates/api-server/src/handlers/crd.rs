// CustomResourceDefinition API handlers

use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
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
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    // Reject empty request bodies with a clear error message.
    if body.is_empty() {
        return Err(rusternetes_common::Error::InvalidResource(
            "request body must not be empty".to_string(),
        ));
    }

    // Detect binary (protobuf/CBOR) bodies early — before attempting JSON parse.
    // If the first byte isn't a valid JSON start character, reject immediately.
    if !body.is_empty() && !matches!(body[0], b'{' | b'[' | b'"' | b'0'..=b'9' | b't' | b'f' | b'n' | b' ' | b'\t' | b'\n' | b'\r') {
        return Err(rusternetes_common::Error::UnsupportedMediaType(
            "the body is not valid JSON (protobuf/CBOR content type is not supported for this resource)".to_string(),
        ));
    }

    // Check for strict field validation mode
    let is_strict = params.get("fieldValidation").map(|v| v.as_str()) == Some("Strict");

    // Try to parse the body as JSON. If it fails, return appropriate error.
    let mut crd: CustomResourceDefinition = match serde_json::from_slice::<CustomResourceDefinition>(&body) {
        Ok(c) => c,
        Err(e) => {
            // If strict mode, check for duplicate fields before falling through
            if is_strict {
                if let Ok(body_str) = std::str::from_utf8(&body) {
                    if let Some(dup_field) = crate::handlers::validation::find_duplicate_json_key_public(body_str) {
                        return Err(rusternetes_common::Error::InvalidResource(format!(
                            "strict decoding error: json: duplicate field \"{}\"", dup_field
                        )));
                    }
                }
            }

            // Try parsing as Value first and inject missing defaults
            if let Ok(mut val) = serde_json::from_slice::<serde_json::Value>(&body) {
                if val.get("apiVersion").is_none() {
                    val["apiVersion"] = serde_json::Value::String("apiextensions.k8s.io/v1".to_string());
                }
                if val.get("kind").is_none() {
                    val["kind"] = serde_json::Value::String("CustomResourceDefinition".to_string());
                }
                if val.get("metadata").is_none() {
                    val["metadata"] = serde_json::json!({});
                }
                serde_json::from_value(val).map_err(|e2| {
                    rusternetes_common::Error::InvalidResource(format!("failed to decode CRD: {}", e2))
                })?
            } else {
                return Err(rusternetes_common::Error::InvalidResource(format!("failed to decode CRD: {}", e)));
            }
        }
    };

    // Strict field validation: reject unknown or duplicate fields when requested
    crate::handlers::validation::validate_strict_fields(&params, &body, &crd)?;
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

    // Initialize status with Established and NamesAccepted conditions
    if crd.status.is_none() {
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        crd.status = Some(
            rusternetes_common::resources::CustomResourceDefinitionStatus {
                conditions: Some(vec![
                    rusternetes_common::resources::CustomResourceDefinitionCondition {
                        type_: "Established".to_string(),
                        status: "True".to_string(),
                        last_transition_time: Some(now.clone()),
                        reason: Some("InitialNamesAccepted".to_string()),
                        message: Some("the initial names have been accepted".to_string()),
                    },
                    rusternetes_common::resources::CustomResourceDefinitionCondition {
                        type_: "NamesAccepted".to_string(),
                        status: "True".to_string(),
                        last_transition_time: Some(now),
                        reason: Some("NoConflicts".to_string()),
                        message: Some("no conflicts found".to_string()),
                    },
                ]),
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
        let value = serde_json::to_value(&crd)
            .map_err(|e| rusternetes_common::Error::Internal(format!("serialize: {}", e)))?;
        return Ok((StatusCode::CREATED, Json(value)));
    }

    let key = build_key("customresourcedefinitions", None, &crd_name);
    // Serialize the CRD to a serde_json::Value, store it, and return the stored
    // value directly. This avoids a Serialize→store→Deserialize round-trip through
    // the strongly-typed struct, which can fail if the storage layer enriches
    // the JSON with fields (uid, resourceVersion) that cause type mismatches.
    let crd_value = serde_json::to_value(&crd)
        .map_err(|e| rusternetes_common::Error::Internal(format!("serialize: {}", e)))?;
    let created: serde_json::Value = state.storage.create(&key, &crd_value).await?;

    info!("CRD created: {}", crd_name);

    // Trigger status updates after creation to generate MODIFIED events.
    // K8s clients watch for CRD status changes (Established condition) using the
    // resourceVersion from the CREATE response. Since we set status during creation,
    // there's no MODIFIED event without this update. We retry multiple times to
    // ensure the watch catches it even if there are timing issues.
    {
        let storage = state.storage.clone();
        let key_clone = key.clone();
        tokio::spawn(async move {
            for delay_ms in [50, 200, 1000] {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                if let Ok(mut crd_val) = storage.get::<serde_json::Value>(&key_clone).await {
                    // Touch the status to trigger a MODIFIED watch event
                    if let Some(status) = crd_val.get_mut("status") {
                        if let Some(obj) = status.as_object_mut() {
                            obj.insert("observedGeneration".to_string(), serde_json::json!(1));
                        }
                    }
                    if storage.update(&key_clone, &crd_val).await.is_ok() {
                        break; // Update succeeded, stop retrying
                    }
                }
            }
        });
    }

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
) -> Result<axum::response::Response> {
    if crate::handlers::watch::is_watch_request(&params) {
        let watch_params = crate::handlers::watch::watch_params_from_query(&params);
        return crate::handlers::watch::watch_cluster_scoped_json(
            state, auth_ctx, "customresourcedefinitions", "apiextensions.k8s.io", watch_params,
        ).await;
    }

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
    let mut crds = state.storage.list::<CustomResourceDefinition>(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut crds, &params)?;

    let list = List::new(
        "CustomResourceDefinitionList",
        "apiextensions.k8s.io/v1",
        crds,
    );
    Ok(Json(list).into_response())
}

/// Update a CustomResourceDefinition
pub async fn update_crd(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<CustomResourceDefinition>> {
    // Reject empty or binary-protobuf request bodies
    if body.is_empty() {
        return Err(rusternetes_common::Error::InvalidResource(
            "request body must not be empty".to_string(),
        ));
    }
    if !matches!(body[0], b'{' | b'[' | b'"' | b'0'..=b'9' | b't' | b'f' | b'n' | b' ' | b'\t' | b'\n' | b'\r') {
        return Err(rusternetes_common::Error::UnsupportedMediaType(
            "request body is not valid JSON; protobuf content type is not supported".to_string(),
        ));
    }

    // Check for strict field validation mode
    let is_strict = params.get("fieldValidation").map(|v| v.as_str()) == Some("Strict");

    // Parse the body manually for better error handling
    let mut crd: CustomResourceDefinition = serde_json::from_slice(&body).map_err(|e| {
        let msg = e.to_string();
        if is_strict && msg.contains("duplicate field") {
            if let Some(field) = msg.split('`').nth(1) {
                return rusternetes_common::Error::InvalidResource(format!(
                    "strict decoding error: json: duplicate field \"{}\"", field
                ));
            }
        }
        // Also check with our manual duplicate detector
        if is_strict {
            if let Ok(body_str) = std::str::from_utf8(&body) {
                if let Some(dup_field) = crate::handlers::validation::find_duplicate_json_key_public(body_str) {
                    return rusternetes_common::Error::InvalidResource(format!(
                        "strict decoding error: json: duplicate field \"{}\"", dup_field
                    ));
                }
            }
        }
        rusternetes_common::Error::InvalidResource(format!("failed to decode CRD: {}", msg))
    })?;

    // Strict field validation: reject unknown or duplicate fields when requested
    crate::handlers::validation::validate_strict_fields(&params, &body, &crd)?;

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
