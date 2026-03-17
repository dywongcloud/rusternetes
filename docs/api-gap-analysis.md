# Kubernetes 1.35 API Gap Analysis

This document compares rusternetes type definitions against the Kubernetes 1.35 OpenAPI spec
(`swagger.json`) and identifies missing fields, missing types, and structural deviations.

**Source of truth**: `~/Downloads/swagger.json` (Kubernetes master branch, ~1.35 era)
**Scope**: `core/v1`, `apps/v1`, `batch/v1`, `autoscaling/v2`, `networking/v1`, `rbac/v1`,
`storage/v1`, `policy/v1`

**Last updated**: 2026-03-17 (commit 1383ff8)

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

### Still missing

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `resources` | `ResourceRequirements` | P1 | Pod-level resource requests (k8s 1.32+) |
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

## 3. core/v1 — Container

**File**: `crates/common/src/resources/pod.rs`

### ✅ Completed (commit 1383ff8)

| Field | Status |
|-------|--------|
| `resizePolicy: Option<Vec<ContainerResizePolicy>>` | ✅ Added |
| `restartPolicy: Option<String>` | ✅ Added (also used for sidecar detection) |
| `ContainerResizePolicy` type | ✅ Added |

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

### Still missing

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `allocatedResourcesStatus` | `[]ResourceStatus` | P1 | Detailed allocated resource status |
| `resources` | `ResourceRequirements` | P1 | Effective resource requirements |
| `stopSignal` | `string` | P2 | Stop signal sent to the container |
| `user` | `ContainerUser` | P2 | User that the container process runs as |
| `volumeMounts` | `[]VolumeMountStatus` | P2 | Status of volume mounts |

### Missing helper types

| Type | Fields | Priority |
|------|--------|----------|
| `ContainerUser` | `linux: LinuxContainerUser` | P2 |
| `LinuxContainerUser` | `uid: int64`, `gid: int64`, `supplementalGroups: []int64` | P2 |
| `ResourceStatus` | `name: string`, `resources: []ResourceHealth` | P2 |
| `ResourceHealth` | `resourceID: string`, `health: string`, `message: string` | P2 |
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

## 6. core/v1 — Volume

**File**: `crates/common/src/resources/pod.rs`

### Missing volume source types (swagger has them, we lack struct support)

| Type | Priority | Notes |
|------|----------|-------|
| `image` (`ImageVolumeSource`) | P1 | OCI image volumes (k8s 1.33+) |
| `ephemeral` (`EphemeralVolumeSource`) | P1 | Inline PVC volumes |
| `projected` (`ProjectedVolumeSource`) | P1 | Combined volume projections |
| `downwardAPI` (`DownwardAPIVolumeSource`) | P1 | Pod metadata as volume |
| `configMap` (`ConfigMapVolumeSource`) | P1 | ConfigMap as volume |
| `secret` (`SecretVolumeSource`) | P1 | Secret as volume |
| `emptyDir` (`EmptyDirVolumeSource`) | P1 | Temp storage |
| `csi` (`CSIVolumeSource`) | P2 | Inline CSI volume |
| `hostPath` | P2 | Already partially present but missing `type` field in pod volume context |
| `persistentVolumeClaim` (`PersistentVolumeClaimVolumeSource`) | P1 | PVC reference |

