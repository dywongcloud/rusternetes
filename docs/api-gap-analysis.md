# Kubernetes 1.35 API Gap Analysis

This document compares rusternetes type definitions against the Kubernetes 1.35 OpenAPI spec
(`swagger.json`) and identifies missing fields, missing types, and structural deviations.

**Source of truth**: `~/Downloads/swagger.json` (Kubernetes master branch, ~1.35 era)
**Scope**: `core/v1`, `apps/v1`, `batch/v1`, `autoscaling/v2`, `networking/v1`, `rbac/v1`,
`storage/v1`, `policy/v1`

**Last updated**: 2026-03-17 (commit 55b1d34)

---

## Priority Legend

- **P0** – Required for conformance / likely causes test failures today
- **P1** – Required for correct kubectl / client behavior
- **P2** – Important but less commonly exercised
- **P3** – Completeness / nice to have
- ✅ **Done** – Implemented

---

## 1. core/v1 — PodSpec

**File**: `crates/common/src/resources/pod.rs`

### ✅ Completed (commit 1383ff8)

| Field | Status |
|-------|--------|
| `hostAliases: Option<Vec<HostAlias>>` | ✅ Added |
| `os: Option<PodOS>` | ✅ Added |
| `schedulingGates: Option<Vec<PodSchedulingGate>>` | ✅ Added |
| `HostAlias` type | ✅ Added |
| `PodOS` type | ✅ Added |
| `PodSchedulingGate` type | ✅ Added |

### ✅ Completed (commit e24e251)

| Field | Status |
|-------|--------|
| `resources: Option<ResourceRequirements>` | ✅ Added (pod-level resource requests, k8s 1.32+) |

### Still missing

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `schedulingGroup` | `PodSchedulingGroup` | P2 | Group-based scheduling |
| `hostnameOverride` | `string` | P2 | Override hostname independently of setHostnameAsFQDN |
| `serviceAccount` | `string` | P2 | Deprecated alias for serviceAccountName (still present in spec) |

### Missing helper types

| Type | Fields | Priority |
|------|--------|----------|
| `PodSchedulingGroup` | `podGroupName: string` | P2 |

### ✅ Structural fixes (commit 1383ff8)

| Item | Status |
|------|--------|
| `PodResourceClaim` | ✅ Flattened — removed `ClaimSource` wrapper, `resourceClaimName` and `resourceClaimTemplateName` now directly on `PodResourceClaim` |

---

## 2. core/v1 — PodStatus

**File**: `crates/common/src/resources/pod.rs`

### ✅ Completed (commit 1383ff8)

| Field | Status |
|-------|--------|
| `hostIPs: Option<Vec<HostIP>>` | ✅ Added |
| `podIPs: Option<Vec<PodIP>>` | ✅ Added (alongside existing `pod_ip`) |
| `nominatedNodeName: Option<String>` | ✅ Added |
| `qosClass: Option<String>` | ✅ Added |
| `startTime: Option<DateTime<Utc>>` | ✅ Added |
| `PodCondition.observedGeneration: Option<i64>` | ✅ Added |
| `HostIP` type | ✅ Added |
| `PodIP` type | ✅ Added |

### Still missing

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `resize` | `string` | P2 | InProgress/Deferred/Infeasible for resource resizing |
| `resourceClaimStatuses` | `[]PodResourceClaimStatus` | P2 | Status of each resource claim |
| `allocatedResources` | `map[string]Quantity` | P2 | Allocated resources for the pod |
| `extendedResourceClaimStatus` | `[]PodExtendedResourceClaimStatus` | P3 | Extended resource claim tracking |
| `observedGeneration` | `int64` | P2 | Generation the kubelet observed when last syncing |

### Missing helper types

| Type | Fields | Priority |
|------|--------|----------|
| `PodResourceClaimStatus` | `name: string`, `resourceClaimName: string` | P2 |

---

## 3. core/v1 — Container ✅ (partial)

**File**: `crates/common/src/resources/pod.rs`

### ✅ Completed (commit 1383ff8)

| Field | Status |
|-------|--------|
| `resizePolicy: Option<Vec<ContainerResizePolicy>>` | ✅ Added |
| `restartPolicy: Option<String>` | ✅ Added (also used for sidecar detection) |
| `ContainerResizePolicy` type | ✅ Added |

