use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    resources::{Event, EventList},
    Result,
};
use rusternetes_storage::{build_key, build_prefix, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// List all events in a namespace
pub async fn list(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<EventList>> {
    info!("Listing events in namespace: {}", namespace);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "events")
        .with_namespace(&namespace)
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("events", Some(&namespace));
    let mut events: Vec<Event> = state.storage.list(&prefix).await?;

    // Apply field and label selector filtering
    crate::handlers::filtering::apply_selectors(&mut events, &params)?;

    Ok(Json(EventList {
        api_version: "v1".to_string(),
        kind: "EventList".to_string(),
        items: events,
    }))
}

/// List all events across all namespaces
pub async fn list_all(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
) -> Result<Json<EventList>> {
    info!("Listing all events across all namespaces");

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "list", "events")
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let prefix = build_prefix("events", None);
    let events: Vec<Event> = state.storage.list(&prefix).await?;

    Ok(Json(EventList {
        api_version: "v1".to_string(),
        kind: "EventList".to_string(),
        items: events,
    }))
}

/// Get a specific event
pub async fn get(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
) -> Result<Json<Event>> {
    info!("Getting event: {}/{}", namespace, name);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "get", "events")
        .with_namespace(&namespace)
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("events", Some(&namespace), &name);
    let event: Event = state.storage.get(&key).await?;

    Ok(Json(event))
}

/// Create a new event
pub async fn create(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut event): Json<Event>,
) -> Result<(StatusCode, Json<Event>)> {
    info!("Creating event in namespace: {}", namespace);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "create", "events")
        .with_namespace(&namespace)
        .with_api_group("");

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Ensure namespace matches
    event.metadata.namespace = Some(namespace.clone());

    // Generate UID if not present
    event.metadata.ensure_uid();

    // Generate name if not present
    if event.metadata.name.is_empty() {
        let name = Event::generate_name(&event.involved_object, &event.reason);
        event.metadata.name = name;
    }

    // Ensure creation timestamp is set
    event.metadata.ensure_creation_timestamp();

    let key = build_key("events", Some(&namespace), &event.metadata.name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!("Dry-run: Event {}/{} validated successfully (not created)", namespace, event.metadata.name);
        return Ok((StatusCode::CREATED, Json(event)));
    }

    let created = state.storage.create(&key, &event).await?;

    Ok((StatusCode::CREATED, Json(created)))
}

/// Update an existing event
pub async fn update(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
    Json(mut event): Json<Event>,
) -> Result<Json<Event>> {
    info!("Updating event: {}/{}", namespace, name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "update", "events")
        .with_namespace(&namespace)
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    // Ensure namespace and name match
    event.metadata.namespace = Some(namespace.clone());
    event.metadata.name = name.clone();

    let key = build_key("events", Some(&namespace), &name);

    // If dry-run, skip storage operation but return the validated resource
    if is_dry_run {
        info!("Dry-run: Event {}/{} validated successfully (not updated)", namespace, name);
        return Ok(Json(event));
    }

    let updated = state.storage.update(&key, &event).await?;

    Ok(Json(updated))
}

/// Delete an event
pub async fn delete(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode> {
    info!("Deleting event: {}/{}", namespace, name);

    // Check if this is a dry-run request
    let is_dry_run = crate::handlers::dryrun::is_dry_run(&params);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user, "delete", "events")
        .with_namespace(&namespace)
        .with_api_group("")
        .with_name(&name);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(rusternetes_common::Error::Forbidden(reason));
        }
    }

    let key = build_key("events", Some(&namespace), &name);

    // If dry-run, skip delete operation
    if is_dry_run {
        info!("Dry-run: Event {}/{} validated successfully (not deleted)", namespace, name);
        return Ok(StatusCode::OK);
    }

    // Get the event for finalizer handling
    let event: Event = state.storage.get(&key).await?;

    // Handle deletion with finalizers
    let deleted_immediately = !crate::handlers::finalizers::handle_delete_with_finalizers(
        &state.storage,
        &key,
        &event,
    )
    .await?;

    if deleted_immediately {
        Ok(StatusCode::NO_CONTENT)
    } else {
        info!(
            "Event {}/{} marked for deletion (has finalizers: {:?})",
            namespace,
            name,
            event.metadata.finalizers
        );
        Ok(StatusCode::OK)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusternetes_common::resources::ObjectReference;
    use rusternetes_common::resources::EventType;

    #[test]
    fn test_event_creation() {
        let obj_ref = ObjectReference {
            kind: Some("Pod".to_string()),
            namespace: Some("default".to_string()),
            name: Some("test-pod".to_string()),
            uid: Some("abc123".to_string()),
            api_version: Some("v1".to_string()),
            resource_version: None,
            field_path: None,
        };

        let event = Event::new(
            "test-event".to_string(),
            "default".to_string(),
            obj_ref,
            "PodStarted".to_string(),
            "Pod started successfully".to_string(),
            EventType::Normal,
        );

        assert_eq!(event.metadata.name, "test-event".to_string());
        assert_eq!(event.metadata.namespace, Some("default".to_string()));
        assert_eq!(event.reason, "PodStarted");
        assert_eq!(event.count, 1);
    }
}

// Use the macro to create a PATCH handler
crate::patch_handler_namespaced!(patch, Event, "events", "");
