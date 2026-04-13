# Conformance Failure Tracker

**Round 138** | Running (~75% done) | 2026-04-13
**Current**: 33 failures at ~330/441

## Round 138 Failures (33 unique, test still running)

### CRD OpenAPI — 6 failures — FIX STAGED ✅ (not deployed)
- **Fix**: 86b048a (with test)

### Webhook — 7 failures — WATCH DOWNSTREAM
- All "waiting for webhook configuration to be ready: timed out"
- Watch context canceled is proximate cause

### Preemption — 3 failures — FIX STAGED ✅ (not deployed)
- **Root cause**: Status PATCH replaced node capacity map instead of merging, wiping extended resources
- **Fix**: 55d52d7 (with test)

### Field Validation — 2 failures — FIX STAGED ✅ (not deployed)
- **Fix**: 858d091 (with tests)

### ReplicaSet — 2 failures — WATCH DOWNSTREAM

### Service — 3 failures — PARTIAL FIX
- NodePort fix staged: f80d0c6
- Deletion timeout: watch downstream
- Rate limiter exhaustion: watch downstream

### StatefulSet — 2 failures — MIXED
- 957: DinD limitation (port conflicts don't fail in Docker)
- 1092: watch downstream

### Proxy — 2 failures — WATCH DOWNSTREAM

### DNS — 1 failure — INVESTIGATING
- Container command execution issue (may be transient)

### Others — 5 failures — WATCH DOWNSTREAM
- DaemonSet, Runtime, Lifecycle hook, Job, EmptyDir(DinD)

## Staged Fixes (7 commits for round 139)

| Commit | Fix | Test |
|--------|-----|------|
| 858d091 | Schema validator collects ALL unknown fields | 3 tests |
| 86b048a | OpenAPI strip ALL Go omitempty defaults | 1 test |
| 31e5e4f | Job successPolicy terminating=0 | — |
| f80d0c6 | kube-proxy NodePort DNAT rules | — |
| 0061469 | Watch channel buffer 16 + bookmark 1s | — |
| 55d52d7 | Status PATCH deep merge (node capacity) | 1 test |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | ~380 | ~61 | 441 | ~86.2% |
| 138 | TBD (33 at ~75%) | TBD | 441 | TBD |