### ✅ Completed (current session)

| Field | Status |
|-------|--------|
| `lifecycle: Option<Lifecycle>` | ✅ Added |

### Still missing

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `restartPolicyRules` | `[]ContainerRestartRule` | P2 | Fine-grained restart rules |
| `stdin` | `bool` | P2 | Keep stdin open |
| `stdinOnce` | `bool` | P2 | Close stdin after first attach |
| `tty` | `bool` | P2 | Allocate TTY |
| `terminationMessagePath` | `string` | P2 | Path for termination message file |
| `terminationMessagePolicy` | `string` | P2 | File or FallbackToLogsOnError |

### Missing helper types

| Type | Fields | Priority |
|------|--------|----------|
| `ContainerRestartRule` | `action: string`, `exitCodes: ContainerRestartRuleOnExitCodes` | P2 |
| `ContainerRestartRuleOnExitCodes` | `operator: string`, `values: []int32` | P2 |

---

## 4. core/v1 — ContainerStatus

**File**: `crates/common/src/resources/pod.rs`

### ✅ Completed (commit 1383ff8)

| Field | Status |
|-------|--------|
| `allocatedResources: Option<HashMap<String, String>>` | ✅ Added |
| `started: Option<bool>` | ✅ Added |

### ✅ Completed (commit e24e251)

| Field | Status |
|-------|--------|
| `allocatedResourcesStatus: Option<Vec<ResourceStatus>>` | ✅ Added |
| `resources: Option<ResourceRequirements>` | ✅ Added |
| `ResourceStatus` type | ✅ Added |
| `ResourceHealth` type | ✅ Added |

### Still missing

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `stopSignal` | `string` | P2 | Stop signal sent to the container |
| `user` | `ContainerUser` | P2 | User that the container process runs as |
| `volumeMounts` | `[]VolumeMountStatus` | P2 | Status of volume mounts |

### Missing helper types

| Type | Fields | Priority |
|------|--------|----------|
| `ContainerUser` | `linux: LinuxContainerUser` | P2 |
| `LinuxContainerUser` | `uid: int64`, `gid: int64`, `supplementalGroups: []int64` | P2 |
| `VolumeMountStatus` | `name: string`, `mountPath: string`, `readOnly: bool`, `recursiveReadOnly: string`, `volumeStatus: VolumeStatus` | P2 |

---

## 5. core/v1 — PodSecurityContext

**File**: `crates/common/src/resources/pod.rs`

### Missing fields

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `seLinuxChangePolicy` | `string` | P2 | MountOption or Recursive |
| `supplementalGroupsPolicy` | `string` | P2 | Merge or Strict |

---

## 6. core/v1 — Volume ✅ (partial)

**File**: `crates/common/src/resources/pod.rs`

### ✅ Completed (current session)

| Field | Status |
|-------|--------|
| `nfs: Option<NFSVolumeSource>` | ✅ Added |
| `iscsi: Option<ISCSIVolumeSource>` | ✅ Added |
| `projected: Option<ProjectedVolumeSource>` | ✅ Added |
| `image: Option<ImageVolumeSource>` | ✅ Added |
| `ProjectedVolumeSource` type | ✅ Added |
| `VolumeProjection` type | ✅ Added |
| `SecretProjection` type | ✅ Added |
| `ConfigMapProjection` type | ✅ Added |
| `ServiceAccountTokenProjection` type | ✅ Added |
| `DownwardAPIProjection` type | ✅ Added |
| `ClusterTrustBundleProjection` type | ✅ Added |
| `ImageVolumeSource` type | ✅ Added |

### Still missing

| Type | Priority | Notes |
|------|----------|-------|
| `csi` (`CSIVolumeSource`) | P2 | Inline CSI volume |
| `hostPath` | P2 | Already partially present but missing `type` field in pod volume context |

---

## 7. core/v1 — Lifecycle / LifecycleHandler ✅ (partial)

**File**: `crates/common/src/resources/pod.rs`

### ✅ Completed (current session)

