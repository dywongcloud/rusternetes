# Kubernetes 1.35 Conformance Behavioral Gaps

This document captures architectural and behavioral issues that will cause conformance
test failures, discovered through a comprehensive audit of API server, kubelet,
controller-manager, and discovery/OpenAPI implementations.

**Last updated**: 2026-03-18

---

## Priority Legend

- **P0** - Will cause widespread conformance test failures
- **P1** - Will cause specific conformance test failures
- **P2** - May cause edge-case failures or affect kubectl UX
- **P3** - Nice to have, unlikely to affect conformance directly

---

## 1. API Server Behavioral Gaps

### P0 — DELETE handlers don't return the deleted object

**Files**: All handlers in `crates/api-server/src/handlers/` (pod.rs, deployment.rs, etc.)

**Issue**: DELETE returns `StatusCode::NO_CONTENT` or `StatusCode::OK` with empty body.
Kubernetes returns the deleted object (or a Status object) in the response body.
Conformance tests read the deleted object from the response to verify deletion.

**Fix**: Change all delete handlers to return `Json<T>` with the deleted resource.

### P0 — metadata.generation never incremented on spec changes

**Files**: All create/update handlers

**Issue**: `metadata.generation` is never set or incremented. Kubernetes sets it to 1
on creation and increments it on every spec change (but NOT status-only changes).
Controllers compare `observedGeneration` vs `generation` to detect unprocessed changes.

**Fix**: Set generation=1 on create. Increment on PUT if spec fields changed
(compare old vs new, excluding metadata and status). Don't increment on status updates.

### P0 — resourceVersion conflict detection not enforced

**Files**: All update handlers, `crates/api-server/src/handlers/status.rs`

**Issue**: PUT/Update doesn't compare the provided `metadata.resourceVersion` against the
stored one. Without this, concurrent updates silently overwrite each other instead of
returning 409 Conflict.

**Fix**: On every PUT, check `request.metadata.resourceVersion == stored.metadata.resourceVersion`.
If mismatch, return 409 Conflict with Status reason "Conflict".

### P1 — No validation of required fields

**Files**: Create handlers

**Issue**: Pod creation accepts pods with no containers, no image, etc. Service creation
accepts services with no ports. These should be rejected with 422 Unprocessable Entity.

**Fix**: Add validation before storage:
- Pod: must have >= 1 container, each container must have image
- Service: must have >= 1 port (unless type ExternalName)
- Container name must be DNS-compliant

### P1 — Default values not set on resources

**Files**: Create handlers

**Issue**: Kubernetes sets defaults before storing resources:
- Pod `spec.restartPolicy` defaults to "Always"
- Pod `spec.dnsPolicy` defaults to "ClusterFirst"
- Service `spec.type` defaults to "ClusterIP"
- Service `spec.sessionAffinity` defaults to "None"
- Container `terminationMessagePath` defaults to "/dev/termination-log"
- Container `terminationMessagePolicy` defaults to "File"
- Container `imagePullPolicy` defaults based on image tag

**Fix**: Add a defaulting step after admission but before storage.

### P1 — DELETE doesn't handle gracePeriodSeconds or propagationPolicy

**Files**: Delete handlers

**Issue**: DELETE query params `gracePeriodSeconds` and `propagationPolicy` are parsed
but ignored. Should affect finalizer behavior and grace period.

### P1 — List resourceVersion hardcoded to "1"

**Files**: List handlers (e.g., `handlers/pod.rs` line 527)

**Issue**: All list responses return `resourceVersion: "1"`. Should return the actual
storage revision so clients can resume watches from that point.

### P2 — Watch bookmarks disabled

**File**: `crates/api-server/src/handlers/watch.rs`

**Issue**: Watch bookmarks are hardcoded to disabled. Clients can't checkpoint
long-running watches.

### P2 — ManagedFields never populated

**Files**: All create/update handlers

**Issue**: `metadata.managedFields` is never set. This tracks field ownership for
server-side apply. kubectl apply relies on this.

---

## 2. Kubelet Behavioral Gaps

### P0 — Container lifecycle hooks (postStart/preStop) not executed

**File**: `crates/kubelet/src/runtime.rs`

