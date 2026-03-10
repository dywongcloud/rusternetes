use crate::types::{ObjectMeta, Phase, ResourceRequirements, TypeMeta};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pod is the smallest deployable unit in Kubernetes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pod {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub metadata: ObjectMeta,
    pub spec: PodSpec,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<PodStatus>,
}

impl Pod {
    pub fn new(name: impl Into<String>, spec: PodSpec) -> Self {
        Self {
            type_meta: TypeMeta {
                kind: "Pod".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: ObjectMeta::new(name),
            spec,
            status: None,
        }
    }
}

/// PodSpec describes the desired state of a pod
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PodSpec {
    pub containers: Vec<Container>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Vec<Volume>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_policy: Option<String>, // Always, OnFailure, Never

    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_selector: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_account_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_network: Option<bool>,

    /// Affinity rules for pod scheduling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affinity: Option<Affinity>,

    /// Tolerations for node taints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerations: Option<Vec<Toleration>>,

    /// Priority value - higher priority pods are scheduled first
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,

    /// Priority class name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_class_name: Option<String>,
}

/// Container represents a single container in a pod
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Container {
    pub name: String,
    pub image: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<ContainerPort>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<EnvVar>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceRequirements>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_mounts: Option<Vec<VolumeMount>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_pull_policy: Option<String>, // Always, Never, IfNotPresent

    #[serde(skip_serializing_if = "Option::is_none")]
    pub liveness_probe: Option<Probe>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub readiness_probe: Option<Probe>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_probe: Option<Probe>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerPort {
    pub container_port: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>, // TCP, UDP, SCTP

    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_from: Option<EnvVarSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVarSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_map_key_ref: Option<ConfigMapKeySelector>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_key_ref: Option<SecretKeySelector>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMapKeySelector {
    pub name: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretKeySelector {
    pub name: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeMount {
    pub name: String,
    pub mount_path: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_dir: Option<EmptyDirVolumeSource>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_path: Option<HostPathVolumeSource>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_map: Option<ConfigMapVolumeSource>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<SecretVolumeSource>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistent_volume_claim: Option<PersistentVolumeClaimVolumeSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmptyDirVolumeSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostPathVolumeSource {
    pub path: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMapVolumeSource {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretVolumeSource {
    pub secret_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistentVolumeClaimVolumeSource {
    pub claim_name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
}

/// PodStatus represents the current state of a pod
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PodStatus {
    pub phase: Phase,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_ip: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pod_ip: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_statuses: Option<Vec<ContainerStatus>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerStatus {
    pub name: String,
    pub ready: bool,
    pub restart_count: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ContainerState>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContainerState {
    Waiting { reason: Option<String> },
    Running { started_at: Option<String> },
    Terminated { exit_code: i32, reason: Option<String> },
}

/// Affinity is a group of affinity scheduling rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Affinity {
    /// Node affinity scheduling rules
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_affinity: Option<NodeAffinity>,

    /// Pod affinity scheduling rules
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pod_affinity: Option<PodAffinity>,

    /// Pod anti-affinity scheduling rules
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pod_anti_affinity: Option<PodAntiAffinity>,
}

/// Node affinity is a group of node affinity scheduling rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAffinity {
    /// Hard node affinity requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_during_scheduling_ignored_during_execution: Option<NodeSelector>,

    /// Soft node affinity preferences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_during_scheduling_ignored_during_execution: Option<Vec<PreferredSchedulingTerm>>,
}

/// A node selector represents the union of the results of one or more label queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSelector {
    /// A list of node selector terms (ORed together)
    pub node_selector_terms: Vec<NodeSelectorTerm>,
}

/// A node selector term is associated with the corresponding weight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSelectorTerm {
    /// A list of node selector requirements by node's labels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_expressions: Option<Vec<NodeSelectorRequirement>>,

    /// A list of node selector requirements by node's fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_fields: Option<Vec<NodeSelectorRequirement>>,
}

/// A node selector requirement is a selector that contains values, a key, and an operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSelectorRequirement {
    /// The label key
    pub key: String,

    /// Operator: In, NotIn, Exists, DoesNotExist, Gt, Lt
    pub operator: String,

    /// An array of string values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
}

/// An empty preferred scheduling term matches all objects with implicit weight 0
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferredSchedulingTerm {
    /// Weight associated with matching the corresponding nodeSelectorTerm, in the range 1-100
    pub weight: i32,

    /// A node selector term, associated with the corresponding weight
    pub preference: NodeSelectorTerm,
}

/// Pod affinity is a group of inter pod affinity scheduling rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodAffinity {
    /// Hard pod affinity requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_during_scheduling_ignored_during_execution: Option<Vec<PodAffinityTerm>>,

    /// Soft pod affinity preferences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_during_scheduling_ignored_during_execution: Option<Vec<WeightedPodAffinityTerm>>,
}

/// Pod anti-affinity is a group of inter pod anti affinity scheduling rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodAntiAffinity {
    /// Hard pod anti-affinity requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_during_scheduling_ignored_during_execution: Option<Vec<PodAffinityTerm>>,

    /// Soft pod anti-affinity preferences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_during_scheduling_ignored_during_execution: Option<Vec<WeightedPodAffinityTerm>>,
}

/// Defines a set of pods that should be co-located
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodAffinityTerm {
    /// A label selector over a set of resources
    pub label_selector: crate::types::LabelSelector,

    /// Namespaces specifies which namespaces the labelSelector applies to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespaces: Option<Vec<String>>,

    /// Topology key for pod placement
    pub topology_key: String,
}

/// The weights of all the matched WeightedPodAffinityTerm fields are added per-node to find the most preferred node(s)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedPodAffinityTerm {
    /// Weight associated with matching the corresponding podAffinityTerm, in the range 1-100
    pub weight: i32,

    /// Required pod affinity term
    pub pod_affinity_term: PodAffinityTerm,
}

/// The pod this Toleration is attached to tolerates any taint that matches the triple using the matching operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Toleration {
    /// Key is the taint key that the toleration applies to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,

    /// Operator represents a key's relationship to the value: Equal or Exists
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<String>,

    /// Value is the taint value the toleration matches to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// Effect indicates the taint effect to match: NoSchedule, PreferNoSchedule, NoExecute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect: Option<String>,

    /// TolerationSeconds represents the period of time the toleration tolerates the taint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toleration_seconds: Option<i64>,
}

/// Probe describes a health check to be performed against a container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Probe {
    /// HTTP GET probe
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_get: Option<HTTPGetAction>,

    /// TCP socket probe
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tcp_socket: Option<TCPSocketAction>,

    /// Exec command probe
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec: Option<ExecAction>,

    /// Number of seconds after the container has started before probes are initiated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_delay_seconds: Option<i32>,

    /// Number of seconds after which the probe times out
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<i32>,

    /// How often (in seconds) to perform the probe
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_seconds: Option<i32>,

    /// Minimum consecutive successes for the probe to be considered successful
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_threshold: Option<i32>,

    /// Minimum consecutive failures for the probe to be considered failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_threshold: Option<i32>,
}

/// HTTPGetAction describes an action based on HTTP Get requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPGetAction {
    /// Path to access on the HTTP server
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Port to access on the container
    pub port: i32,

