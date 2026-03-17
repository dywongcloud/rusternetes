# Kubernetes 1.35 API Gap Analysis

This document compares rusternetes type definitions against the Kubernetes 1.35 OpenAPI spec
(`swagger.json`) and identifies missing fields, missing types, and structural deviations.

**Source of truth**: `~/Downloads/swagger.json` (Kubernetes master branch, ~1.35 era)
**Scope**: `core/v1`, `apps/v1`, `batch/v1`, `autoscaling/v2`, `networking/v1`, `rbac/v1`,
`storage/v1`, `policy/v1`, `coordination/v1`, `discovery/v1`, `events.k8s.io/v1`,
`meta/v1` (ObjectMeta, Status, etc.)

**Last updated**: 2026-03-17 (comprehensive swagger.json audit)

---

## Priority Legend

- **P0** – Required for conformance / likely causes test failures today
- **P1** – Required for correct kubectl / client behavior
- **P2** – Important but less commonly exercised
- **P3** – Completeness / nice to have
- ✅ **Done** – Implemented

---

## ================================
## NEWLY DISCOVERED GAPS (This Audit)
## ================================

### NEW-1. meta/v1 — Status response object (P0 - CRITICAL)

**File**: `crates/common/src/error.rs` (ad-hoc JSON), `crates/common/src/types.rs` (missing struct)

The Kubernetes `metav1.Status` type is NOT implemented as a proper struct. Currently, error responses
are built ad-hoc in `error.rs` with only `kind`, `apiVersion`, `status`, `message`, and `code` fields.
The Kubernetes spec requires additional fields that kubectl and other clients depend on.

| Item | K8s Type | Priority | Notes |
|------|----------|----------|-------|
| `Status` struct | `metav1.Status` | **P0** | Missing: `reason`, `details`, `metadata` (ListMeta) |
| `StatusDetails` struct | `metav1.StatusDetails` | **P0** | Missing entirely — needed for validation errors |
| `StatusCause` struct | `metav1.StatusCause` | **P0** | Missing entirely — needed for field-level errors |
| `Status.reason` field | `string` | **P0** | e.g. "NotFound", "AlreadyExists", "Conflict" — kubectl uses this |
| `Status.details` field | `StatusDetails` | **P1** | Resource name/kind in error responses |

**Why P0**: kubectl parses `reason` from Status responses to decide error handling behavior (e.g.,
retry on Conflict, display validation errors from `details.causes`). Conformance tests check
for correct Status responses. The current ad-hoc JSON is missing critical fields.

### NEW-2. core/v1 — Event fields (P1)

**File**: `crates/common/src/resources/event.rs`

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `eventTime` | `MicroTime` | P1 | Used by events.v1 API, kubectl |
| `reportingComponent` | `string` | P1 | Component that reported the event |
| `reportingInstance` | `string` | P1 | Instance of the reporting component |

### NEW-3. events.k8s.io/v1 — Event (P1)

**File**: Missing entirely

The `events.k8s.io/v1.Event` type is completely missing. This is the modern events API that
kubectl and many controllers use.

| Field | K8s Type | Priority |
|-------|----------|----------|
| `action` | `string` (required) | P1 |
| `eventTime` | `MicroTime` (required) | P1 |
| `regarding` | `ObjectReference` | P1 |
| `related` | `ObjectReference` | P1 |
| `reportingController` | `string` (required) | P1 |
| `reportingInstance` | `string` (required) | P1 |
| `reason` | `string` | P1 |
| `note` | `string` | P1 |
| `type` | `string` | P1 |
| `series` | `EventSeries` | P2 |
| `deprecatedCount` | `int32` | P2 |
| `deprecatedFirstTimestamp` | `Time` | P2 |
| `deprecatedLastTimestamp` | `Time` | P2 |
| `deprecatedSource` | `EventSource` | P2 |

### NEW-4. coordination/v1 — LeaseSpec missing fields (P1)

**File**: `crates/common/src/resources/coordination.rs`

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `preferredHolder` | `string` | P1 | Preferred lease holder (leader election) |
| `strategy` | `string` | P1 | Lease strategy (OldestEmulationVersion, etc.) |

### NEW-5. discovery/v1 — EndpointHints missing field (P2)

**File**: `crates/common/src/resources/endpointslice.rs`

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `forNodes` | `[]ForNode` | P2 | Node-level topology hints |
| `ForNode` type | `name: string` | P2 | New type needed |

### NEW-6. policy/v1 — PodDisruptionBudgetStatus missing field (P1)