**Issue**: `Container.lifecycle` field exists but is completely ignored. Neither
postStart nor preStop hooks are executed. Many conformance tests verify these.

**Fix**: After container creation, execute postStart handler (exec/httpGet/tcpSocket).
Before container stop, execute preStop handler and wait for completion within
gracePeriodSeconds.

### P0 — Startup probes not checked

**File**: `crates/kubelet/src/runtime.rs`

**Issue**: Startup probes are never evaluated. Without startup probes passing,
liveness and readiness probes should not run. Containers with slow startup will
be killed by liveness probes before they're ready.

**Fix**: Check startup probe first. Only start liveness/readiness probes after
startup probe succeeds. Set `ContainerStatus.started = true` when startup probe passes.

### P0 — Resource limits not enforced on containers

**File**: `crates/kubelet/src/runtime.rs`

**Issue**: CPU/memory limits from `container.resources.limits` are never passed to
Docker/Podman HostConfig. Containers can consume unlimited resources.

**Fix**: Set `host_config.memory`, `host_config.cpu_quota`, `host_config.cpu_period`
from resource limits when creating containers.

### P0 — Service account token auto-mounting not called

**File**: `crates/kubelet/src/runtime.rs`

**Issue**: `create_serviceaccount_token_volume()` exists but is never called in
`start_pod()`. Service account tokens are not mounted. Pods cannot authenticate
to the API server.

**Fix**: Call `create_serviceaccount_token_volume()` during pod startup if
`automountServiceAccountToken` is not false.

### P1 — terminationGracePeriodSeconds ignored

**File**: `crates/kubelet/src/runtime.rs`

**Issue**: Pod spec `terminationGracePeriodSeconds` is ignored. A hardcoded 10-second
timeout is used instead.

**Fix**: Read `spec.terminationGracePeriodSeconds` (default 30) and use it as the
Docker stop timeout.

### P1 — Probe failure/success thresholds not implemented

**File**: `crates/kubelet/src/runtime.rs`

**Issue**: Probes treat a single failure as terminal. Kubernetes requires
`failureThreshold` consecutive failures (default 3) before marking a probe failed,
and `successThreshold` consecutive successes (default 1) before marking it passed.

### P1 — Pod start_time never set

**File**: `crates/kubelet/src/kubelet.rs`

**Issue**: `PodStatus.startTime` is always None. Should be set when kubelet starts
running the pod.

### P1 — Init container statuses never populated

**File**: `crates/kubelet/src/kubelet.rs`

**Issue**: `pod.status.initContainerStatuses` is never set. Clients can't determine
which init containers have completed.

### P1 — Service environment variables not injected

**File**: `crates/kubelet/src/runtime.rs`

**Issue**: When `enableServiceLinks` is true (default), Kubernetes injects
`{SVC_NAME}_SERVICE_HOST` and `{SVC_NAME}_SERVICE_PORT` env vars for every Service
in the same namespace. This is completely missing.

### P1 — DNS policy and custom DNS config ignored

**File**: `crates/kubelet/src/runtime.rs`

**Issue**: `pod.spec.dnsPolicy` and `pod.spec.dnsConfig` fields are ignored. Different
policies (ClusterFirst, Default, None) should produce different resolv.conf content.

### P1 — Host aliases not added to /etc/hosts

**File**: `crates/kubelet/src/runtime.rs`

**Issue**: `pod.spec.hostAliases` is never applied to the generated /etc/hosts file.

### P2 — Container restart count not properly tracked

**File**: `crates/kubelet/src/runtime.rs`

**Issue**: Restart count reads from existing pod status but is never incremented
when containers are actually restarted.

### P2 — QoS class never computed or set

**File**: `crates/kubelet/src/kubelet.rs`

**Issue**: `PodStatus.qosClass` is never set. Should be computed from resource
requests/limits: Guaranteed, Burstable, or BestEffort.

---

## 3. Controller-Manager Behavioral Gaps

### P0 — Deployment controller missing rolling update strategy

**File**: `crates/controller-manager/src/controllers/deployment.rs`

**Issue**: Rolling updates don't respect `maxSurge` and `maxUnavailable`. Deployments
scale directly to desired replicas instantly instead of gradually rolling.

### P0 — Deployment controller doesn't set status conditions

