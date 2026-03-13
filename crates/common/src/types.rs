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

    /// DeletionGracePeriodSeconds is the number of seconds before the object should be deleted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deletion_grace_period_seconds: Option<i64>,

    /// Labels are key-value pairs for categorization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,

    /// Annotations are key-value pairs for arbitrary metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<HashMap<String, String>>,

    /// Finalizers are pre-deletion hooks that must complete before deletion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finalizers: Option<Vec<String>>,

    /// OwnerReferences are references to objects that own this object
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_references: Option<Vec<OwnerReference>>,
}

fn generate_uid() -> String {
    String::new()
}

impl Default for ObjectMeta {
    fn default() -> Self {
        Self {
            name: String::new(),
            namespace: None,
            uid: String::new(),
            resource_version: None,
            creation_timestamp: None,
            deletion_timestamp: None,
            deletion_grace_period_seconds: None,
            labels: None,
            annotations: None,
            finalizers: None,
            owner_references: None,
        }
    }
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
            deletion_grace_period_seconds: None,
            labels: None,
            annotations: None,
            finalizers: None,
            owner_references: None,
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

    pub fn with_annotations(mut self, annotations: HashMap<String, String>) -> Self {
        self.annotations = Some(annotations);
        self
    }

    pub fn with_owner_reference(mut self, owner: OwnerReference) -> Self {
        self.owner_references = Some(vec![owner]);
        self
    }

    pub fn with_finalizers(mut self, finalizers: Vec<String>) -> Self {
        self.finalizers = Some(finalizers);
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

    /// Check if the object is being deleted (has deletion timestamp)
    pub fn is_being_deleted(&self) -> bool {
        self.deletion_timestamp.is_some()
    }

    /// Add a finalizer to the object
    pub fn add_finalizer(&mut self, finalizer: String) {
        let finalizers = self.finalizers.get_or_insert_with(Vec::new);
        if !finalizers.contains(&finalizer) {
            finalizers.push(finalizer);
        }
    }

    /// Remove a finalizer from the object
    pub fn remove_finalizer(&mut self, finalizer: &str) {
        if let Some(finalizers) = &mut self.finalizers {
            finalizers.retain(|f| f != finalizer);
        }
    }

    /// Check if the object has any finalizers
    pub fn has_finalizers(&self) -> bool {
        self.finalizers.as_ref().map_or(false, |f| !f.is_empty())
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
    Active,
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
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirements {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub requests: Option<HashMap<String, String>>,
}

/// OwnerReference contains information about an object that owns another object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OwnerReference {
    /// API version of the referent
    pub api_version: String,

    /// Kind of the referent
    pub kind: String,

    /// Name of the referent
    pub name: String,

    /// UID of the referent
    pub uid: String,

    /// If true, AND if the owner has the "foregroundDeletion" finalizer, then
    /// the owner cannot be deleted from the key-value store until this
    /// reference is removed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_owner_deletion: Option<bool>,

    /// If true, this reference points to the managing controller
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controller: Option<bool>,
}

impl OwnerReference {
    /// Create a new owner reference
    pub fn new(
        api_version: impl Into<String>,
        kind: impl Into<String>,
        name: impl Into<String>,
        uid: impl Into<String>,
    ) -> Self {
        Self {
            api_version: api_version.into(),
            kind: kind.into(),
            name: name.into(),
            uid: uid.into(),
            block_owner_deletion: None,
            controller: None,
        }
    }

    /// Mark this reference as the managing controller
    pub fn with_controller(mut self, is_controller: bool) -> Self {
        self.controller = Some(is_controller);
        self
    }

    /// Set whether this reference blocks owner deletion
    pub fn with_block_owner_deletion(mut self, block: bool) -> Self {
        self.block_owner_deletion = Some(block);
        self
    }
}

/// Deletion propagation policy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DeletionPropagation {
    /// Orphan dependents (default for some resources)
    Orphan,
    /// Foreground deletion - delete dependents first
    Foreground,
    /// Background deletion - delete owner first, let GC clean up dependents
    Background,
}

/// ListMeta describes metadata for list objects
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ListMeta {
    /// resourceVersion is a version string for the list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_version: Option<String>,

    /// continue token for pagination
    #[serde(skip_serializing_if = "Option::is_none", rename = "continue")]
    pub continue_token: Option<String>,

    /// remainingItemCount is the number of subsequent items in the list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_item_count: Option<i64>,
}

impl Default for ListMeta {
    fn default() -> Self {
        Self {
            resource_version: None,
            continue_token: None,
            remaining_item_count: None,
        }
    }
}

/// List is a generic wrapper for Kubernetes list responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct List<T> {
    /// APIVersion defines the versioned schema of this representation
    pub api_version: String,

    /// Kind is a string value representing the REST resource
    pub kind: String,

    /// Standard list metadata
    pub metadata: ListMeta,

    /// List of objects
    pub items: Vec<T>,
}

impl<T> List<T> {
    /// Create a new List with the specified kind and API version
    pub fn new(kind: impl Into<String>, api_version: impl Into<String>, items: Vec<T>) -> Self {
        Self {
            kind: kind.into(),
            api_version: api_version.into(),
            metadata: ListMeta::default(),
            items,
        }
    }

    /// Create a new List with resource version
    pub fn with_resource_version(mut self, resource_version: impl Into<String>) -> Self {
        self.metadata.resource_version = Some(resource_version.into());
        self
    }
}
