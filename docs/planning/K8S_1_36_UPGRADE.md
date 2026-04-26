# Kubernetes 1.36 Conformance Upgrade — Exhaustive Plan

## Context

Kubernetes 1.36 released April 22, 2026. Rūsternetes targets v1.35 at 91.4% (403/441, Round 155). This plan is based on a line-by-line reading of `CHANGELOG/CHANGELOG-1.36.md` (598 lines), systematic analysis of every API type file in `staging/src/k8s.io/api/`, and identification of every conformance test added in 1.36.

**Only 5 new conformance tests were added in 1.36** (found by searching `test/e2e/` for `[Conformance]`):
1. MutatingAdmissionPolicy — API operations
2. MutatingAdmissionPolicy — Binding API operations  
3. MutatingAdmissionPolicy — mutate a Deployment (replicas=1337)
4. MutatingAdmissionPolicy — mutate a Deployment with annotations
5. ImageVolume — pod with Always pull policy

The primary risk is NOT new tests but behavioral changes that break existing passing tests.

---

## Phase 0: Version Strings & Conformance Image

**Effort: 1-2 hours | BLOCKING — sonobuoy won't run 1.36 tests without this**

| File | Location | Change |
|------|----------|--------|
| `crates/api-server/src/handlers/discovery.rs` | `get_version()` ~line 1397 | `minor: "35"→"36"`, `git_version: "v1.35.0"→"v1.36.0"` |
| `crates/kubelet/src/kubelet.rs` | lines 398-399, 539-540 | `"v1.35.0-rusternetes"→"v1.36.0-rusternetes"` |
| `crates/kubelet/src/kubelet.rs` | lines 399, 540 | `kube_proxy_version: ""` (DisableNodeKubeProxyVersion GA, PR#136673) |
| `crates/kubectl/src/commands/version.rs` | lines 43, 49, 84, 89, 98-99 | All `"35"→"36"` |
| `scripts/run-conformance.sh` | line 65 | `conformance:v1.35.0→v1.36.0` |

**After this**: Run baseline conformance with v1.36 image to see how many existing tests still pass.

---

## Phase 1: New Conformance Tests (5 tests)

### 1.1 MutatingAdmissionPolicy (4 tests) — PR#136039

**Effort: 4-6 days**

Source: `test/e2e/apimachinery/mutatingadmissionpolicy.go`

Test details:
- Test 1: Discovery — `admissionregistration.k8s.io/v1` must list `mutatingadmissionpolicies` resource
- Test 2: Discovery — must list `mutatingadmissionpolicybindings` resource  
- Test 3: Create policy with CEL `Object{spec: Object.spec{replicas: 1337}}`, create Deployment, verify replicas changed to 1337
- Test 4: Create policy adding annotations via CEL ApplyConfiguration, verify annotations applied

**Implementation:**

**Types** — create `crates/common/src/resources/mutating_admission_policy.rs`:
- `MutatingAdmissionPolicy` / `MutatingAdmissionPolicyList`
- `MutatingAdmissionPolicySpec`: `param_kind`, `match_constraints`, `variables: Vec<Variable>`, `mutations: Vec<Mutation>`, `failure_policy`, `match_conditions`, `reinvocation_policy`
- `Mutation`: `patch_type: String` ("ApplyConfiguration"|"JSONPatch"), `apply_configuration: Option<ApplyConfiguration>`, `json_patch: Option<JSONPatch>`
- `ApplyConfiguration`: `expression: String`
- `JSONPatch`: `expression: String`
- `MutatingAdmissionPolicyBinding` / `MutatingAdmissionPolicyBindingList`
- `MutatingAdmissionPolicyBindingSpec`: `policy_name`, `param_ref`, `match_resources`
- Reuse from VAP: `ParamKind`, `MatchResources`, `NamedRuleWithOperations`, `Variable`, `ParamRef`, `FailurePolicy`, `MatchCondition`

**Handlers** — create `crates/api-server/src/handlers/mutating_admission_policy.rs`:
- Full CRUD + watch + status for both resources (model after `validating_admission_policy.rs`)

**Routes** — `crates/api-server/src/router.rs`:
```
/apis/admissionregistration.k8s.io/v1/mutatingadmissionpolicies[/:name[/status]]
/apis/admissionregistration.k8s.io/v1/mutatingadmissionpolicybindings[/:name]
+ watch variants
```

**Discovery** — `crates/api-server/src/handlers/discovery.rs`:
- Add both resources to `get_admissionregistration_v1_resources()` and aggregated discovery

**CEL mutation** — extend `crates/common/src/cel.rs`:
- `evaluate_to_json()` — evaluate CEL expression returning JSON for ApplyConfiguration patches
- Existing `CELContext::for_admission()` already provides `object`, `oldObject`, `params`, `request`

**Admission chain** — new function in `crates/api-server/src/admission_webhook.rs`:
- `run_mutating_admission_policies()` — load bindings, match, evaluate CEL, apply patches
- Must actually mutate the object — test 3 verifies replicas changed, test 4 verifies annotations added

**OpenAPI** — add schema definitions for new types

### 1.2 ImageVolume (1 test) — PR#136711

**Effort: 2-3 days**

Source: `test/e2e/common/node/image_volume.go`
Test: Create pod with image volume, `pull_policy: Always`, verify volume mounted

**Implementation** — `crates/kubelet/src/runtime.rs` in `create_volume()`:
1. Pull image via `docker.create_image()` using `reference`
2. Create temp container from image
3. Extract filesystem via `docker.download_from_container("/")`
4. Untar to volume directory
5. Remove temp container
6. Respect `pull_policy` (Always/IfNotPresent/Never)

---

## Phase 2: Behavioral Changes (could break existing passing tests)

### 2.1 StrictIPCIDRValidation (beta, default-on) — PR#137053

**Effort: 0.5-1 day | Risk: MEDIUM**

Rejects:
- IPv4 with leading zeros: `010.0.0.1` → INVALID
- IPv4-mapped IPv6: `::ffff:1.2.3.4` → INVALID
- CIDR with leading zeros in prefix: `10.0.0.0/024` → INVALID
- CIDR with host bits set: `192.168.1.5/24` → INVALID (must be `192.168.0.0/24`)

Create validation utilities, apply in: `handlers/service.rs`, `handlers/pod.rs`, `handlers/node.rs`, `handlers/networkpolicy.rs`, `ip_allocator.rs`

### 2.2 CRD numeric format validation — PR#136582

**Effort: 0.5 day | Risk: LOW-MEDIUM**

CRD schemas with `format: int32` enforce -2147483648 to 2147483647. `format: int64` enforces int64 range. Validation ratcheting for existing values.

Modify: `crates/common/src/schema_validation.rs`

### 2.3 Service name validation relaxed — PR#136389

**Effort: 1-2 hours | Risk: LOW**

`RelaxedServiceNameValidation` beta, default-on. Service names validated with `NameIsDNSLabel()` instead of stricter rules. If we reject names K8s accepts, tests fail.

### 2.4 Pod lifecycle hook termination fix — PR#136598

**Effort: 0.5-1 day | Risk: MEDIUM**

preStop hooks can no longer run for full `terminationGracePeriodSeconds`. Must complete within a fraction, leaving time for SIGTERM.

Modify: `crates/kubelet/src/runtime.rs` — cap preStop hook duration to `gracePeriod - 2s`

### 2.5 SSA empty array/map fix — PR#135391

**Effort: 0.5 day | Risk: LOW-MEDIUM**

Empty arrays and maps were incorrectly treated as absent in server-side apply and client-go Extract functions. Atomic elements from associative lists were incorrectly duplicated.

Check: `crates/api-server/src/patch.rs` — verify SSA handles empty arrays/maps correctly

### 2.6 CRD SSA field ownership for status subresource — PR#137689

**Effort: 0.5 day | Risk: LOW**

Metadata ownership must be correctly tracked for writes to `/status` subresource. Custom resources must not update metadata from `/status`.

Check: `crates/api-server/src/handlers/custom_resource.rs` — status subresource handler

### 2.7 ServiceCIDR status field wiping — PR#137715

**Effort: 1-2 hours | Risk: LOW**

Writes to ServiceCIDR main resource now ignore status field changes. `ServiceCIDRStatusFieldWiping` feature gate.

Modify: `crates/api-server/src/handlers/servicecidr.rs` — strip status from non-status updates

### 2.8 Scheduler scoring change — PR#135573

**Effort: 1-2 hours | Risk: LOW**

`NodeResourcesBalancedAllocation` now considers balance with AND without the requested pod. Could change scheduling decisions.

Check: `crates/scheduler/src/plugins/` — if we implement this plugin

### 2.9 StatefulSet availableReplicas timing fix — PR#135428

**Effort: 0.5 day | Risk: LOW**

StatefulSets now count `.status.availableReplicas` at the correct time without delay, resulting in faster rollout progress.

Check: `crates/controller-manager/src/controllers/statefulset.rs`

### 2.10 MaxUnavailableStatefulSet disabled by default — PR#137904

**Effort: 1 hour | Risk: LOW**

v1.35 regression fix — `MaxUnavailableStatefulSet` feature disabled by default.

### 2.11 Pod resize at admission time — PR#136043

**Effort: 0.5 day | Risk: LOW (we don't implement pod resize yet)**

Pod resize exceeding node capacity now fails at admission, not in pod status. Non-Linux resize also fails at admission.

---

## Phase 3: New API Fields (must round-trip for conformance)

Even if not functionally implemented, these fields must be accepted by the API server and preserved on read-back. Conformance tests may set them.

### 3.1 Core API

| Struct | Field | Type | Gate | PR |
|--------|-------|------|------|-----|
| `PodSpec` | `scheduling_group` | `Option<PodSchedulingGroup>` | GenericWorkload | #136976 |
| `VolumeProjection` | `volume_attributes_class` | `Option<VolumeAttributesClassProjection>` | VolumeAttributesClass | #134556 |
| `ContainerStatus` | `image_volume_digest` | `Option<String>` | ImageVolumeWithDigest | #132807 |

### 3.2 Storage API

| Struct | Field | Type | Gate | PR |
|--------|-------|------|------|-----|
| `CSIDriverSpec` | `service_account_token_in_secrets` | `Option<bool>` | CSIServiceAccountTokenSecrets | #136596 |
| `CSIDriverSpec` | `node_allocatable_update_period_seconds` | `Option<i64>` | MutableCSINodeAllocatableCount | #136230 |
| `CSIDriverSpec` | `prevent_pod_scheduling_if_missing` | `Option<bool>` | VolumeLimitScaling | #137343 |
| `CSIDriverSpec` | `se_linux_mount` | `Option<bool>` | SELinuxMountReadWriteOncePod | #136912 |
| `VolumeError` | `error_code` | `Option<i32>` | MutableCSINodeAllocatableCount | #136230 |

### 3.3 Coordination API

| Struct | Field | Type | Gate | PR |
|--------|-------|------|------|-----|
| `LeaseSpec` | `strategy` | `Option<String>` | CoordinatedLeaderElection | - |
| `LeaseSpec` | `preferred_holder` | `Option<String>` | CoordinatedLeaderElection | - |

### 3.4 Admission API

MutatingAdmissionPolicy types — covered in Phase 1.1

### 3.5 Discovery API

EndpointSlice `endpoints` field marked optional — already handled (`#[serde(default)]`)

---

## Phase 4: GA Features That May Be Tested Indirectly

### 4.1 ProcMountType validation (GA) — PR#137454
Validate `securityContext.procMount` accepts only "Default" or "Unmasked". Translate to Docker security opts.

### 4.2 DeclarativeValidation / CEL in CRD schemas (GA) — PR#136793
`x-kubernetes-validations` CEL rules evaluated during CR validation. Extend `schema_validation.rs`.

### 4.3 UserNamespacesSupport (GA) — PR#136792
`pod.spec.host_users: false` should set Docker `userns_mode`. Our field exists; kubelet enforcement missing.

### 4.4 NodeLogQuery (GA) — PR#137544
New `/logs` endpoint on kubelet. Add route + proxy from API server.

### 4.5 SELinuxChangePolicy (GA) — PR#136912
Field exists. No functional enforcement needed for Docker kubelet. Verify round-trip.

### 4.6 KubeletFineGrainedAuthz (GA) — PR#136116
Fine-grained authorization on kubelet API endpoints. Defer unless conformance tests target it.

### 4.7 RestartAllContainersOnContainerExits (Beta, default-on) — PR#136681
When a container exits with `restartPolicy: Always`, restart ALL containers. Modify kubelet restart logic.

### 4.8 InPlacePodLevelResourcesVerticalScaling (Beta, default-on) — PR#137684
`pod.spec.resources` for pod-level CPU/memory. Field exists; kubelet resource computation needs update.

### 4.9 ExtendWebSocketsToKubelet (Beta, default-on) — PR#136256
WebSocket exec/attach/portforward proxied directly to kubelet. We already support WebSocket exec.

### 4.10 ConstrainedImpersonation (Beta, default-on) — PR#137609
Limits on impersonation. Low conformance risk.

### 4.11 ExternalServiceAccountTokenSigner (GA) — PR#136118
External SA token signer. Our JWT signing should be compatible.

### 4.12 CSIServiceAccountTokenSecrets (GA) — PR#136596
CSI driver SA tokens in secrets. Add field to CSIDriverSpec (covered in Phase 3.2).

---

## Phase 5: Additional Changes Found in CHANGELOG

### 5.1 Deprecations to handle
- Service `.spec.externalIPs` — add `Warning` response header (PR#137293)
- `git-repo` volume plugin disabled (PR#136400) — we don't implement it, no action
- `FieldsV1.Raw` direct access deprecated (PR#137304) — no impact on our Rust implementation
- `AllowlistEntry.Name` → `AllowlistEntry.Command` (PR#137272) — credential plugin, no impact

### 5.2 Removals to verify
- Portworx in-tree plugin removed (PR#135322) — not implemented, no action
- `scheduling.k8s.io/v1alpha1` removed (PR#136976) — not implemented, no action  
- `SnapshotMetadataService v1alpha1` removed (PR#137564) — not implemented, no action
- `v1alpha1 WebhookAdmissionConfiguration` removed (PR#137379) — not implemented, no action
- `ProtoMessage()` marker methods removed (PR#137084) — no impact on our Rust implementation

### 5.3 Controller behavioral changes
- ReplicaSet can read its own writes (PR#137212) — prevents spurious reconciliation
- StatefulSet can read its own Pod/PVC writes (PR#137254) — prevents spurious reconciliation
- DaemonSet defers syncing on stale cache (PR#134937) — prevents duplicate pod creation
- Job controller defers syncing on stale cache (PR#137210) — prevents duplicate pods
- GC correctly handles externally deleted objects (PR#136817) — prevents spurious error logs

### 5.4 Metric renames (ACTION REQUIRED per CHANGELOG)
- `volume_operation_total_errors` → `volume_operation_errors_total` (PR#136399)
- `etcd_bookmark_counts` → `etcd_bookmark_total` (PR#136483)
- We don't expose these metrics, so no action needed

### 5.5 Audit policy `group: "*"` wildcard support — PR#135262
Kube-apiserver audit policy now supports `group: "*"` to match all API groups. Low priority.

### 5.6 Liveness probe fails on expired loopback cert — PR#136477
API server liveness probes fail when loopback client cert expires. May not affect us.

### 5.7 Audit log rotation defaults changed — PR#136478
`maxage=366`, `maxbackup=100`. We don't implement audit log rotation.

### 5.8 RBAC cluster role changes — PR#135418
Added write/read permissions for workloads to admin/edit/view cluster roles. May need to update bootstrapped ClusterRoles.

### 5.9 CoreDNS updated to v1.14.2 — PR#137605
May need to update CoreDNS image reference.

### 5.10 Pause image updated to v3.10.2 — PR#138199
May need to update pause image reference in kubelet.

---

## Phase 6: kubectl Changes

### 6.1 New features to implement

| Feature | PR | Priority |
|---------|-----|----------|
| `kubectl get node -o wide` — ARCH column | #132402 | MEDIUM — conformance may test wide output |
| `kubectl describe service` — appProtocol field | #135744 | LOW |
| `kubectl describe cronjob` — timezone field | #136663 | LOW |
| `kubectl explain -r` shorthand for --recursive | #135283 | LOW |
| `kubectl wait` — multiple conditions | #136855 | MEDIUM |
| `kubectl exec/logs` — list valid containers on mismatch | #136973 | LOW |
| `kubectl scale` — output reflects expected replicas | #136945 | LOW |
| `kubectl get ingressclass` — (default) marker | #134422 | LOW |
| `kubectl describe node` — ResourceSlices listing | #131744 | LOW |
| `kubectl logs -f` — wait for containers to start | #136411 | MEDIUM |
| `kubectl apply --dry-run=client` — merged manifest output | #135513 | MEDIUM |
| `kubectl describe` — events only for single object by default | #137145 | LOW |
| Default debug profile `legacy` → `general` | #135874 | LOW |
| `kubectl diff --show-secret` flag | #137019 | LOW |
| `kubectl attach/run --detach-keys` flag | #134997 | LOW |

### 6.2 Bug fixes to verify

| Fix | PR | Impact |
|-----|-----|--------|
| `kubectl delete` multiple StatefulSet pods | #135563 | May affect conformance |
| `kubectl describe node` pod-level resources | #137394 | May affect conformance |
| `kubectl describe` uppercase acronyms in CR fields | #135683 | LOW |
| `kubectl label` shows "modified" | #134849 | LOW |
| Panic fix: nil resource requests + populated status | #136534 | LOW |
| Panic fix: exec terminal size queue | #135918 | LOW |
| `kubectl run -i/-it` missing output | #136010 | LOW |

### 6.3 Priority assessment

Most kubectl changes are NOT conformance-tested directly — conformance tests use the Go client, not kubectl. However, some conformance tests DO shell out to kubectl for specific operations. Focus on:
1. Version string update (Phase 0)
2. `kubectl get node -o wide` ARCH column (if tested)
3. `kubectl logs -f` wait behavior (if tested)

---

## Phase 7: Kube-Proxy Changes

| Change | PR | Impact |
|--------|-----|--------|
| IPVS/WinKernel: backends recheck rules regularly (v1.34 regression fix) | #135631 | May affect service routing tests |
| conntrack cleanup optimization (reduced time complexity) | #135511 | Performance only |
| nftables mode fixed for nft v1.1.3 | #137501 | We use iptables, no impact |
| Windows dual-stack LB sharing fix | #136241 | No impact (Linux only) |
| kube-proxy log spam fix (unready endpoints) | #136743 | No impact |
| nf_conntrack_max capped to 1,048,576 | #137002 | May need to verify |
| Pod IP reuse handling (terminated but not deleted) | #135593 | May affect service routing |

---

## Phase 8: OpenAPI, Validation, and Defaulting Changes

### 8.1 OpenAPI schema correctness — PR#134675
Updated API server internal API group to improve openapi schema correctness for fields being optional or required. Our OpenAPI handler (`crates/api-server/src/openapi.rs`) may need updates to match.

### 8.2 Validation changes not covered elsewhere
- `imageMinimumGCAge` — negative duration values now rejected (PR#135997)
- `restartPolicyRules` validation error messages changed from "bytes" to "items" (PR#137136)
- `RestartContainer` resize restart policy disallowed on non-sidecar initContainers (PR#137458)

### 8.3 HPA changes
- Scaling to/from zero when `HPAScaleToZero` enabled (PR#135118)
- Fixed v2 HPA resources with object metrics + averageValue via v1 API (PR#137856)
- `MutablePodResourcesForSuspendedJobs` and `MutableSchedulingDirectivesForSuspendedJobs` now enabled by default (PR#135965)

### 8.4 Container image version updates
| Image | Old Version | New Version | PR |
|-------|------------|-------------|-----|
| CoreDNS | v1.12.0 (approx) | v1.14.2 | #137605 |
| pause | v3.10 | v3.10.2 | #138199 |
| etcd | v3.5.x | v3.6.8 | #137107 |

Check: `scripts/bootstrap-cluster.sh` and `crates/kubelet/src/runtime.rs` for image references

### 8.5 Kubelet configuration additions
- `MemoryReservationPolicy` field in KubeletConfiguration (PR#137584) — cgroup v2 memory QoS
- `ReloadKubeletClientCAFile` (beta, default-on) — hot-reload client CA (PR#136762)
- `EnableSystemLogQuery` config field for NodeLogQuery (PR#137544)

---

## Phase 9: New/Changed Resource Types and API Groups

### 9.1 New API groups in K8s 1.36 not in rūsternetes

| API Group | Version | Resources | Status | Action |
|-----------|---------|-----------|--------|--------|
| `scheduling.k8s.io` | `v1alpha2` | Workload, PodGroup | Alpha | Defer |
| `resource.k8s.io` | `v1beta2` | ResourceSlice, DeviceTaintRule | Beta | Defer (DRA v1beta2 for taints) |
| `certificates.k8s.io` | `v1beta1` | PodCertificateRequest | Beta | Defer |
| `coordination.k8s.io` | `v1alpha2` | LeaseCandidate | Alpha | Defer |

### 9.2 New resource types within existing API groups

| Group | Resource | Version | Status | Action |
|-------|----------|---------|--------|--------|
| `admissionregistration.k8s.io/v1` | `MutatingAdmissionPolicy` | GA | **MUST implement** (Phase 1.1) |
| `admissionregistration.k8s.io/v1` | `MutatingAdmissionPolicyBinding` | GA | **MUST implement** (Phase 1.1) |
| `resource.k8s.io/v1` | `DeviceTaintRule` | Beta (v1beta2) | Defer |
| `resource.k8s.io/v1` | `ResourcePoolStatusRequest` | Alpha | Defer |

### 9.3 Resources we serve that gained new fields in 1.36

| Resource | New Fields | Phase |
|----------|-----------|-------|
| `Pod` | `spec.schedulingGroup` (alpha) | Phase 3.1 |
| `Pod` | `spec.resources` (beta, pod-level) | Phase 4.8 |
| `ContainerStatus` | `imageVolumeDigest` (alpha) | Phase 3.1 |
| `CSIDriver` | `serviceAccountTokenInSecrets`, `nodeAllocatableUpdatePeriodSeconds`, `preventPodSchedulingIfMissing`, `seLinuxMount` | Phase 3.2 |
| `Lease` | `strategy`, `preferredHolder` (alpha) | Phase 3.3 |
| `VolumeError` | `errorCode` | Phase 3.2 |
| `VolumeProjection` | `volumeAttributesClass` (alpha) | Phase 3.1 |
| `EndpointSlice` | `endpoints` now optional in OpenAPI | Already handled |
| `PersistentVolumeClaim` | `Unused` condition (alpha) | Defer |

### 9.4 Resources we serve where validation/behavior changed

| Resource | Change | Phase |
|----------|--------|-------|
| `Service` | `externalIPs` deprecation warning | Phase 5.1 |
| `Service` | relaxed name validation | Phase 2.3 |
| `ServiceCIDR` | status wiped on main resource write | Phase 2.7 |
| `CronJob` | `schedule` now required | Phase 3.4 |
| `CustomResourceDefinition` | numeric format range validation | Phase 2.2 |
| `CustomResourceDefinition` | SSA metadata ownership for status subresource | Phase 2.6 |
| All resources with IP/CIDR fields | strict validation (no leading zeros) | Phase 2.1 |

---

## Phase 10: Alpha Features (NOT conformance-tested — defer entirely)

- PodGroup/Workload APIs (scheduling.k8s.io/v1alpha2) — PR#136976
- ResourcePoolStatusRequest (v1alpha1) — PR#137028
- TopologyAwareWorkloadScheduling — PR#137271
- WorkloadAwarePreemption — PR#136589
- DRANativeResources — PR#136725
- DRAListTypeAttributes — PR#137190
- PersistentVolumeClaimUnusedSinceTime — PR#137862
- ManifestBasedAdmissionControlConfig — PR#137346

---

## Implementation Sequence

```
Week 1:
  Day 1:  Phase 0 — all version strings + conformance image + DisableNodeKubeProxyVersion
          Run baseline conformance with v1.36 image
          Phase 2.3 — service name validation relaxed
          Phase 4.1 — ProcMountType validation
          Phase 4.5 — SELinuxChangePolicy verify
  
  Day 2:  Phase 2.1 — StrictIPCIDRValidation
          Phase 2.2 — CRD numeric format validation
          Phase 3 — add all new API fields to types
  
  Day 3-5: Phase 1.1 — MutatingAdmissionPolicy
           Types, handlers, routes, discovery

Week 2:
  Day 1-2: Phase 1.1 continued — CEL mutation engine, admission chain, OpenAPI
  Day 3:   Phase 1.2 — ImageVolume kubelet execution
  Day 4:   Phase 2.4 — Pod lifecycle hook termination fix
           Phase 2.5 — SSA empty array/map fix
           Phase 2.7 — ServiceCIDR status field wiping
  Day 5:   Phase 4.2 — DeclarativeValidation (CEL in CRD schemas)

Week 3:
  Day 1:   Phase 4.3 — User namespace kubelet enforcement
  Day 2:   Phase 4.4 — NodeLogQuery endpoint
  Day 3:   Phase 4.7 — RestartAllContainersOnContainerExits
  Day 4-5: Run conformance, identify regressions, fix failures

Week 4:
  Buffer for regression fixes. Phase 5 items as needed.
```

---

## Verification

1. **Phase 0 baseline**: Run v1.36 conformance, record pass/fail vs v1.35 baseline
2. **After Phase 1**: The 5 new conformance tests must pass
3. **After Phase 2**: No regressions from behavioral changes (compare to Phase 0 baseline)
4. **Unit tests**: Each new type needs serde round-trip tests
5. **Target**: Match or exceed 91.4% against v1.36 conformance suite

---

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| MAP CEL mutation engine complexity | HIGH | Start with test 3 (replicas=1337, simplest case); reuse existing VAP CEL engine |
| ImageVolume Docker filesystem extraction | MEDIUM | Use create_container + download_from_container (existing bollard pattern) |
| StrictIPCIDRValidation breaks many tests | MEDIUM | Centralize validation; apply only on create/update with ratcheting |
| Unknown behavioral changes not caught | MEDIUM | Run Phase 0 baseline FIRST to identify unexpected failures |
| Baseline regression from version bump alone | LOW | Many API changes are field additions that serde(default) handles |

---

## Source References

| Topic | Rūsternetes | K8s 1.36 |
|-------|-------------|----------|
| VAP (template for MAP) | `crates/common/src/resources/validating_admission_policy.rs` | `staging/src/k8s.io/api/admissionregistration/v1/types.go` |
| MAP conformance tests | N/A (to create) | `test/e2e/apimachinery/mutatingadmissionpolicy.go` |
| ImageVolume test | N/A (to create) | `test/e2e/common/node/image_volume.go` |
| CEL engine | `crates/common/src/cel.rs` | `staging/src/k8s.io/apiserver/pkg/cel/` |
| IP validation | N/A (to create) | `staging/src/k8s.io/apimachinery/pkg/util/validation/ip.go` |
| CRD validation | `crates/common/src/schema_validation.rs` | `staging/src/k8s.io/apiextensions-apiserver/pkg/apiserver/validation/formats.go` |
| Feature gates | N/A | `pkg/features/kube_features.go` |
| Pod types | `crates/common/src/resources/pod.rs` | `staging/src/k8s.io/api/core/v1/types.go` |
| CSIDriver types | `crates/common/src/resources/` | `staging/src/k8s.io/api/storage/v1/types.go` |
| Lease types | `crates/common/src/resources/lease.rs` | `staging/src/k8s.io/api/coordination/v1/types.go` |
| SSA/patch | `crates/api-server/src/patch.rs` | `staging/src/k8s.io/apiserver/pkg/endpoints/handlers/` |
| CHANGELOG | N/A | `CHANGELOG/CHANGELOG-1.36.md` (lines 123-571) |
