# Conformance Failure Tracker

**Round 125** | 329/441 (74.6%) | 112 failures | 2026-04-04
**Round 127** | In progress (~34 failures so far) | 2026-04-07

## Round 127 Conformance Failures

### 1. CRD Discovery Endpoint Missing (7 failures) — FIXING
- `crd_publish_openapi.go:202,244,285,451` — failed to create CRD: context deadline exceeded
- `custom_resource_definition.go:161` — creating/cannot create CRD: context deadline exceeded
- `aggregated_discovery.go:227` — context deadline exceeded
- **Root cause**: `/apis/{group}/{version}` returns 404 for CRD groups. Go test creates CRD then polls `waitForDiscoveryResource` which checks this endpoint for an `APIResourceList`. Endpoint was missing from `custom_resource_fallback`.
- **Fix**: Added handler for `/apis/{group}/{version}` that returns `APIResourceList` by looking up CRDs. Also improved `/apis/{group}` to use actual CRD versions.
- **Status**: FIX IN PROGRESS — building

### 2. OpenAPI/Protobuf Download Failure (3 failures)
- `kubectl/builder.go:97` (x3, incl BeforeEach/AfterEach) — `failed to download openapi: proto: cannot parse invalid wire-format data`
- **Root cause**: kubectl tries to download OpenAPI v3 spec for validation and gets data it can't parse as protobuf.
- **Status**: TODO

### 3. DNS Resolution Failures (2 failures)
- `dns_common.go:476` (x2) — Unable to read agnhost_udp@... context deadline exceeded
- **Root cause**: DNS lookups fail — pods can't resolve service names via DNS.
- **Status**: TODO

### 4. StatefulSet Issues (3 failures)
- `statefulset.go:2479` — scaled unexpectedly 3 -> 2 replicas
- `statefulset.go:957` — Pod ss-0 expected to be re-created at least once
- `statefulset.go:454` — Pod ss2-0 has wrong image after rolling update
- **Status**: TODO

### 5. Scheduling/Preemption (2 failures)
- `preemption.go:181` — Timed out after 300s
- `preemption.go:1025` — RS never had desired .status.availableReplicas
- **Status**: TODO

### 6. DaemonSet (1 failure)
- `daemon_set.go:1276` — Expected 0 to equal 1
- **Status**: TODO

### 7. Pod Exec WebSocket (1 failure)
- `pods.go:600` — Got message from server that didn't start with channel 1 (STDOUT): sends channel 3 status before STDOUT
- **Status**: TODO

### 8. Service Endpoint Reachability (1 failure)
- `service.go:768` — service not reachable within 2m0s timeout
- **Status**: TODO

### 9. Init Container (1 failure)
- `init_container.go:565` — init1 should be complete but reported incomplete
- **Status**: TODO

### 10. Runtime/Termination Message (1 failure)
- `runtime.go:169` — Expected "DONE" to equal "" (termination message set on success)
- **Status**: TODO

### 11. Field Validation (1 failure)
- `field_validation.go:105` — strict decoding error format wrong
- **Status**: TODO

### 12. Proxy (1 failure)
- `proxy.go:271` — Unable to reach service through proxy
- **Status**: TODO

### 13. Deployment Rollover (1 failure)
- `deployment.go:995` — total pods available: 0
- **Status**: TODO

### 14. Webhook (2 failures)
- `webhook.go:2465` — Webhook request failed: error sending request
- `webhook.go` — timed out waiting for webhook config to be ready
- **Status**: TODO

### 15. Job (1 failure)
- `job.go:595` — Expected nil to equal 0 (job.status.ready)
- **Status**: TODO

### 16. Service Account (1 failure)
- `service_accounts.go:817` — timed out waiting for the condition
- **Status**: TODO

### 17. Pod Hostname (1 failure)
- Gave up waiting 2m0s for 1 pods to come up
- **Status**: TODO

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
| 127 | TBD | TBD | 441 | TBD |