**File**: `crates/common/src/resources/policy.rs`

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `disruptedPods` | `map[string]Time` | P1 | Tracks pods whose eviction was processed |

### NEW-7. storage/v1 — StorageClass missing field (P1)

**File**: `crates/common/src/resources/volume.rs`

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `mountOptions` | `[]string` | P1 | Mount options for PVs created by this class |

### NEW-8. storage/v1 — CSIDriverSpec missing fields (P2)

**File**: `crates/common/src/resources/csi.rs`

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `nodeAllocatableUpdatePeriodSeconds` | `int64` | P2 | Periodic update interval |

### NEW-9. apps/v1 — ReplicaSetStatus missing field (P2)

**File**: `crates/common/src/resources/workloads.rs`

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `terminatingReplicas` | `int32` | P2 | Number of terminating pods |

### NEW-10. ServicePort.targetPort type mismatch (P1)

**File**: `crates/common/src/resources/service.rs`

| Issue | Current | Expected | Priority |
|-------|---------|----------|----------|
| `targetPort` uses `u16` only | `Option<u16>` | `IntOrString` (int or named port) | P1 |

kubectl and clients send named ports (e.g., `"http"`) as targetPort. Our `u16` type will
reject these. This should be `Option<String>` or a proper `IntOrString` enum.

---

## ================================
## PREVIOUSLY TRACKED GAPS
## ================================

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
| `serviceAccount` | `string` | P2 | Deprecated alias for serviceAccountName (still present in spec) |
| `schedulingGroup` | `PodSchedulingGroup` | P3 | Group-based scheduling (alpha) |
| `hostnameOverride` | `string` | P3 | Override hostname (alpha) |

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
| `observedGeneration` | `int64` | P2 | Generation the kubelet observed when last syncing |
| `allocatedResources` | `map[string]Quantity` | P3 | Allocated resources for the pod |
| `extendedResourceClaimStatus` | `[]PodExtendedResourceClaimStatus` | P3 | Extended resource claim tracking |

---

## 3. core/v1 — Container ✅ (partial)

**File**: `crates/common/src/resources/pod.rs`

### ✅ Completed (commit 1383ff8)

| Field | Status |
|-------|--------|
| `resizePolicy: Option<Vec<ContainerResizePolicy>>` | ✅ Added |
| `restartPolicy: Option<String>` | ✅ Added (also used for sidecar detection) |
| `ContainerResizePolicy` type | ✅ Added |

### ✅ Completed (swagger audit)

| Field | Status |
|-------|--------|
| `lifecycle: Option<Lifecycle>` | ✅ Added |
| `envFrom: Option<Vec<EnvFromSource>>` | ✅ Added |
| `volumeDevices: Option<Vec<VolumeDevice>>` | ✅ Added |
| `terminationMessagePath: Option<String>` | ✅ Added |
| `terminationMessagePolicy: Option<String>` | ✅ Added |
| `stdin: Option<bool>` | ✅ Added |
| `stdinOnce: Option<bool>` | ✅ Added |
| `tty: Option<bool>` | ✅ Added |
| `EnvFromSource` type | ✅ Added |
| `ConfigMapEnvSource` type | ✅ Added |
| `SecretEnvSource` type | ✅ Added |
| `VolumeDevice` type | ✅ Added |

### Still missing

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `restartPolicyRules` | `[]ContainerRestartRule` | P3 | Fine-grained restart rules (k8s 1.35 alpha) |

---

## 4. core/v1 — ContainerStatus

**File**: `crates/common/src/resources/pod.rs`

### ✅ Completed (commit 1383ff8 + e24e251 + swagger audit)

| Field | Status |
|-------|--------|
| `allocatedResources: Option<HashMap<String, String>>` | ✅ Added |
| `started: Option<bool>` | ✅ Added |
| `allocatedResourcesStatus: Option<Vec<ResourceStatus>>` | ✅ Added |
| `resources: Option<ResourceRequirements>` | ✅ Added |
| `lastState: Option<ContainerState>` | ✅ Added |
| `imageID: Option<String>` | ✅ Added |
| `ContainerState::Terminated`: `signal`, `message`, `startedAt`, `finishedAt`, `containerID` | ✅ Added |
| `ContainerState::Waiting`: `message` | ✅ Added |

### Still missing

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `user` | `ContainerUser` | P2 | User that the container process runs as |
| `volumeMounts` | `[]VolumeMountStatus` | P2 | Status of volume mounts |
| `stopSignal` | `string` | P2 | Stop signal sent to the container |

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

