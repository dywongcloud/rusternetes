use crate::types::{ObjectMeta, TypeMeta};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Lease defines a lease concept
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lease {
    #[serde(flatten)]
    pub type_meta: TypeMeta,

    pub metadata: ObjectMeta,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<LeaseSpec>,
}

impl Lease {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            type_meta: TypeMeta {
                kind: "Lease".to_string(),
                api_version: "coordination.k8s.io/v1".to_string(),
            },
            metadata: ObjectMeta::new(name).with_namespace(namespace),
            spec: None,
        }
    }

    pub fn with_spec(mut self, spec: LeaseSpec) -> Self {
        self.spec = Some(spec);
        self
    }
}

/// LeaseSpec is a specification of a Lease
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaseSpec {
    /// holderIdentity contains the identity of the holder of a current lease
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holder_identity: Option<String>,

    /// leaseDurationSeconds is a duration that candidates for a lease need to wait to force acquire it
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lease_duration_seconds: Option<i32>,

    /// acquireTime is the time when the current lease was acquired
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acquire_time: Option<DateTime<Utc>>,

    /// renewTime is the time when the current holder of a lease has last updated the lease
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renew_time: Option<DateTime<Utc>>,

    /// leaseTransitions is the number of transitions of a lease between holders
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lease_transitions: Option<i32>,
}
