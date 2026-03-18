# Kubernetes 1.35 API Gap Analysis

This document compares rusternetes type definitions against the Kubernetes 1.35 OpenAPI spec
(`swagger.json`) and identifies missing fields, missing types, and structural deviations.

**Source of truth**: `~/Downloads/swagger.json` (Kubernetes master branch, ~1.35 era)
**Scope**: `core/v1`, `apps/v1`, `batch/v1`, `autoscaling/v2`, `networking/v1`, `rbac/v1`,
`storage/v1`, `policy/v1`, `coordination/v1`, `discovery/v1`, `events.k8s.io/v1`,
`meta/v1` (ObjectMeta, Status, etc.)

**Last updated**: 2026-03-17 (comprehensive swagger.json audit — Phase 3A/3B/3C complete)

---

## Priority Legend

- **P0** – Required for conformance / likely causes test failures today
- **P1** – Required for correct kubectl / client behavior
- **P2** – Important but less commonly exercised
- **P3** – Completeness / nice to have
- ✅ **Done** – Implemented

---

## ================================
## ITEMS DISCOVERED AND FIXED IN THIS AUDIT
## ================================

### ✅ NEW-1. meta/v1 — Status response object (P0 - FIXED)

| Item | Status |
|------|--------|
| `Status` struct with `reason`, `details`, `metadata` | ✅ Done (types.rs) |
| `StatusDetails` struct | ✅ Done (types.rs) |
| `StatusCause` struct | ✅ Done (types.rs) |
| `error.rs` refactored to use proper Status struct | ✅ Done |

### ✅ NEW-2. core/v1 — Event fields (P1 - FIXED)

| Field | Status |
|-------|--------|
| `eventTime` | ✅ Done |
| `reportingComponent` | ✅ Done |
| `reportingInstance` | ✅ Done |

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

### ✅ NEW-4. coordination/v1 — LeaseSpec fields (P1 - FIXED)

| Field | Status |
|-------|--------|
| `preferredHolder` | ✅ Done |
| `strategy` | ✅ Done |

### ✅ NEW-5. discovery/v1 — EndpointHints (P2 - FIXED)

| Field | Status |
|-------|--------|
| `forNodes` + `ForNode` type | ✅ Done |

### ✅ NEW-6. policy/v1 — PodDisruptionBudgetStatus (P1 - FIXED)

| Field | Status |
|-------|--------|
| `disruptedPods` | ✅ Done |

### ✅ NEW-7. storage/v1 — StorageClass (P1 - FIXED)

| Field | Status |
|-------|--------|
| `mountOptions` | ✅ Done |

### ✅ NEW-8. storage/v1 — CSIDriverSpec (P2 - FIXED)

| Field | Status |
|-------|--------|
| `nodeAllocatableUpdatePeriodSeconds` | ✅ Done |

### ✅ NEW-9. apps/v1 — ReplicaSetStatus (P2 - FIXED)

| Field | Status |
|-------|--------|
| `terminatingReplicas` | ✅ Done |

### ✅ NEW-10. ServicePort.targetPort type (P1 - FIXED)

| Item | Status |
|------|--------|
| Changed `target_port` from `Option<u16>` to `Option<IntOrString>` | ✅ Done |

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

### ✅ Completed (Phase 3C)

| Field | Status |
|-------|--------|
| `resize` | ✅ Done |
| `resourceClaimStatuses` + `PodResourceClaimStatus` type | ✅ Done |
| `observedGeneration` | ✅ Done |

### Still missing (P3)

| Field | K8s Type | Priority | Notes |
|-------|----------|----------|-------|
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

### ✅ Completed (Phase 3C)

| Field | Status |
|-------|--------|
| `user` + `ContainerUser`/`LinuxContainerUser` types | ✅ Done |
| `volumeMounts` + `VolumeMountStatus` type | ✅ Done |
| `stopSignal` | ✅ Done |

---

## 5. core/v1 — PodSecurityContext ✅

**File**: `crates/common/src/resources/pod.rs`

### ✅ Completed (Phase 3C)

| Field | Status |
|-------|--------|
| `seLinuxChangePolicy` | ✅ Done |
| `supplementalGroupsPolicy` | ✅ Done |

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

### ✅ Lifecycle.stopSignal already present (verified)

---

## 8. core/v1 — EphemeralContainer ✅ (partial)

**File**: `crates/common/src/resources/pod.rs`

### ✅ Completed

| Field | Status |
|-------|--------|
| `resizePolicy: Option<Vec<ContainerResizePolicy>>` | ✅ Added |
| `restartPolicy: Option<String>` | ✅ Added |
| `resources: Option<ResourceRequirements>` | ✅ Added |

### ✅ Completed (Phase 3C)

| Field | Status |
|-------|--------|
| `stdin`, `stdinOnce`, `tty` | ✅ Already present |
| `terminationMessagePath`, `terminationMessagePolicy` | ✅ Done |

### Still missing (P3)