### ✅ Completed

| Field | Status |
|-------|--------|
| `nfs: Option<NFSVolumeSource>` | ✅ Added |
| `iscsi: Option<ISCSIVolumeSource>` | ✅ Added |
| `projected: Option<ProjectedVolumeSource>` | ✅ Added |
| `image: Option<ImageVolumeSource>` | ✅ Added |
| All projection types | ✅ Added |

### Still missing

| Type | Priority | Notes |
|------|----------|-------|
| `csi` (`CSIVolumeSource`) | P2 | Inline CSI volume |

---

## 7. core/v1 — Lifecycle / LifecycleHandler ✅ (partial)

**File**: `crates/common/src/resources/pod.rs`

### ✅ Completed

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

### ✅ Completed

| Field | Status |
|-------|--------|
| `resizePolicy: Option<Vec<ContainerResizePolicy>>` | ✅ Added |
| `restartPolicy: Option<String>` | ✅ Added |
| `resources: Option<ResourceRequirements>` | ✅ Added |

### Still missing

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
| `stdin`, `stdinOnce`, `tty` | `bool` | P2 | |
| `terminationMessagePath`, `terminationMessagePolicy` | `string` | P2 | |
| `restartPolicyRules` | `[]ContainerRestartRule` | P3 | Alpha |

---

## 9-13. apps/v1 — Deployments, StatefulSets, DaemonSets ✅

All previously completed items remain done. See Phase 2 completion table below.

### Still missing

| Field | K8s Type | Priority |
|-------|----------|----------|
| `DeploymentStatus.terminatingReplicas` | `int32` | P2 |
| `ReplicaSetStatus.terminatingReplicas` | `int32` | P2 |
| `StatefulSetSpec.ordinals` | `StatefulSetOrdinals` | P2 |

---

## 14-16. batch/v1 — Jobs, CronJobs ✅ (partial)

### Still missing

| Field | K8s Type | Priority |
|-------|----------|----------|
| `JobSpec.backoffLimitPerIndex` | `int32` | P2 |
| `JobSpec.maxFailedIndexes` | `int32` | P2 |
| `JobSpec.podFailurePolicy` | `PodFailurePolicy` | P2 |
| `JobSpec.podReplacementPolicy` | `string` | P2 |
| `JobSpec.successPolicy` | `SuccessPolicy` | P2 |
| `JobSpec.managedBy` | `string` | P2 |
| `JobStatus.completedIndexes` | `string` | P2 |
| `JobStatus.failedIndexes` | `string` | P2 |
| `JobStatus.uncountedTerminatedPods` | `UncountedTerminatedPods` | P2 |

---

## 17-30. Other completed sections

See Phase 2 completion table below. All previously tracked items remain done.

---

## ================================
## IMPLEMENTATION PLAN
## ================================

### Phase 3A — P0 fixes (CRITICAL — do first)

These are blocking conformance tests and correct kubectl behavior.

| # | Item | File | Est. Complexity |
|---|------|------|-----------------|
| 1 | **`Status` struct** with `reason`, `details`, `metadata` fields | `types.rs` + `error.rs` | Medium |
| 2 | **`StatusDetails` struct** with `name`, `group`, `kind`, `uid`, `causes`, `retryAfterSeconds` | `types.rs` | Small |
| 3 | **`StatusCause` struct** with `field`, `message`, `reason` | `types.rs` | Small |
| 4 | **Refactor error.rs** to use proper `Status` struct instead of ad-hoc JSON | `error.rs` | Medium |
| 5 | **Map Error variants to Status reasons**: NotFound→"NotFound", AlreadyExists→"AlreadyExists", Conflict→"Conflict", InvalidResource→"Invalid" | `error.rs` | Small |

### Phase 3B — P1 fixes (Important for kubectl/client compatibility)

| # | Item | File | Est. Complexity |
|---|------|------|-----------------|
| 6 | **`ServicePort.targetPort`**: Change from `Option<u16>` to `Option<String>` (IntOrString) | `service.rs` + all instantiation sites | Medium |
| 7 | **`StorageClass.mountOptions`**: Add `Option<Vec<String>>` | `volume.rs` | Small |
| 8 | **`LeaseSpec`**: Add `preferredHolder` and `strategy` fields | `coordination.rs` | Small |
| 9 | **`PodDisruptionBudgetStatus.disruptedPods`**: Add `Option<HashMap<String, DateTime<Utc>>>` | `policy.rs` | Small |
| 10 | **core/v1 Event**: Add `eventTime`, `reportingComponent`, `reportingInstance` | `event.rs` | Small |
| 11 | **events.k8s.io/v1 Event**: New type + handler + routes | `event.rs` + `handlers/event.rs` | Large |

