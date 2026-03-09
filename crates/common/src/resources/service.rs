use crate::types::{ObjectMeta, TypeMeta};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Service is an abstraction for exposing applications running on a set of Pods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub metadata: ObjectMeta,
    pub spec: ServiceSpec,
}

impl Service {
    pub fn new(name: impl Into<String>, spec: ServiceSpec) -> Self {
        Self {
            type_meta: TypeMeta {
                kind: "Service".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: ObjectMeta::new(name),
            spec,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<HashMap<String, String>>,

    pub ports: Vec<ServicePort>,

    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub service_type: Option<ServiceType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_ip: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_ips: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_affinity: Option<String>, // ClientIP or None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicePort {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    pub port: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_port: Option<u16>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>, // TCP, UDP, SCTP

    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceType {
    ClusterIP,
    NodePort,
    LoadBalancer,
    ExternalName,
}