| Field | K8s Type | Priority |
|-------|----------|----------|
| `restartPolicyRules` | `[]ContainerRestartRule` | P3 (alpha) |

---

## 9-13. apps/v1 — Deployments, StatefulSets, DaemonSets ✅

### ✅ Completed (Phase 3C)

| Field | Status |
|-------|--------|
| `DeploymentStatus.terminatingReplicas` | ✅ Done |
| `ReplicaSetStatus.terminatingReplicas` | ✅ Done |
| `StatefulSetSpec.ordinals` + `StatefulSetOrdinals` type | ✅ Done |

---

## 14-16. batch/v1 — Jobs, CronJobs ✅

### ✅ Completed (Phase 3C)

| Field | Status |
|-------|--------|
| `JobSpec.backoffLimitPerIndex` | ✅ Done |
| `JobSpec.maxFailedIndexes` | ✅ Done |
| `JobSpec.podFailurePolicy` | ✅ Done |
| `JobSpec.podReplacementPolicy` | ✅ Done |
| `JobSpec.successPolicy` | ✅ Done |
| `JobSpec.managedBy` | ✅ Done |
| `JobStatus.completedIndexes` | ✅ Done |
| `JobStatus.failedIndexes` | ✅ Done |
| `JobStatus.uncountedTerminatedPods` + `UncountedTerminatedPods` type | ✅ Done |

---

## 17-30. Other completed sections

See Phase 2 completion table below. All previously tracked items remain done.

---

## ================================
## IMPLEMENTATION PLAN
## ================================

### ✅ Phase 3A — P0 fixes (COMPLETE)

| # | Item | Status |
|---|------|--------|
| 1 | `Status` struct with `reason`, `details`, `metadata` | ✅ Done |
| 2 | `StatusDetails` struct | ✅ Done |
| 3 | `StatusCause` struct | ✅ Done |
| 4 | Refactor error.rs to use proper Status struct | ✅ Done |
| 5 | Map Error variants to Status reasons | ✅ Done |

### ✅ Phase 3B — P1 fixes (COMPLETE)

| # | Item | Status |
|---|------|--------|
| 6 | `ServicePort.targetPort`: `Option<u16>` → `Option<IntOrString>` | ✅ Done |
| 7 | `StorageClass.mountOptions` | ✅ Done |
| 8 | `LeaseSpec`: `preferredHolder`, `strategy` | ✅ Done |
| 9 | `PodDisruptionBudgetStatus.disruptedPods` | ✅ Done |
| 10 | core/v1 Event: `eventTime`, `reportingComponent`, `reportingInstance` | ✅ Done |
| 11 | events.k8s.io/v1 Event: New type + handler | Still TODO (P1) |

### ✅ Phase 3C — P2 fixes (COMPLETE)

| # | Item | Status |
|---|------|--------|
| 12 | `PodSecurityContext`: `seLinuxChangePolicy`, `supplementalGroupsPolicy` | ✅ Done |
| 13 | `PVCStatus`: `currentVolumeAttributesClassName`, `modifyVolumeStatus` + type | ✅ Done |
| 14 | `PVStatus.lastPhaseTransitionTime` | ✅ Done |
| 15 | `PVSpec.volumeAttributesClassName` | ✅ Done |
| 16 | `NodeSystemInfo.swap` + `NodeSwapStatus` type | ✅ Done |
| 17 | `NodeStatus`: `config`, `features`, `runtimeHandlers` + 7 new types | ✅ Done |
| 18 | `Lifecycle.stopSignal` | ✅ Already present |
| 19 | `HPAScalingRules.tolerance` | ✅ Done |
| 20 | `DeploymentStatus.terminatingReplicas` | ✅ Done |
| 21 | `ReplicaSetStatus.terminatingReplicas` | ✅ Done |
| 22 | `StatefulSetSpec.ordinals` + `StatefulSetOrdinals` type | ✅ Done |
| 23 | `JobSpec`: all 6 fields + helper types | ✅ Done |
| 24 | `JobStatus`: `completedIndexes`, `failedIndexes`, `uncountedTerminatedPods` | ✅ Done |
| 25 | `ContainerStatus`: `user`, `volumeMounts`, `stopSignal` + types | ✅ Done |
| 26 | `PodStatus`: `resize`, `resourceClaimStatuses`, `observedGeneration` | ✅ Done |
| 27 | `EphemeralContainer`: `terminationMessagePath`, `terminationMessagePolicy` | ✅ Done |
| 28 | `Volume.csi` (inline CSIVolumeSource) | ✅ Already present |
| 29 | `EndpointHints.forNodes` + `ForNode` type | ✅ Done |
| 30 | `CSIDriverSpec.nodeAllocatableUpdatePeriodSeconds` | ✅ Done |
| 31 | `PodSpec.serviceAccount` (deprecated alias) | ✅ Done |
| 32 | `EnvVarSource.fileKeyRef` + `FileKeySelector` type | ✅ Done |

### Phase 4 — P3 (Nice to have / legacy / alpha) — NOT STARTED

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
