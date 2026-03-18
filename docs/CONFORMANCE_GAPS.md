# Kubernetes 1.35 Conformance Behavioral Gaps

This document captures architectural and behavioral issues that will cause conformance
test failures, discovered through a comprehensive audit of API server, kubelet,
controller-manager, and discovery/OpenAPI implementations.

**Last updated**: 2026-03-18 (Phase 1 fixes complete)

---

## Priority Legend

- **P0** - Will cause widespread conformance test failures
- **P1** - Will cause specific conformance test failures
- **P2** - May cause edge-case failures or affect kubectl UX
- **P3** - Nice to have, unlikely to affect conformance directly

---

## 1. API Server Behavioral Gaps

### ✅ P0 — DELETE handlers don't return the deleted object — ALREADY FIXED

All delete handlers already return `Json<T>` with the deleted resource.

### ✅ P0 — metadata.generation never incremented on spec changes — FIXED

New `lifecycle.rs` module: `set_initial_generation()` on create, `maybe_increment_generation()`
on update (compares spec, excludes metadata/status). Applied to pod, deployment, service handlers.

### ✅ P0 — resourceVersion conflict detection not enforced — FIXED

`check_resource_version()` in `lifecycle.rs`. Applied to pod, deployment, service update handlers.

### ✅ P1 — No validation of required fields — FIXED

Pod create validates: >= 1 container, each container must have image and name.

### ✅ P1 — Default values not set on resources — FIXED

Pod create sets defaults: restartPolicy=Always, dnsPolicy=ClusterFirst,
terminationMessagePath, terminationMessagePolicy, imagePullPolicy based on tag.

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

### P0 — Startup probes not checked

**File**: `crates/kubelet/src/runtime.rs`

**Issue**: Startup probes are never evaluated. Without startup probes passing,
liveness and readiness probes should not run.

### ✅ P0 — Resource limits not enforced on containers — FIXED

CPU/memory limits now passed to Docker/Podman HostConfig (memory, cpu_period, cpu_quota).
Added `parse_memory_quantity()` and `parse_cpu_quantity()` helpers.

### ✅ P0 — Service account token auto-mounting — ALREADY HANDLED

Confirmed handled by admission controller (`inject_service_account_token`).

### ✅ P1 — terminationGracePeriodSeconds — FIXED

Now reads `spec.terminationGracePeriodSeconds` (default 30s) instead of hardcoded 10s.

### P1 — Probe failure/success thresholds not implemented

Probes treat a single failure as terminal. Should require `failureThreshold` consecutive failures.

### ✅ P1 — Pod start_time — FIXED

Now set when pod enters Running phase.

### ✅ P1 — Init container statuses — FIXED

Kubelet now builds init container statuses (Terminated/Completed) for running pods.

### P1 — Service environment variables not injected

When `enableServiceLinks` is true (default), `{SVC_NAME}_SERVICE_HOST` and
`{SVC_NAME}_SERVICE_PORT` should be injected for every Service in the namespace.

### P1 — DNS policy and custom DNS config ignored

`pod.spec.dnsPolicy` and `pod.spec.dnsConfig` fields are ignored.

### ✅ P1 — Host aliases — FIXED

`pod.spec.hostAliases` entries now added to generated /etc/hosts file.

### P2 — Container restart count not properly tracked

Restart count reads from existing pod status but is never incremented on actual restarts.

### ✅ P2 — QoS class — FIXED

Now computed and set (Guaranteed/Burstable/BestEffort) from resource requests/limits.

---

## 3. Controller-Manager Behavioral Gaps

### P0 — Deployment controller missing rolling update strategy

**File**: `crates/controller-manager/src/controllers/deployment.rs`

**Issue**: Rolling updates don't respect `maxSurge` and `maxUnavailable`. Deployments
scale directly to desired replicas instantly instead of gradually rolling.

### ✅ P0 — Deployment controller doesn't set status conditions — FIXED

Now sets Available (MinimumReplicasAvailable/Unavailable) and Progressing conditions.

### ✅ P0 — observedGeneration not tracked by any controller — FIXED

All controllers (deployment, replicaset, statefulset, daemonset, job) now set
`status.observedGeneration = metadata.generation` after reconciliation.

### ✅ P1 — Pod readiness check uses phase instead of Ready condition — FIXED

replicaset.rs and endpoints.rs now check `conditions[type=Ready].status == "True"`
instead of `phase == Running`.

### P1 — StatefulSet doesn't manage PVCs from volumeClaimTemplates

StatefulSets should create PVCs from `spec.volumeClaimTemplates` for each replica.

### P1 — DaemonSet doesn't match taint tolerations

DaemonSet creates pods on all matching nodes without checking taints.

### P1 — Node controller doesn't respect PodDisruptionBudgets during eviction

When evicting pods from a NotReady node, PDBs are not consulted.

### P2 — Job doesn't track start/completion times

`JobStatus.startTime` and `completionTime` are never set.

---

## 4. Discovery / OpenAPI Gaps

### ✅ P0 — OpenAPI endpoints not registered in router — FIXED

Routes for `/openapi/v2`, `/openapi/v3`, `/swagger.json` now wired into public_routes.

### ✅ P1 — Missing /livez endpoint — FIXED

`/livez` endpoint added, reusing healthz handler.

### ✅ P1 — Discovery missing deletecollection verb — FIXED

Added `deletecollection` to verbs for pods, services, configmaps, secrets,
serviceaccounts, PVCs, endpoints, events, deployments, replicasets, statefulsets, daemonsets.

### P2 — Missing /apis/{group} intermediate discovery endpoint

No endpoint for `/apis/apps`, `/apis/batch`, etc. (individual API group details).

---

## Implementation Priority

### ✅ COMPLETED (Phase 1):

1. ✅ DELETE handlers return deleted object (already done)
2. ✅ generation field incremented on spec changes
3. ✅ resourceVersion conflict detection
4. ✅ OpenAPI endpoints wired in router
5. ✅ Service account token auto-mounting (handled by admission)
6. ✅ Resource limits enforcement
7. ✅ Deployment status conditions
8. ✅ observedGeneration tracking (all controllers)
9. ✅ Field validation (required fields)
10. ✅ Default value setting
11. ✅ terminationGracePeriodSeconds
12. ✅ Pod start_time
13. ✅ Pod readiness check (condition-based)
14. ✅ /livez endpoint
15. ✅ Discovery deletecollection verb
16. ✅ Host aliases
17. ✅ QoS class
18. ✅ Init container statuses

### Remaining P0 (do next):

19. Container lifecycle hooks (postStart/preStop) (kubelet)
20. Startup probe support (kubelet)
21. Deployment rolling update strategy (controller-manager)

### Remaining P1:

22. Probe failure/success thresholds (kubelet)
23. Service env vars (kubelet)
24. DNS policy/config (kubelet)
25. StatefulSet PVC management (controller-manager)
26. DaemonSet taint tolerations (controller-manager)

### Remaining P2:

27. Watch bookmarks (API server)
28. List resourceVersion (API server)
29. Container restart count tracking (kubelet)
30. Job timestamps (controller-manager)
31. PDB respect during eviction (controller-manager)
32. /apis/{group} discovery endpoints (API server)
