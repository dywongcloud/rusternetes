# Kubernetes 1.35 API Gap Analysis

This document compares rusternetes type definitions against the Kubernetes 1.35 OpenAPI spec
(`swagger.json`) and identifies missing fields, missing types, and structural deviations.

**Source of truth**: `~/Downloads/swagger.json` (Kubernetes master branch, ~1.35 era)
**Scope**: `core/v1`, `apps/v1`, `batch/v1`, `autoscaling/v2`, `networking/v1`, `rbac/v1`,
`storage/v1`, `policy/v1`

---

## Priority Legend

- **P0** – Required for conformance / likely causes test failures today
- **P1** – Required for correct kubectl / client behavior
- **P2** – Important but less commonly exercised
- **P3** – Completeness / nice to have

---

## 1. core/v1 — PodSpec

**File**: `crates/common/src/resources/pod.rs`

### Missing fields (swagger has them, we do not)

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `hostAliases` | `[]HostAlias` | P1 | /etc/hosts entries injected into pods |
| `os` | `PodOS` | P1 | Target OS (linux/windows) |
| `resources` | `ResourceRequirements` | P1 | Pod-level resource requests (k8s 1.32+) |
| `schedulingGates` | `[]PodSchedulingGate` | P1 | Gate-based scheduling |
| `schedulingGroup` | `PodSchedulingGroup` | P2 | Group-based scheduling |
| `hostnameOverride` | `string` | P2 | Override hostname independently of setHostnameAsFQDN |
| `serviceAccount` | `string` | P2 | Deprecated alias for serviceAccountName (still present in spec) |

### Missing helper types (needed by PodSpec fields above)

| Type | Fields | Priority |
|------|--------|----------|
| `HostAlias` | `ip: string`, `hostnames: []string` | P1 |
| `PodOS` | `name: string` | P1 |
| `PodSchedulingGate` | `name: string` | P1 |
| `PodSchedulingGroup` | `podGroupName: string` | P2 |

### Structural issues

| Item | Issue | Priority |
|------|-------|----------|
| `PodResourceClaim` | We have `source: ClaimSource` with `resourceClaimName`/`resourceClaimTemplateName` inside it; K8s 1.35 flattened these to `resourceClaimName` and `resourceClaimTemplateName` directly on `PodResourceClaim` (no `ClaimSource` wrapper) | P0 |

---

## 2. core/v1 — PodStatus

**File**: `crates/common/src/resources/pod.rs`

### Missing fields

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `hostIPs` | `[]HostIP` | P1 | All host IPs (dual-stack) |
| `podIPs` | `[]PodIP` | P1 | All pod IPs (dual-stack); we only have `pod_ip: String` |
| `nominatedNodeName` | `string` | P1 | Node nominated for preemption |
| `qosClass` | `string` | P1 | Guaranteed/Burstable/BestEffort |
| `startTime` | `Time` | P1 | When pod was acknowledged by kubelet |
| `message` | `string` | P1 | Human-readable message about why pod is in this state |
| `reason` | `string` | P1 | Brief CamelCase reason pod is in this state |
| `resize` | `string` | P2 | InProgress/Deferred/Infeasible for resource resizing |
| `resourceClaimStatuses` | `[]PodResourceClaimStatus` | P2 | Status of each resource claim |
| `allocatedResources` | `map[string]Quantity` | P2 | Allocated resources for the pod |
| `extendedResourceClaimStatus` | `[]PodExtendedResourceClaimStatus` | P3 | Extended resource claim tracking |
| `observedGeneration` | `int64` | P2 | Generation the kubelet observed when last syncing |

### Structural issues

| Item | Issue | Priority |
|------|-------|----------|
| `pod_ip` | We have `pod_ip: Option<String>`; K8s uses `podIP: string` (singular) + `podIPs: []PodIP` (plural for dual-stack). We should keep `pod_ip` but also add `pod_ips`. | P1 |
| `conditions` | Field exists but `PodCondition` is missing `observedGeneration: int64` | P1 |

