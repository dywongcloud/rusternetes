# Conformance Failure Tracker

**Round 135** | 373/441 (84.6%) | 2026-04-11
**Round 134** | 370/441 (83.9%) | 2026-04-10

## Round 135 Results — 68 failures (3 fewer than round 134)

### Tests fixed (3 new passes)
- Init container tests (d9c9d34)
- 1 other test

### Webhook Service Readiness — 12 failures (HIGHEST PRIORITY)
- All 12 webhook tests fail with "waiting for webhook configuration to be ready: timed out"
- ClusterIP resolution fix (46b54c0) deployed but webhook pod never starts
- **Action needed**: Investigate why webhook deployment pods don't start

### CRD OpenAPI — 9 failures (FIX STAGED: 0188c3c)
- OpenAPI handler still uses typed deserialization losing nested items
- **Fix staged**: 0188c3c uses raw JSON — not deployed yet

### DNS — 6 failures
- Rate limiter timeout, pods not starting

### Service Networking — 6 failures
- ClusterIP still unreachable from exec pods
- kube-proxy FILTER rules deployed but service still shows "Connection refused"

### Preemption — 4 failures (FIX STAGED: e1f4bd0)
- Extended resources not checked in preemption
- **Fix staged**: e1f4bd0 handles all resource types — not deployed yet

### EmptyDir — 4 failures (webhook cascade)
- Stale webhook blocks pod creation

### Field Validation — 3 failures (FIX STAGED: a18febe)
- Unknown top-level fields not rejected; YAML dup format
- **Fix staged**: a18febe rejects unknown CR extra fields — not deployed yet

### Deployment/RS/RC/StatefulSet — 8 failures
- Various controller issues, some watch-related

### Other — 6 failures
- Discovery PreferredVersion (NEW)
- Job successPolicy (NEW)
- SA mount token (NEW)
- kubectl proxy, describe
- Lifecycle hook, container runtime, init container, pod resize, hostport, namespace deletion, aggregator, OIDC

## Staged Fixes (for next deploy)

| Commit | Fix | Expected Tests |
|--------|-----|---------------|
| e1f4bd0 | Preemption extended resources | 4 preemption |
| 0188c3c | OpenAPI raw JSON CRD schemas | 9 CRD OpenAPI |
| 361752a | EndpointSlice mirroring cleanup | 1 mirroring |
| a18febe | CRD strict unknown top-level fields | 2 field validation |
| 3ba5e20 | Explicit trailing slash routes | 1 kubectl proxy |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 104 | 405 | 36 | 441 | 91.8% |
| 127 | 397 | 44 | 441 | 90.0% |
| 132 | 363 | 78 | 441 | 82.3% |
| 133 | 370 | 71 | 441 | 83.9% |
| 134 | 370 | 71 | 441 | 83.9% |
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | TBD | TBD | 441 | TBD |