| Field | Status |
|-------|--------|
| `lifecycle: Option<Lifecycle>` on `Container` | ✅ Added |
| `Lifecycle` type (`post_start`, `pre_stop`) | ✅ Added |
| `LifecycleHandler` type (`exec`, `http_get`, `tcp_socket`, `sleep`) | ✅ Added |
| `SleepAction` type (`seconds: i64`) | ✅ Added |

### Still missing

| Field | K8s Type | Priority |
|-------|----------|----------|
| `stopSignal` on `Lifecycle` | `string` | P2 |

---

## 8. core/v1 — EphemeralContainer ✅ (partial)

**File**: `crates/common/src/resources/pod.rs`

### ✅ Completed (current session)

| Field | Status |
|-------|--------|
| `resizePolicy: Option<Vec<ContainerResizePolicy>>` | ✅ Added |
| `restartPolicy: Option<String>` | ✅ Added |
| `resources: Option<ResourceRequirements>` | ✅ Added |

### Still missing

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `restartPolicyRules` | `[]ContainerRestartRule` | P2 | |
| `stdin`, `stdinOnce`, `tty` | `bool` | P2 | |
| `terminationMessagePath`, `terminationMessagePolicy` | `string` | P2 | |

---

## 9. apps/v1 — DeploymentSpec ✅

**File**: `crates/common/src/resources/deployment.rs`

### ✅ Completed (commit 8f34586)

| Field | Status |
|-------|--------|
| `paused: Option<bool>` | ✅ Added |
| `progressDeadlineSeconds: Option<i32>` | ✅ Added |
| `replicas: i32` → `Option<i32>` | ✅ Fixed |

---

## 10. apps/v1 — DeploymentStatus ✅

**File**: `crates/common/src/resources/deployment.rs`

### ✅ Completed (commit 8f34586)

| Field | Status |
|-------|--------|
| `conditions: Option<Vec<DeploymentCondition>>` | ✅ Added |
| `collisionCount: Option<i32>` | ✅ Added |
| `observedGeneration: Option<i64>` | ✅ Added |
| `DeploymentCondition` type | ✅ Added |

### Still missing

| Field | K8s Type | Priority |
|-------|----------|----------|
| `terminatingReplicas` | `int32` | P2 |

---

## 11. apps/v1 — StatefulSetSpec ✅

**File**: `crates/common/src/resources/workloads.rs`

### ✅ Completed (commit 8f34586)

| Field | Status |
|-------|--------|
| `replicas: i32` → `Option<i32>` | ✅ Fixed |
| `minReadySeconds: Option<i32>` | ✅ Added |
| `revisionHistoryLimit: Option<i32>` | ✅ Added |
| `volumeClaimTemplates: Option<Vec<PersistentVolumeClaim>>` | ✅ Added |
| `persistentVolumeClaimRetentionPolicy` | ✅ Added (`StatefulSetPersistentVolumeClaimRetentionPolicy`) |

### Still missing

| Field | K8s Type | Priority |
|-------|----------|----------|
| `ordinals` | `StatefulSetOrdinals` | P2 |

---

## 12. apps/v1 — StatefulSetStatus ✅

**File**: `crates/common/src/resources/workloads.rs`

### ✅ Completed (commit 8f34586)

| Field | Status |
|-------|--------|
| `availableReplicas: Option<i32>` | ✅ Added |
| `collisionCount: Option<i32>` | ✅ Added |
| `observedGeneration: Option<i64>` | ✅ Added |
| `currentRevision: Option<String>` | ✅ Added |
| `updateRevision: Option<String>` | ✅ Added |
| `conditions: Option<Vec<StatefulSetCondition>>` | ✅ Added |
| `StatefulSetCondition` type | ✅ Added |
| `readyReplicas`, `currentReplicas`, `updatedReplicas` → `Option<i32>` | ✅ Fixed |

---

## 13. apps/v1 — DaemonSetSpec / DaemonSetStatus ✅

**File**: `crates/common/src/resources/workloads.rs`

### ✅ Completed (commit 8f34586)

