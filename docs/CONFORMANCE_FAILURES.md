# Conformance Failure Tracker

**Round 125** | 329/441 (74.6%) | 112 failures | 2026-04-04
**Round 127** | ~397/441 (90.0%) | 44 failures | 2026-04-08

NOTE: Round 127 ran on binaries built BEFORE commits eaba1ef, 3a927d1, 2d3c799.
Several fixes committed before round 127 were NOT included in the running binary.

## Round 127 Conformance Failures (44 total)

### 1. JSON Watch Handlers — return on error/stream-end (systemic) — FIXED
- `watch_cluster_scoped_json` and `watch_namespaced_json` had 3 bugs:
  - `Some(Err(e))` → returned immediately instead of continuing (kills watch on transient error)
  - `None` → returned instead of reconnecting from watch cache
  - Bookmark interval was 15s instead of 5s (typed handlers use 5s)
- These handlers serve CRD watches, generic/custom resource watches, DRA resources
- 1841 occurrences of `Watch failed: context canceled` in e2e log (many downstream of this)
- **Fix**: Changed error handling to continue on transient errors, reconnect from cache on stream end, unified bookmark interval to 5s
- **Status**: FIXED (pending build verification)

### 2. CRD Discovery/Creation Failures (10 failures) — downstream of #1
- `crd_publish_openapi.go:161,202,244,285,318,451` — failed to create CRD: context deadline exceeded
- `custom_resource_definition.go:104,161` — creating CRD: context deadline exceeded
- `field_validation.go:570` — cannot create crd context deadline exceeded
- `aggregated_discovery.go:282` — context deadline exceeded (CRD test)
- **Root cause**: CRD watches use `watch_cluster_scoped_json` which was broken (fix in #1)
- **Status**: Expected to be fixed by #1

### 3. Aggregated Discovery — Accept header q-value tiebreaking (2 failures) — FIXED
- `aggregated_discovery.go:227` — Expected admissionregistration.k8s.io/v1, Resource=validatingwebhookconfigurations to be present
- **Root cause**: Go discovery client sends Accept header with aggregated types FIRST and plain JSON LAST, all with implicit q=1.0. Our `wants_aggregated_discovery` used `>` comparison, returning false when q-values are equal.
- **Fix**: Use position-based tiebreaking — when q-values are equal, the type listed first wins (HTTP convention). Aggregated types first → aggregated response; plain JSON first (sonobuoy) → legacy response.
- **Status**: FIXED (pending build verification)

### 4. OpenAPI/Protobuf Download Failure (3 failures)
- `kubectl/builder.go:97` (x3) — `failed to download openapi` / error running kubectl create
- **Root cause**: kubectl's client-go calls OpenAPISchema() which requests protobuf Accept. Previous fix returned minimal protobuf, but still failing.
- **Status**: TODO

### 5. DNS Resolution Failures (4 failures)
- `dns_common.go:476` (x4) — Unable to read agnhost_udp@... context deadline exceeded
- **Root cause**: DNS lookups fail — pods can't resolve service names via DNS
- **Status**: TODO

### 6. StatefulSet Issues (3 failures)
- `statefulset.go:2479` — scaled unexpectedly 3 -> 2 replicas
- `statefulset.go:957` — Pod ss-0 expected to be re-created at least once
- `statefulset.go:454` — Pod ss2-0 has wrong image after rolling update
- **Status**: TODO

### 7. Scheduling/Preemption (3 failures)
- `preemption.go:181` — Timed out after 300s
- `preemption.go:516` — 0/2 nodes available: no node matched scheduling constraints
- `preemption.go:1025` — RS never had desired .status.availableReplicas
- **Status**: TODO

### 8. Webhook (3 failures)
- `webhook.go:904` — timed out waiting for webhook config to be ready
- `webhook.go:1269` — timed out waiting for webhook config to be ready
- `webhook.go:2465` — Webhook request failed: error sending request
- **Status**: TODO

### 9. DaemonSet (1 failure)
- `daemon_set.go:1276` — Expected 0 to equal 1
- **Status**: TODO

### 10. Pod Exec WebSocket (1 failure)
- `pods.go:600` — Got message from server that didn't start with channel 1 (STDOUT): sends channel 3 (`{"status":"Success"}`) before STDOUT data
- **Root cause**: Status on channel 3 arrives before stdout despite ordering logic in streaming.rs
- **Status**: TODO

### 11. Service Endpoint Reachability (2 failures)
- `service.go:768` — service not reachable within 2m0s timeout
- `service.go:870` — extra port mappings on slices, context deadline exceeded
- **Status**: TODO

### 12. Init Container (1 failure)
- `init_container.go:565` — expects `containers with incomplete status: [init2]` but got `[init1 init2]`
- **Root cause**: Kubelet isn't processing init containers fast enough — init1 should have completed but hasn't
- **Status**: TODO (kubelet issue)

### 13. Runtime/Container Restart Count (1 failure)
- `runtime.go:115` — Expected container restart count 0 to equal 2
- **Root cause**: Kubelet not tracking/reporting container restarts properly
- **Status**: TODO (kubelet issue)

### 14. Runtime/Termination Message (1 failure) — ALREADY FIXED (not deployed)
- `runtime.go:169` — Expected "DONE" to equal "" (termination message set on success)
- **Fix**: Commit 3a927d1 — only fallback to logs on non-zero exit code
- **Status**: FIXED (not in round 127 binary, will be in next build)

### 15. Field Validation (1 failure) — ALREADY FIXED (not deployed)
- `field_validation.go:105` — duplicate fields reported as `json: unknown field` instead of `duplicate field`
- **Fix**: Commit eaba1ef — changed format to `duplicate field "..."`
- **Status**: FIXED (not in round 127 binary, will be in next build)

### 16. Proxy (2 failures)
- `proxy.go:271` — Unable to reach service through proxy: context deadline exceeded
- `proxy.go:503` — Pod didn't start within timeout
- **Status**: TODO

### 17. Service Latency — Protobuf Deployment Create (1 failure)
- `service_latency.go:142` — failed to decode: missing field `template` at line 1 column 493
- **Root cause**: Client-go sends Deployment CREATE as native protobuf. Our middleware can't decode native protobuf for standard K8s types — brace-scanning the binary produces incomplete JSON missing the template field.
- **Status**: TODO — architectural issue with protobuf handling

### 18. Deployment Rollover (1 failure)
- `deployment.go:995` — total pods available: 0
- **Status**: TODO

### 19. Service Account (2 failures)
- `service_accounts.go:667` — tls: failed to verify certificate: x509: certificate signed by unknown authority
- `service_accounts.go:817` — timed out waiting for the condition
- **Status**: TODO

### 20. Pod Hostname / RC (1 failure)
- `rc.go:509` — Gave up waiting 2m0s for 1 pods to come up (Watch failed: context canceled)
- Found 92 pods matching name when expecting 1 — RC creating too many replicas
- **Status**: TODO

### 21. Ephemeral Containers — generation not incremented (1 failure) — FIXED
- `ephemeral_containers.go:138` — Expected pod generation 2, got 1
- **Root cause**: Pod PATCH handler did not call `maybe_increment_generation` when spec changed via patch (only UPDATE handler did)
- **Fix**: Added generation increment logic to pod patch handler
- **Status**: FIXED (pending build verification)

### 22. Job Ready Field (1 failure) — ALREADY FIXED (not deployed)
- `job.go:595` — Expected nil to equal 0 (job.status.ready)
- **Fix**: Commit 2d3c799 — set ready: Some(count) instead of None
- **Status**: FIXED (not in round 127 binary, will be in next build)

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 103 | 245 | 196 | 441 | 55.6% |
| 104 | 405 | 36 | 441 | 91.8% |
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | 310 | 131 | 441 | 70.3% |
| 124 | 295 | 146 | 441 | 66.9% |
| 125 | 329 | 112 | 441 | 74.6% |
| 127 | 397 | 44 | 441 | 90.0% |
