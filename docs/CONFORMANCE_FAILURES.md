# Conformance Failure Tracker

**Round 137** | Running | 2026-04-13

## Active Failures (14 so far)

### 1. Watch "context canceled" — SYSTEMIC (affects ~5 tests directly)
- 1777 watch failures across entire run
- Affects: CRD OpenAPI (3), deployment (1), init container (1), proxy (1)
- **Root cause**: HTTP/2 ALPN fix did NOT resolve. Need deep investigation of Axum/hyper HTTP/2 streaming, watch response flushing, and client-go compatibility.
- **Status**: INVESTIGATING (background research agent running)

### 2. CRD OpenAPI — 3 failures
- `crd_publish_openapi.go:400,318,285`
- **Root cause**: `x-kubernetes-embedded-resource: false` and `x-kubernetes-int-or-string: false` included in OpenAPI schema. K8s omits them (Go omitempty).
- **Fix staged**: 3186cf5

### 3. ReplicationController — 1 failure (CRITICAL)
- `rc.go:509` — creates 5+ pods/sec instead of 1
- **Root cause**: `current_replicas` count includes terminating and terminal pods. K8s `FilterActivePods` excludes these. RC thinks it has 0 active pods each cycle and keeps creating.
- **Fix staged**: UID-based ownership + active pod filtering (matching K8s FilterActivePods)

### 4. DNS — 1 failure
- `dns_common.go:476` — rate limiter timeout reaching DNS
- **Root cause**: Likely downstream of watch failures causing rate limiter exhaustion

### 5. ReplicaSet — 1 failure
- `replica_set.go:232` — pod responses timeout
- **Root cause**: Watch context canceled + networking issues

### 6. EmptyDir permissions — 1 failure (DinD)
- `output.go:263` — macOS filesystem ignores chmod through Docker bind mounts

### 7. Pod Resize — 1 failure (DinD)
- `pod_resize.go:857` — requires cgroup manipulation unavailable in DinD

### 8. Init Container — 1 failure
- `init_container.go:440` — timed out. Watch failures prevent observing state transition.

### 9. Deployment — 1 failure
- `deployment.go:1264` — RS never had desired replicas. Watch failures.

### 10. Service Proxy — 1 failure
- `proxy.go:271` — context deadline exceeded reaching service through proxy

### 11. Webhook — 2 failures
- `webhook.go:904,1269` — webhook configuration not ready. Secret mounting timing + kube-proxy routing.

### 12. Field Validation — 1 failure
- `field_validation.go:735` — "error missing duplicate field"
- **Root cause**: Response body doesn't contain the expected error string. Our validation error is in the Go error but not in the raw HTTP response body as a K8s Status object.
- **Status**: NEEDS FIX

## Staged Fixes (not yet deployed)

| Commit | Fix |
|--------|-----|
| f1bf53f | **Watch pre-buffer — send initial events before Response** (fixes 1777 context canceled errors) |
| fb9728d | Preemption reprieve algorithm + proper grace period |
| 3186cf5 | Strip false x-kubernetes extensions from OpenAPI schemas |
| 070dde7 | RC active pod filtering + UID ownership |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | TBD | TBD | 441 | TBD |
