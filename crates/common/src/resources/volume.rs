use crate::types::{ObjectMeta, TypeMeta};
use crate::resources::service_account::ObjectReference;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// PersistentVolume represents a storage resource in the cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistentVolume {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub metadata: ObjectMeta,
    pub spec: PersistentVolumeSpec,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<PersistentVolumeStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistentVolumeSpec {
    /// Storage capacity
    pub capacity: HashMap<String, String>,

    /// Volume source
    #[serde(flatten)]
    pub volume_source: PersistentVolumeSource,

    /// Access modes
    pub access_modes: Vec<PersistentVolumeAccessMode>,

    /// Reclaim policy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistent_volume_reclaim_policy: Option<PersistentVolumeReclaimPolicy>,

    /// Storage class name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_class_name: Option<String>,

    /// Mount options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mount_options: Option<Vec<String>>,

    /// Volume mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_mode: Option<PersistentVolumeMode>,

    /// Node affinity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_affinity: Option<VolumeNodeAffinity>,

    /// Claim reference (binding to a PVC)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_ref: Option<ObjectReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PersistentVolumeSource {
    HostPath(HostPathVolumeSource),
    #[serde(rename = "nfs")]
    NFS(NFSVolumeSource),
    #[serde(rename = "iscsi")]
    ISCSI(ISCSIVolumeSource),
    Local(LocalVolumeSource),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostPathVolumeSource {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<HostPathType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HostPathType {
    DirectoryOrCreate,
    Directory,
    FileOrCreate,
    File,
    Socket,
    CharDevice,
    BlockDevice,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NFSVolumeSource {
    pub server: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ISCSIVolumeSource {
    pub target_portal: String,
    pub iqn: String,
    pub lun: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fs_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalVolumeSource {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fs_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PersistentVolumeAccessMode {
    ReadWriteOnce,
    ReadOnlyMany,
    ReadWriteMany,
    ReadWriteOncePod,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PersistentVolumeReclaimPolicy {
    Retain,
    Recycle,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PersistentVolumeMode {
    Filesystem,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeNodeAffinity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<NodeSelector>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSelector {
    pub node_selector_terms: Vec<NodeSelectorTerm>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSelectorTerm {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_expressions: Option<Vec<NodeSelectorRequirement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_fields: Option<Vec<NodeSelectorRequirement>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSelectorRequirement {
    pub key: String,
    pub operator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistentVolumeStatus {
    pub phase: PersistentVolumePhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PersistentVolumePhase {
    Pending,
    Available,
    Bound,
    Released,
    Failed,
}

/// PersistentVolumeClaim is a request for storage by a user
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistentVolumeClaim {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub metadata: ObjectMeta,
    pub spec: PersistentVolumeClaimSpec,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<PersistentVolumeClaimStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistentVolumeClaimSpec {
    /// Access modes
    pub access_modes: Vec<PersistentVolumeAccessMode>,

    /// Resource requirements
    pub resources: ResourceRequirements,

    /// Volume name (if binding to specific PV)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_name: Option<String>,

    /// Storage class name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_class_name: Option<String>,

    /// Volume mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_mode: Option<PersistentVolumeMode>,

    /// Selector for PV
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<LabelSelector>,

    /// Data source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_source: Option<TypedLocalObjectReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirements {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requests: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelSelector {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_labels: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_expressions: Option<Vec<LabelSelectorRequirement>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelSelectorRequirement {
    pub key: String,
    pub operator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypedLocalObjectReference {
    pub api_group: Option<String>,
    pub kind: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistentVolumeClaimStatus {
    pub phase: PersistentVolumeClaimPhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_modes: Option<Vec<PersistentVolumeAccessMode>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<PersistentVolumeClaimCondition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PersistentVolumeClaimPhase {
    Pending,
    Bound,
    Lost,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistentVolumeClaimCondition {
    pub r#type: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_probe_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_transition_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// StorageClass describes the parameters for a class of storage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageClass {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub metadata: ObjectMeta,

    /// Provisioner name
    pub provisioner: String,

    /// Parameters for the provisioner
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<HashMap<String, String>>,

    /// Reclaim policy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reclaim_policy: Option<PersistentVolumeReclaimPolicy>,

    /// Volume binding mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_binding_mode: Option<VolumeBindingMode>,

    /// Allowed topologies
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_topologies: Option<Vec<TopologySelectorTerm>>,

    /// Allow volume expansion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_volume_expansion: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VolumeBindingMode {
    Immediate,
    WaitForFirstConsumer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologySelectorTerm {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_label_expressions: Option<Vec<TopologySelectorLabelRequirement>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologySelectorLabelRequirement {
    pub key: String,
    pub values: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pv_serialization() {
        let mut capacity = HashMap::new();
        capacity.insert("storage".to_string(), "10Gi".to_string());

        let pv = PersistentVolume {
            type_meta: TypeMeta {
                kind: "PersistentVolume".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: ObjectMeta::new("test-pv"),
            spec: PersistentVolumeSpec {
                capacity,
                volume_source: PersistentVolumeSource::HostPath(HostPathVolumeSource {
                    path: "/mnt/data".to_string(),
                    r#type: Some(HostPathType::DirectoryOrCreate),
                }),
                access_modes: vec![PersistentVolumeAccessMode::ReadWriteOnce],
                persistent_volume_reclaim_policy: Some(PersistentVolumeReclaimPolicy::Retain),
                storage_class_name: Some("manual".to_string()),
                mount_options: None,
                volume_mode: Some(PersistentVolumeMode::Filesystem),
                node_affinity: None,
                claim_ref: None,
            },
            status: Some(PersistentVolumeStatus {
                phase: PersistentVolumePhase::Available,
                message: None,
                reason: None,
            }),
        };

        let json = serde_json::to_string(&pv).unwrap();
        let _deserialized: PersistentVolume = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_pvc_serialization() {
        let mut requests = HashMap::new();
        requests.insert("storage".to_string(), "5Gi".to_string());

        let pvc = PersistentVolumeClaim {
            type_meta: TypeMeta {
                kind: "PersistentVolumeClaim".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: ObjectMeta::new("test-pvc").with_namespace("default"),
            spec: PersistentVolumeClaimSpec {
                access_modes: vec![PersistentVolumeAccessMode::ReadWriteOnce],
                resources: ResourceRequirements {
                    limits: None,
                    requests: Some(requests),
                },
                volume_name: None,
                storage_class_name: Some("manual".to_string()),
                volume_mode: Some(PersistentVolumeMode::Filesystem),
                selector: None,
                data_source: None,
            },
            status: Some(PersistentVolumeClaimStatus {
                phase: PersistentVolumeClaimPhase::Pending,
                access_modes: None,
                capacity: None,
                conditions: None,
            }),
        };

        let json = serde_json::to_string(&pvc).unwrap();
        let _deserialized: PersistentVolumeClaim = serde_json::from_str(&json).unwrap();
    }
}
