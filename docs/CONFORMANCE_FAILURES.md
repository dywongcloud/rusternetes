# Conformance Issue Tracker

**Round 118** | IN PROGRESS | 239/441 done | 161 passed, 78 failed (67.4%)

## Failure Summary — 65 unique failures

### By Category

| Category | Count | Root Cause | Fix Status |
|----------|-------|-----------|------------|
| CRD/etcd watch timeouts | 7 | etcd gRPC stream ending | FIXED 4991385 keepalive — not deployed |
| Job completion timeouts | 5 | Same etcd watch issue | FIXED 4991385 — not deployed |
| kubectl protobuf | 4 | No real protobuf encoding | Unfixable without protobuf library |
| Service networking | 6 | Docker Desktop userspace bypass | Platform limitation |
| EmptyDir permissions | 3 | Docker Desktop bind mount umask | Platform limitation |
| DNS rate limiter | 3 | Cascading from informer retries | Improves with etcd keepalive |
| StatefulSet | 4 | Scale-down timing + rolling update | 1 FIXED (805c044), 1 needs debug |
| Webhook | 4 | Webhook service readiness | TLS fix deployed, may need more |
| Preemption/scheduling | 5 | Scheduler predicates + latency | 1 FIXED (d165195 Unschedulable) |
| RC/ReplicaSet | 5 | Latency + rate limiter | Improves with etcd keepalive |
| Deployment | 3 | Revision + latency | Partially fixed |
| SA tokens | 3 | Missing TokenRequest API | Needs kubelet TokenRequest |
| Other | 13 | Various | See below |

### Pending Fixes (not deployed)

| Fix | Commit | Expected Impact |
|-----|--------|----------------|
| etcd keepalive | 4991385 | ~12 tests (CRD + job + RC timeouts) |
| StatefulSet scale-down | 805c044 | 1 test |
| Scheduler Unschedulable | d165195 | ~2 tests |
| Sysctl all errors | d165195 | 1 test |
| LimitRange separation | c99e0db | 1 test |
| CreateContainerError | 8af3c12 | 1 test |
| WebSocket exec delay | 4d7f7e3 | 1 test |
| **Total** | | **~19 tests** |

### Platform Limitations (unfixable)

| Category | Tests | Reason |
|----------|-------|--------|
| Service networking | 6 | Docker Desktop iptables DNAT bypassed |
| EmptyDir permissions | 3 | macOS bind mount umask |
| kubectl protobuf | 4 | Need real K8s OpenAPI protobuf |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 116 | 128 | 94 | 222/441 | 57.7% |
| 117 | 89 | 44 | 133/441 | 66.9% |
| 118 | 161 | 78 | 239/441 | 67.4% (in progress) |
| 119 (projected) | ~180 | ~59 | ~239 | ~75% (with pending fixes) |
