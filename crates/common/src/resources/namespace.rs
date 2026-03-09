use crate::types::{ObjectMeta, Phase, TypeMeta};
use serde::{Deserialize, Serialize};

/// Namespace provides a scope for resource names
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Namespace {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub metadata: ObjectMeta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<NamespaceStatus>,
}

impl Namespace {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            type_meta: TypeMeta {
                kind: "Namespace".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: ObjectMeta::new(name),
            status: Some(NamespaceStatus {
                phase: Phase::Running,
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceStatus {
    pub phase: Phase,
}
