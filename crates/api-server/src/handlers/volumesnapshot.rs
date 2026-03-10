use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::VolumeSnapshot,
    List,
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

pub async fn create_volumesnapshot(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut vs): Json<VolumeSnapshot>,
) -> Result<(StatusCode, Json<VolumeSnapshot>)> {
    info!(
        "Creating VolumeSnapshot: {}/{}",
        namespace, vs.metadata.name
    );

    // Check authorization (namespace-scoped)
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "volumesnapshots")
        .with_api_group("snapshot.storage.k8s.io")
        .with_namespace(&namespace);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    vs.metadata.namespace = Some(namespace.clone());
    vs.metadata.ensure_uid();
    vs.metadata.ensure_creation_timestamp();

    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: VolumeSnapshot validated successfully (not created)");
        return Ok((StatusCode::CREATED, Json(vs)));
    }

    let key = build_key("volumesnapshots", Some(&namespace), &vs.metadata.name);
    let created = state.storage.create(&key, &vs).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_volumesnapshot(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<VolumeSnapshot>> {
    info!("Getting VolumeSnapshot: {}/{}", namespace, name);

    let attrs = RequestAttributes::new(auth_ctx.user, "get", "volumesnapshots")
        .with_api_group("snapshot.storage.k8s.io")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("volumesnapshots", Some(&namespace), &name);
    let vs = state.storage.get(&key).await?;

    Ok(Json(vs))
}

pub async fn list_volumesnapshots(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<List<VolumeSnapshot>>> {
    info!("Listing VolumeSnapshots in namespace: {}", namespace);

    let attrs = RequestAttributes::new(auth_ctx.user, "list", "volumesnapshots")
        .with_api_group("snapshot.storage.k8s.io")
        .with_namespace(&namespace);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("volumesnapshots", Some(&namespace));
    let mut vss = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut vss, &params)?;

    let list = List::new("VolumeSnapshotList", "snapshot.storage.k8s.io/v1", vss);
    Ok(Json(list))
}

pub async fn list_all_volumesnapshots(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<List<VolumeSnapshot>>> {
    info!("Listing all VolumeSnapshots");

    let attrs = RequestAttributes::new(auth_ctx.user, "list", "volumesnapshots")
        .with_api_group("snapshot.storage.k8s.io");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("volumesnapshots", None);
    let mut vss = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut vss, &params)?;

    let list = List::new("VolumeSnapshotList", "snapshot.storage.k8s.io/v1", vss);
    Ok(Json(list))
}

pub async fn update_volumesnapshot(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut vs): Json<VolumeSnapshot>,
) -> Result<Json<VolumeSnapshot>> {
    info!("Updating VolumeSnapshot: {}/{}", namespace, name);

    let attrs = RequestAttributes::new(auth_ctx.user, "update", "volumesnapshots")
        .with_api_group("snapshot.storage.k8s.io")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    vs.metadata.name = name.clone();
    vs.metadata.namespace = Some(namespace.clone());

    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: VolumeSnapshot validated successfully (not updated)");
        return Ok(Json(vs));
    }

    let key = build_key("volumesnapshots", Some(&namespace), &name);
    let updated = state.storage.update(&key, &vs).await?;

    Ok(Json(updated))
}

pub async fn delete_volumesnapshot(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("Deleting VolumeSnapshot: {}/{}", namespace, name);

    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "volumesnapshots")
        .with_api_group("snapshot.storage.k8s.io")
        .with_namespace(&namespace)
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("volumesnapshots", Some(&namespace), &name);

    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);
    if is_dry_run {
        info!("Dry-run: VolumeSnapshot validated successfully (not deleted)");
        return Ok(StatusCode::OK);
    }

    // Get the resource for finalizer handling
    let resource: VolumeSnapshot = state.storage.get(&key).await?;

    // Handle deletion with finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &resource,
    )
    .await?;

    if deleted_immediately {
        Ok(StatusCode::NO_CONTENT)
    } else {
        info!(
            "VolumeSnapshot marked for deletion (has finalizers: {:?})",
            resource.metadata.finalizers
        );
        Ok(StatusCode::OK)
    }
}

// Use the macro to create a PATCH handler
crate::patch_handler_namespaced!(patch_volumesnapshot, VolumeSnapshot, "volumesnapshots", "snapshot.storage.k8s.io");