| Field | Status |
|-------|--------|
| `DaemonSetSpec.minReadySeconds: Option<i32>` | ✅ Added |
| `DaemonSetSpec.revisionHistoryLimit: Option<i32>` | ✅ Added |
| `DaemonSetStatus.numberAvailable: Option<i32>` | ✅ Added |
| `DaemonSetStatus.numberUnavailable: Option<i32>` | ✅ Added |
| `DaemonSetStatus.updatedNumberScheduled: Option<i32>` | ✅ Added |
| `DaemonSetStatus.observedGeneration: Option<i64>` | ✅ Added |
| `DaemonSetStatus.collisionCount: Option<i32>` | ✅ Added |
| `DaemonSetStatus.conditions: Option<Vec<DaemonSetCondition>>` | ✅ Added |
| `DaemonSetCondition` type | ✅ Added |
| `RollingUpdateDaemonSet.maxSurge: Option<String>` | ✅ Added |

---

## 14. batch/v1 — JobSpec ✅ (partial)

**File**: `crates/common/src/resources/workloads.rs`

### ✅ Completed (commit 8f34586)

| Field | Status |
|-------|--------|
| `selector: Option<LabelSelector>` | ✅ Added |
| `manualSelector: Option<bool>` | ✅ Added |
| `suspend: Option<bool>` | ✅ Added |
| `ttlSecondsAfterFinished: Option<i32>` | ✅ Added |
| `completionMode: Option<String>` | ✅ Added |

### Still missing (P2)

| Field | K8s Type | Priority |
|-------|----------|----------|
| `backoffLimitPerIndex` | `int32` | P2 |
| `maxFailedIndexes` | `int32` | P2 |
| `podFailurePolicy` | `PodFailurePolicy` | P2 |
| `podReplacementPolicy` | `string` | P2 |
| `successPolicy` | `SuccessPolicy` | P2 |
| `managedBy` | `string` | P2 |

---

## 15. batch/v1 — JobStatus ✅ (partial)

**File**: `crates/common/src/resources/workloads.rs`

### ✅ Completed (commit 8f34586)

| Field | Status |
|-------|--------|
| `startTime: Option<DateTime<Utc>>` | ✅ Added |
| `completionTime: Option<DateTime<Utc>>` | ✅ Added |
| `ready: Option<i32>` | ✅ Added |
| `terminating: Option<i32>` | ✅ Added |

### Still missing (P2)

| Field | K8s Type | Priority |
|-------|----------|----------|
| `completedIndexes` | `string` | P2 |
| `failedIndexes` | `string` | P2 |
| `uncountedTerminatedPods` | `UncountedTerminatedPods` | P2 |

---

## 16. batch/v1 — CronJobSpec ✅

**File**: `crates/common/src/resources/workloads.rs`

### ✅ Completed (commit 8f34586)

| Field | Status |
|-------|--------|
| `startingDeadlineSeconds: Option<i64>` | ✅ Added |
| `timeZone: Option<String>` | ✅ Added |

---

## 17. core/v1 — ServiceSpec ✅

**File**: `crates/common/src/resources/service.rs`

### ✅ Completed (commit 8f34586)

| Field | Status |
|-------|--------|
| `healthCheckNodePort: Option<i32>` | ✅ Added |
| `loadBalancerClass: Option<String>` | ✅ Added |
| `loadBalancerIP: Option<String>` | ✅ Added |
| `loadBalancerSourceRanges: Option<Vec<String>>` | ✅ Added |
| `allocateLoadBalancerNodePorts: Option<bool>` | ✅ Added |
| `publishNotReadyAddresses: Option<bool>` | ✅ Added |
| `sessionAffinityConfig: Option<SessionAffinityConfig>` | ✅ Added |
| `trafficDistribution: Option<String>` | ✅ Added |
| `ServicePort.appProtocol: Option<String>` | ✅ Added |
| `ServiceStatus.conditions: Option<Vec<Condition>>` | ✅ Added |
| `LoadBalancerIngress.ipMode: Option<String>` | ✅ Added |
| `LoadBalancerIngress.ports: Option<Vec<PortStatus>>` | ✅ Added |
| `SessionAffinityConfig` type | ✅ Added |
| `ClientIPConfig` type | ✅ Added |
| `PortStatus` type | ✅ Added |

---

## 18. core/v1 — NodeSpec ✅ (partial)

**File**: `crates/common/src/resources/node.rs`

### ✅ Completed (current session)

| Field | Status |
|-------|--------|
| `podCIDRs: Option<Vec<String>>` | ✅ Added |

