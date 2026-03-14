use crate::types::{ObjectMeta, TypeMeta};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Node is a worker machine in Kubernetes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub metadata: ObjectMeta,
    pub spec: Option<NodeSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<NodeStatus>,
}

impl Node {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            type_meta: TypeMeta {
                kind: "Node".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: ObjectMeta::new(name),
            spec: None,
            status: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pod_cidr: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,

    #[serde(skip_serializing_if = "is_unschedulable_none")]
    #[serde(default)]
    pub unschedulable: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub taints: Option<Vec<Taint>>,
}

fn is_unschedulable_none(value: &Option<bool>) -> bool {
    // Always serialize unschedulable field, even if it's Some(false)
    // Only skip if it's None
    value.is_none()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Taint {
    pub key: String,
    pub value: Option<String>,
    pub effect: String, // NoSchedule, PreferNoSchedule, NoExecute
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allocatable: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<NodeCondition>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub addresses: Option<Vec<NodeAddress>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_info: Option<NodeSystemInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeCondition {
    #[serde(rename = "type")]
    pub condition_type: String, // Ready, MemoryPressure, DiskPressure, PIDPressure, NetworkUnavailable

    pub status: String, // True, False, Unknown

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_heartbeat_time: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_transition_time: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeAddress {
    #[serde(rename = "type")]
    pub address_type: String, // Hostname, ExternalIP, InternalIP

    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSystemInfo {
    pub machine_id: String,
    pub system_uuid: String,
    pub boot_id: String,
    pub kernel_version: String,
    pub os_image: String,
    pub container_runtime_version: String,
    pub kubelet_version: String,
    pub kube_proxy_version: String,
    pub operating_system: String,
    pub architecture: String,
}