### Missing helper types

| Type | Fields | Priority |
|------|--------|----------|
| `HostIP` | `ip: string` | P1 |
| `PodIP` | `ip: string` | P1 |
| `PodResourceClaimStatus` | `name: string`, `resourceClaimName: string` | P2 |

---

## 3. core/v1 — Container

**File**: `crates/common/src/resources/pod.rs`

### Missing fields

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `resizePolicy` | `[]ContainerResizePolicy` | P1 | Per-resource resize policy |
| `restartPolicy` | `string` | P1 | Container-level restart policy (sidecar support) |
| `restartPolicyRules` | `[]ContainerRestartRule` | P2 | Fine-grained restart rules |
| `stdin` | `bool` | P2 | Keep stdin open |
| `stdinOnce` | `bool` | P2 | Close stdin after first attach |
| `tty` | `bool` | P2 | Allocate TTY |
| `terminationMessagePath` | `string` | P2 | Path for termination message file |
| `terminationMessagePolicy` | `string` | P2 | File or FallbackToLogsOnError |

### Missing helper types

| Type | Fields | Priority |
|------|--------|----------|
| `ContainerResizePolicy` | `resourceName: string`, `restartPolicy: string` | P1 |
| `ContainerRestartRule` | `action: string`, `exitCodes: ContainerRestartRuleOnExitCodes` | P2 |
| `ContainerRestartRuleOnExitCodes` | `operator: string`, `values: []int32` | P2 |

---

## 4. core/v1 — ContainerStatus

**File**: `crates/common/src/resources/pod.rs`

### Missing fields

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `allocatedResources` | `map[string]Quantity` | P1 | Resources allocated to this container |
| `allocatedResourcesStatus` | `[]ResourceStatus` | P1 | Detailed allocated resource status |
| `resources` | `ResourceRequirements` | P1 | Effective resource requirements |
| `started` | `bool` | P1 | Whether the container has passed its startup probe |
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

## 9. apps/v1 — DeploymentSpec

**File**: `crates/common/src/resources/deployment.rs`

### Missing fields

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `paused` | `bool` | P1 | Pause rollout |
| `progressDeadlineSeconds` | `int32` | P1 | Deadline for progress |

### Structural issues

| Item | Issue | Priority |
|------|-------|----------|
| `replicas` | We have `pub replicas: i32` (required); K8s has it optional (`*int32`) defaulting to 1 | P1 |

---

## 10. apps/v1 — DeploymentStatus

**File**: `crates/common/src/resources/deployment.rs`

### Missing fields

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `conditions` | `[]DeploymentCondition` | P0 | Controllers/clients rely on this |
| `collisionCount` | `int32` | P1 | Pod template hash collision counter |
| `observedGeneration` | `int64` | P1 | Generation observed by controller |
| `terminatingReplicas` | `int32` | P2 | k8s 1.35 new field |

### Missing types

| Type | Fields | Priority |
|------|--------|----------|
| `DeploymentCondition` | `type`, `status`, `reason`, `message`, `lastUpdateTime`, `lastTransitionTime` | P0 |

---

## 11. apps/v1 — StatefulSetSpec

**File**: `crates/common/src/resources/workloads.rs`

### Missing fields

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `minReadySeconds` | `int32` | P1 | Min seconds for pod to be ready |
| `revisionHistoryLimit` | `int32` | P1 | History limit |
| `volumeClaimTemplates` | `[]PersistentVolumeClaim` | P0 | Core StatefulSet feature |
| `persistentVolumeClaimRetentionPolicy` | `StatefulSetPVCRetentionPolicy` | P1 | Delete or Retain PVCs |
| `ordinals` | `StatefulSetOrdinals` | P2 | Custom ordinal numbering |

### Structural issues

| Item | Issue | Priority |
|------|-------|----------|
| `replicas` | Required in our struct; should be `Option<i32>` defaulting to 1 | P1 |