### Still missing

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `configSource` | `NodeConfigSource` | P2 | Dynamic kubelet config |
| `externalID` | `string` | P3 | Deprecated external ID |

---

## 19. core/v1 — NodeStatus

**File**: `crates/common/src/resources/node.rs`

### ✅ Completed (commit e24e251)

| Field | Status |
|-------|--------|
| `images: Option<Vec<ContainerImage>>` | ✅ Added |
| `volumesInUse: Option<Vec<String>>` | ✅ Added |
| `volumesAttached: Option<Vec<AttachedVolume>>` | ✅ Added |
| `daemonEndpoints: Option<NodeDaemonEndpoints>` | ✅ Added |
| `ContainerImage` type | ✅ Added |
| `AttachedVolume` type | ✅ Added |
| `NodeDaemonEndpoints` type | ✅ Added |
| `DaemonEndpoint` type | ✅ Added |

### Still missing

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `config` | `NodeConfigStatus` | P2 | Config source status |
| `features` | `NodeFeatures` | P2 | Feature gate status |
| `runtimeHandlers` | `[]NodeRuntimeHandler` | P2 | Available runtime handlers |
| `declaredFeatures` | object | P2 | Declared node features |
| `phase` | `string` | P3 | Deprecated node phase |

### Missing types

| Type | Fields | Priority |
|------|--------|----------|
| `NodeConfigStatus` | `assigned`, `active`, `lastKnownGood`, `error` | P2 |
| `NodeFeatures` | `supplementalGroupsPolicy: bool` | P2 |
| `NodeRuntimeHandler` | `name: string`, `features: NodeRuntimeHandlerFeatures` | P2 |
| `NodeRuntimeHandlerFeatures` | `recursiveReadOnlyMounts: bool`, `userNamespaces: bool` | P2 |

### Structural issues

| Item | Issue | Priority |
|------|-------|----------|
| `NodeSystemInfo` | Missing `swap: NodeSwapStatus` field | P2 |

---

## 20. core/v1 — PersistentVolumeSpec

**File**: `crates/common/src/resources/volume.rs`

### ✅ Structural fixes (commit 1383ff8)

| Item | Status |
|------|--------|
| `PersistentVolumeSource` enum | ✅ Removed — replaced with flat optional fields on `PersistentVolumeSpec`: `host_path`, `nfs`, `iscsi`, `local`, `csi` |

### Missing volume source types in PV context

| Type | Priority |
|------|----------|
| `AWSElasticBlockStore`, `AzureDisk`, `AzureFile` | P2 |
| `CephFS`, `Cinder`, `FC`, `FlexVolume`, `Flocker` | P3 |
| `GCEPersistentDisk`, `Glusterfs`, `PhotonPersistentDisk` | P3 |
| `PortworxVolume`, `Quobyte`, `RBD`, `ScaleIO` | P3 |
| `StorageOS`, `VsphereVirtualDisk`, `CSI` | P2 |

### Missing fields on PersistentVolumeSpec

| Field | K8s Type | Priority |
|-------|----------|----------|
| `volumeAttributesClassName` | `string` | P2 |

### PersistentVolumeStatus ✅ (partial)

| Field | Status |
|-------|--------|
| `reason: Option<String>` | ✅ Already present |
| `message: Option<String>` | ✅ Already present |

### Still missing on PersistentVolumeStatus

| Field | K8s Type | Priority |
|-------|----------|----------|
| `lastPhaseTransitionTime` | `Time` | P2 |

---

## 21. core/v1 — PersistentVolumeClaimSpec / Status ✅ (partial)

**File**: `crates/common/src/resources/volume.rs`

### ✅ Completed (current session)

| Field | Status |
|-------|--------|
| `dataSource: Option<TypedLocalObjectReference>` | ✅ Added |
| `dataSourceRef: Option<TypedObjectReference>` | ✅ Added |
| `volumeAttributesClassName: Option<String>` | ✅ Added (as `volume_attributes_class_name`) |
| `TypedObjectReference` type | ✅ Added |
| `allocatedResources: Option<HashMap<String, String>>` on Status | ✅ Added |
| `allocatedResourceStatuses: Option<HashMap<String, String>>` on Status | ✅ Added |

