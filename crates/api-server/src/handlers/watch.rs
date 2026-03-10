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
    Result, Error,
};
use rusternetes_storage::{build_prefix, Storage, WatchEvent};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::Arc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, error, info};

/// Kubernetes watch event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum WatchEventType {
    Added,
    Modified,
    Deleted,
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
    #[serde(rename = "resourceVersion")]
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
}

/// Generic watch handler for namespaced resources
pub async fn watch_namespaced<T>(
    state: Arc<ApiServerState>,
    auth_ctx: AuthContext,
    namespace: String,
    resource_type: &str,
    api_group: &str,
) -> Result<Response>
where
    T: Serialize + DeserializeOwned + Send + Sync + 'static + Clone + HasMetadata,
{
    info!("Starting watch for {} in namespace {}", resource_type, namespace);

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

    // Create watch stream
    let prefix = build_prefix(resource_type, Some(&namespace));
    let watch_stream = state.storage.watch(&prefix).await?;

    // Create channel for sending events to client
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<std::result::Result<String, std::io::Error>>();

    // Spawn task to convert watch events to HTTP response
    tokio::spawn(async move {
        futures::pin_mut!(watch_stream);

        while let Some(event) = watch_stream.next().await {
            match event {
                Ok(WatchEvent::Added(key, value)) => {
                    debug!("Watch event - Added: {}", key);
                    if let Ok(object) = serde_json::from_str::<T>(&value) {
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
                Ok(WatchEvent::Modified(key, value)) => {
                    debug!("Watch event - Modified: {}", key);
                    if let Ok(object) = serde_json::from_str::<T>(&value) {
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
                Ok(WatchEvent::Deleted(key)) => {
                    debug!("Watch event - Deleted: {}", key);
                    // For delete events, we need to extract the name from the key
                    let _name = key.rsplit('/').next().unwrap_or("");
                    // Create a minimal object with just metadata for delete events
                    // This would need the actual resource type to construct properly
                    // For now, we'll skip delete events that can't be deserialized
                }
                Err(e) => {
                    error!("Watch stream error: {}", e);
                    break;
                }
            }
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
) -> Result<Response>
where
    T: Serialize + DeserializeOwned + Send + Sync + 'static + Clone + HasMetadata,
{
    info!("Starting watch for cluster-scoped {}", resource_type);

    // Check authorization
    let attrs = RequestAttributes::new(auth_ctx.user.clone(), "watch", resource_type)
        .with_api_group(api_group);

    match state.authorizer.authorize(&attrs).await? {
        Decision::Allow => {}
        Decision::Deny(reason) => {
            return Err(Error::Forbidden(reason));
        }
    }

    // Create watch stream
    let prefix = build_prefix(resource_type, None);
    let watch_stream = state.storage.watch(&prefix).await?;

    // Create channel for sending events to client
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<std::result::Result<String, std::io::Error>>();

    // Spawn task to convert watch events to HTTP response
    tokio::spawn(async move {
        futures::pin_mut!(watch_stream);

        while let Some(event) = watch_stream.next().await {
            match event {
                Ok(WatchEvent::Added(key, value)) => {
                    debug!("Watch event - Added: {}", key);
                    if let Ok(object) = serde_json::from_str::<T>(&value) {
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
                Ok(WatchEvent::Modified(key, value)) => {
                    debug!("Watch event - Modified: {}", key);
                    if let Ok(object) = serde_json::from_str::<T>(&value) {
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
                Ok(WatchEvent::Deleted(key)) => {
                    debug!("Watch event - Deleted: {}", key);
                    // For delete events, we'll skip them for now
                    // A complete implementation would need to maintain resource metadata
                }
                Err(e) => {
                    error!("Watch stream error: {}", e);
                    break;
                }
            }
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

/// Trait for types that have metadata (all Kubernetes resources)
pub trait HasMetadata {
    fn metadata(&self) -> &ObjectMeta;
}

// Implement for common resource types
impl HasMetadata for rusternetes_common::resources::Pod {
    fn metadata(&self) -> &ObjectMeta {
        &self.metadata
    }
}

impl HasMetadata for rusternetes_common::resources::Service {
    fn metadata(&self) -> &ObjectMeta {
        &self.metadata
    }
}

impl HasMetadata for rusternetes_common::resources::Deployment {
    fn metadata(&self) -> &ObjectMeta {
        &self.metadata
    }
}

impl HasMetadata for rusternetes_common::resources::ConfigMap {
    fn metadata(&self) -> &ObjectMeta {
        &self.metadata
    }
}

impl HasMetadata for rusternetes_common::resources::Secret {
    fn metadata(&self) -> &ObjectMeta {
        &self.metadata
    }
}

impl HasMetadata for rusternetes_common::resources::Node {
    fn metadata(&self) -> &ObjectMeta {
        &self.metadata
    }
}

impl HasMetadata for rusternetes_common::resources::Namespace {
    fn metadata(&self) -> &ObjectMeta {
        &self.metadata
    }
}

// Concrete handler functions for specific resources

/// Watch pods in a namespace
pub async fn watch_pods(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(_params): Query<WatchParams>,
) -> Result<impl IntoResponse> {
    watch_namespaced::<rusternetes_common::resources::Pod>(
        state,
        auth_ctx,
        namespace,
        "pods",
        "",
    )
    .await
}

/// Watch services in a namespace
pub async fn watch_services(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(_params): Query<WatchParams>,
) -> Result<impl IntoResponse> {
    watch_namespaced::<rusternetes_common::resources::Service>(
        state,
        auth_ctx,
        namespace,
        "services",
        "",
    )
    .await
}

/// Watch deployments in a namespace
pub async fn watch_deployments(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(_params): Query<WatchParams>,
) -> Result<impl IntoResponse> {
    watch_namespaced::<rusternetes_common::resources::Deployment>(
        state,
        auth_ctx,
        namespace,
        "deployments",
        "apps",
    )
    .await
}

/// Watch configmaps in a namespace
pub async fn watch_configmaps(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(_params): Query<WatchParams>,
) -> Result<impl IntoResponse> {
    watch_namespaced::<rusternetes_common::resources::ConfigMap>(
        state,
        auth_ctx,
        namespace,
        "configmaps",
        "",
    )
    .await
}

/// Watch secrets in a namespace
pub async fn watch_secrets(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Path(namespace): Path<String>,
    Query(_params): Query<WatchParams>,
) -> Result<impl IntoResponse> {
    watch_namespaced::<rusternetes_common::resources::Secret>(
        state,
        auth_ctx,
        namespace,
        "secrets",
        "",
    )
    .await
}

/// Watch nodes (cluster-scoped)
pub async fn watch_nodes(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(_params): Query<WatchParams>,
) -> Result<impl IntoResponse> {
    watch_cluster_scoped::<rusternetes_common::resources::Node>(
        state,
        auth_ctx,
        "nodes",
        "",
    )
    .await
}

/// Watch namespaces (cluster-scoped)
pub async fn watch_namespaces(
    State(state): State<Arc<ApiServerState>>,
    Extension(auth_ctx): Extension<AuthContext>,
    Query(_params): Query<WatchParams>,
) -> Result<impl IntoResponse> {
    watch_cluster_scoped::<rusternetes_common::resources::Namespace>(
        state,
        auth_ctx,
        "namespaces",
        "",
    )
    .await
}