### Missing types

| Type | Fields | Priority |
|------|--------|----------|
| `StatefulSetPersistentVolumeClaimRetentionPolicy` | `whenDeleted: string`, `whenScaled: string` | P1 |
| `StatefulSetOrdinals` | `start: int32` | P2 |

---

## 12. apps/v1 — StatefulSetStatus

**File**: `crates/common/src/resources/workloads.rs`

### Missing fields

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `availableReplicas` | `int32` | P0 | kubectl shows this |
| `collisionCount` | `int32` | P1 | |
| `conditions` | `[]StatefulSetCondition` | P1 | |
| `currentRevision` | `string` | P1 | Current update revision |
| `observedGeneration` | `int64` | P1 | |
| `updateRevision` | `string` | P1 | Update revision hash |

### Missing types

| Type | Fields | Priority |
|------|--------|----------|
| `StatefulSetCondition` | `type`, `status`, `reason`, `message`, `lastTransitionTime` | P1 |

---

## 13. apps/v1 — DaemonSetSpec / DaemonSetStatus

**File**: `crates/common/src/resources/workloads.rs`

### DaemonSetSpec — Missing fields

| Field | K8s Type | Priority |
|-------|----------|----------|
| `minReadySeconds` | `int32` | P1 |
| `revisionHistoryLimit` | `int32` | P1 |

### DaemonSetStatus — Missing fields

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `numberAvailable` | `int32` | P0 | |
| `numberUnavailable` | `int32` | P0 | |
| `updatedNumberScheduled` | `int32` | P1 | |
| `observedGeneration` | `int64` | P1 | |
| `collisionCount` | `int32` | P1 | |
| `conditions` | `[]DaemonSetCondition` | P1 | |

### Missing types

| Type | Fields | Priority |
|------|--------|----------|
| `DaemonSetCondition` | `type`, `status`, `reason`, `message`, `lastTransitionTime` | P1 |
| `RollingUpdateDaemonSet.maxSurge` | `maxSurge: IntOrString` | P1 — we only have `maxUnavailable` |

---

## 14. batch/v1 — JobSpec

**File**: `crates/common/src/resources/workloads.rs`

### Missing fields

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `selector` | `LabelSelector` | P1 | Label selector for pods |
| `manualSelector` | `bool` | P1 | Allow custom selector |
| `suspend` | `bool` | P1 | Suspend job execution |
| `ttlSecondsAfterFinished` | `int32` | P1 | Auto-cleanup after completion |
| `completionMode` | `string` | P1 | NonIndexed or Indexed |
| `backoffLimitPerIndex` | `int32` | P2 | Per-index backoff |
| `maxFailedIndexes` | `int32` | P2 | Max failed indexes |
| `podFailurePolicy` | `PodFailurePolicy` | P2 | Failure handling policy |
| `podReplacementPolicy` | `string` | P2 | TerminatingOrFailed/Failed |
| `successPolicy` | `SuccessPolicy` | P2 | Success criteria for indexed jobs |
| `managedBy` | `string` | P2 | Controller that manages this job |

### Missing types

| Type | Fields | Priority |
|------|--------|----------|
| `PodFailurePolicy` | `rules: []PodFailurePolicyRule` | P2 |
| `PodFailurePolicyRule` | `action`, `onExitCodes`, `onPodConditions` | P2 |
| `PodFailurePolicyOnExitCodesRequirement` | `containerName`, `operator`, `values` | P2 |
| `PodFailurePolicyOnPodConditionsPattern` | `type`, `status` | P2 |
| `SuccessPolicy` | `rules: []SuccessPolicyRule` | P2 |
| `SuccessPolicyRule` | `succeededIndexes`, `succeededCount` | P2 |

---

## 15. batch/v1 — JobStatus

**File**: `crates/common/src/resources/workloads.rs`

