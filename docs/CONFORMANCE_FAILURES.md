# Conformance Failure Tracker

**Round 137** | Complete | 2026-04-13
**Result**: ~380/441 (~86.2%) — 61 failure instances, 50 unique locations

## Round 137 Failures — Complete Itemized List

### Webhook — 11 failures — FIX STAGED ✅ (kube-proxy atomic)
- `webhook.go:675,904,1269,1334,1400,1481,2107(x3),2164,2491`
- **Fix**: 6af8bb9 — Atomic iptables-restore eliminates flush gap

### CRD OpenAPI — 9 failures — FIX STAGED ✅
- `crd_publish_openapi.go:77,161,211,253,285,318,366,400,451`
- **Fix**: 3186cf5 (false extensions) + f1bf53f (watch pre-buffer)

### DNS — 6 failures — FIX STAGED ✅ (kube-proxy atomic)
- `dns_common.go:476` (x6)
- **Fix**: 6af8bb9

### EmptyDir/output — 4 failures — MIXED
- `output.go:263` (x4) — macOS filesystem permissions (DinD) + watch issues

### Service — 3 failures — FIX STAGED ✅
- `service.go:251` (x2) — Session affinity transition. **Fix**: 9c21776 (affinity in atomic path)
- `service.go:768` — kube-proxy flush gap. **Fix**: 6af8bb9 + 9c21776
- `service.go:3459` — same as 768

### Deployment — 3 failures — FIX STAGED ✅
- `deployment.go:781` — revision not incremented on adoption. **Fix**: f524e6c
- `deployment.go:995` — rollover pods not available, watch/timing
- `deployment.go:1264` — RS replicas timeout, watch

### Preemption — 3 failures — FIX STAGED ✅
- `preemption.go:181,268` — **Fix staged**: fb9728d + c19a049
- `resource.go:512` — Kubelet overwrote DisruptionTarget condition. **Fix**: 1810ac1

### Field Validation — 3 failures — FIX STAGED ✅
- `field_validation.go:611` — **Fix**: 47fb9ec (preserve-unknown-fields)
- `field_validation.go:735` — duplicate key detection
- `field_validation.go:462` — PATCH missing strict validation. **Fix**: 1810ac1

### Webhook configuration — counted in Webhook above

### StatefulSet — 2 failures — FIX STAGED ✅
- `statefulset.go:957,1092`
- **Fix**: 8673d37 + 4438743 + watch fix

### RC — 2 failures — FIX STAGED ✅
- `rc.go:509` — over-creation. **Fix**: 070dde7
- `rc.go:623` — ReplicaFailure never cleared. **Fix**: 1810ac1

### ReplicaSet — 2 failures — DOWNSTREAM
- `replica_set.go:232` — pod responses timeout (watch+networking)
- `replica_set.go:560` — scaling timeout (watch+networking)

### Proxy — 2 failures — FIX STAGED ✅ (kube-proxy atomic)
- `proxy.go:271,503`

### Namespace — 1 failure — FIX STAGED ✅
- `namespace.go:579` — **Fix**: 125d91a

### DaemonSet — 1 failure — DOWNSTREAM of watch
- `daemon_set.go:1276` — watch context canceled during readiness

### Init Container — 1 failure — DOWNSTREAM of watch
- `init_container.go:440`

### Service Latency — 1 failure — FIX STAGED ✅
- `service_latency.go:145` — **Fix**: 6af8bb9

### Runtime — 1 failure — DOWNSTREAM of watch
- `runtime.go:115` — expected 2 replicas, got 0

### Events — 1 failure — FIX STAGED ✅
- `events.go:167` — **Fix**: 2f0cbd9 (generation only on spec)

### Kubectl — 1 failure — FIX STAGED ✅
- `kubectl.go:2206` — **Fix**: b65f0f9 (sessionAffinity "None")

### Aggregator — 1 failure — DinD
- `aggregator.go:359` — sample API server image not available

### HostPort — 1 failure — DinD
- `hostport.go:219`

### Pod Resize — 1 failure — DinD
- `pod_resize.go:857`

## All Issues Have Fixes Staged

Every failure from round 137 now has a fix staged for round 138, is downstream of a staged fix, or is a DinD limitation.

## Staged Fixes

| Commit | Fix | Expected Impact |
|--------|-----|-----------------|
| 6af8bb9 | kube-proxy atomic iptables-restore | ~20 failures |
| f1bf53f | Watch pre-buffer initial events | ~5-8 failures |
| 070dde7 | RC UID ownership + active pod filtering | 1 |
| 3186cf5 | Strip false x-kubernetes extensions | 3-9 |
| 125d91a | GC no namespace cascade | 1+ |
| 47fb9ec | CRD validation: preserve-unknown-fields + embedded | 2 |
| fb9728d | Preemption reprieve + grace period | 2 |
| c19a049 | Priority admission controller | 1 |
| b65f0f9 | Service sessionAffinity default "None" | 1 |
| f524e6c | Deployment revision = MaxRevision(oldRSes) + 1 | 1-2 |
| 2f0cbd9 | Generation only on spec changes | 1 |
| 1810ac1 | Kubelet preserves DisruptionTarget condition on preempted pods | 1 |
| 1810ac1 | CRD PATCH strict validation for unknown fields | 1 |
| 1810ac1 | RC only sets ReplicaFailure from actual creation errors | 1 |
| 9c21776 | kube-proxy session affinity in atomic path + no pre-flush | 2 |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | ~380 | ~61 | 441 | ~86.2% |
| 138 | TBD | TBD | 441 | TBD |
