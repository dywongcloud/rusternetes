# Conformance Failure Tracker

**Round 135** | 373/441 (84.6%) | 2026-04-11
**Round 134** | 370/441 (83.9%) | 2026-04-10

## Round 135 — 68 failures, 57 unique locations

### Webhook — 12 failures (ROOT CAUSE FOUND)
- `webhook.go:520,675,904,1269,1334,1400,1481,2107(x3),2164,2491`
- **Root cause**: kube-proxy flush gap — every sync flushed ALL iptables rules because hash was order-dependent and NEVER matched. Webhook service ClusterIP rules existed for only ~50ms/second. With FailurePolicy=Ignore, failed calls were silently ignored → readiness check never saw denial.
- **Fix committed**: 3012663 order-independent XOR hashing eliminates unnecessary flushes
- **Additional findings from K8s comparison**: Missing objectSelector (7cf9bd5), missing matchPolicy support, missing UID verification on responses

### CRD OpenAPI — 9 failures (FIX STAGED 0188c3c)
- `crd_publish_openapi.go:77,161,214,253,285,318,366,400,451`
- OpenAPI handler uses typed deserialization losing nested `items` schemas
- **Fix staged**: 0188c3c uses raw JSON — needs deploy

### DNS — 6 failures
- `dns_common.go:476` (x6)
- Rate limiter timeout, pods not starting

### Service Networking — 6 failures
- `service.go:768,886,3459`, `proxy.go:271,503`, `service_latency.go:145`
- ClusterIP service unreachable from exec pods

### EmptyDir — 4 failures (webhook cascade)
- `output.go:263` (x4)
- Stale webhook configuration blocks pod creation

### Preemption — 4 failures (FIX STAGED e1f4bd0)
- `predicates.go:1041(x2)`, `preemption.go:535,1052`
- Extended resources not checked in preemption
- **Fix staged**: e1f4bd0 — needs deploy

### Field Validation — 3 failures (FIX STAGED a18febe)
- `field_validation.go:462,611,735`
- Unknown top-level CR fields not rejected; YAML dup format
- **Fix staged**: a18febe — needs deploy

### Apps Controllers — 8 failures
- `deployment.go:1008,1322`, `replica_set.go:232,560`, `rc.go:509,623`, `statefulset.go:957,1092`
- Various controller timing/watch issues

### DaemonSet — 1 failure
- `daemon_set.go:1276`

### Job — 1 failure
- `job.go:556`

### Init Container — 1 failure
- `init_container.go:440`

### Discovery — 1 failure (NEW)
- `discovery.go:131` — PreferredVersion validation

### Namespace — 1 failure
- `namespace.go:609`

### ResourceQuota — 1 failure
- `resource_quota.go:282`

### Auth — 2 failures
- `service_accounts.go:129,667` — SA mount token, OIDC

### kubectl — 2 failures
- `kubectl.go:1881,2206` — proxy, describe

### Node — 4 failures
- `lifecycle_hook.go:132`, `runtime.go:115`, `init_container.go:440`, `pod_resize.go:857`

### Other — 2 failures
- `hostport.go:219`, `aggregator.go:359`, `endpointslicemirroring.go:202`

## Staged Fixes (for round 136)

| Commit | Fix | Expected Tests |
|--------|-----|---------------|
| e1f4bd0 | Preemption extended resources | ~4 |
| 0188c3c | OpenAPI raw JSON CRD schemas | ~9 |
| 361752a | EndpointSlice mirroring cleanup | 1 |
| a18febe | CRD strict unknown top-level fields | ~2 |
| 3ba5e20 | Explicit trailing slash routes | 1 |

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
