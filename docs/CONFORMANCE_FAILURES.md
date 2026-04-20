# Conformance Failure Tracker

## Fixes Applied (ready for next run)

| # | Test(s) | Fix | Commit |
|---|---------|-----|--------|
| 1 | node_lifecycle.go:95 | 60s startup grace period + not-ready taint | 79fde16 |
| 2 | statefulset.go:1205 | SS condition merge — preserves custom conditions | 9344dec |
| 3 | rc.go:212 | RC status write reduction + condition merge | 9344dec |
| 4 | resource_quota.go:1047 | RQ status comparison + fresh resourceVersion | ffeb5c6 |
| 5 | disruption.go:187 | PDB status comparison + condition preservation | ffeb5c6 |
| 6 | deployment.go:883 | Old RS cleanup (revisionHistoryLimit) + idempotent RS | ffeb5c6 |
| 7 | aggregator.go:359 | New APIService availability controller | 7ea342d |
| 8 | daemon_set.go:332 | DS retry failed pods in same cycle + pod watch | 7ea342d |
| 9 | init_container.go:241 | Kubelet refreshes init container status on terminal | 7ea342d |
| 10 | job.go:1192 | Job status guards on early-return paths | 7ea342d |
| 11 | rc.go:453 | RC CAS retry guard — skip if status matches | 7ea342d |
| 12 | replica_set.go:534 | RS CAS retry guard — skip if status matches | 7ea342d |
| 13 | deployment status tests | Deployment condition merge + fresh resourceVersion | ffeb5c6 |
| 14 | DaemonSet status tests | DS condition preservation | earlier |
| 15 | kubelet orphans | Startup cleanup + fast-path deletion | earlier |

## Critical Infrastructure Issues

| # | Issue | Root Cause |
|---|-------|------------|
| 1 | Namespace deletion hangs on stale pods | Namespace controller can't finalize namespaces with Succeeded/Failed pods. GC or kubelet doesn't delete terminal pod records from storage. Blocks conformance restarts. |
| 2 | Stale etcd state across cluster restarts | Pod records persist in etcd after cluster restart but Docker containers are gone. Kubelet startup cleanup only removes Docker containers, doesn't clean API-side pod records. |
| 3 | run-conformance.sh hangs on stuck cleanup | Script waits indefinitely for sonobuoy namespace deletion. Needs timeout and force-delete fallback. |

## Remaining Pre-existing (not yet fixed)

| # | Test(s) | Root Cause | Priority |
|---|---------|------------|----------|
| 1 | deployment.go:1259 | Proportional scaling — RS availableReplicas convergence | Medium |
| 2 | crd_publish_openapi.go x3 | CRD OpenAPI schema not served dynamically | Low |
| 3 | output.go:263 x5 | EmptyDir POSIX permissions — needs container-local path | Low |
| 4 | hostport.go:219 | HostPort conflict detection | Low |
| 5 | service.go:768 | Service endpoint unreachable — networking | Medium |
| 6 | service.go:3459 | Service delete timeout | Low |
| 7 | proxy.go:271 | Service proxy unreachable — networking | Medium |
| 8 | replica_set.go:232 | RS pod proxy — networking | Medium |
| 9 | runtime.go:129 | Container state after restart | Low |
| 10 | daemon_set.go:1276 | GC deletes DS pod | Low |
| 11 | garbage_collector.go:635 | GC orphan RS propagation | Low |
| 12 | pod_resize.go:857 | Pod resize cgroup — not implemented | Low |