    /// Host name to connect to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,

    /// Scheme to use for connecting (HTTP or HTTPS)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,

    /// Custom headers to set in the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_headers: Option<Vec<HTTPHeader>>,
}

/// HTTPHeader describes a custom header to be used in HTTP probes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPHeader {
    pub name: String,
    pub value: String,
}

/// TCPSocketAction describes an action based on opening a socket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TCPSocketAction {
    /// Port to connect to on the container
    pub port: i32,

    /// Host name to connect to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
}

/// ExecAction describes a command-based action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecAction {
    /// Command to execute
    pub command: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pod_with_pvc_volume_serialization() {
        let json = r#"{
  "apiVersion": "v1",
  "kind": "Pod",
  "metadata": {
    "name": "test-pod",
    "namespace": "default"
  },
  "spec": {
    "containers": [
      {
        "name": "test-container",
        "image": "nginx:latest",
        "volumeMounts": [
          {
            "name": "test-volume",
            "mountPath": "/data"
          }
        ]
      }
    ],
    "volumes": [
      {
        "name": "test-volume",
        "persistentVolumeClaim": {
          "claimName": "test-pvc"
        }
      }
    ]
  }
}"#;

        // Test deserialization
        let pod: Pod = serde_json::from_str(json).expect("Failed to deserialize Pod");

        assert_eq!(pod.metadata.name, "test-pod");
        assert_eq!(pod.spec.containers.len(), 1);
        assert_eq!(pod.spec.containers[0].name, "test-container");

        // Check volumes
        assert!(pod.spec.volumes.is_some(), "volumes should be Some");
        let volumes = pod.spec.volumes.as_ref().unwrap();
        assert_eq!(volumes.len(), 1);
        assert_eq!(volumes[0].name, "test-volume");
        assert!(volumes[0].persistent_volume_claim.is_some(), "persistent_volume_claim should be Some");
        assert_eq!(volumes[0].persistent_volume_claim.as_ref().unwrap().claim_name, "test-pvc");

        // Check volume mounts
        assert!(pod.spec.containers[0].volume_mounts.is_some(), "volume_mounts should be Some");
        let mounts = pod.spec.containers[0].volume_mounts.as_ref().unwrap();
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].name, "test-volume");
        assert_eq!(mounts[0].mount_path, "/data");

        // Test serialization
        let serialized = serde_json::to_string_pretty(&pod).expect("Failed to serialize Pod");
        
        // Verify round-trip
        let pod2: Pod = serde_json::from_str(&serialized).expect("Failed to deserialize serialized Pod");
        assert!(pod2.spec.volumes.is_some());
        assert_eq!(pod2.spec.volumes.as_ref().unwrap()[0].name, "test-volume");
    }
}
