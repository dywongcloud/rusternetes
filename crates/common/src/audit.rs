// Audit logging for Kubernetes API requests
//
// This module provides audit logging functionality that tracks all API requests
// for security, compliance, and debugging purposes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tracing::{error, info};

/// Audit event representing an API request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    /// API version of the audit event
    pub api_version: String,
    /// Kind is always "Event"
    pub kind: String,
    /// Level of audit detail
    pub level: AuditLevel,
    /// Unique ID for this audit event
    pub audit_id: String,
    /// Stage of the request (RequestReceived, ResponseStarted, ResponseComplete, Panic)
    pub stage: AuditStage,
    /// Request URI
    pub request_uri: String,
    /// HTTP verb (GET, POST, PUT, DELETE, etc.)
    pub verb: String,
    /// Authenticated user information
    pub user: UserInfo,
    /// Resource being accessed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_ref: Option<ObjectReference>,
    /// HTTP response code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_status: Option<ResponseStatus>,
    /// Request received timestamp
    pub request_received_timestamp: DateTime<Utc>,
    /// Stage timestamp
    pub stage_timestamp: DateTime<Utc>,
    /// Annotations (optional metadata)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuditLevel {
    /// No logging
    None,
    /// Metadata only (no request/response bodies)
    Metadata,
    /// Metadata + request body (no response body)
    Request,
    /// Metadata + request body + response body
    RequestResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuditStage {
    /// Request received
    RequestReceived,
    /// Response headers sent
    ResponseStarted,
    /// Response complete
    ResponseComplete,
    /// Panic during request processing
    Panic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub username: String,
    pub uid: String,
    pub groups: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<std::collections::HashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectReference {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseStatus {
    pub code: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Audit policy defining what to log
#[derive(Debug, Clone)]
pub struct AuditPolicy {
    /// Minimum level to log
    pub level: AuditLevel,
    /// Whether to log requests
    pub log_requests: bool,
    /// Whether to log responses
    pub log_responses: bool,
    /// Whether to log metadata changes
    pub log_metadata: bool,
}

impl Default for AuditPolicy {
    fn default() -> Self {
        Self {
            level: AuditLevel::Metadata,
            log_requests: true,
            log_responses: false,
            log_metadata: true,
        }
    }
}

/// Audit backend for writing audit events
#[async_trait::async_trait]
pub trait AuditBackend: Send + Sync {
    /// Write an audit event
    async fn log(&self, event: AuditEvent) -> Result<(), String>;

    /// Flush any buffered events
    async fn flush(&self) -> Result<(), String>;
}

/// File-based audit backend
pub struct FileAuditBackend {
    file: Arc<Mutex<tokio::fs::File>>,
    path: String,
}

impl FileAuditBackend {
    pub async fn new(path: String) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        info!("Audit logging enabled: writing to {}", path);

        Ok(Self {
            file: Arc::new(Mutex::new(file)),
            path,
        })
    }
}

#[async_trait::async_trait]
impl AuditBackend for FileAuditBackend {
    async fn log(&self, event: AuditEvent) -> Result<(), String> {
        // Serialize event to JSON
        let json = match serde_json::to_string(&event) {
            Ok(j) => j,
            Err(e) => {
                error!("Failed to serialize audit event: {}", e);
                return Err(format!("Serialization error: {}", e));
            }
        };

        // Write to file
        let mut file = self.file.lock().await;
        if let Err(e) = file.write_all(json.as_bytes()).await {
            error!("Failed to write audit event to {}: {}", self.path, e);
            return Err(format!("Write error: {}", e));
        }
        if let Err(e) = file.write_all(b"\n").await {
            error!("Failed to write newline to audit log: {}", e);
            return Err(format!("Write error: {}", e));
        }

        Ok(())
    }

    async fn flush(&self) -> Result<(), String> {
        let mut file = self.file.lock().await;
        if let Err(e) = file.flush().await {
            error!("Failed to flush audit log: {}", e);
            return Err(format!("Flush error: {}", e));
        }
        Ok(())
    }
}

/// Audit logger
pub struct AuditLogger {
    backend: Arc<dyn AuditBackend>,
    policy: AuditPolicy,
}

impl AuditLogger {
    pub fn new(backend: Arc<dyn AuditBackend>, policy: AuditPolicy) -> Self {
        Self { backend, policy }
    }

    /// Log an API request
    pub async fn log_request(
        &self,
        request_uri: String,
        verb: String,
        user: UserInfo,
        object_ref: Option<ObjectReference>,
    ) -> String {
        if self.policy.level == AuditLevel::None {
            return String::new();
        }

        let audit_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        let event = AuditEvent {
            api_version: "audit.k8s.io/v1".to_string(),
            kind: "Event".to_string(),
            level: self.policy.level.clone(),
            audit_id: audit_id.clone(),
            stage: AuditStage::RequestReceived,
            request_uri,
            verb,
            user,
            object_ref,
            response_status: None,
            request_received_timestamp: now,
            stage_timestamp: now,
            annotations: None,
        };

        if let Err(e) = self.backend.log(event).await {
            error!("Failed to log audit event: {}", e);
        }

        audit_id
    }

    /// Log an API response
    pub async fn log_response(
        &self,
        audit_id: String,
        request_uri: String,
        verb: String,
        user: UserInfo,
        object_ref: Option<ObjectReference>,
        status_code: u16,
        message: Option<String>,
    ) {
        if self.policy.level == AuditLevel::None {
            return;
        }

        let now = Utc::now();

        let event = AuditEvent {
            api_version: "audit.k8s.io/v1".to_string(),
            kind: "Event".to_string(),
            level: self.policy.level.clone(),
            audit_id,
            stage: AuditStage::ResponseComplete,
            request_uri,
            verb,
            user,
            object_ref,
            response_status: Some(ResponseStatus {
                code: status_code,
                message,
            }),
            request_received_timestamp: now, // Should be the original timestamp
            stage_timestamp: now,
            annotations: None,
        };

        if let Err(e) = self.backend.log(event).await {
            error!("Failed to log audit event: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_level() {
        let level = AuditLevel::Metadata;
        assert_eq!(level, AuditLevel::Metadata);
    }

    #[test]
    fn test_audit_stage() {
        let stage = AuditStage::RequestReceived;
        assert_eq!(stage, AuditStage::RequestReceived);
    }

    #[test]
    fn test_audit_policy_default() {
        let policy = AuditPolicy::default();
        assert_eq!(policy.level, AuditLevel::Metadata);
        assert!(policy.log_requests);
        assert!(!policy.log_responses);
        assert!(policy.log_metadata);
    }

    #[tokio::test]
    async fn test_audit_event_serialization() {
        let event = AuditEvent {
            api_version: "audit.k8s.io/v1".to_string(),
            kind: "Event".to_string(),
            level: AuditLevel::Metadata,
            audit_id: "test-id".to_string(),
            stage: AuditStage::RequestReceived,
            request_uri: "/api/v1/pods".to_string(),
            verb: "GET".to_string(),
            user: UserInfo {
                username: "test-user".to_string(),
                uid: "123".to_string(),
                groups: vec!["system:authenticated".to_string()],
                extra: None,
            },
            object_ref: None,
            response_status: None,
            request_received_timestamp: Utc::now(),
            stage_timestamp: Utc::now(),
            annotations: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("test-user"));
        assert!(json.contains("GET"));
    }
}
