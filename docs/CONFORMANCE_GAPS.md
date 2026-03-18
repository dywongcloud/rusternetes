# Kubernetes 1.35 Conformance Behavioral Gaps

This document captures architectural and behavioral issues that would cause conformance
test failures, discovered through a comprehensive audit of API server, kubelet,
controller-manager, and discovery/OpenAPI implementations.

**Last updated**: 2026-03-18 (all P0/P1 fixes complete, most P2 complete)

---

## Priority Legend

- **P0** - Will cause widespread conformance test failures
- **P1** - Will cause specific conformance test failures
- **P2** - May cause edge-case failures or affect kubectl UX
- **P3** - Nice to have, unlikely to affect conformance directly

---

## 1. API Server Behavioral Gaps

### ✅ P0 — DELETE handlers return deleted object — DONE

All delete handlers return `Json<T>` with the deleted resource.

### ✅ P0 — metadata.generation incremented on spec changes — DONE

New `lifecycle.rs` module: `set_initial_generation()` on create, `maybe_increment_generation()`
on update (compares spec, excludes metadata/status). Applied to pod, deployment, service handlers.

### ✅ P0 — resourceVersion conflict detection — DONE

`check_resource_version()` in `lifecycle.rs`. Applied to pod, deployment, service update handlers.

### ✅ P0 — OpenAPI endpoints wired in router — DONE

Routes for `/openapi/v2`, `/openapi/v3`, `/swagger.json` now in public_routes.

### ✅ P1 — Validation of required fields — DONE

Pod create validates: >= 1 container, each container must have image and name.

### ✅ P1 — Default values set on resources — DONE

Pod create sets defaults: restartPolicy=Always, dnsPolicy=ClusterFirst,
terminationMessagePath, terminationMessagePolicy, imagePullPolicy based on tag.

### ✅ P1 — /livez endpoint — DONE

`/livez` endpoint added, reusing healthz handler.

### ✅ P1 — Discovery deletecollection verb — DONE

Added `deletecollection` to verbs for pods, services, configmaps, secrets,
serviceaccounts, PVCs, endpoints, events, deployments, replicasets, statefulsets, daemonsets.

### ✅ P2 — Watch bookmarks — DONE

Re-enabled: clients requesting allowWatchBookmarks now receive bookmark events.

### ✅ P2 — List resourceVersion — DONE

Uses timestamp-based resource version instead of hardcoded "1".

### Remaining API Server gaps (P2):

| Item | Priority |
|------|----------|
| DELETE gracePeriodSeconds/propagationPolicy parsing | P2 |
| ManagedFields tracking for server-side apply | P2 |
| /apis/{group} intermediate discovery endpoints | P2 |

---

## 2. Kubelet Behavioral Gaps

### ✅ P0 — Container lifecycle hooks (postStart/preStop) — DONE

Implements exec and httpGet handlers for postStart (after container start)
and preStop (before container stop) with grace period awareness.

### ✅ P0 — Startup probes — DONE

Startup probes gate liveness/readiness probes. ContainerStatus.started set
after startup probe passes.

### ✅ P0 — Resource limits enforcement — DONE

CPU/memory limits passed to Docker/Podman HostConfig (memory, cpu_period, cpu_quota).
Added `parse_memory_quantity()` and `parse_cpu_quantity()` helpers.

### ✅ P0 — Service account token auto-mounting — DONE

Handled by admission controller (`inject_service_account_token`).

### ✅ P1 — terminationGracePeriodSeconds — DONE

Reads `spec.terminationGracePeriodSeconds` (default 30s) instead of hardcoded 10s.

### ✅ P1 — Probe failure/success thresholds — DONE

Tracks consecutive failures/successes per container using failureThreshold (default 3)
and successThreshold (default 1).

### ✅ P1 — Pod start_time — DONE

Set when pod enters Running phase.

### ✅ P1 — Init container statuses — DONE

Kubelet builds init container statuses (Terminated/Completed) for running pods.

### ✅ P1 — Service environment variables — DONE

Injects {SVC}_SERVICE_HOST, {SVC}_SERVICE_PORT, {SVC}_PORT_* for all Services in namespace
when enableServiceLinks is true (default).

### ✅ P1 — DNS policy and custom DNS config — DONE

Implements ClusterFirst, ClusterFirstWithHostNet, Default, None policies.
Applies dnsConfig overrides (nameservers, searches, options).

### ✅ P1 — Host aliases — DONE

`pod.spec.hostAliases` entries added to generated /etc/hosts file.

### ✅ P2 — Container restart count — DONE

Reads from Docker/Podman inspect response instead of stale pod status.

### ✅ P2 — QoS class — DONE

Computed and set (Guaranteed/Burstable/BestEffort) from resource requests/limits.

---

## 3. Controller-Manager Behavioral Gaps

### ✅ P0 — Deployment rolling update strategy — DONE

Gradual scale-up/down per reconcile cycle, respecting maxSurge/maxUnavailable.
Supports both percentage ("25%") and absolute values.

### ✅ P0 — Deployment status conditions — DONE

Sets Available (MinimumReplicasAvailable/Unavailable) and Progressing conditions.

### ✅ P0 — observedGeneration tracking — DONE

All controllers (deployment, replicaset, statefulset, daemonset, job) set
`status.observedGeneration = metadata.generation` after reconciliation.

### ✅ P1 — Pod readiness check — DONE

replicaset.rs and endpoints.rs check `conditions[type=Ready].status == "True"`
instead of `phase == Running`.

### ✅ P1 — StatefulSet PVC management — DONE

Creates PVCs from volumeClaimTemplates for each replica ordinal with owner references.

### ✅ P1 — DaemonSet taint tolerations — DONE

Nodes with untolerated NoSchedule/NoExecute taints are skipped.

### ✅ P2 — Job start/completion times — DONE

start_time set when first pod starts, completion_time set on Complete/Failed.

### Remaining Controller-Manager gaps (P2):

| Item | Priority |
|------|----------|
| Node controller: PDB respect during eviction | P2 |

---

## 4. Discovery / OpenAPI Gaps

All P0/P1 discovery items are fixed. See API Server section above.

### Remaining:

| Item | Priority |
|------|----------|
| /apis/{group} intermediate discovery endpoints | P2 |

---

## Summary

### Completed: 29 of 32 items

| Component | P0 | P1 | P2 | Done |
|-----------|----|----|-----|------|
| API Server | 4/4 | 4/4 | 2/5 | 10/13 |
| Kubelet | 4/4 | 7/7 | 2/2 | 13/13 |
| Controller-Manager | 3/3 | 3/3 | 1/2 | 7/8 |
| Discovery/OpenAPI | (in API Server) | | | |
| **Total** | **11/11** | **14/14** | **5/9** | **29/32** (skipping 1 dup) |

### 3 Remaining P2 items:

1. DELETE gracePeriodSeconds/propagationPolicy (API server)
2. ManagedFields tracking (API server)
3. PDB respect during eviction (controller-manager)

All P0 and P1 conformance items are complete.