### Still missing

| Field | K8s Type | Priority |
|-------|----------|----------|
| `currentVolumeAttributesClassName` on Status | `string` | P2 |
| `modifyVolumeStatus` on Status | `ModifyVolumeStatus` | P2 |

### Missing types

| Type | Fields | Priority |
|------|--------|----------|
| `ModifyVolumeStatus` | `targetVolumeAttributesClassName: string`, `status: string` | P2 |

---

## 22. core/v1 — NamespaceStatus

**File**: `crates/common/src/resources/namespace.rs`

### ✅ Completed (commit e24e251)

| Field | Status |
|-------|--------|
| `conditions: Option<Vec<NamespaceCondition>>` | ✅ Added |
| `NamespaceCondition` type | ✅ Added |

---

## 23. autoscaling/v2 — HPA ✅ (partial)

**File**: `crates/common/src/resources/autoscaling.rs`

### ✅ Completed (current session)

| Item | Status |
|------|--------|
| `HorizontalPodAutoscalerCondition.last_transition_time` | ✅ Fixed: `Option<String>` → `Option<DateTime<Utc>>` |
| `HorizontalPodAutoscalerStatus.last_scale_time` | ✅ Fixed: `Option<String>` → `Option<DateTime<Utc>>` |
| `MetricStatus.containerResource` | ✅ Added: `container_resource: Option<ContainerResourceMetricStatus>` |
| `ContainerResourceMetricStatus` type | ✅ Added |
| `currentReplicas: i32` | ✅ Already present as non-optional |

### Still missing

| Item | Issue | Priority |
|------|-------|----------|
| `HPAScalingRules.tolerance` | Missing `tolerance: Quantity` field (k8s 1.35 new field) | P2 |

---

## 24. core/v1 — ResourceQuota ✅

**File**: `crates/common/src/resources/policy.rs`

### ✅ Already implemented

| Resource | Status |
|----------|--------|
| `ResourceQuota` struct with `spec` and `status` | ✅ Present |
| `ResourceQuotaSpec`: `hard`, `scopes`, `scopeSelector` | ✅ Present |
| `ResourceQuotaStatus`: `hard`, `used` | ✅ Present |
| `ScopeSelector`, `ScopedResourceSelectorRequirement` | ✅ Present |

---

## 25. core/v1 — LimitRange ✅

**File**: `crates/common/src/resources/policy.rs`

### ✅ Already implemented

| Resource | Status |
|----------|--------|
| `LimitRange` struct | ✅ Present |
| `LimitRangeSpec`, `LimitRangeItem` | ✅ Present |

---

## 26. core/v1 — EnvVarSource / Downward API

**File**: `crates/common/src/resources/pod.rs`

### Missing fields on EnvVarSource

| Field | K8s Type | Priority |
|-------|----------|----------|
| `fileKeyRef` | `FileKeySelector` | P2 |

### Missing types

| Type | Fields | Priority |
|------|--------|----------|
| `FileKeySelector` | `volumeName`, `path`, `key`, `optional` | P2 |

---

## 27. storage/v1 — StorageClass, VolumeAttachment, CSIDriver, CSINode ✅ (partial)

**File**: `crates/common/src/resources/volume.rs` (StorageClass), `crates/common/src/resources/csi.rs` (CSIDriver)

### ✅ Already implemented

| Resource | Status |
|----------|--------|
| `StorageClass` | ✅ Present (volume.rs) |
| `CSIDriver` + `CSIDriverSpec` | ✅ Present (csi.rs) |

### Still missing

| Resource | Priority |
|----------|----------|
| `VolumeAttachment` + `VolumeAttachmentSpec` + `VolumeAttachmentStatus` | P2 |
| `CSINode` + `CSINodeSpec` + `CSINodeDriver` | P2 |
| `CSIStorageCapacity` | P3 |
| `VolumeAttributesClass` | P3 |

---

## 28. core/v1 — ObjectMeta / types.rs ✅ (partial)

**File**: `crates/common/src/types.rs`

### ✅ Completed (commit 8f34586)

| Type | Status |
|------|--------|
| `Condition` (standard metav1.Condition) | ✅ Added |