### Phase 3C — P2 fixes (Feature completeness)

| # | Item | File |
|---|------|------|
| 12 | `PodSecurityContext`: `seLinuxChangePolicy`, `supplementalGroupsPolicy` | `pod.rs` |
| 13 | `PVCStatus`: `currentVolumeAttributesClassName`, `modifyVolumeStatus` + `ModifyVolumeStatus` type | `volume.rs` |
| 14 | `PVStatus.lastPhaseTransitionTime` | `volume.rs` |
| 15 | `PVSpec.volumeAttributesClassName` | `volume.rs` |
| 16 | `NodeSystemInfo.swap` + `NodeSwapStatus` type | `node.rs` |
| 17 | `NodeStatus`: `config`, `features`, `runtimeHandlers`, `declaredFeatures` + 4 new types | `node.rs` |
| 18 | `Lifecycle.stopSignal` | `pod.rs` |
| 19 | `HPAScalingRules.tolerance` | `autoscaling.rs` |
| 20 | `DeploymentStatus.terminatingReplicas` | `deployment.rs` |
| 21 | `ReplicaSetStatus.terminatingReplicas` | `workloads.rs` |
| 22 | `StatefulSetSpec.ordinals` + `StatefulSetOrdinals` type | `workloads.rs` |
| 23 | `JobSpec`: `backoffLimitPerIndex`, `maxFailedIndexes`, `podFailurePolicy`, `podReplacementPolicy`, `successPolicy`, `managedBy` + helper types | `workloads.rs` |
| 24 | `JobStatus`: `completedIndexes`, `failedIndexes`, `uncountedTerminatedPods` + `UncountedTerminatedPods` type | `workloads.rs` |
| 25 | `ContainerStatus`: `user` + `ContainerUser`/`LinuxContainerUser`, `volumeMounts` + `VolumeMountStatus`, `stopSignal` | `pod.rs` |
| 26 | `PodStatus`: `resize`, `resourceClaimStatuses`, `observedGeneration` + `PodResourceClaimStatus` type | `pod.rs` |
| 27 | `EphemeralContainer`: `stdin`, `stdinOnce`, `tty`, `terminationMessagePath`, `terminationMessagePolicy` | `pod.rs` |
| 28 | `Volume.csi` (inline `CSIVolumeSource`) | `pod.rs` |
| 29 | `EndpointHints.forNodes` + `ForNode` type | `endpointslice.rs` |
| 30 | `CSIDriverSpec.nodeAllocatableUpdatePeriodSeconds` | `csi.rs` |
| 31 | `PodSpec.serviceAccount` (deprecated alias) | `pod.rs` |
| 32 | `EnvVarSource.fileKeyRef` + `FileKeySelector` type | `pod.rs` |

### Phase 4 — P3 (Nice to have / legacy / alpha)

| Item | Priority |
|------|----------|
| Legacy PV volume backends (AzureDisk, CephFS, GCE, etc.) — all deprecated | P3 |
| `PodSchedulingGroup` + `schedulingGroup` (alpha) | P3 |
| `ContainerRestartRule` + `restartPolicyRules` (alpha) | P3 |
| `PodExtendedResourceClaimStatus` | P3 |
| `PodSpec.hostnameOverride` (alpha) | P3 |
| `VolumeAttributesClass` | P3 |
| `NodeSpec.externalID` (deprecated) | P3 |
| `NodeStatus.phase` (deprecated) | P3 |
| `VolumeProjection.podCertificate` + `PodCertificateProjection` (alpha) | P3 |

---

## ================================
## COMPLETED ITEMS (All Phases)
## ================================

### ✅ Phase 1 — P0 fixes (Complete)

| Item | Status |
|------|--------|
| `DeploymentStatus.conditions` + `DeploymentCondition` | ✅ Done |
| `DaemonSetStatus` missing counters | ✅ Done |
| `PodResourceClaim` flattening | ✅ Done |
| `PersistentVolumeSource` enum → flat struct | ✅ Done |

