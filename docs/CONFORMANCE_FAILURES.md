# Conformance Issue Tracker

**Round 118** | COMPLETE | 299/441 passed, 142 failed (67.8%)

## Fixes Ready for Deploy (Round 119)

| Fix | Commit | Expected Tests |
|-----|--------|----------------|
| etcd gRPC keepalive | 4991385 | ~12 (CRD/job/RC watch timeouts) |
| StatefulSet scale-down | 805c044 | 1 |
| Scheduler Unschedulable | d165195 | ~2 |
| Sysctl all errors | d165195 | 1 |
| LimitRange pod defaulting | c99e0db | 1 |
| CreateContainerError preserved | 8af3c12 | 1 |
| WebSocket exec delay | 4d7f7e3 | 1 |
| Webhook info logging | 8a42d81 | diagnostic |
| **Total** | | **~19** |

## Root Causes Identified (Need Code Fixes)

| Issue | Count | Root Cause | Fix Needed |
|-------|-------|-----------|------------|
| CRD/Job etcd watch timeout | ~22 | gRPC stream ending | FIXED — keepalive 4991385 |
| Webhook readiness | ~12 | Webhook matching/calling not triggering | Need to debug matching logic |
| RC quota enforcement | ~2 | RC controller bypasses API server admission | Route pod creation through API |
| Deployment available=0 | ~3 | Pods not reporting ready fast enough | Kubelet readiness timing |
| SA token pod-name | ~3 | Kubelet uses static tokens | Bound token fix deployed but tests still fail |
| Terminated container reason | 1 | Status being overwritten | Need to trace status flow |
| StatefulSet rolling update | ~2 | Template hash comparison | Logging deployed, need data |

## Platform Limitations (Cannot Fix)

| Issue | Count | Reason |
|-------|-------|--------|
| Service networking | ~10 | Docker Desktop iptables DNAT bypassed |
| EmptyDir/Secret permissions | ~8 | macOS bind mount umask |
| kubectl protobuf | ~8 | Need real K8s OpenAPI protobuf encoding |
| DNS rate limiter | ~6 | Cascading from other failures |
| Pod latency | ~10 | Docker Desktop + controller intervals |

## Other Failures (~48)

- Preemption/scheduling: ~5 (scheduler predicates)
- ReplicaSet: ~4 (latency)
- DaemonSet: ~2 (pod startup)
- Namespace: ~1 (async deletion)
- Events: ~1 (event format)
- ConfigMap/Secret volume: ~2 (update propagation timing)
- CSIStorageCapacity: ~1
- Aggregator: ~1 (sample API server)
- HostPort: ~1 (Docker Desktop)
- Disruption: ~1 (PDB cause format)
- Lifecycle hooks: ~1 (service networking)
- Various other: ~27

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 119 | ~318 | ~123 | 441 | ~72% (projected) |