Status: **largely complete**. No remaining critical gaps.

---

## 29. core/v1 — ResourceRequirements (types.rs)

**File**: `crates/common/src/types.rs`

### ✅ Completed (commit e24e251)

| Field | Status |
|-------|--------|
| `claims: Option<Vec<ResourceClaim>>` | ✅ Added |
| `ResourceClaim` type | ✅ Added |

---

## 30. policy/v1 — PodDisruptionBudgetSpec / Status ✅

**File**: `crates/common/src/resources/policy.rs`

### ✅ Already implemented

| Field | Status |
|-------|--------|
| `unhealthyPodEvictionPolicy: Option<String>` | ✅ Present |
| `conditions: Option<Vec<PodDisruptionBudgetCondition>>` | ✅ Present |
| `PodDisruptionBudgetCondition` type | ✅ Present |

---

## Summary: Implementation Plan

### ✅ Phase 1 — P0 fixes (Complete)

| Item | Status |
|------|--------|
| `DeploymentStatus.conditions` + `DeploymentCondition` | ✅ Done |
| `DaemonSetStatus` missing counters (`numberAvailable`, `numberUnavailable`, `updatedNumberScheduled`) | ✅ Done |
| `PodResourceClaim` flattening (remove `ClaimSource` wrapper, flatten `resourceClaimName`/`resourceClaimTemplateName` directly) | ✅ Done |
| `PersistentVolumeSource` enum → flat struct (all volume types as optional fields on `PersistentVolumeSpec`) | ✅ Done |

### ✅ Phase 2 — P1 fixes (Partially complete)

| Item | Status |
|------|--------|
| `DeploymentSpec`: `paused`, `progressDeadlineSeconds`, `replicas` optional | ✅ Done |
| `StatefulSetSpec`: `volumeClaimTemplates`, `minReadySeconds`, `revisionHistoryLimit`, `persistentVolumeClaimRetentionPolicy`, `replicas` optional | ✅ Done |
| `StatefulSetStatus`: `availableReplicas`, `collisionCount`, `conditions`, `currentRevision`, `updateRevision`, `observedGeneration` | ✅ Done |
| `DaemonSetSpec`: `minReadySeconds`, `revisionHistoryLimit` | ✅ Done |
| `RollingUpdateDaemonSet.maxSurge` | ✅ Done |
| `JobSpec`: `selector`, `suspend`, `ttlSecondsAfterFinished`, `completionMode`, `manualSelector` | ✅ Done |
| `JobStatus`: `startTime`, `completionTime`, `ready`, `terminating` | ✅ Done |
| `CronJobSpec`: `startingDeadlineSeconds`, `timeZone` | ✅ Done |
| `ServiceSpec`: all 8 missing fields | ✅ Done |
| `ServicePort.appProtocol` | ✅ Done |
| `ServiceStatus.conditions` | ✅ Done |
| `LoadBalancerIngress`: `ipMode`, `ports` | ✅ Done |
| `SessionAffinityConfig`, `ClientIPConfig`, `PortStatus` types | ✅ Done |
| `Condition` type (metav1.Condition) | ✅ Done |
| `PodStatus`: `hostIPs`, `podIPs`, `nominatedNodeName`, `qosClass`, `startTime` | ✅ Done |
| `PodCondition.observedGeneration` | ✅ Done |
| `PodSpec`: `hostAliases`, `os`, `schedulingGates` + helper types | ✅ Done |
| `Container`: `resizePolicy` + `ContainerResizePolicy` type | ✅ Done |
| `ContainerStatus`: `allocatedResources`, `started` | ✅ Done |
| `PodSpec.resources` (pod-level resource requests) | ✅ Done (commit e24e251) |
| `ContainerStatus`: `allocatedResourcesStatus`, `resources` | ✅ Done (commit e24e251) |
| `NodeStatus`: `images`, `volumesInUse`, `volumesAttached`, `daemonEndpoints` | ✅ Done (commit e24e251) |
| `NamespaceStatus.conditions` | ✅ Done (commit e24e251) |
| `ResourceRequirements.claims` | ✅ Done (commit e24e251) |
| `ResourceQuota`: full implementation | ✅ Already implemented (policy.rs) |
| `StorageClass`: full implementation | ✅ Already implemented (volume.rs) |
| `Container.lifecycle` + `Lifecycle`/`LifecycleHandler`/`SleepAction` types | ✅ Done (current session) |
| `EphemeralContainer`: `resizePolicy`, `restartPolicy`, `resources` | ✅ Done (current session) |
| `Volume` struct: `nfs`, `iscsi`, `projected`, `image` + projection types | ✅ Done (current session) |
| `NodeSpec.podCIDRs` | ✅ Done (current session) |
| `PersistentVolumeClaimSpec`: `dataSourceRef`, `volumeAttributesClassName` | ✅ Done (current session) |
| HPA: `MetricStatus.containerResource`, timestamp types | ✅ Done (current session) |
| `PodDisruptionBudgetSpec.unhealthyPodEvictionPolicy` | ✅ Already implemented (policy.rs) |
| `PodDisruptionBudgetStatus.conditions` | ✅ Already implemented (policy.rs) |
| `PersistentVolumeStatus.reason/message` | ✅ Already implemented (volume.rs) |

