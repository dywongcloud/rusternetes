# Conformance Failure Tracker

**Round 137** | Running | 2026-04-13

## Active Failures (14 so far)

### 1. Watch "context canceled" — SYSTEMIC (~5 tests) — FIX STAGED ✅
- 1777 watch failures across entire run
- **Root cause**: Watch handler spawned background task to send events, returned Response with empty Body. Hyper blocked polling empty channel. Client-go timed out waiting for first DATA frame.
- **Fix**: f1bf53f — Pre-buffer initial events before returning Response
- K8s ref: watch.go:205-282

### 2. CRD OpenAPI — 3 failures — FIX STAGED ✅
- `crd_publish_openapi.go:400,318,285`
- **Root cause**: `x-kubernetes-embedded-resource: false` and `x-kubernetes-int-or-string: false` in schema. K8s omits false values.
- **Fix**: 3186cf5 — Strip false x-kubernetes extensions

### 3. ReplicationController — 1 failure — FIX STAGED ✅
- `rc.go:509` — creates 5+ pods/sec
- **Root cause**: current_replicas included terminating/terminal pods. K8s FilterActivePods excludes these.
- **Fix**: 070dde7 — UID ownership + active pod filtering

### 4. DNS — 1 failure — DOWNSTREAM OF #1
- `dns_common.go:476` — rate limiter timeout
- Watch failures exhaust rate limiter. Should improve with watch fix.

### 5. ReplicaSet — 1 failure — DOWNSTREAM OF #1
- `replica_set.go:232` — pod responses timeout
- Watch context canceled + networking. Should improve with watch fix.

### 6. EmptyDir permissions — 1 failure — DinD LIMITATION
- `output.go:263` — macOS filesystem ignores chmod

### 7. Pod Resize — 1 failure — DinD LIMITATION
- `pod_resize.go:857` — cgroup manipulation unavailable

### 8. Init Container — 1 failure — DOWNSTREAM OF #1
- `init_container.go:440` — watch failures prevent observing state transition

### 9. Deployment — 1 failure — DOWNSTREAM OF #1
- `deployment.go:1264` — RS never had desired replicas due to watch failures

### 10. Service Proxy — 1 failure — NEEDS FIX ❌
- `proxy.go:271` — context deadline exceeded reaching service through proxy
- **Root cause**: INVESTIGATING — API server proxy handler or kube-proxy routing

### 11. Webhook — 2 failures — NEEDS FIX ❌
- `webhook.go:904,1269` — webhook configuration not ready
- **Root cause**: INVESTIGATING — Secret mounting timing + kube-proxy routing to webhook ClusterIP

### 12. Field Validation — 1 failure — NEEDS FIX ❌
- `field_validation.go:735` — duplicate field error not in response body
- **Root cause**: YAML with duplicate keys sent via apply-patch+yaml with fieldValidation=Strict. We don't detect duplicate YAML keys and return error in HTTP response body as K8s Status object.
- **Status**: INVESTIGATING

## Staged Fixes (not yet deployed)

| Commit | Fix | Expected Impact |
|--------|-----|-----------------|
| f1bf53f | Watch pre-buffer initial events | ~5-8 failures (systemic) |
| 070dde7 | RC UID ownership + active pod filtering | 1 failure |
| 3186cf5 | Strip false x-kubernetes extensions | 3 failures |
| fb9728d | Preemption reprieve + grace period | preemption reliability |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | TBD | TBD | 441 | TBD |
