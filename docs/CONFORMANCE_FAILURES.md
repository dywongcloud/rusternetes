# Conformance Failure Tracker

**Round 137** | Complete | 2026-04-13
**Result**: ~399/441 (~90.5%) — 42 unique failures

## Round 137 Failures (42 unique)

### Webhook — 11 failures — FIX STAGED ✅
- `webhook.go:675,904,1269,1334,1400,1481,2107(x2),2164,2491` + watch failures
- **Root cause**: kube-proxy flush gap. Individual `iptables -F` + `iptables -A` commands create a window where NO ClusterIP rules exist. Any webhook call during rebuild gets "connection refused". K8s uses `iptables-restore --noflush` for atomic replacement (proxier.go:1495).
- **Fix**: 6af8bb9 — Atomic iptables-restore, eliminates flush gap

### CRD OpenAPI — 8 failures — FIX STAGED ✅
- `crd_publish_openapi.go:77,161,211,285,318,366,400,451`
- **Root cause**: (1) `x-kubernetes-embedded-resource: false` / `x-kubernetes-int-or-string: false` serialized when K8s omits them. (2) Watch context canceled during schema polling.
- **Fix**: 3186cf5 — Strip false x-kubernetes extensions + f1bf53f watch pre-buffer

### DNS — 5 failures — FIX STAGED ✅
- `dns_common.go:476` (x5)
- **Root cause**: kube-proxy flush gap breaks ClusterIP routing to CoreDNS (10.96.0.10)
- **Fix**: 6af8bb9 — Atomic iptables-restore

### EmptyDir/output — 3 failures — MIXED
- `output.go:263` (x3) — some are macOS permission issues, some may be watch-related

### Preemption — 2 failures — FIX STAGED ✅
- `preemption.go:181,268`
- **Fix**: fb9728d — Reprieve algorithm + c19a049 priority admission

### Service — 2 failures — FIX STAGED ✅
- `service.go:251,3459`
- **Root cause**: kube-proxy flush gap
- **Fix**: 6af8bb9 — Atomic iptables-restore

### Proxy — 2 failures — FIX STAGED ✅
- `proxy.go:271,503`
- **Root cause**: kube-proxy flush gap breaks service proxy routing
- **Fix**: 6af8bb9 — Atomic iptables-restore

### Deployment — 2 failures — PARTIALLY DOWNSTREAM
- `deployment.go:781,1264` — RS replicas timeout + watch context canceled

### StatefulSet — 2 failures — FIX STAGED ✅
- `statefulset.go:957,1092`
- **Fix**: 8673d37 generation + 4438743 counting + watch fix

### Field Validation — 2 failures — FIX STAGED ✅
- `field_validation.go:611,735`
- **Fix**: 47fb9ec — CRD preserve-unknown-fields + embedded resources

### Namespace — 1 failure — FIX STAGED ✅
- `namespace.go:579` — GC cascade-deleted namespace resources ignoring finalizers
- **Fix**: 125d91a — Removed GC namespace cascade

### RC — 1 failure — FIX STAGED ✅
- `rc.go:509` — creates 5+ pods/sec
- **Fix**: 070dde7 — UID ownership + active pod filtering

### DaemonSet — 1 failure — INVESTIGATING
- `daemon_set.go:1276`

### ReplicaSet — 1 failure — DOWNSTREAM
- `replica_set.go:232` — pod responses timeout

### Init Container — 1 failure — DOWNSTREAM
- `init_container.go:440` — watch timeout

### Service Latency — 1 failure — FIX STAGED ✅
- `service_latency.go:145` — deployment not ready, kube-proxy flush gap
- **Fix**: 6af8bb9

### Runtime — 1 failure — INVESTIGATING
- `runtime.go:115`

### Events — 1 failure — INVESTIGATING
- `events.go:167`

### Kubectl — 1 failure — INVESTIGATING
- `kubectl.go:2206`

### HostPort — 1 failure — DinD
- `hostport.go:219`

### Pod Resize — 1 failure — DinD
- `pod_resize.go:857`

## Staged Fixes (not yet deployed)

| Commit | Fix | Expected Impact |
|--------|-----|-----------------|
| 6af8bb9 | **kube-proxy atomic iptables-restore** | ~20 failures (webhook+DNS+service+proxy) |
| f1bf53f | Watch pre-buffer initial events | ~5-8 failures (systemic) |
| 070dde7 | RC UID ownership + active pod filtering | 1 failure |
| 3186cf5 | Strip false x-kubernetes extensions | 3-8 failures |
| 125d91a | GC no longer cascade-deletes namespace resources | 1+ failures |
| 47fb9ec | CRD validation: preserve-unknown-fields + embedded resources | 2 failures |
| fb9728d | Preemption reprieve + grace period | 2 failures |
| c19a049 | Priority admission controller | preemption reliability |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | ~399 | ~42 | 441 | ~90.5% |
| 138 | TBD | TBD | 441 | TBD |
