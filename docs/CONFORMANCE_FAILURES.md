# Conformance Issue Tracker

**Round 118** | COMPLETE | 299/441 passed, 142 failed (67.8%)

## Fixes Ready for Round 119 (~48 additional passes expected)

| Fix | Commit | Expected Tests |
|-----|--------|----------------|
| etcd gRPC keepalive | 4991385 | ~12 (CRD/job/RC/CSI watch timeouts) |
| ConfigMap webhook pipeline | fac86a3 | ~12 (webhook readiness tests) |
| SA token bound in Secret volume | 0a30348 | ~3 |
| ResourceQuota enforcement | 7985cf9 | ~2 |
| CrashLoopBackOff backoff | fa0122b | ~1 |
| Stale webhook cleanup on NS delete | 88f9c37 | ~5 (preemption/scheduling) |
| Deployment direct pod count | 36ff92b | ~3 |
| Namespace lifecycle split | 2a0ff37 | ~1 |
| StatefulSet scale-down | 805c044 | ~1 |
| Scheduler Unschedulable | d165195 | ~2 |
| Sysctl all errors | d165195 | ~1 |
| LimitRange pod defaulting | c99e0db | ~1 |
| CreateContainerError preserved | 8af3c12 | ~1 |
| WebSocket exec delay | 4d7f7e3 | ~1 |
| StatefulSet rolling update logging | various | diagnostic |
| **Total** | | **~48** |

## Remaining Issues (~20 tests)

### Platform Limitations (~26 tests, cannot fix)
- Service networking (~10): Docker Desktop iptables DNAT bypassed
- EmptyDir/Secret permissions (~8): macOS bind mount umask
- kubectl protobuf (~8): Need real K8s OpenAPI protobuf encoding

### Latency/Timing (~12 tests)
- DNS rate limiter (~6): Cascading from informer retries — improves with keepalive
- Pod startup latency (~4): Docker Desktop + controller intervals
- RC/ReplicaSet (~2): Rate limiter exhaustion — improves with keepalive

### Need Deeper Work (~8 tests)
- DaemonSet rolling update (~2): DS controller rolling update logic
- ConfigMap/Secret volume refresh (~2): Update propagation timing
- Events (~1): Event list empty after create
- Aggregator (~1): Multi-container pod startup ordering
- Other (~2): Various edge cases

## Projected Results

| Round | Pass | Fail | Rate |
|-------|------|------|------|
| 118 | 299 | 142 | 67.8% |
| 119 | ~347 | ~94 | **~79%** |
