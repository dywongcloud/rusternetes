# Conformance Failure Tracker

**Round 138** | Complete (e2e terminated) | 2026-04-13

## Root Causes Found

### Watch — SYSTEMIC — FIX STAGED + K8s SOURCE ✅
- **Root cause**: HTTP/2 flow control windows at spec default (64KB). With many concurrent watch streams, windows fill → events stall → client-go cancels.
- **K8s source**: secure_serving.go:175-199 sets 256KB/stream, 25MB/connection
- **Fix**: 5b7048f

### Kubelet heartbeat blocked — FIX STAGED ✅
- **Root cause**: Heartbeat was in same tokio::select! as sync_loop. Slow docker.stop_container (30s grace) blocked heartbeat → node NotReady → node controller evicted ALL pods including e2e runner.
- **K8s source**: kubelet runs heartbeat and sync in separate goroutines
- **Fix**: e430f8d — heartbeat in separate tokio task

### Webhook — 7 failures — PARTIALLY UNDERSTOOD
- **Root cause**: Kubelet sync loop contention. Pod scheduled at 21:07:11 but kubelet didn't start it until 21:07:39 (28s delay). Kubelet was stuck processing other pods. With 30s webhook readiness timeout, only 1 second left after pod starts.
- **Underlying issue**: Kubelet sync loop blocks when processing slow pods (stop_container). Per-pod concurrency with tokio::spawn helps but outer loop contention remains.
- Heartbeat fix prevents node eviction. Webhook timing still tight.

### CRD OpenAPI — 6 failures — FIX STAGED + TESTED ✅
- **Fix**: 86b048a (test: 3 levels deep)

### Preemption — 3 failures — FIX STAGED + TESTED ✅
- **Fix**: 55d52d7 (test: deep_merge)

### Field Validation — 2 failures — FIX STAGED + TESTED ✅
- **Fix**: 858d091 (3 tests)

### CRD Error Responses — FIX STAGED ✅
- **Fix**: 294358e — 10 handlers fixed

### DaemonSet — 1 failure — INVESTIGATING
- 0 watch failures near test. Pod created but not observed as running within timeout.

### Proxy — 2 failures — INVESTIGATING
- 0 watch failures near test. Pod not starting in time.

### Other fixes staged:
- Job: 31e5e4f (test: success_policy_terminating_zero)
- NodePort: f80d0c6
- Watch tuning: 0061469

## Staged Fixes (12 commits)

| Commit | Fix | Test |
|--------|-----|------|
| 5b7048f | **HTTP/2 flow control: K8s window sizes** | K8s source ✅ |
| e430f8d | **Kubelet heartbeat in separate task** | K8s source ✅ |
| 858d091 | Schema validator collects ALL unknown fields | 3 tests ✅ |
| 86b048a | OpenAPI strip ALL Go omitempty defaults | 1 test ✅ |
| 55d52d7 | Status PATCH deep merge (node capacity) | 1 test ✅ |
| 31e5e4f | Job successPolicy terminating=0 | 1 test ✅ |
| 294358e | CRD error responses: K8s Status JSON | — |
| f80d0c6 | kube-proxy NodePort DNAT rules | — |
| 0061469 | Watch channel buffer 16 + bookmark 1s | — |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | ~380 | ~61 | 441 | ~86.2% |
| 138 | TERMINATED (node NotReady) | ~35+ | 441 | — |
