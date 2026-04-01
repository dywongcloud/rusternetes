# Conformance Issue Tracker

**Round 118** | COMPLETE | 441/441 | 299 passed, 142 failed (67.8%)

## Final Results

299/441 passed (67.8%) — up from 283/441 (64.2%) in round 110.
16 additional tests passing compared to round 110.

## Pending Fixes for Round 119

| Fix | Commit | Expected Impact |
|-----|--------|----------------|
| etcd gRPC keepalive | 4991385 | ~12 tests (CRD/job/RC watch timeouts) |
| StatefulSet scale-down | 805c044 | 1 test |
| Scheduler Unschedulable | d165195 | ~2 tests |
| Sysctl all errors | d165195 | 1 test |
| LimitRange pod defaulting | c99e0db | 1 test |
| CreateContainerError preserved | 8af3c12 | 1 test |
| WebSocket exec delay | 4d7f7e3 | 1 test |
| **Total** | | **~19 tests** |

## Failure Breakdown (142 failures)

| Category | Count | Root Cause |
|----------|-------|-----------|
| CRD/etcd watch timeouts | ~15 | etcd gRPC stream ending — FIXED (keepalive) |
| kubectl protobuf | 4 | No real protobuf encoding — platform limitation |
| Service networking | ~10 | Docker Desktop userspace bypass — platform limitation |
| EmptyDir/Secret permissions | ~8 | Docker Desktop bind mount umask — platform limitation |
| DNS rate limiter | 3 | Cascading from informer retries |
| StatefulSet | ~5 | Scale-down + rolling update |
| Webhook | ~5 | Webhook service readiness/TLS |
| Preemption/scheduling | ~5 | Scheduler predicates + latency |
| RC/ReplicaSet | ~5 | Latency + rate limiter |
| Deployment | ~3 | Revision + status |
| SA tokens | ~3 | Missing TokenRequest API |
| Job | ~6 | etcd watch stream — FIXED (keepalive) |
| Other | ~70 | Various latency, timing, networking |

## Progress History

| Round | Pass | Fail | Total | Rate | Notes |
|-------|------|------|-------|------|-------|
| 110 | 283 | 158 | 441 | 64.2% | Baseline |
| 116 | ~128 | ~94 | 222/441 | 57.7% | Pre-deploy regression |
| 117 | ~89 | ~44 | 133/441 | 66.9% | First deploy of fixes |
| 118 | 299 | 142 | 441 | **67.8%** | All major fixes deployed |
| 119 | ~318 | ~123 | 441 | **~72%** | Projected with pending fixes |