### Phase 3 — P2 fixes (Not started)

| Item | Priority |
|------|----------|
| `PodSecurityContext`: `seLinuxChangePolicy`, `supplementalGroupsPolicy` | P2 |
| `PersistentVolumeClaimStatus`: `currentVolumeAttributesClassName`, `modifyVolumeStatus` | P2 |
| `PersistentVolumeStatus.lastPhaseTransitionTime` | P2 |
| `PersistentVolumeSpec.volumeAttributesClassName` | P2 |
| `NodeSystemInfo.swap` | P2 |
| `Lifecycle.stopSignal` | P2 |
| `HPAScalingRules.tolerance` | P2 |
| `DeploymentStatus.terminatingReplicas` | P2 |
| `StatefulSetSpec.ordinals` | P2 |
| `JobSpec`: `podFailurePolicy`, `successPolicy`, `podReplacementPolicy` | P2 |
| `JobStatus`: `completedIndexes`, `failedIndexes`, `uncountedTerminatedPods` | P2 |
| `Container`: `stdin`, `stdinOnce`, `tty`, `terminationMessagePath`, `terminationMessagePolicy` | P2 |
| `ContainerStatus`: `stopSignal`, `user`, `volumeMounts` | P2 |
| `PodStatus`: `resize`, `resourceClaimStatuses`, `observedGeneration` | P2 |
| `NodeStatus`: `config`, `features`, `runtimeHandlers` | P2 |
| `Volume`: `csi` (inline CSI volume) | P2 |

### Phase 4 — P3 (Nice to have / legacy)

| Item | Priority |
|------|----------|
| Legacy PV volume backends (AzureDisk, CephFS, GCE, etc.) | P3 |
| `NodeConfigSource` / dynamic kubelet config types | P3 |
| `CSIDriver`, `CSINode`, `CSIStorageCapacity` | P3 |
| `VolumeAttributesClass` | P3 |
| Deprecated fields (`serviceAccount` alias, `externalID`, etc.) | P3 |

---

## Notes on Implementation

- **Serde compatibility**: When adding fields, always use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields. Kubernetes clients send partial objects; unrecognized fields must be tolerated (no `deny_unknown_fields`).
- **IntOrString**: Several fields (`maxUnavailable`, `maxSurge`, `targetPort`) are K8s `IntOrString` — currently represented as `Option<String>`. This is acceptable as strings can hold both `"30%"` and `"3"`. Consider a proper `IntOrString` enum if conformance tests check type fidelity.
- **Quantity**: Fields typed as `Quantity` in K8s (CPU, memory) are represented as `String` in rusternetes. This is correct.
- **Time fields**: K8s `Time` maps to `Option<DateTime<Utc>>` with our chrono setup. Several status fields use `Option<String>` instead — these should be corrected to `Option<DateTime<Utc>>`.
- **`replicas` optionality**: ✅ Fixed for `Deployment` and `StatefulSet` — now `Option<i32>` defaulting to 1 in controller logic.
- **Bulk struct updates**: When adding fields to widely-used structs like `PodSpec`, run a Python brace-matching script rather than manual edits — there are 80+ instantiation sites across the workspace. See past session notes.
