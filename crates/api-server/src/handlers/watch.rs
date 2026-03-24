#![allow(dead_code)]

use crate::{middleware::AuthContext, state::ApiServerState};
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Extension,
};
use futures::StreamExt;
use rusternetes_common::{
    authz::{Decision, RequestAttributes},
    types::ObjectMeta,
    Error, Result,
};
use rusternetes_storage::{build_prefix, Storage, WatchEvent};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, timeout};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, error, info};

/// Kubernetes watch event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum WatchEventType {
    Added,
    Modified,
    Deleted,
    Bookmark,
    Error,
}

/// Kubernetes watch event wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sWatchEvent<T> {
    #[serde(rename = "type")]
    pub event_type: WatchEventType,
    pub object: T,
}

/// Query parameters for watch requests
#[derive(Debug, Deserialize)]
pub struct WatchParams {
    /// Resource version to watch from
    #[serde(
        rename = "resourceVersion",
        deserialize_with = "deserialize_empty_string_as_none",
        default
    )]
    pub resource_version: Option<String>,

    /// Timeout in seconds
    #[serde(rename = "timeoutSeconds")]
    pub timeout_seconds: Option<u64>,

    /// Label selector
    #[serde(rename = "labelSelector")]
    pub label_selector: Option<String>,

    /// Field selector
    #[serde(rename = "fieldSelector")]
    pub field_selector: Option<String>,

    /// Watch for changes
    pub watch: Option<bool>,

    /// Allow watch bookmarks
    #[serde(rename = "allowWatchBookmarks")]
    pub allow_watch_bookmarks: Option<bool>,

    /// Send initial events (consistent reads from cache, K8s 1.30+)
    /// When true, send all existing resources as ADDED events followed by
    /// a BOOKMARK to signal initial list is complete.
    #[serde(rename = "sendInitialEvents")]
    pub send_initial_events: Option<bool>,
}

/// Deserialize empty strings as None for resourceVersion
fn deserialize_empty_string_as_none<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    Ok(opt.filter(|s| !s.is_empty()))
}

/// Normalize a resourceVersion value: treat empty string as None (= "start from current")
pub fn normalize_resource_version(rv: Option<String>) -> Option<String> {
    rv.filter(|s| !s.is_empty())
}

