# Conformance Failure Tracker

**Round 137** | Running (25+ failures at ~200/441) | 2026-04-13

## Active Failures

### Watch — SYSTEMIC — FIX STAGED ✅
- 1777 "context canceled" errors affecting ~5 tests directly
- **Fix**: f1bf53f — Pre-buffer initial events before Response

### CRD OpenAPI — 4 failures — FIX STAGED ✅
- `crd_publish_openapi.go:400,318,285,161`
- **Fix**: 3186cf5 — Strip false x-kubernetes extensions

### ReplicationController — 1 failure — FIX STAGED ✅
- `rc.go:509` — creates 5+ pods/sec
- **Fix**: 070dde7 — UID ownership + active pod filtering

### Namespace — 1 failure — FIX STAGED ✅
- `namespace.go:579` — "namespace was deleted unexpectedly"
- **Root cause**: GC's cascade_delete_namespace force-deleted all resources ignoring finalizers, racing with namespace controller. K8s GC does NOT handle namespace cleanup — that's the NamespacedResourcesDeleter's job.
- **Fix**: 125d91a — Removed GC namespace cascade, namespace controller handles it

### Webhook — 7 failures — PARTIALLY DOWNSTREAM
- `webhook.go:675,904,1269,1400,1481,2107,2164`
- All "waiting for webhook configuration to be ready: timed out"
- **Root cause**: Webhook pod readiness probe (20s initial delay) + endpoint creation + kube-proxy sync. With GC fix, pods won't be force-deleted. Watch fix should help too. Remaining issue is kube-proxy timing.
- **Status**: Should improve with watch + GC fixes

### Field Validation — 2 failures — NEEDS FIX ❌
- `field_validation.go:611` — "Unknown field 'apiversion' at template" — CRD strict validation rejecting wrong fields when x-kubernetes-preserve-unknown-fields is set
- `field_validation.go:735` — duplicate field detection works but response body format may not match K8s expectation
- **Status**: NEEDS DEEP K8s COMPARISON

### DNS — 2 failures — DOWNSTREAM
- `dns_common.go:476` (x2) — rate limiter timeout, downstream of watch + kube-proxy

### Deployment — 1 failure — DOWNSTREAM
- `deployment.go:1264` — RS replicas timeout, downstream of watch

### ReplicaSet — 1 failure — DOWNSTREAM
- `replica_set.go:232` — pod responses timeout, downstream of watch + networking

### StatefulSet — 1 failure — DOWNSTREAM
- `statefulset.go:1092` — patch timing, fix staged (8673d37 generation + 4438743 counting)

### Init Container — 1 failure — DOWNSTREAM
- `init_container.go:440` — watch timeout

### Service Proxy — 1 failure — DOWNSTREAM
- `proxy.go:271` — service proxy timeout, downstream of kube-proxy

### Service Latency — 1 failure — DOWNSTREAM
- `service_latency.go:145` — deployment not ready, same as webhook timing

### EmptyDir — 1 failure — DinD
- `output.go:263` — macOS filesystem permissions

### Pod Resize — 1 failure — DinD
- `pod_resize.go:857` — cgroup limitation

## Staged Fixes (not yet deployed)

| Commit | Fix | Expected Impact |
|--------|-----|-----------------|
| f1bf53f | Watch pre-buffer initial events | ~5-8 failures (systemic) |
| 070dde7 | RC UID ownership + active pod filtering | 1 failure |
| 3186cf5 | Strip false x-kubernetes extensions | 3-4 failures |
| 125d91a | GC no longer cascade-deletes namespace resources | 1+ failures (namespace + webhook timing) |
| fb9728d | Preemption reprieve + grace period | preemption reliability |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | TBD | TBD | 441 | TBD |
