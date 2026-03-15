use crate::types::{ObjectMeta, Phase, ResourceRequirements, TypeMeta};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Macro to create a skip_serializing_if function for Option<T> where T has all optional fields.
/// This prevents serializing empty structs as {} when all fields are None.
macro_rules! skip_if_empty {
    ($fn_name:ident, $type:ty, $($field:ident),+) => {
        fn $fn_name(value: &Option<$type>) -> bool {
            match value {
                None => true,
                Some(v) => {
                    $(v.$field.is_none())&&+
                }
            }
        }
    };
}

/// Pod is the smallest deployable unit in Kubernetes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pod {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub metadata: ObjectMeta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<PodSpec>,
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
            spec: Some(spec),
            status: None,
        }
    }
}

/// PodSpec describes the desired state of a pod
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PodSpec {
    pub containers: Vec<Container>,

    /// Init containers run before app containers and must complete successfully
    /// Sidecar containers are init containers with restartPolicy: Always
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init_containers: Option<Vec<Container>>,

    /// Ephemeral containers are temporary containers for debugging
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_containers: Option<Vec<EphemeralContainer>>,

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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_pid: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_ipc: Option<bool>,

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

    /// AutomountServiceAccountToken indicates whether a service account token should be automatically mounted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automount_service_account_token: Option<bool>,

    /// TopologySpreadConstraints describes how a group of pods ought to spread across topology domains
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topology_spread_constraints: Option<Vec<TopologySpreadConstraint>>,

    /// Overhead represents the resource overhead associated with running a pod
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overhead: Option<HashMap<String, String>>,

    /// SchedulerName is the name of the scheduler to be used to schedule this pod
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler_name: Option<String>,

    /// ResourceClaims defines which ResourceClaims must be allocated and reserved before the Pod is allowed to start
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_claims: Option<Vec<PodResourceClaim>>,
}

/// PodResourceClaim references a ResourceClaim that must be allocated for the pod
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PodResourceClaim {
    /// Name uniquely identifies this resource claim inside the pod
    pub name: String,

    /// Source describes where to find the ResourceClaim
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ClaimSource>,
}

/// ClaimSource describes a reference to a ResourceClaim
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimSource {
    /// ResourceClaimName is the name of a ResourceClaim object in the same namespace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_claim_name: Option<String>,

    /// ResourceClaimTemplateName is the name of a ResourceClaimTemplate object in the same namespace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_claim_template_name: Option<String>,
}

/// EphemeralContainer is a temporary container added to a running pod for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EphemeralContainer {
    pub name: String,
    pub image: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<EnvVar>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_mounts: Option<Vec<VolumeMount>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_pull_policy: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_context: Option<SecurityContext>,

    /// TargetContainerName is the name of the container to attach to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_container_name: Option<String>,

    /// Stdin enables redirecting stdin to the ephemeral container
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdin: Option<bool>,

    /// StdinOnce closes stdin after the first attach
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdin_once: Option<bool>,

    /// TTY allocates a pseudo-TTY for the ephemeral container
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tty: Option<bool>,
}