**File**: `crates/controller-manager/src/controllers/deployment.rs`

**Issue**: Deployment status conditions (Available, Progressing) are always None.
Conformance tests check these conditions.

### P0 — observedGeneration not tracked by any controller

**Files**: All controllers in `crates/controller-manager/src/controllers/`

**Issue**: No controller sets `status.observedGeneration` to match `metadata.generation`.
Controllers should set this after reconciling to signal they've processed the latest spec.

### P1 — Pod readiness check uses phase instead of Ready condition

**Files**: `replicaset.rs`, `endpoints.rs`

**Issue**: Controllers check `pod.status.phase == Running` instead of checking
`pod.status.conditions` for `type: Ready, status: True`. A running pod may not be ready.

### P1 — StatefulSet doesn't manage PVCs from volumeClaimTemplates

**File**: `crates/controller-manager/src/controllers/statefulset.rs`

**Issue**: StatefulSets should create PVCs from `spec.volumeClaimTemplates` for each
replica. Currently ignored.

### P1 — DaemonSet doesn't match taint tolerations

**File**: `crates/controller-manager/src/controllers/daemonset.rs`

**Issue**: DaemonSet creates pods on all matching nodes without checking if the pod
tolerates the node's taints.

### P1 — Node controller doesn't respect PodDisruptionBudgets during eviction

**File**: `crates/controller-manager/src/controllers/node.rs`

**Issue**: When evicting pods from a NotReady node, PDBs are not consulted.

### P2 — Job doesn't track start/completion times

**File**: `crates/controller-manager/src/controllers/job.rs`

**Issue**: `JobStatus.startTime` and `completionTime` are never set.

---

## 4. Discovery / OpenAPI Gaps

### P0 — OpenAPI endpoints not registered in router

**Files**: `crates/api-server/src/router.rs`, `handlers/openapi.rs`

**Issue**: Routes for `/openapi/v3`, `/openapi/v2`, `/swagger.json` exist as handlers
but are NOT wired into the router. kubectl and clients need these for schema validation.

**Fix**: Add routes to public_routes in router.rs.

### P1 — Missing /livez endpoint

**File**: `crates/api-server/src/router.rs`

**Issue**: Only `/healthz` and `/readyz` are registered. `/livez` is missing. This is
the standard liveness probe endpoint.

### P1 — Discovery missing deletecollection verb

**File**: `crates/api-server/src/handlers/discovery.rs`

**Issue**: Most APIResource definitions omit `deletecollection` from the verbs list,
even though the handlers support it. kubectl uses discovery to determine if
`kubectl delete --all` is supported.

### P2 — Missing /apis/{group} intermediate discovery endpoint

**File**: `crates/api-server/src/handlers/discovery.rs`

**Issue**: No endpoint for `/apis/apps`, `/apis/batch`, etc. (individual API group
details). Some clients query these to discover preferred versions.

---

## Implementation Priority

### Must-fix for conformance (do first):

1. DELETE handlers return deleted object (API server)
2. generation field incremented on spec changes (API server)
3. resourceVersion conflict detection (API server)
4. OpenAPI endpoints wired in router (API server)
5. Container lifecycle hooks (postStart/preStop) (kubelet)
6. Startup probe support (kubelet)
7. Service account token auto-mounting (kubelet)
8. Resource limits enforcement (kubelet)
9. Deployment rolling update strategy (controller-manager)
10. Deployment status conditions (controller-manager)
11. observedGeneration tracking (all controllers)

### High-impact fixes:

12. Field validation (required fields) (API server)
13. Default value setting (API server)
14. terminationGracePeriodSeconds (kubelet)
15. Probe thresholds (kubelet)
16. Pod start_time (kubelet)
17. Service env vars (kubelet)
18. Pod readiness check (controller-manager)
19. StatefulSet PVC management (controller-manager)
20. /livez endpoint (API server)
21. Discovery deletecollection verb (API server)

### Medium-priority:

22. Watch bookmarks (API server)
23. List resourceVersion (API server)
24. DNS policy/config (kubelet)
25. Host aliases (kubelet)
26. DaemonSet taint tolerations (controller-manager)
27. Job timestamps (controller-manager)
28. PDB respect during eviction (controller-manager)