### ✅ Phase 2 — P1 fixes (Complete)

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
| `PodSpec.resources` (pod-level resource requests) | ✅ Done |
| `ContainerStatus`: `allocatedResourcesStatus`, `resources` | ✅ Done |
| `NodeStatus`: `images`, `volumesInUse`, `volumesAttached`, `daemonEndpoints` | ✅ Done |
| `NamespaceStatus.conditions` | ✅ Done |
| `ResourceRequirements.claims` | ✅ Done |
| `ResourceQuota`: full implementation | ✅ Already implemented |
| `StorageClass`: full implementation | ✅ Already implemented |
| `Container.lifecycle` + `Lifecycle`/`LifecycleHandler`/`SleepAction` types | ✅ Done |
| `EphemeralContainer`: `resizePolicy`, `restartPolicy`, `resources` | ✅ Done |
| `Volume` struct: `nfs`, `iscsi`, `projected`, `image` + projection types | ✅ Done |
| `NodeSpec.podCIDRs` | ✅ Done |
| `PersistentVolumeClaimSpec`: `dataSourceRef`, `volumeAttributesClassName` | ✅ Done |
| HPA: `MetricStatus.containerResource`, timestamp types | ✅ Done |
| `PodDisruptionBudget`: `unhealthyPodEvictionPolicy`, `conditions` | ✅ Already implemented |
| `PersistentVolumeStatus.reason/message` | ✅ Already implemented |
| `Container`: `envFrom`, `volumeDevices`, `terminationMessagePath/Policy`, `stdin/stdinOnce/tty` | ✅ Done |
| `ContainerStatus`: `lastState`, `imageID` | ✅ Done |
| `ContainerState` fields: `signal`, `message`, `startedAt`, `finishedAt`, `containerID` | ✅ Done |
| `VolumeMount`: `subPathExpr`, `mountPropagation`, `recursiveReadOnly` | ✅ Done |
| `EnvFromSource`, `ConfigMapEnvSource`, `SecretEnvSource`, `VolumeDevice` types | ✅ Done |
| `ControllerRevision` (apps/v1) | ✅ Already implemented |
| `VolumeAttachment` + spec/status (storage/v1) | ✅ Already implemented |
| `CSINode` + spec/drivers (storage/v1) | ✅ Already implemented |
| `CSIStorageCapacity` (storage/v1) | ✅ Already implemented |
| `RuntimeClass` (node.k8s.io/v1) | ✅ Already implemented |
| Endpoints, EndpointSubset, EndpointAddress, EndpointPort | ✅ Complete |
| EndpointSlice, Endpoint, EndpointConditions | ✅ Complete |
| ConfigMap, Secret | ✅ Complete |
| ServiceAccount | ✅ Complete |
| Lease, LeaseSpec (partial) | ✅ Mostly complete |
| Binding | ✅ Complete |
| ComponentStatus, ComponentCondition | ✅ Complete |
| PodTemplate, PodTemplateSpec | ✅ Complete |
| NetworkPolicy, Ingress, IngressClass | ✅ Complete |
| RBAC: ClusterRole, ClusterRoleBinding, Role, RoleBinding, PolicyRule, Subject, RoleRef | ✅ Complete |
| ObjectMeta, TypeMeta, ListMeta, OwnerReference, ManagedFieldsEntry | ✅ Complete |
| ObjectReference | ✅ Complete |

---

## Notes on Implementation

- **Serde compatibility**: When adding fields, always use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields. Kubernetes clients send partial objects; unrecognized fields must be tolerated (no `deny_unknown_fields`).
- **IntOrString**: Several fields (`maxUnavailable`, `maxSurge`, `targetPort`) are K8s `IntOrString` — currently represented as `Option<String>`. This is acceptable as strings can hold both `"30%"` and `"3"`. Consider a proper `IntOrString` enum if conformance tests check type fidelity.
- **Quantity**: Fields typed as `Quantity` in K8s (CPU, memory) are represented as `String` in rusternetes. This is correct.
- **Time fields**: K8s `Time` maps to `Option<DateTime<Utc>>` with our chrono setup. Several status fields use `Option<String>` instead — these should be corrected to `Option<DateTime<Utc>>`.
- **`replicas` optionality**: ✅ Fixed for `Deployment` and `StatefulSet` — now `Option<i32>` defaulting to 1 in controller logic.
- **Bulk struct updates**: When adding fields to widely-used structs like `PodSpec`, run a Python brace-matching script rather than manual edits — there are 80+ instantiation sites across the workspace. See past session notes.
- **Status struct**: The `metav1.Status` struct is critical for Kubernetes API compliance. Every error response MUST include `kind: "Status"`, `apiVersion: "v1"`, `status: "Failure"`, `message`, `reason` (machine-readable), `code` (HTTP status code), and optionally `details` with resource-specific information. kubectl uses `reason` to decide retry behavior.