/// Generic watch handler for namespaced resources
pub async fn watch_namespaced<T>(
    state: Arc<ApiServerState>,
    auth_ctx: AuthContext,
    namespace: String,
    resource_type: &str,
    api_group: &str,
    params: WatchParams,
) -> Result<Response>
where
    T: Serialize + DeserializeOwned + Send + Sync + 'static + Clone + HasMetadata,
{
    info!(
        "Starting watch for {} in namespace {} (timeout: {:?}s, bookmarks: {})",
        resource_type,
        namespace,
        params.timeout_seconds,
        params.allow_watch_bookmarks.unwrap_or(false)
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user.clone(), "watch", resource_type)
        .with_namespace(&namespace)
        .with_api_group(api_group);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(Error::Forbidden(reason));
        }
    }

    // Create watch stream via the shared watch cache (one etcd watch per prefix)
    let prefix = build_prefix(resource_type, Some(&namespace));

    // Subscribe to watch FIRST, then list. This ensures we don't miss events
    // that happen between the list and the watch start (race condition fix).
    let watch_rx = state.watch_cache.subscribe(&prefix).await;
    let watch_stream = crate::watch_cache::broadcast_to_stream(watch_rx);

    // List existing resources to send as initial ADDED events
    let existing_resources = state.storage.list::<T>(&prefix).await?;

    // Create channel for sending events to client
    let (tx, rx) =
        tokio::sync::mpsc::unbounded_channel::<std::result::Result<String, std::io::Error>>();

    // Extract parameters
    let allow_bookmarks = params.allow_watch_bookmarks.unwrap_or(false);
    let send_initial_events = params.send_initial_events.unwrap_or(false);
    let timeout_duration = params.timeout_seconds.map(|s| Duration::from_secs(s));
    let label_selector = params.label_selector.clone();
    let field_selector = params.field_selector.clone();
    let requested_rv = params.resource_version.clone();
    let (bookmark_kind, bookmark_api_version) =
        resource_type_to_kind_and_version(resource_type, api_group);

    // Determine whether to send initial ADDED events:
    // - If sendInitialEvents=true: always send
    // - If resourceVersion is "0" or absent: send initial events
    // - If resourceVersion is a specific value (non-zero): skip initial events
    let should_send_initial = send_initial_events
        || requested_rv.as_deref() == Some("0")
        || requested_rv.is_none();

    // Spawn task to convert watch events to HTTP response
    tokio::spawn(async move {
        // Track the latest resourceVersion for bookmarks
        let mut latest_resource_version: Option<String> = Some(
            chrono::Utc::now().timestamp().to_string(),
        );

        // Send initial state as ADDED events (only when appropriate)
        if should_send_initial {
        for object in existing_resources {
            // Update latest resourceVersion
            if let Some(rv) = object.metadata().resource_version.as_ref() {
                latest_resource_version = Some(rv.clone());
            }

            // Filter by label and field selectors
            if !matches_label_selector(object.metadata(), &label_selector)
                || !matches_field_selector(object.metadata(), &field_selector)
            {
                continue;
            }

            let k8s_event = K8sWatchEvent {
                event_type: WatchEventType::Added,
                object,
            };
            if let Ok(json) = serde_json::to_string(&k8s_event) {
                if tx.send(Ok(format!("{}\n", json))).is_err() {
                    return; // Client disconnected
                }
            }
        }
        } // end should_send_initial

        // When sendInitialEvents=true, send an initial BOOKMARK after the ADDED
        // events to signal "initial list is complete". The bookmark must have the
        // annotation "k8s.io/initial-events-end": "true" — client-go checks for
        // this specific annotation to know initial sync is done.
        if send_initial_events {
            if let Some(ref rv) = latest_resource_version {
                let mut annotations = std::collections::HashMap::new();
                annotations.insert(
                    "k8s.io/initial-events-end".to_string(),
                    "true".to_string(),
                );
                let bookmark = BookmarkObject {
                    kind: Some(bookmark_kind.clone()),
                    api_version: Some(bookmark_api_version.clone()),
                    metadata: ObjectMeta {
                        resource_version: Some(rv.clone()),
                        annotations: Some(annotations),
                        ..Default::default()
                    },
                };
                let k8s_event = K8sWatchEvent {
                    event_type: WatchEventType::Bookmark,
                    object: bookmark,
                };
                if let Ok(json) = serde_json::to_string(&k8s_event) {
                    let _ = tx.send(Ok(format!("{}\n", json)));
                }
                debug!("Sent initial-events-end bookmark with resourceVersion: {}", rv);
            }
        }

        // Create bookmark interval (60 seconds) if bookmarks are enabled
        let mut bookmark_interval = if allow_bookmarks || send_initial_events {
            Some(interval(Duration::from_secs(60)))
        } else {
            None
        };

        // Box-pin the watch stream so it can be replaced on reconnect
        let mut watch_stream: std::pin::Pin<Box<dyn futures::Stream<Item = rusternetes_common::Result<WatchEvent>> + Send>> = Box::pin(watch_stream);

        // Watch loop with timeout support
        let watch_future = async {
            loop {
                tokio::select! {
                    // Process watch events
                    event_opt = watch_stream.next() => {
                        match event_opt {
                            Some(Ok(WatchEvent::Added(key, value))) => {
                                debug!("Watch event - Added: {}", key);
                                if let Ok(object) = serde_json::from_str::<T>(&value) {
                                    // Update latest resourceVersion
                                    if let Some(rv) = object.metadata().resource_version.as_ref() {
                                        latest_resource_version = Some(rv.clone());
                                    }

                                    // Filter by label and field selectors
                                    if !matches_label_selector(object.metadata(), &label_selector)
                                        || !matches_field_selector(object.metadata(), &field_selector)
                                    {
                                        continue;
                                    }

                                    let k8s_event = K8sWatchEvent {
                                        event_type: WatchEventType::Added,
                                        object,
                                    };
                                    if let Ok(json) = serde_json::to_string(&k8s_event) {
                                        if tx.send(Ok(format!("{}\n", json))).is_err() {
                                            break; // Client disconnected
                                        }
                                    }
                                }
                            }
                            Some(Ok(WatchEvent::Modified(key, value))) => {
                                debug!("Watch event - Modified: {}", key);
                                if let Ok(object) = serde_json::from_str::<T>(&value) {
                                    // Update latest resourceVersion
                                    if let Some(rv) = object.metadata().resource_version.as_ref() {
                                        latest_resource_version = Some(rv.clone());
                                    }

                                    // Filter by label and field selectors
                                    if !matches_label_selector(object.metadata(), &label_selector)
                                        || !matches_field_selector(object.metadata(), &field_selector)
                                    {
                                        continue;
                                    }

                                    let k8s_event = K8sWatchEvent {
                                        event_type: WatchEventType::Modified,
                                        object,
                                    };
                                    if let Ok(json) = serde_json::to_string(&k8s_event) {
                                        if tx.send(Ok(format!("{}\n", json))).is_err() {
                                            break; // Client disconnected
                                        }
                                    }
                                }
                            }
                            Some(Ok(WatchEvent::Deleted(key, prev_value))) => {
                                debug!("Watch event - Deleted: {}", key);
                                // For DELETE events, Kubernetes requires the full object with metadata
                                if let Ok(object) = serde_json::from_str::<T>(&prev_value) {
                                    // Update latest resourceVersion
                                    if let Some(rv) = object.metadata().resource_version.as_ref() {
                                        latest_resource_version = Some(rv.clone());
                                    }

                                    // Filter by label and field selectors
                                    if !matches_label_selector(object.metadata(), &label_selector)
                                        || !matches_field_selector(object.metadata(), &field_selector)
                                    {
                                        continue;
                                    }

                                    let k8s_event = K8sWatchEvent {
                                        event_type: WatchEventType::Deleted,
                                        object,
                                    };
                                    if let Ok(json) = serde_json::to_string(&k8s_event) {
                                        if tx.send(Ok(format!("{}\n", json))).is_err() {
                                            break; // Client disconnected
                                        }
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                // Empty watch responses and transient errors are normal —
                                // etcd sends keep-alive responses with no events. Don't break.
                                debug!("Watch stream transient error (continuing): {}", e);
                                continue;
                            }
                            None => {
                                // Watch stream ended — resubscribe from cache.
                                // The cache handles etcd reconnection internally.
                                let new_rx = state.watch_cache.subscribe(&prefix).await;
                                watch_stream = Box::pin(crate::watch_cache::broadcast_to_stream(new_rx));
                                continue;
                            }
                        }
                    }
                    // Send periodic bookmarks if enabled
                    _ = async {
                        if let Some(ref mut interval) = bookmark_interval {
                            interval.tick().await;
                        } else {
                            // If bookmarks are disabled, park this branch forever
                            futures::future::pending::<()>().await
                        }
                    } => {
                        if allow_bookmarks || send_initial_events {
                            if let Some(ref rv) = latest_resource_version {
                                debug!("Sending bookmark with resourceVersion: {}", rv);
                                let bookmark = BookmarkObject {
                                    kind: Some(bookmark_kind.clone()),
                                    api_version: Some(bookmark_api_version.clone()),
                                    metadata: ObjectMeta {
                                        resource_version: Some(rv.clone()),
                                        ..Default::default()
                                    },
                                };
                                let k8s_event = K8sWatchEvent {
                                    event_type: WatchEventType::Bookmark,
                                    object: bookmark,
                                };
                                if let Ok(json) = serde_json::to_string(&k8s_event) {
                                    let _ = tx.send(Ok(format!("{}\n", json)));
                                    // Don't break on bookmark send failure — the client
                                    // might have reset just the bookmark stream but the
                                    // watch connection is still alive.
                                }
                            }
                        }
                    }
                }
            }
        };

        // Apply timeout if specified
        if let Some(timeout_dur) = timeout_duration {
            match timeout(timeout_dur, watch_future).await {
                Ok(_) => {
                    debug!("Watch stream completed normally");
                }
                Err(_) => {
                    info!("Watch stream timeout after {:?}", timeout_dur);
                    // Send final bookmark before closing if bookmarks are enabled
                    if allow_bookmarks || send_initial_events {
                        if let Some(ref rv) = latest_resource_version {
                            let bookmark = BookmarkObject {
                                    kind: Some(bookmark_kind.clone()),
                                    api_version: Some(bookmark_api_version.clone()),
                                    metadata: ObjectMeta {
                                    resource_version: Some(rv.clone()),
                                    ..Default::default()
                                },
                            };
                            let k8s_event = K8sWatchEvent {
                                event_type: WatchEventType::Bookmark,
                                object: bookmark,
                            };
                            if let Ok(json) = serde_json::to_string(&k8s_event) {
                                let _ = tx.send(Ok(format!("{}\n", json)));
                            }
                        }
                    }
                }
            }
        } else {
            // No timeout, run forever
            watch_future.await;
        }
    });

    // Convert receiver to stream
    let stream = UnboundedReceiverStream::new(rx);

    // Build response with proper headers for streaming
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .body(Body::from_stream(stream))
        .map_err(|e| Error::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// Generic watch handler for cluster-scoped resources
pub async fn watch_cluster_scoped<T>(
    state: Arc<ApiServerState>,
    auth_ctx: AuthContext,
    resource_type: &str,
    api_group: &str,
    params: WatchParams,
) -> Result<Response>
where
    T: Serialize + DeserializeOwned + Send + Sync + 'static + Clone + HasMetadata,
{
    info!(
        "Starting watch for cluster-scoped {} (timeout: {:?}s, bookmarks: {})",
        resource_type,
        params.timeout_seconds,
        params.allow_watch_bookmarks.unwrap_or(false)
    );

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user.clone(), "watch", resource_type)
        .with_api_group(api_group);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(Error::Forbidden(reason));
        }
    }

    // Create watch stream via the shared watch cache
    let prefix = build_prefix(resource_type, None);

    // Subscribe FIRST, then list (avoid race condition)
    let watch_rx = state.watch_cache.subscribe(&prefix).await;
    let watch_stream = crate::watch_cache::broadcast_to_stream(watch_rx);

    // List existing resources to send as initial ADDED events
    let existing_resources = state.storage.list::<T>(&prefix).await?;

    // Create channel for sending events to client
    let (tx, rx) =
        tokio::sync::mpsc::unbounded_channel::<std::result::Result<String, std::io::Error>>();

    // Extract parameters
    let allow_bookmarks = params.allow_watch_bookmarks.unwrap_or(false);
    let send_initial_events = params.send_initial_events.unwrap_or(false);
    let timeout_duration = params.timeout_seconds.map(|s| Duration::from_secs(s));
    let label_selector = params.label_selector.clone();
    let field_selector = params.field_selector.clone();
    let requested_rv = params.resource_version.clone();
    let (bookmark_kind, bookmark_api_version) =
        resource_type_to_kind_and_version(resource_type, api_group);

    // Determine whether to send initial ADDED events
    let should_send_initial = send_initial_events
        || requested_rv.as_deref() == Some("0")
        || requested_rv.is_none();

    // Spawn task to convert watch events to HTTP response
    tokio::spawn(async move {
        // Track the latest resourceVersion for bookmarks
        let mut latest_resource_version: Option<String> = Some(
            chrono::Utc::now().timestamp().to_string(),
        );

        // Send initial state as ADDED events (only when appropriate)
        if should_send_initial {
        for object in existing_resources {
            // Update latest resourceVersion
            if let Some(rv) = object.metadata().resource_version.as_ref() {
                latest_resource_version = Some(rv.clone());
            }

            // Filter by label and field selectors
            if !matches_label_selector(object.metadata(), &label_selector)
                || !matches_field_selector(object.metadata(), &field_selector)
            {
                continue;
            }

            let k8s_event = K8sWatchEvent {
                event_type: WatchEventType::Added,
                object,
            };
            if let Ok(json) = serde_json::to_string(&k8s_event) {
                if tx.send(Ok(format!("{}\n", json))).is_err() {
                    return; // Client disconnected
                }
            }
        }
        } // end should_send_initial

        // When sendInitialEvents=true, send an initial BOOKMARK after the ADDED
        // events to signal "initial list is complete". The bookmark must have the
        // annotation "k8s.io/initial-events-end": "true" — client-go checks for
        // this specific annotation to know initial sync is done.
        if send_initial_events {
            if let Some(ref rv) = latest_resource_version {
                let mut annotations = std::collections::HashMap::new();
                annotations.insert(
                    "k8s.io/initial-events-end".to_string(),
                    "true".to_string(),
                );
                let bookmark = BookmarkObject {
                    kind: Some(bookmark_kind.clone()),
                    api_version: Some(bookmark_api_version.clone()),
                    metadata: ObjectMeta {
                        resource_version: Some(rv.clone()),
                        annotations: Some(annotations),
                        ..Default::default()
                    },
                };
                let k8s_event = K8sWatchEvent {
                    event_type: WatchEventType::Bookmark,
                    object: bookmark,
                };
                if let Ok(json) = serde_json::to_string(&k8s_event) {
                    let _ = tx.send(Ok(format!("{}\n", json)));
                }
                debug!("Sent initial-events-end bookmark with resourceVersion: {}", rv);
            }
        }

        // Create bookmark interval (60 seconds) if bookmarks are enabled
        let mut bookmark_interval = if allow_bookmarks || send_initial_events {
            Some(interval(Duration::from_secs(60)))
        } else {
            None
        };

        // Box-pin the watch stream so it can be replaced on reconnect
        let mut watch_stream: std::pin::Pin<Box<dyn futures::Stream<Item = rusternetes_common::Result<WatchEvent>> + Send>> = Box::pin(watch_stream);

        // Watch loop with timeout support
        let watch_future = async {
            loop {
                tokio::select! {
                    // Process watch events
                    event_opt = watch_stream.next() => {
                        match event_opt {
                            Some(Ok(WatchEvent::Added(key, value))) => {
                                debug!("Watch event - Added: {}", key);
                                if let Ok(object) = serde_json::from_str::<T>(&value) {
                                    // Update latest resourceVersion
                                    if let Some(rv) = object.metadata().resource_version.as_ref() {
                                        latest_resource_version = Some(rv.clone());
                                    }

                                    // Filter by label and field selectors
                                    if !matches_label_selector(object.metadata(), &label_selector)
                                        || !matches_field_selector(object.metadata(), &field_selector)
                                    {
                                        continue;
                                    }

                                    let k8s_event = K8sWatchEvent {
                                        event_type: WatchEventType::Added,
                                        object,
                                    };
                                    if let Ok(json) = serde_json::to_string(&k8s_event) {
                                        if tx.send(Ok(format!("{}\n", json))).is_err() {
                                            break; // Client disconnected
                                        }
                                    }
                                }
                            }
                            Some(Ok(WatchEvent::Modified(key, value))) => {
                                debug!("Watch event - Modified: {}", key);
                                if let Ok(object) = serde_json::from_str::<T>(&value) {
                                    // Update latest resourceVersion
                                    if let Some(rv) = object.metadata().resource_version.as_ref() {
                                        latest_resource_version = Some(rv.clone());
                                    }

                                    // Filter by label and field selectors
                                    if !matches_label_selector(object.metadata(), &label_selector)
                                        || !matches_field_selector(object.metadata(), &field_selector)
                                    {
                                        continue;
                                    }

                                    let k8s_event = K8sWatchEvent {
                                        event_type: WatchEventType::Modified,
                                        object,
                                    };
                                    if let Ok(json) = serde_json::to_string(&k8s_event) {
                                        if tx.send(Ok(format!("{}\n", json))).is_err() {
                                            break; // Client disconnected
                                        }
                                    }
                                }
                            }
                            Some(Ok(WatchEvent::Deleted(key, prev_value))) => {
                                debug!("Watch event - Deleted: {}", key);
                                // For DELETE events, Kubernetes requires the full object with metadata
                                if let Ok(object) = serde_json::from_str::<T>(&prev_value) {
                                    // Update latest resourceVersion
                                    if let Some(rv) = object.metadata().resource_version.as_ref() {
                                        latest_resource_version = Some(rv.clone());
                                    }

                                    // Filter by label and field selectors
                                    if !matches_label_selector(object.metadata(), &label_selector)
                                        || !matches_field_selector(object.metadata(), &field_selector)
                                    {
                                        continue;
                                    }

                                    let k8s_event = K8sWatchEvent {
                                        event_type: WatchEventType::Deleted,
                                        object,
                                    };
                                    if let Ok(json) = serde_json::to_string(&k8s_event) {
                                        if tx.send(Ok(format!("{}\n", json))).is_err() {
                                            break; // Client disconnected
                                        }
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                // Empty watch responses and transient errors are normal —
                                // etcd sends keep-alive responses with no events. Don't break.
                                debug!("Watch stream transient error (continuing): {}", e);
                                continue;
                            }
                            None => {
                                // Watch stream ended — resubscribe from cache.
                                // The cache handles etcd reconnection internally.
                                let new_rx = state.watch_cache.subscribe(&prefix).await;
                                watch_stream = Box::pin(crate::watch_cache::broadcast_to_stream(new_rx));
                                continue;
                            }
                        }
                    }
                    // Send periodic bookmarks if enabled
                    _ = async {
                        if let Some(ref mut interval) = bookmark_interval {
                            interval.tick().await;
                        } else {
                            // If bookmarks are disabled, park this branch forever
                            futures::future::pending::<()>().await
                        }
                    } => {
                        if allow_bookmarks || send_initial_events {
                            if let Some(ref rv) = latest_resource_version {
                                debug!("Sending bookmark with resourceVersion: {}", rv);
                                let bookmark = BookmarkObject {
                                    kind: Some(bookmark_kind.clone()),
                                    api_version: Some(bookmark_api_version.clone()),
                                    metadata: ObjectMeta {
                                        resource_version: Some(rv.clone()),
                                        ..Default::default()
                                    },
                                };
                                let k8s_event = K8sWatchEvent {
                                    event_type: WatchEventType::Bookmark,
                                    object: bookmark,
                                };
                                if let Ok(json) = serde_json::to_string(&k8s_event) {
                                    let _ = tx.send(Ok(format!("{}\n", json)));
                                    // Don't break on bookmark send failure — the client
                                    // might have reset just the bookmark stream but the
                                    // watch connection is still alive.
                                }
                            }
                        }
                    }
                }
            }
        };

        // Apply timeout if specified
        if let Some(timeout_dur) = timeout_duration {
            match timeout(timeout_dur, watch_future).await {
                Ok(_) => {
                    debug!("Watch stream completed normally");
                }
                Err(_) => {
                    info!("Watch stream timeout after {:?}", timeout_dur);
                    // Send final bookmark before closing if bookmarks are enabled
                    if allow_bookmarks || send_initial_events {
                        if let Some(ref rv) = latest_resource_version {
                            let bookmark = BookmarkObject {
                                    kind: Some(bookmark_kind.clone()),
                                    api_version: Some(bookmark_api_version.clone()),
                                    metadata: ObjectMeta {
                                    resource_version: Some(rv.clone()),
                                    ..Default::default()
                                },
                            };
                            let k8s_event = K8sWatchEvent {
                                event_type: WatchEventType::Bookmark,
                                object: bookmark,
                            };
                            if let Ok(json) = serde_json::to_string(&k8s_event) {
                                let _ = tx.send(Ok(format!("{}\n", json)));
                            }
                        }
                    }
                }
            }
        } else {
            // No timeout, run forever
            watch_future.await;
        }
    });

    // Convert receiver to stream
    let stream = UnboundedReceiverStream::new(rx);

    // Build response with proper headers for streaming
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .body(Body::from_stream(stream))
        .map_err(|e| Error::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// Check if an object matches a label selector
fn matches_label_selector(metadata: &ObjectMeta, selector: &Option<String>) -> bool {
    let selector = match selector {
        Some(s) if !s.is_empty() => s,
        _ => return true, // No selector = match all
    };

    let labels = match &metadata.labels {
        Some(l) => l,
        None => return false, // No labels but selector exists = no match
    };

    // Parse selector: "key=value,key2=value2" or "key!=value"
    for requirement in selector.split(',') {
        let requirement = requirement.trim();
        if requirement.is_empty() {
            continue;
        }

        if let Some((key, value)) = requirement.split_once('=') {
            // Handle != (key!=value)
            if key.ends_with('!') {
                let key = key.trim_end_matches('!');
                if labels.get(key).map_or(false, |v| v == value) {
                    return false; // Must NOT equal
                }
            } else {
                // key=value: must match
                if labels.get(key).map_or(true, |v| v != value) {
                    return false;
                }
            }
        } else {
            // Just a key with no value — check existence
            if !labels.contains_key(requirement) {
                return false;
            }
        }
    }
    true
}

/// Check if an object matches a field selector (common: metadata.name, metadata.namespace)
fn matches_field_selector(metadata: &ObjectMeta, selector: &Option<String>) -> bool {
    let selector = match selector {
        Some(s) if !s.is_empty() => s,
        _ => return true,
    };

    for requirement in selector.split(',') {
        let requirement = requirement.trim();
        if let Some((field, value)) = requirement.split_once('=') {
            match field {
                "metadata.name" => {
                    if metadata.name != value {
                        return false;
                    }
                }
                "metadata.namespace" => {
                    if metadata.namespace.as_deref() != Some(value) {
                        return false;
                    }
                }
                _ => {} // Unknown fields pass through
            }
        }
    }
    true
}

/// Derive the Kind and apiVersion from resource_type and api_group
fn resource_type_to_kind_and_version(resource_type: &str, api_group: &str) -> (String, String) {
    let kind = match resource_type {
        "pods" => "Pod",
        "services" => "Service",
        "deployments" => "Deployment",
        "replicasets" => "ReplicaSet",
        "statefulsets" => "StatefulSet",
        "daemonsets" => "DaemonSet",
        "jobs" => "Job",
        "cronjobs" => "CronJob",
        "configmaps" => "ConfigMap",
        "secrets" => "Secret",
        "serviceaccounts" => "ServiceAccount",
        "namespaces" => "Namespace",
        "nodes" => "Node",
        "persistentvolumes" => "PersistentVolume",
        "persistentvolumeclaims" => "PersistentVolumeClaim",
        "endpoints" => "Endpoints",
        "endpointslices" => "EndpointSlice",
        "events" => "Event",
        "ingresses" => "Ingress",
        "networkpolicies" => "NetworkPolicy",
        "leases" => "Lease",
        "clusterroles" => "ClusterRole",
        "clusterrolebindings" => "ClusterRoleBinding",
        "roles" => "Role",
        "rolebindings" => "RoleBinding",
        "storageclasses" => "StorageClass",
        "customresourcedefinitions" => "CustomResourceDefinition",
        "poddisruptionbudgets" => "PodDisruptionBudget",
        "ipaddresses" => "IPAddress",
        other => {
            // CamelCase heuristic: capitalize first letter, remove trailing 's'
            let s = other.strip_suffix('s').unwrap_or(other);
            return (
                format!("{}{}", &s[..1].to_uppercase(), &s[1..]),
                if api_group.is_empty() {
                    "v1".to_string()
                } else {
                    format!("{}/v1", api_group)
                },
            );
        }
    };
    let api_version = if api_group.is_empty() {
        "v1".to_string()
    } else {
        format!("{}/v1", api_group)
    };
    (kind.to_string(), api_version)
}

/// Trait for types that have metadata (all Kubernetes resources)
pub trait HasMetadata {
    fn metadata(&self) -> &ObjectMeta;
    fn metadata_mut(&mut self) -> &mut ObjectMeta;
}

/// Bookmark object containing only metadata with resourceVersion
/// Note: Bookmarks in Kubernetes watch streams don't need apiVersion/kind
/// as they are just checkpoint markers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkObject {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(rename = "apiVersion", skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    pub metadata: ObjectMeta,
}

// Implement for common resource types
// Macro to reduce boilerplate for HasMetadata implementations
macro_rules! impl_has_metadata {
    ($($type:ty),*) => {
        $(
            impl HasMetadata for $type {
                fn metadata(&self) -> &ObjectMeta {
                    &self.metadata
                }
                fn metadata_mut(&mut self) -> &mut ObjectMeta {
                    &mut self.metadata
                }
            }
        )*
    };
}

impl_has_metadata!(
    rusternetes_common::resources::Pod,
    rusternetes_common::resources::Service,
    rusternetes_common::resources::Deployment,
    rusternetes_common::resources::ConfigMap,
    rusternetes_common::resources::Secret,
    rusternetes_common::resources::Node,
    rusternetes_common::resources::Namespace,
    rusternetes_common::resources::Endpoints,
    rusternetes_common::resources::EndpointSlice,
    rusternetes_common::resources::StatefulSet,
    rusternetes_common::resources::ReplicaSet,
    rusternetes_common::resources::DaemonSet,
    rusternetes_common::resources::Job,
    rusternetes_common::resources::CronJob,
    rusternetes_common::resources::Event,
    rusternetes_common::resources::ServiceAccount,
    rusternetes_common::resources::PersistentVolume,
    rusternetes_common::resources::PersistentVolumeClaim,
    rusternetes_common::resources::Lease,
    rusternetes_common::resources::Ingress,
    rusternetes_common::resources::NetworkPolicy,
    rusternetes_common::resources::PodDisruptionBudget,
    rusternetes_common::resources::IPAddress,
    rusternetes_common::resources::PodTemplate,
    rusternetes_common::resources::ControllerRevision
);

// Concrete handler functions for specific resources

/// Watch pods in a namespace
pub async fn watch_pods(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<WatchParams>,
) -> Result<impl IntoResponse> {
    watch_namespaced::<rusternetes_common::resources::Pod>(
        state, auth_ctx, namespace, "pods", "", params,
    )
    .await
}

/// Watch services in a namespace
pub async fn watch_services(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<WatchParams>,
) -> Result<Response> {
    watch_namespaced::<rusternetes_common::resources::Service>(
        state, auth_ctx, namespace, "services", "", params,
    )
    .await
}

/// Watch deployments in a namespace
pub async fn watch_deployments(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<WatchParams>,
) -> Result<impl IntoResponse> {
    watch_namespaced::<rusternetes_common::resources::Deployment>(
        state,
        auth_ctx,
        namespace,
        "deployments",
        "apps",
        params,
    )
    .await
}

/// Watch configmaps in a namespace
pub async fn watch_configmaps(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<WatchParams>,
) -> Result<impl IntoResponse> {
    watch_namespaced::<rusternetes_common::resources::ConfigMap>(
        state,
        auth_ctx,
        namespace,
        "configmaps",
        "",
        params,
    )
    .await
}

/// Watch secrets in a namespace
pub async fn watch_secrets(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<WatchParams>,
) -> Result<impl IntoResponse> {
    watch_namespaced::<rusternetes_common::resources::Secret>(
        state, auth_ctx, namespace, "secrets", "", params,
    )
    .await
}

/// Watch nodes (cluster-scoped)
pub async fn watch_nodes(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<WatchParams>,
) -> Result<impl IntoResponse> {
    watch_cluster_scoped::<rusternetes_common::resources::Node>(
        state, auth_ctx, "nodes", "", params,
    )
    .await
}

/// Watch namespaces (cluster-scoped)
pub async fn watch_namespaces(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<WatchParams>,
) -> Result<Response> {
    watch_cluster_scoped::<rusternetes_common::resources::Namespace>(
        state,
        auth_ctx,
        "namespaces",
        "",
        params,
    )
    .await
}

/// Watch endpoints in a namespace
pub async fn watch_endpoints(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<WatchParams>,
) -> Result<Response> {
    watch_namespaced::<rusternetes_common::resources::Endpoints>(
        state,
        auth_ctx,
        namespace,
        "endpoints",
        "",
        params,
    )
    .await
}

/// Watch endpointslices in a namespace
pub async fn watch_endpointslices(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<WatchParams>,
) -> Result<Response> {
    watch_namespaced::<rusternetes_common::resources::EndpointSlice>(
        state,
        auth_ctx,
        namespace,
        "endpointslices",
        "discovery.k8s.io",
        params,
    )
    .await
}

/// Watch statefulsets in a namespace
pub async fn watch_statefulsets(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<WatchParams>,
) -> Result<Response> {
    watch_namespaced::<rusternetes_common::resources::StatefulSet>(
        state,
        auth_ctx,
        namespace,
        "statefulsets",
        "apps",
        params,
    )
    .await
}

/// Watch replicasets in a namespace
pub async fn watch_replicasets(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<WatchParams>,
) -> Result<Response> {
    watch_namespaced::<rusternetes_common::resources::ReplicaSet>(
        state,
        auth_ctx,
        namespace,
        "replicasets",
        "apps",
        params,
    )
    .await
}

/// Watch daemonsets in a namespace
pub async fn watch_daemonsets(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<WatchParams>,
) -> Result<Response> {
    watch_namespaced::<rusternetes_common::resources::DaemonSet>(
        state,
        auth_ctx,
        namespace,
        "daemonsets",
        "apps",
        params,
    )
    .await
}

/// Watch jobs in a namespace
pub async fn watch_jobs(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<WatchParams>,
) -> Result<Response> {
    watch_namespaced::<rusternetes_common::resources::Job>(
        state, auth_ctx, namespace, "jobs", "batch", params,
    )
    .await
}

/// Watch cronjobs in a namespace
pub async fn watch_cronjobs(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<WatchParams>,
) -> Result<Response> {
    watch_namespaced::<rusternetes_common::resources::CronJob>(
        state, auth_ctx, namespace, "cronjobs", "batch", params,
    )
    .await
}

/// Watch events in a namespace
pub async fn watch_events(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<WatchParams>,
) -> Result<Response> {
    watch_namespaced::<rusternetes_common::resources::Event>(
        state, auth_ctx, namespace, "events", "", params,
    )
    .await
}

/// Watch serviceaccounts in a namespace
pub async fn watch_serviceaccounts(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<WatchParams>,
) -> Result<Response> {
    watch_namespaced::<rusternetes_common::resources::ServiceAccount>(
        state,
        auth_ctx,
        namespace,
        "serviceaccounts",
        "",
        params,
    )
    .await
}

/// Watch persistentvolumes (cluster-scoped)
pub async fn watch_persistentvolumes(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(params): Query<WatchParams>,
) -> Result<Response> {
    watch_cluster_scoped::<rusternetes_common::resources::PersistentVolume>(
        state,
        auth_ctx,
        "persistentvolumes",
        "",
        params,
    )
    .await
}

/// Watch persistentvolumeclaims in a namespace
pub async fn watch_persistentvolumeclaims(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(params): Query<WatchParams>,
) -> Result<Response> {
    watch_namespaced::<rusternetes_common::resources::PersistentVolumeClaim>(
        state,
        auth_ctx,
        namespace,
        "persistentvolumeclaims",
        "",
        params,
    )
    .await
}
