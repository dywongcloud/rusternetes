# Conformance Issue Tracker

**Round 118** | COMPLETE | 299/441 passed, 142 failed (67.8%)

## Fixes Ready for Round 119 (~37 additional passes expected)

| Fix | Commit | Expected Tests |
|-----|--------|----------------|
| etcd gRPC keepalive | 4991385 | ~12 (CRD/job/RC/CSI watch timeouts) |
| ConfigMap webhook pipeline | fac86a3 | ~12 (webhook readiness tests) |
| SA token bound in Secret volume | 0a30348 | ~3 (SA pod-name extra info) |
| ResourceQuota enforcement | 7985cf9 | ~2 (RC/RS quota tests) |
| CrashLoopBackOff backoff | fa0122b | ~1 (terminated reason) |
| StatefulSet scale-down | 805c044 | 1 |
| Scheduler Unschedulable | d165195 | ~2 |
| Sysctl all errors | d165195 | 1 |
| LimitRange pod defaulting | c99e0db | 1 |
| CreateContainerError preserved | 8af3c12 | 1 |
| WebSocket exec delay | 4d7f7e3 | 1 |
| **Total** | | **~37** |

## Remaining Issues (after round 119 deploy)

### Platform Limitations (~26 tests, cannot fix)
- Service networking (~10): Docker Desktop iptables DNAT bypassed
- EmptyDir/Secret permissions (~8): macOS bind mount umask
- kubectl protobuf (~8): Need real K8s OpenAPI protobuf encoding

### Fixed (not yet deployed)
- Preemption/scheduling: FIXED 88f9c37 — stale webhook configs cleaned up on namespace delete
- Deployment readiness: FIXED 36ff92b — count pods directly instead of stale RS status
- Namespace lifecycle: FIXED 2a0ff37 — split finalizer removal across two cycles

### Remaining (~22 tests)
- DaemonSet (~2): Rolling update + pod timing
- RC/ReplicaSet (~5): Pod connectivity, rate limiter exhaustion
- ConfigMap/Secret volume (~2): Update propagation timing (logging added)
- Events (~1): Event list returns empty after create
- Aggregator (~1): Sample API server pod not ready
- DNS rate limiter (~6): Cascading from informer retries
- Pod latency (~4): Docker Desktop + controller intervals
- Other (~1): Various

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 119 | ~336 | ~105 | 441 | **~76%** (projected with 37 fixes) |
