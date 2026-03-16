use crate::types::{ObjectMeta, TypeMeta};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// ControllerRevision represents an immutable snapshot of state data
/// Used by controllers like StatefulSet and DaemonSet to track rollback history
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerRevision {
    #[serde(flatten)]
    pub type_meta: TypeMeta,

    pub metadata: ObjectMeta,

    /// Data is the serialized representation of the state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,

    /// Revision indicates the revision of the state represented by Data
    pub revision: i64,
}

impl ControllerRevision {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>, revision: i64) -> Self {
        Self {
            type_meta: TypeMeta {
                kind: "ControllerRevision".to_string(),
                api_version: "apps/v1".to_string(),
            },
            metadata: ObjectMeta::new(name).with_namespace(namespace),
            data: None,
            revision,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_controller_revision_creation() {
        let revision_data = json!({
            "spec": {
                "replicas": 3,
                "template": {
                    "metadata": {
                        "labels": {
                            "app": "nginx"
                        }
                    }
                }
            }
        });

        let cr = ControllerRevision::new("nginx-revision-1", "default", 1).with_data(revision_data);

        assert_eq!(cr.metadata.name, "nginx-revision-1");
        assert_eq!(cr.metadata.namespace, Some("default".to_string()));
        assert_eq!(cr.type_meta.kind, "ControllerRevision");
        assert_eq!(cr.type_meta.api_version, "apps/v1");
        assert_eq!(cr.revision, 1);
        assert!(cr.data.is_some());
    }

    #[test]
    fn test_controller_revision_without_data() {
        let cr = ControllerRevision::new("test-revision", "default", 5);

        assert_eq!(cr.revision, 5);
        assert!(cr.data.is_none());
    }
}