### Missing fields

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `startTime` | `Time` | P1 | When job started |
| `completionTime` | `Time` | P1 | When job completed |
| `ready` | `int32` | P1 | Number of ready pods |
| `terminating` | `int32` | P1 | Number of terminating pods |
| `completedIndexes` | `string` | P2 | Completed indexes (indexed jobs) |
| `failedIndexes` | `string` | P2 | Failed indexes |
| `uncountedTerminatedPods` | `UncountedTerminatedPods` | P2 | Pods not yet counted in status |

### Missing types

| Type | Fields | Priority |
|------|--------|----------|
| `UncountedTerminatedPods` | `succeeded: []string`, `failed: []string` | P2 |

---

## 16. batch/v1 — CronJobSpec

**File**: `crates/common/src/resources/workloads.rs`

### Missing fields

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `startingDeadlineSeconds` | `int64` | P1 | Deadline to start if missed |
| `timeZone` | `string` | P1 | IANA timezone for schedule |

---

## 17. core/v1 — ServiceSpec

**File**: `crates/common/src/resources/service.rs`

### Missing fields

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `healthCheckNodePort` | `int32` | P1 | NodePort for health check |
| `loadBalancerClass` | `string` | P1 | Class of load balancer |
| `loadBalancerIP` | `string` | P1 | Desired LB IP (deprecated but present) |
| `loadBalancerSourceRanges` | `[]string` | P1 | Allowed source CIDRs for LB |
| `allocateLoadBalancerNodePorts` | `bool` | P1 | Whether to allocate node ports for LB |
| `publishNotReadyAddresses` | `bool` | P1 | Route to not-ready pods |
| `sessionAffinityConfig` | `SessionAffinityConfig` | P1 | Affinity timeout config |
| `trafficDistribution` | `string` | P1 | PreferClose/etc. routing hint |

### Missing types

| Type | Fields | Priority |
|------|--------|----------|
| `SessionAffinityConfig` | `clientIP: ClientIPConfig` | P1 |
| `ClientIPConfig` | `timeoutSeconds: int32` | P1 |

### Structural issues

| Item | Issue | Priority |
|------|-------|----------|
| `ServiceStatus.conditions` | Missing `conditions: []Condition` from ServiceStatus | P1 |
| `LoadBalancerIngress` | Missing `ipMode: string` and `ports: []PortStatus` fields | P1 |
| `ServicePort.appProtocol` | Missing `appProtocol: string` field | P1 |

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

### Structural issues

| Item | Issue | Priority |
|------|-------|----------|
| `PersistentVolumeSource` | We model it as an enum; Kubernetes spec uses a flat struct with all volume types as optional fields. This causes serialization incompatibility. | P0 |

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

## 28. core/v1 — ObjectMeta (types.rs)

**File**: `crates/common/src/types.rs`

Status: **largely complete** after recent additions. No critical gaps identified.

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

**File**: `crates/common/src/resources/policy.rs` (presumably exists)

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

### Phase 1 — P0 fixes (Conformance blockers)

These are likely causing current conformance failures or will as soon as more tests run.

1. **`PodResourceClaim` flattening** — remove `ClaimSource` wrapper, flatten `resourceClaimName`/`resourceClaimTemplateName` directly onto `PodResourceClaim`
2. **`PersistentVolumeSource` enum → flat struct** — restructure to match K8s flat optional-field pattern
3. **`DeploymentStatus.conditions`** — add `conditions: Option<Vec<DeploymentCondition>>`
4. **`DaemonSetStatus` missing counters** — add `numberAvailable`, `numberUnavailable`, `updatedNumberScheduled`

### Phase 2 — P1 fixes (kubectl / client correctness)

These will cause broken responses that kubectl or the test framework may detect.

