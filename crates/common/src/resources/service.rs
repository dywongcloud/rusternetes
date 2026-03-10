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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ServiceStatus>,
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
            status: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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

/// ServiceStatus represents the current status of a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    #[serde(skip_serializing_if = "Option::is_none", rename = "loadBalancer")]
    pub load_balancer: Option<LoadBalancerStatus>,
}

/// LoadBalancerStatus represents the status of a load balancer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerStatus {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub ingress: Vec<LoadBalancerIngress>,
}

/// LoadBalancerIngress represents the status of a load balancer ingress point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerIngress {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
}