/// TopologySpreadConstraint specifies how to spread pods across topology domains
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologySpreadConstraint {
    /// MaxSkew describes the degree to which pods may be unevenly distributed
    pub max_skew: i32,

    /// TopologyKey is the key of node labels
    pub topology_key: String,

    /// WhenUnsatisfiable indicates how to deal with a pod if it doesn't satisfy the spread constraint
    /// Possible values: DoNotSchedule, ScheduleAnyway
    pub when_unsatisfiable: String,

    /// LabelSelector is used to find matching pods
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_selector: Option<crate::types::LabelSelector>,

    /// MinDomains indicates a minimum number of eligible domains
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_domains: Option<i32>,

    /// NodeAffinityPolicy indicates how we will treat Pod's nodeAffinity/nodeSelector
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_affinity_policy: Option<String>,

    /// NodeTaintsPolicy indicates how we will treat node taints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_taints_policy: Option<String>,

    /// MatchLabelKeys is a set of pod label keys to select pods
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_label_keys: Option<Vec<String>>,
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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_context: Option<SecurityContext>,

    /// RestartPolicy for the container. Only applies to init containers.
    /// Possible values: Always (sidecar container that runs alongside main containers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privileged: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_as_user: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_as_non_root: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_privilege_escalation: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Capabilities>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub seccomp_profile: Option<SeccompProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub drop: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeccompProfile {
    pub r#type: String,  // RuntimeDefault, Unconfined, Localhost

    #[serde(skip_serializing_if = "Option::is_none")]
    pub localhost_profile: Option<String>,
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

// Generate skip functions for structs with all-optional fields
// NOTE: Do NOT use skip_if_empty for structs with mutually exclusive fields:
// - EnvVarSource: only one of (config_map_key_ref, secret_key_ref, field_ref, resource_field_ref) should be set
// - ClaimSource: only one of (resource_claim_name, resource_claim_template_name) should be set
// Using skip_if_empty would incorrectly skip serialization when only one field is set.
skip_if_empty!(skip_empty_security_context, SecurityContext, privileged, run_as_user, run_as_non_root, allow_privilege_escalation, capabilities, seccomp_profile);
skip_if_empty!(skip_empty_capabilities, Capabilities, add, drop);
skip_if_empty!(skip_empty_empty_dir_volume_source, EmptyDirVolumeSource, medium);
skip_if_empty!(skip_empty_config_map_volume_source, ConfigMapVolumeSource, name, items, default_mode, optional);
skip_if_empty!(skip_empty_secret_volume_source, SecretVolumeSource, secret_name, items, default_mode, optional);
skip_if_empty!(skip_empty_downward_api_volume_source, DownwardAPIVolumeSource, items, default_mode);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvVar {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_from: Option<EnvVarSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvVarSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_map_key_ref: Option<ConfigMapKeySelector>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_key_ref: Option<SecretKeySelector>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_ref: Option<ObjectFieldSelector>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_field_ref: Option<ResourceFieldSelector>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigMapKeySelector {
    pub name: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

    /// Downward API volume source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downward_api: Option<DownwardAPIVolumeSource>,

    /// CSI (Container Storage Interface) ephemeral inline volume
    #[serde(skip_serializing_if = "Option::is_none")]
    pub csi: Option<crate::resources::csi::CSIVolumeSource>,

    /// Generic ephemeral volume with volumeClaimTemplate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<EphemeralVolumeSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmptyDirVolumeSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostPathVolumeSource {
    pub path: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigMapVolumeSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<KeyToPath>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_mode: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretVolumeSource {
    #[serde(skip_serializing_if = "Option::is_none", alias = "secret_name")]
    pub secret_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<KeyToPath>>,

    #[serde(skip_serializing_if = "Option::is_none", alias = "default_mode")]
    pub default_mode: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyToPath {
    pub key: String,
    pub path: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistentVolumeClaimVolumeSource {
    pub claim_name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
}

/// DownwardAPIVolumeSource represents a downward API volume
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownwardAPIVolumeSource {
    /// Items is a list of downward API volume file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<DownwardAPIVolumeFile>>,

    /// Optional: mode bits to use on created files by default
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_mode: Option<i32>,
}

/// DownwardAPIVolumeFile represents information to create the file containing the pod field
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownwardAPIVolumeFile {
    /// Required: Path is the relative path name of the file to be created
    pub path: String,

    /// Required: Selects a field of the pod
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_ref: Option<ObjectFieldSelector>,

    /// Selects a resource of the container
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_field_ref: Option<ResourceFieldSelector>,

    /// Optional: mode bits to use on this file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<i32>,
}

/// ObjectFieldSelector selects an API object field
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectFieldSelector {
    /// Path of the field to select in the specified API version
    pub field_path: String,

    /// Version of the schema the FieldPath is written in terms of, defaults to "v1"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
}

/// ResourceFieldSelector selects a resource of the container
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceFieldSelector {
    /// Container name: required for volumes, optional for env vars
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,

    /// Required: resource to select
    pub resource: String,

    /// Specifies the output format of the exposed resources, defaults to "1"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub divisor: Option<String>,
}

/// EphemeralVolumeSource represents an ephemeral volume with a volume claim template
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EphemeralVolumeSource {
    /// Will be used to create a stand-alone PVC to provision the volume
    pub volume_claim_template: Option<PersistentVolumeClaimTemplate>,
}

/// PersistentVolumeClaimTemplate is used to produce PVC objects
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistentVolumeClaimTemplate {
    /// May contain labels and annotations that will be copied into the PVC
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ObjectMeta>,

    /// The specification for the PersistentVolumeClaim
    pub spec: crate::resources::volume::PersistentVolumeClaimSpec,
}

/// PodStatus represents the current state of a pod
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PodStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<Phase>,

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

    /// Status of init containers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init_container_statuses: Option<Vec<ContainerStatus>>,

    /// Status of ephemeral containers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_container_statuses: Option<Vec<ContainerStatus>>,
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
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ContainerState {
    Waiting { reason: Option<String> },
    Running { started_at: Option<String> },
    Terminated { exit_code: i32, reason: Option<String> },
}

/// Affinity is a group of affinity scheduling rules
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct NodeAffinity {
    /// Hard node affinity requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_during_scheduling_ignored_during_execution: Option<NodeSelector>,

    /// Soft node affinity preferences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_during_scheduling_ignored_during_execution: Option<Vec<PreferredSchedulingTerm>>,
}

/// A node selector represents the union of the results of one or more label queries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NodeSelector {
    /// A list of node selector terms (ORed together)
    pub node_selector_terms: Vec<NodeSelectorTerm>,
}

/// A node selector term is associated with the corresponding weight
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NodeSelectorTerm {
    /// A list of node selector requirements by node's labels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_expressions: Option<Vec<NodeSelectorRequirement>>,

    /// A list of node selector requirements by node's fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_fields: Option<Vec<NodeSelectorRequirement>>,
}

/// A node selector requirement is a selector that contains values, a key, and an operator
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreferredSchedulingTerm {
    /// Weight associated with matching the corresponding nodeSelectorTerm, in the range 1-100
    pub weight: i32,

    /// A node selector term, associated with the corresponding weight
    pub preference: NodeSelectorTerm,
}

/// Pod affinity is a group of inter pod affinity scheduling rules
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct WeightedPodAffinityTerm {
    /// Weight associated with matching the corresponding podAffinityTerm, in the range 1-100
    pub weight: i32,

    /// Required pod affinity term
    pub pod_affinity_term: PodAffinityTerm,
}

/// The pod this Toleration is attached to tolerates any taint that matches the triple using the matching operator
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct HTTPHeader {
    pub name: String,
    pub value: String,
}

/// TCPSocketAction describes an action based on opening a socket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TCPSocketAction {
    /// Port to connect to on the container
    pub port: i32,

    /// Host name to connect to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
}

/// ExecAction describes a command-based action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
        let spec = pod.spec.as_ref().unwrap();
        assert_eq!(spec.containers.len(), 1);
        assert_eq!(spec.containers[0].name, "test-container");

        // Check volumes
        assert!(spec.volumes.is_some(), "volumes should be Some");
        let volumes = spec.volumes.as_ref().unwrap();
        assert_eq!(volumes.len(), 1);
        assert_eq!(volumes[0].name, "test-volume");
        assert!(volumes[0].persistent_volume_claim.is_some(), "persistent_volume_claim should be Some");
        assert_eq!(volumes[0].persistent_volume_claim.as_ref().unwrap().claim_name, "test-pvc");

        // Check volume mounts
        assert!(spec.containers[0].volume_mounts.is_some(), "volume_mounts should be Some");
        let mounts = spec.containers[0].volume_mounts.as_ref().unwrap();
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].name, "test-volume");
        assert_eq!(mounts[0].mount_path, "/data");

        // Test serialization
        let serialized = serde_json::to_string_pretty(&pod).expect("Failed to serialize Pod");

        // Verify round-trip
        let pod2: Pod = serde_json::from_str(&serialized).expect("Failed to deserialize serialized Pod");
        let spec2 = pod2.spec.as_ref().unwrap();
        assert!(spec2.volumes.is_some());
        assert_eq!(spec2.volumes.as_ref().unwrap()[0].name, "test-volume");
    }

    #[test]
    fn test_pod_with_init_containers() {
        let spec = PodSpec {
            init_containers: Some(vec![
                Container {
                    name: "init-myservice".to_string(),
                    image: "busybox:1.28".to_string(),
                    command: Some(vec!["sh".to_string(), "-c".to_string(), "echo initializing".to_string()]),
                    args: None,
                    working_dir: None,
                    ports: None,
                    env: None,
                    resources: None,
                    volume_mounts: None,
                    image_pull_policy: None,
                    liveness_probe: None,
                    readiness_probe: None,
                    startup_probe: None,
                    security_context: None,
                    restart_policy: None,
                },
                Container {
                    name: "init-mydb".to_string(),
                    image: "busybox:1.28".to_string(),
                    command: Some(vec!["sh".to_string(), "-c".to_string(), "echo waiting for db".to_string()]),
                    args: None,
                    working_dir: None,
                    ports: None,
                    env: None,
                    resources: None,
                    volume_mounts: None,
                    image_pull_policy: None,
                    liveness_probe: None,
                    readiness_probe: None,
                    startup_probe: None,
                    security_context: None,
                    restart_policy: None,
                },
            ]),
            containers: vec![Container {
                name: "app".to_string(),
                image: "nginx:latest".to_string(),
                command: None,
                args: None,
                working_dir: None,
                ports: Some(vec![ContainerPort {
                    container_port: 80,
                    name: Some("http".to_string()),
                    protocol: Some("TCP".to_string()),
                    host_port: None,
                }]),
                env: None,
                resources: None,
                volume_mounts: None,
                image_pull_policy: None,
                liveness_probe: None,
                readiness_probe: None,
                startup_probe: None,
                security_context: None,
                restart_policy: None,
            }],
            ephemeral_containers: None,
            volumes: None,
            restart_policy: Some("Always".to_string()),
            node_name: None,
            node_selector: None,
            service_account_name: None,
            hostname: None,
            host_network: None,
            host_pid: None,
            host_ipc: None,
            affinity: None,
            tolerations: None,
            priority: None,
            priority_class_name: None,
            automount_service_account_token: None,
            topology_spread_constraints: None,
            overhead: None,
            scheduler_name: None,
            resource_claims: None,
        };

        let pod = Pod::new("myapp-pod", spec);

        assert_eq!(pod.metadata.name, "myapp-pod");
        let pod_spec = pod.spec.as_ref().unwrap();

        // Verify init containers
        assert!(pod_spec.init_containers.is_some());
        let init_containers = pod_spec.init_containers.as_ref().unwrap();
        assert_eq!(init_containers.len(), 2);
        assert_eq!(init_containers[0].name, "init-myservice");
        assert_eq!(init_containers[1].name, "init-mydb");

        // Verify main containers
        assert_eq!(pod_spec.containers.len(), 1);
        assert_eq!(pod_spec.containers[0].name, "app");
    }

    #[test]
    fn test_pod_with_init_containers_serialization() {
        let json = r#"{
  "apiVersion": "v1",
  "kind": "Pod",
  "metadata": {
    "name": "myapp-pod",
    "namespace": "default"
  },
  "spec": {
    "initContainers": [
      {
        "name": "init-myservice",
        "image": "busybox:1.28",
        "command": ["sh", "-c", "until nslookup myservice; do echo waiting for myservice; sleep 2; done;"]
      },
      {
        "name": "init-mydb",
        "image": "busybox:1.28",
        "command": ["sh", "-c", "until nslookup mydb; do echo waiting for mydb; sleep 2; done;"]
      }
    ],
    "containers": [
      {
        "name": "myapp-container",
        "image": "nginx:latest",
        "ports": [
          {
            "containerPort": 80
          }
        ]
      }
    ]
  }
}"#;

        let pod: Pod = serde_json::from_str(json).unwrap();
        assert_eq!(pod.metadata.name, "myapp-pod");

        let spec = pod.spec.as_ref().unwrap();
        assert!(spec.init_containers.is_some());

        let init_containers = spec.init_containers.as_ref().unwrap();
        assert_eq!(init_containers.len(), 2);
        assert_eq!(init_containers[0].name, "init-myservice");
        assert_eq!(init_containers[0].image, "busybox:1.28");
        assert_eq!(init_containers[1].name, "init-mydb");

        assert_eq!(spec.containers.len(), 1);
        assert_eq!(spec.containers[0].name, "myapp-container");

        // Test round-trip serialization
        let serialized = serde_json::to_string(&pod).unwrap();
        let pod2: Pod = serde_json::from_str(&serialized).unwrap();
        assert!(pod2.spec.as_ref().unwrap().init_containers.is_some());
        assert_eq!(pod2.spec.as_ref().unwrap().init_containers.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_pod_status_with_init_container_statuses() {
        let status = PodStatus {
            phase: Some(Phase::Running),
            message: None,
            reason: None,
            host_ip: Some("192.168.1.1".to_string()),
            pod_ip: Some("10.0.0.1".to_string()),
            container_statuses: Some(vec![ContainerStatus {
                name: "app".to_string(),
                ready: true,
                restart_count: 0,
                state: Some(ContainerState::Running {
                    started_at: Some("2024-01-01T00:00:00Z".to_string()),
                }),
                image: Some("nginx:latest".to_string()),
                container_id: Some("containerd://abc123".to_string()),
            }]),
            init_container_statuses: Some(vec![
                ContainerStatus {
                    name: "init-myservice".to_string(),
                    ready: true,
                    restart_count: 0,
                    state: Some(ContainerState::Terminated {
                        exit_code: 0,
                        reason: Some("Completed".to_string()),
                    }),
                    image: Some("busybox:1.28".to_string()),
                    container_id: Some("containerd://def456".to_string()),
                },
                ContainerStatus {
                    name: "init-mydb".to_string(),
                    ready: true,
                    restart_count: 0,
                    state: Some(ContainerState::Terminated {
                        exit_code: 0,
                        reason: Some("Completed".to_string()),
                    }),
                    image: Some("busybox:1.28".to_string()),
                    container_id: Some("containerd://ghi789".to_string()),
                },
            ]),
            ephemeral_container_statuses: None,
        };

        assert_eq!(status.phase, Some(Phase::Running));
        assert!(status.init_container_statuses.is_some());

        let init_statuses = status.init_container_statuses.as_ref().unwrap();
        assert_eq!(init_statuses.len(), 2);
        assert_eq!(init_statuses[0].name, "init-myservice");
        assert_eq!(init_statuses[1].name, "init-mydb");

        // Verify both init containers completed successfully
        for init_status in init_statuses {
            if let Some(ContainerState::Terminated { exit_code, .. }) = &init_status.state {
                assert_eq!(*exit_code, 0);
            }
        }
    }

    #[test]
    fn test_env_var_with_field_ref_serialization() {
        // Test deserialization of env var with fieldRef
        let json = r#"{"name":"NODE_NAME","valueFrom":{"fieldRef":{"fieldPath":"spec.nodeName"}}}"#;
        let env_var: EnvVar = serde_json::from_str(json).expect("Failed to deserialize EnvVar");

        assert_eq!(env_var.name, "NODE_NAME");
        assert!(env_var.value.is_none());
        assert!(env_var.value_from.is_some());

        let value_from = env_var.value_from.as_ref().unwrap();
        assert!(value_from.field_ref.is_some());
        assert_eq!(value_from.field_ref.as_ref().unwrap().field_path, "spec.nodeName");

        // Test serialization - should preserve the fieldRef
        let serialized = serde_json::to_string(&env_var).expect("Failed to serialize EnvVar");
        println!("Serialized: {}", serialized);

        // Verify valueFrom is in the serialized output
        assert!(serialized.contains("valueFrom"));
        assert!(serialized.contains("fieldRef"));
        assert!(serialized.contains("spec.nodeName"));

        // Test round-trip
        let env_var2: EnvVar = serde_json::from_str(&serialized).expect("Failed to deserialize serialized EnvVar");
        assert!(env_var2.value_from.is_some());
        assert!(env_var2.value_from.as_ref().unwrap().field_ref.is_some());
    }

    #[test]
    fn test_env_var_with_field_ref_via_value() {
        // Test the round-trip through serde_json::Value (what the API server does)
        let json = r#"{"name":"NODE_NAME","valueFrom":{"fieldRef":{"fieldPath":"spec.nodeName"}}}"#;
        let env_var: EnvVar = serde_json::from_str(json).expect("Failed to deserialize EnvVar");

        // Convert to Value and back (simulating webhook flow)
        let value = serde_json::to_value(&env_var).expect("Failed to convert to Value");
        println!("As Value: {}", serde_json::to_string_pretty(&value).unwrap());

        let env_var2: EnvVar = serde_json::from_value(value).expect("Failed to convert from Value");

        // Verify fieldRef is still there after round-trip through Value
        assert!(env_var2.value_from.is_some(), "valueFrom should be Some after Value round-trip");
        let value_from = env_var2.value_from.as_ref().unwrap();
        assert!(value_from.field_ref.is_some(), "fieldRef should be Some after Value round-trip");
        assert_eq!(value_from.field_ref.as_ref().unwrap().field_path, "spec.nodeName");
    }

    #[test]
    fn test_pod_spec_clone_serialization() {
        use super::*;

        // Create a PodSpec with env vars that have valueFrom.fieldRef
        let original_spec = PodSpec {
            containers: vec![Container {
                name: "test".to_string(),
                image: "busybox".to_string(),
                env: Some(vec![
                    EnvVar {
                        name: "NODE_NAME".to_string(),
                        value: None,
                        value_from: Some(EnvVarSource {
                            field_ref: Some(ObjectFieldSelector {
                                field_path: "spec.nodeName".to_string(),
                                api_version: None,
                            }),
                            config_map_key_ref: None,
                            secret_key_ref: None,
                            resource_field_ref: None,
                        }),
                    },
                ]),
                command: None,
                args: None,
                working_dir: None,
                ports: None,
                resources: None,
                volume_mounts: None,
                image_pull_policy: None,
                liveness_probe: None,
                readiness_probe: None,
                startup_probe: None,
                security_context: None,
                restart_policy: None,
            }],
            init_containers: None,
            ephemeral_containers: None,
            volumes: None,
            restart_policy: None,
            node_name: None,
            node_selector: None,
            service_account_name: None,
            hostname: None,
            host_network: None,
            host_pid: None,
            host_ipc: None,
            affinity: None,
            tolerations: None,
            priority: None,
            priority_class_name: None,
            automount_service_account_token: None,
            topology_spread_constraints: None,
            overhead: None,
            scheduler_name: None,
            resource_claims: None,
        };

        // Clone it (like DaemonSet controller does)
        let cloned_spec = original_spec.clone();

        // Serialize and deserialize (like etcd storage does)
        let json = serde_json::to_string(&cloned_spec).unwrap();
        println!("Serialized JSON:\n{}", json);

        let deserialized: PodSpec = serde_json::from_str(&json).unwrap();

        // Check if valueFrom.fieldRef is preserved
        let env_var = &deserialized.containers[0].env.as_ref().unwrap()[0];
        assert_eq!(env_var.name, "NODE_NAME");
        assert!(env_var.value_from.is_some(), "value_from should be Some");
        assert!(env_var.value_from.as_ref().unwrap().field_ref.is_some(),
                "field_ref should be Some");
        assert_eq!(
            env_var.value_from.as_ref().unwrap().field_ref.as_ref().unwrap().field_path,
            "spec.nodeName"
        );
    }

    #[test]
    fn test_claim_source_serialization() {
        // Test that ClaimSource with only resource_claim_name set serializes correctly
        // (not as an empty object {})
        let claim_source = ClaimSource {
            resource_claim_name: Some("my-claim".to_string()),
            resource_claim_template_name: None,
        };

        let json = serde_json::to_string(&claim_source).expect("Failed to serialize ClaimSource");
        println!("Serialized ClaimSource: {}", json);

        // Should NOT be an empty object
        assert!(!json.contains("{}"), "ClaimSource should not serialize as empty object");

        // Should contain the resource_claim_name field
        assert!(json.contains("resourceClaimName"), "Should contain resourceClaimName field");
        assert!(json.contains("my-claim"), "Should contain the claim name");

        // Test round-trip
        let deserialized: ClaimSource = serde_json::from_str(&json)
            .expect("Failed to deserialize ClaimSource");
        assert_eq!(deserialized.resource_claim_name, Some("my-claim".to_string()));
        assert_eq!(deserialized.resource_claim_template_name, None);

        // Test with resource_claim_template_name set instead
        let claim_source2 = ClaimSource {
            resource_claim_name: None,
            resource_claim_template_name: Some("my-template".to_string()),
        };

        let json2 = serde_json::to_string(&claim_source2).expect("Failed to serialize ClaimSource");
        println!("Serialized ClaimSource (template): {}", json2);

        assert!(!json2.contains("{}"), "ClaimSource should not serialize as empty object");
        assert!(json2.contains("resourceClaimTemplateName"), "Should contain resourceClaimTemplateName field");
        assert!(json2.contains("my-template"), "Should contain the template name");

        // Test round-trip for template
        let deserialized2: ClaimSource = serde_json::from_str(&json2)
            .expect("Failed to deserialize ClaimSource with template");
        assert_eq!(deserialized2.resource_claim_name, None);
        assert_eq!(deserialized2.resource_claim_template_name, Some("my-template".to_string()));
    }

    #[test]
    fn test_pod_resource_claim_serialization() {
        // Test that PodResourceClaim with ClaimSource serializes correctly
        let resource_claim = PodResourceClaim {
            name: "test-claim".to_string(),
            source: Some(ClaimSource {
                resource_claim_name: Some("my-resource-claim".to_string()),
                resource_claim_template_name: None,
            }),
        };

        let json = serde_json::to_string(&resource_claim)
            .expect("Failed to serialize PodResourceClaim");
        println!("Serialized PodResourceClaim: {}", json);

        // Verify source is present and not empty
        assert!(json.contains("source"), "Should contain source field");
        assert!(json.contains("resourceClaimName"), "Should contain resourceClaimName in source");
        assert!(json.contains("my-resource-claim"), "Should contain the claim name");

        // Test round-trip
        let deserialized: PodResourceClaim = serde_json::from_str(&json)
            .expect("Failed to deserialize PodResourceClaim");
        assert_eq!(deserialized.name, "test-claim");
        assert!(deserialized.source.is_some(), "source should be Some");

        let source = deserialized.source.unwrap();
        assert_eq!(source.resource_claim_name, Some("my-resource-claim".to_string()));
        assert_eq!(source.resource_claim_template_name, None);
    }
}