5. **`PodStatus`**: add `podIPs`, `hostIPs`, `nominatedNodeName`, `qosClass`, `startTime`, `message`, `reason`
6. **`PodSpec`**: add `hostAliases`, `os`, `resources`, `schedulingGates`
7. **`Container`**: add `resizePolicy`, `restartPolicy`, `stdin`, `stdinOnce`, `tty`, `terminationMessagePath`, `terminationMessagePolicy`
8. **`ContainerStatus`**: add `allocatedResources`, `resources`, `started`
9. **`DeploymentSpec`**: add `paused`, `progressDeadlineSeconds`; make `replicas` optional
10. **`StatefulSetSpec`**: add `volumeClaimTemplates`, `minReadySeconds`, `revisionHistoryLimit`, `persistentVolumeClaimRetentionPolicy`
11. **`StatefulSetStatus`**: add `availableReplicas`, `collisionCount`, `conditions`, `currentRevision`, `updateRevision`, `observedGeneration`
12. **`JobSpec`**: add `selector`, `suspend`, `ttlSecondsAfterFinished`, `completionMode`
13. **`JobStatus`**: add `startTime`, `completionTime`, `ready`
14. **`CronJobSpec`**: add `startingDeadlineSeconds`, `timeZone`
15. **`ServiceSpec`**: add `healthCheckNodePort`, `loadBalancerClass`, `loadBalancerIP`, `loadBalancerSourceRanges`, `allocateLoadBalancerNodePorts`, `publishNotReadyAddresses`, `sessionAffinityConfig`, `trafficDistribution`
16. **`ServiceStatus.conditions`**: add conditions field
17. **`NodeStatus`**: add `images`, `volumesInUse`, `volumesAttached`, `daemonEndpoints`
18. **`NamespaceStatus.conditions`**: add `NamespaceCondition`
19. **`ResourceQuota`**: implement full resource (completely missing)
20. **`StorageClass`**: implement (completely missing)
21. **`ResourceRequirements.claims`**: add DRA claims field

### Phase 3 — P2 fixes (Completeness)

22. **`DaemonSetSpec`**: add `minReadySeconds`, `revisionHistoryLimit`
23. **`Volume`** struct: expand from 4-variant enum to full flat struct with all K8s volume types
24. **`PodSecurityContext`**: add `seLinuxChangePolicy`, `supplementalGroupsPolicy`
25. **`LimitRange`**: implement (completely missing)
26. **`PersistentVolumeClaimSpec/Status`**: add missing fields
27. **`NodeSpec`**: add `podCIDRs`
28. **`NodeSystemInfo.swap`**: add `NodeSwapStatus`
29. **`Lifecycle`**: add `sleep: SleepAction`, `stopSignal`
30. **`HPAScalingRules.tolerance`**: add field

### Phase 4 — P3 (nice to have / legacy)

31. Legacy volume backends in PV context (AzureDisk, CephFS, etc.)
32. `NodeConfigSource` / dynamic kubelet config types
33. `CSIDriver`, `CSINode`, `CSIStorageCapacity`
34. `VolumeAttributesClass`
35. Deprecated fields (`serviceAccount` alias, `externalID`, etc.)

---

## Notes on Implementation

- **Serde compatibility**: When adding fields, always use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields. Kubernetes clients send partial objects; unrecognized fields must be tolerated (no `deny_unknown_fields`).
- **IntOrString**: Several fields (`maxUnavailable`, `maxSurge`, `targetPort`) are K8s `IntOrString` — currently represented as `Option<String>`. This is acceptable as strings can hold both `"30%"` and `"3"`. Consider a proper `IntOrString` enum if conformance tests check type fidelity.
- **Quantity**: Fields typed as `Quantity` in K8s (CPU, memory) are represented as `String` in rusternetes. This is correct.
- **Time fields**: K8s `Time` maps to `Option<DateTime<Utc>>` with our chrono setup. Several status fields use `Option<String>` instead — these should be corrected to `Option<DateTime<Utc>>`.
- **`replicas` optionality**: K8s spec has `replicas` as `*int32` (pointer = optional, defaults to 1). Our Deployment and StatefulSet have it as required `i32`. This may cause deserialization failures when clients omit it.
