use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ObjectMeta is metadata that all persisted resources must have
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObjectMeta {
    /// Name must be unique within a namespace
    #[serde(default)]
    pub name: String,

    /// Namespace defines the space within which the resource name must be unique
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,

    /// UID is a unique identifier for the resource (auto-generated if not provided)
    #[serde(default = "generate_uid", skip_serializing_if = "String::is_empty")]
    pub uid: String,

    /// ResourceVersion is an opaque value for concurrency control
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_version: Option<String>,

    /// CreationTimestamp is the creation time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creation_timestamp: Option<DateTime<Utc>>,

    /// DeletionTimestamp is the time when the resource will be deleted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deletion_timestamp: Option<DateTime<Utc>>,

    /// Labels are key-value pairs for categorization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,

    /// Annotations are key-value pairs for arbitrary metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<HashMap<String, String>>,
}

fn generate_uid() -> String {
    String::new()
}

impl ObjectMeta {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: None,
            uid: uuid::Uuid::new_v4().to_string(),
            resource_version: None,
            creation_timestamp: Some(Utc::now()),
            deletion_timestamp: None,
            labels: None,
            annotations: None,
        }
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    pub fn with_labels(mut self, labels: HashMap<String, String>) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Ensure uid is populated (generate if empty)
    pub fn ensure_uid(&mut self) {
        if self.uid.is_empty() {
            self.uid = uuid::Uuid::new_v4().to_string();
        }
    }

    /// Ensure creation timestamp is set
    pub fn ensure_creation_timestamp(&mut self) {
        if self.creation_timestamp.is_none() {
            self.creation_timestamp = Some(Utc::now());
        }
    }
}

/// TypeMeta describes the type of the object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TypeMeta {
    /// Kind is the object's type
    pub kind: String,

    /// APIVersion defines the versioned schema of the object
    pub api_version: String,
}

/// Resource status phase
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Phase {
    Pending,
    Running,
    Succeeded,
    Failed,
    Unknown,
}

/// Label selector for matching resources
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LabelSelector {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_labels: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_expressions: Option<Vec<LabelSelectorRequirement>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LabelSelectorRequirement {
    pub key: String,
    pub operator: String, // In, NotIn, Exists, DoesNotExist
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
}

/// Resource requirements for compute resources
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceRequirements {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub requests: Option<HashMap<String, String>>,
}
