# Conformance Failure Tracker

**Round 137** | Complete | 2026-04-13
**Result**: ~399/441 (~90.5%) — 42 unique failures

## Round 137 Failures — Root Cause Analysis (42 unique)

### Webhook — 11 failures — FIX STAGED ✅
- **Root cause**: kube-proxy flush gap during iptables rebuild
- **Fix**: 6af8bb9 — Atomic iptables-restore

### CRD OpenAPI — 8 failures — FIX STAGED ✅
- **Root cause**: false x-kubernetes extensions + watch context canceled
- **Fix**: 3186cf5 + f1bf53f

### DNS — 5 failures — FIX STAGED ✅
- **Root cause**: kube-proxy flush gap breaks CoreDNS ClusterIP routing
- **Fix**: 6af8bb9

### EmptyDir/output — 3 failures — 1 DinD + 2 watch
- 1 macOS filesystem permissions (DinD), 2 watch context canceled

### Preemption — 2 failures — FIX STAGED ✅
- **Fix**: fb9728d + c19a049

### Service — 2 failures — FIX STAGED ✅
- **Root cause**: kube-proxy flush gap
- **Fix**: 6af8bb9

### Proxy — 2 failures — FIX STAGED ✅
- **Root cause**: kube-proxy flush gap
- **Fix**: 6af8bb9

### Deployment — 2 failures — NEEDS FIX ❌
- `deployment.go:781` — Deployment revision annotation not incremented when adopting RS. K8s creates a new RS with revision max+1 even when adopting. Our code sets deployment revision to max (not max+1).
- `deployment.go:1264` — RS replicas timeout + watch context canceled
- **Status**: Needs deployment revision handling fix

### StatefulSet — 2 failures — FIX STAGED ✅
- **Fix**: 8673d37 + 4438743 + watch fix

### Field Validation — 2 failures — FIX STAGED ✅
- **Fix**: 47fb9ec

### Namespace — 1 failure — FIX STAGED ✅
- **Fix**: 125d91a

### RC — 1 failure — FIX STAGED ✅
- **Fix**: 070dde7

### DaemonSet — 1 failure — DOWNSTREAM of watch fix
- Watch context canceled during pod readiness check

### ReplicaSet — 1 failure — DOWNSTREAM of watch + kube-proxy
- Pod responses timeout

### Init Container — 1 failure — DOWNSTREAM of watch
- Watch timeout during state transition

### Service Latency — 1 failure — FIX STAGED ✅
- **Fix**: 6af8bb9

### Runtime — 1 failure — DOWNSTREAM of watch
- Expected 2 ready replicas, got 0. Watch context canceled.

### Events — 1 failure — NEEDS FIX ❌
- `events.go:167` — Event PATCH doesn't properly preserve/update Series field. DeepEqual comparison fails after patching. The `series` field with `count` and `lastObservedTime` is not correctly round-tripped through PATCH.
- **Status**: Needs investigation of Event Series PATCH handling

### Kubectl — 1 failure — FIX STAGED ✅
- `kubectl.go:2206` — `sessionAffinity` not defaulted to "None"
- **Fix**: b65f0f9

### HostPort — 1 failure — DinD
### Pod Resize — 1 failure — DinD

## Staged Fixes (not yet deployed)

| Commit | Fix | Expected Impact |
|--------|-----|-----------------|
| 6af8bb9 | **kube-proxy atomic iptables-restore** | ~20 failures |
| f1bf53f | Watch pre-buffer initial events | ~5-8 failures |
| 070dde7 | RC UID ownership + active pod filtering | 1 |
| 3186cf5 | Strip false x-kubernetes extensions | 3-8 |
| 125d91a | GC no longer cascade-deletes namespace resources | 1+ |
| 47fb9ec | CRD validation: preserve-unknown-fields + embedded resources | 2 |
| fb9728d | Preemption reprieve + grace period | 2 |
| c19a049 | Priority admission controller | 1 |
| b65f0f9 | Service sessionAffinity default to "None" | 1 |

## Still Need Fix

| Issue | Root Cause |
|-------|-----------|
| deployment.go:781 | Deployment revision not incremented on RS adoption |
| events.go:167 | Event Series field not preserved through PATCH |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | ~399 | ~42 | 441 | ~90.5% |
| 138 | TBD | TBD | 441 | TBD |
