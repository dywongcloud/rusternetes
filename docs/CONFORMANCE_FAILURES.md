# Conformance Failure Tracker

**Round 137** | Running | 2026-04-13
**Baseline**: Round 135 = 373/441 (84.6%), Round 136 = ABORTED

## Round 137 Failures (14 failures so far — deep analysis in progress)

### 1. CRD OpenAPI — 3 failures
- `crd_publish_openapi.go:400,318,285`
- **Root cause**: Schema includes `x-kubernetes-embedded-resource: false` and `x-kubernetes-int-or-string: false`. K8s omits false extension values (Go omitempty). Also watch context canceled during CRD schema polling.
- **Fix staged**: 3186cf5 strips false x-kubernetes extensions from OpenAPI schemas
- **Status**: Fix committed, needs redeploy

### 2. ReplicationController — 1 failure (CRITICAL BUG)
- `rc.go:509` — "Found 5/10/15/20+ pods out of 1"
- **Root cause**: RC controller creates 5+ pods per second when it should create 1. The `is_owned_by` check or adopt/release logic is failing — newly created pods are not being recognized as owned, causing the controller to create more pods every reconciliation cycle (1s interval). Grows to 45+ pods in 10 seconds.
- **K8s comparison needed**: K8s RC controller uses owner UID matching, not just name matching. Need deep comparison.
- **Status**: NEEDS FIX — fundamental controller bug

### 3. DNS — 1 failure
- `dns_common.go:476` — "rate: Wait(n=1) would exceed context deadline"
- **Root cause**: DNS resolution fails with rate limiter timeout. Likely kube-proxy routing issue — ClusterIP rules for CoreDNS may be getting flushed during test service creation.
- **Status**: INVESTIGATING

### 4. ReplicaSet — 1 failure
- `replica_set.go:232` — "checking pod responses: Timed out after 120s"
- **Root cause**: Pods running but not responding to HTTP requests. Related to pod-to-pod networking through kube-proxy. Watch context canceled errors present.
- **Status**: INVESTIGATING — likely networking issue

### 5. EmptyDir permissions — 1 failure (DinD limitation)
- `output.go:263` — test "(root,0666,default)"
- **Root cause**: macOS filesystem doesn't support full Unix permissions through Docker Desktop bind mounts. agnhost mounttest calls `umask(0)` (mt.go:74-75) but macOS ignores chmod on bind-mounted volumes.
- **Status**: DinD/macOS limitation

### 6. Pod Resize — 1 failure (DinD limitation)
- `pod_resize.go:857`
- **Root cause**: In-place pod resource resize requires cgroup manipulation not available in DinD.
- **Status**: DinD limitation

### 7. Init Container — 1 failure
- `init_container.go:440` — "timed out waiting for the condition"
- **Root cause**: Watch context canceled during init container readiness wait. The init container state machine fix may not be working correctly, or watch failures prevent observing the state transition.
- **Status**: INVESTIGATING

### 8. Deployment — 1 failure
- `deployment.go:1264` — "replicaset never had desired number of .spec.replicas"
- **Root cause**: Watch context canceled. ReplicaSet replicas not converging — same root cause as RC over-creation? Or deployment controller proportional scaling issue.
- **Status**: INVESTIGATING

### 9. Service Proxy — 1 failure
- `proxy.go:271` — "Unable to reach service through proxy: context deadline exceeded"
- **Root cause**: Service proxy forwarding fails. The API server pod proxy handler may not be routing correctly through the ClusterIP, or kube-proxy rules are incomplete.
- **Status**: INVESTIGATING

### 10. Webhook — 2 failures
- `webhook.go:904,1269` — "waiting for webhook configuration to be ready: timed out"
- **Root cause**: Webhook service not becoming ready in time. The webhook deployment's pod needs to start and pass readiness probes. May be related to Secret volume mounting (sample-webhook-secret not found error observed in kubelet logs) or kube-proxy routing to webhook ClusterIP.
- **Status**: INVESTIGATING

### 11. Field Validation — 1 failure
- `field_validation.go:735` — "line 9: key "foo" already set in map"
- **Root cause**: Duplicate field validation error format. K8s expects a specific error message format for duplicate fields in strict validation mode. Our error says "key already set in map" but K8s may expect "duplicate field".
- **Status**: NEEDS FIX

### CRITICAL OBSERVATION: Watch Failures
**1777 "Watch failed: context canceled" errors** across the entire run. This is the #1 systemic issue affecting multiple tests. The HTTP/2 ALPN fix (069e807) did NOT resolve this. Root cause needs deeper investigation — could be:
- HTTP/2 stream multiplexing issues in hyper/axum
- Watch response body not flushing properly
- Bookmark keepalives not preventing client-go timeouts
- TLS/connection management between Go client-go and Rust rustls

## Staged for Round 138 (not yet deployed)

| Commit | Fix | K8s Ref |
|--------|-----|---------|
| fb9728d | Preemption — K8s "remove all, reprieve" victim selection | default_preemption.go:233-300 |
| fb9728d | Preemption — proper grace period (not forced 0) | preemption.go:177-219 |
| 3186cf5 | Strip false x-kubernetes extensions from OpenAPI schemas | openapi/v2/conversion.go |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 104 | 405 | 36 | 441 | 91.8% |
| 127 | 397 | 44 | 441 | 90.0% |
| 132 | 363 | 78 | 441 | 82.3% |
| 133 | 370 | 71 | 441 | 83.9% |
| 134 | 370 | 71 | 441 | 83.9% |
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | TBD | TBD | 441 | TBD |