**Current state**: `Volume` is an enum with only `HostPath`, `NFS`, `ISCSI`, `Local` variants. It needs to become a struct with optional fields for each volume type (matching Kubernetes's approach), plus the `name` field.

---

## 7. core/v1 — Lifecycle / LifecycleHandler

**File**: `crates/common/src/resources/pod.rs`

### Missing fields on LifecycleHandler

| Field | K8s Type | Priority |
|-------|----------|----------|
| `sleep` | `SleepAction` | P1 |

### Missing fields on Lifecycle

| Field | K8s Type | Priority |
|-------|----------|----------|
| `stopSignal` | `string` | P2 |

### Missing types

| Type | Fields | Priority |
|------|--------|----------|
| `SleepAction` | `seconds: int64` | P1 |

---

## 8. core/v1 — EphemeralContainer

**File**: `crates/common/src/resources/pod.rs`

### Missing fields (same gaps as Container above, plus)

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `resizePolicy` | `[]ContainerResizePolicy` | P1 | |
| `restartPolicy` | `string` | P1 | |
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

## 18. core/v1 — NodeSpec

**File**: `crates/common/src/resources/node.rs`

### Missing fields

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `podCIDRs` | `[]string` | P1 | All pod CIDRs (dual-stack) |
| `configSource` | `NodeConfigSource` | P2 | Dynamic kubelet config |
| `externalID` | `string` | P3 | Deprecated external ID |

---

## 19. core/v1 — NodeStatus

**File**: `crates/common/src/resources/node.rs`

### Missing fields

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `images` | `[]ContainerImage` | P1 | Images present on node |
| `volumesInUse` | `[]string` | P1 | Volumes in use |
| `volumesAttached` | `[]AttachedVolume` | P1 | Attached volumes |
| `daemonEndpoints` | `NodeDaemonEndpoints` | P1 | Kubelet endpoint |
| `config` | `NodeConfigStatus` | P2 | Config source status |
| `features` | `NodeFeatures` | P2 | Feature gate status |
| `runtimeHandlers` | `[]NodeRuntimeHandler` | P2 | Available runtime handlers |
| `declaredFeatures` | object | P2 | Declared node features |
| `phase` | `string` | P3 | Deprecated node phase |

### Missing types

| Type | Fields | Priority |
|------|--------|----------|
| `ContainerImage` | `names: []string`, `sizeBytes: int64` | P1 |
| `AttachedVolume` | `name: string`, `devicePath: string` | P1 |
| `NodeDaemonEndpoints` | `kubeletEndpoint: DaemonEndpoint` | P1 |
| `DaemonEndpoint` | `Port: int32` | P1 |
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

### Missing fields on PersistentVolumeStatus

| Field | K8s Type | Priority |
|-------|----------|----------|
| `reason` | `string` | P1 |
| `message` | `string` | P1 |
| `lastPhaseTransitionTime` | `Time` | P2 |

---

## 21. core/v1 — PersistentVolumeClaimSpec / Status

**File**: `crates/common/src/resources/volume.rs`

### PersistentVolumeClaimSpec — Missing fields

| Field | K8s Type | Priority |
|-------|----------|----------|
| `dataSource` | `TypedLocalObjectReference` | P1 |
| `dataSourceRef` | `TypedObjectReference` | P1 |
| `volumeAttributesClassName` | `string` | P2 |

### PersistentVolumeClaimStatus — Missing fields

| Field | K8s Type | Priority |
|-------|----------|----------|
| `allocatedResources` | `map[string]Quantity` | P1 |
| `allocatedResourceStatuses` | `map[string]string` | P1 |
| `currentVolumeAttributesClassName` | `string` | P2 |
| `modifyVolumeStatus` | `ModifyVolumeStatus` | P2 |

### Missing types

| Type | Fields | Priority |
|------|--------|----------|
| `ModifyVolumeStatus` | `targetVolumeAttributesClassName: string`, `status: string` | P2 |

---

## 22. core/v1 — NamespaceStatus

**File**: `crates/common/src/resources/namespace.rs`

### Missing fields

| Field | K8s Type | Priority |
|-------|----------|----------|
| `conditions` | `[]NamespaceCondition` | P1 |

### Missing types

| Type | Fields | Priority |
|------|--------|----------|
| `NamespaceCondition` | `type`, `status`, `lastTransitionTime`, `reason`, `message` | P1 |

---

## 23. autoscaling/v2 — HPA

**File**: `crates/common/src/resources/autoscaling.rs`

### Gaps vs swagger

| Item | Issue | Priority |
|------|-------|----------|
| `HorizontalPodAutoscalerStatus.conditions` | Present but condition timestamps are `Option<String>` instead of `Option<DateTime<Utc>>` | P1 |
| `HPAScalingRules.tolerance` | Missing `tolerance: Quantity` field (k8s 1.35 new field) | P2 |
| `MetricStatus` | Missing `containerResource` variant | P1 |
| `HorizontalPodAutoscalerStatus` | We are missing `currentReplicas: i32` as always-present field | P1 |

---

## 24. core/v1 — ResourceQuota

**File**: Not implemented — **completely missing**

| Resource | Priority |
|----------|----------|
| `ResourceQuota` struct with `spec` and `status` | P1 |
| `ResourceQuotaSpec`: `hard`, `scopes`, `scopeSelector` | P1 |
| `ResourceQuotaStatus`: `hard`, `used` | P1 |
| `ScopeSelector`, `ScopedResourceSelectorRequirement` | P1 |

---

## 25. core/v1 — LimitRange

**File**: Not implemented — **completely missing**

| Resource | Priority |
|----------|----------|
| `LimitRange` struct | P2 |
| `LimitRangeSpec`, `LimitRangeItem` | P2 |

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

## 27. storage/v1 — StorageClass, VolumeAttachment, CSIDriver, CSINode

**File**: Not implemented — **completely missing**

| Resource | Priority |
|----------|----------|
| `StorageClass` | P1 |
| `VolumeAttachment` + `VolumeAttachmentSpec` + `VolumeAttachmentStatus` | P2 |
| `CSIDriver` + `CSIDriverSpec` | P2 |
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

### Missing fields

| Field | K8s Type | Priority |
|-------|----------|----------|
| `claims` | `[]ResourceClaim` | P1 — DRA support |

### Missing types

| Type | Fields | Priority |
|------|--------|----------|
| `ResourceClaim` | `name: string`, `request: string` | P1 |

---

## 30. policy/v1 — PodDisruptionBudgetSpec / Status

**File**: `crates/common/src/resources/policy.rs`

### PodDisruptionBudgetSpec — Missing fields

| Field | K8s Type | Priority |
|-------|----------|----------|
| `unhealthyPodEvictionPolicy` | `string` | P1 |

### PodDisruptionBudgetStatus — Missing fields

| Field | K8s Type | Priority |
|-------|----------|----------|
| `conditions` | `[]Condition` | P1 |

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
| `PodSpec.resources` (pod-level resource requests) | ❌ Still needed |
| `ContainerStatus`: `allocatedResourcesStatus`, `resources` | ❌ Still needed |
| `NodeStatus`: `images`, `volumesInUse`, `volumesAttached`, `daemonEndpoints` | ❌ Still needed |
| `NamespaceStatus.conditions` | ❌ Still needed |
| `ResourceQuota`: full implementation | ❌ Still needed |
| `StorageClass`: full implementation | ❌ Still needed |
| `ResourceRequirements.claims` | ❌ Still needed |

### Phase 3 — P2 fixes (Not started)

| Item | Priority |
|------|----------|
| `Volume` struct: expand from enum to flat struct with all K8s volume types | P2 |
| `PodSecurityContext`: `seLinuxChangePolicy`, `supplementalGroupsPolicy` | P2 |
| `LimitRange`: full implementation | P2 |
| `PersistentVolumeClaimSpec/Status`: missing fields | P2 |
| `NodeSpec.podCIDRs` | P2 |
| `NodeSystemInfo.swap` | P2 |
| `Lifecycle.sleep` (`SleepAction`), `Lifecycle.stopSignal` | P2 |
| `HPAScalingRules.tolerance` | P2 |
| `DeploymentStatus.terminatingReplicas` | P2 |
| `StatefulSetSpec.ordinals` | P2 |
| `JobSpec`: `podFailurePolicy`, `successPolicy`, `podReplacementPolicy` | P2 |
| `JobStatus`: `completedIndexes`, `failedIndexes`, `uncountedTerminatedPods` | P2 |
| `PodDisruptionBudget`: `unhealthyPodEvictionPolicy`, `conditions` | P2 |

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
