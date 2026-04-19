# Conformance Failure Tracker

## Current Run (Round 152 — release builds, etcd, work queue + all fixes)

**Status at 125min: 220 passed, 29 failed, 249/441 done (88.4%)**

### All 29 Failures

| # | Test | Pre-existing? | Category |
|---|------|---------------|----------|
| 1 | aggregator.go:359 | Yes | Aggregator proxy — deployment not ready |
| 2 | crd_publish_openapi.go:285 | Yes | CRD OpenAPI schema not dynamic |
| 3 | crd_publish_openapi.go:318 | Yes | CRD OpenAPI schema |
| 4 | crd_publish_openapi.go:451 | Yes | CRD OpenAPI schema |
| 5 | garbage_collector.go:635 | Yes | GC orphan RS propagation |
| 6 | resource_quota.go:1047 | **New?** | ResourceQuota — investigate |
| 7 | daemon_set.go:1276 | Yes | GC deletes DS pod |
| 8 | daemon_set.go:332 | Yes | DS retry failed pods |
| 9 | deployment.go:1259 | Yes | Deployment proportional scaling |
| 10 | deployment.go:883 | Yes | Deployment delete old RS |
| 11 | disruption.go:187 | **New?** | PDB eviction — investigate |
| 12 | job.go:1192 | **New** | Job PATCH conflict — controller writes status 12x during reconcile |
| 13 | rc.go:212 | **New** | RC PATCH conflict — controller writes status 8x during reconcile. Fix committed. |
| 14 | rc.go:453 | **New?** | RC test — investigate |
| 15 | replica_set.go:232 | Yes | RS serve image — pod proxy |
| 16 | replica_set.go:534 | **New?** | RS test — investigate |
| 17 | statefulset.go:1205 | **New** | SS status endpoints — custom condition overwritten. Fix committed. |
| 18 | init_container.go:241 | Yes | Init container ready |
| 19 | pod_resize.go:857 | **New?** | Pod resize — investigate |
| 20 | runtime.go:129 | Yes | Container state after restart |
| 21-25 | output.go:263 x5 | Yes | EmptyDir perms |
| 26 | hostport.go:219 | Yes | HostPort conflict |
| 27 | proxy.go:271 | Yes | Service proxy unreachable |
| 28 | service.go:3459 | Yes | Service delete timeout |
| 29 | service.go:768 | Yes | Service endpoint unreachable |

### Summary

- **Pre-existing (same as 90.2% baseline):** ~22 failures
- **New from work queue / status writes:** ~3 (job, rc, statefulset — all resourceVersion conflicts from controllers writing status too frequently)
- **Needs investigation:** ~4 (resource_quota, disruption, rc:453, replica_set:534, pod_resize)
- **Already fixed (committed, not deployed):** SS condition merge, RC status reduction, kubelet startup cleanup

### Root Cause: Controller status write frequency

The systematic issue causing NEW failures: controllers write to storage multiple times during a single reconcile (status updates, pod creation, adoption). Each write increments resourceVersion. Conformance tests that GET→PATCH a resource hit conflicts because the controller modified it between the GET and PATCH.

K8s pattern: controllers batch changes and do ONE status update at the end of reconciliation. Our controllers do multiple mid-reconcile writes.

Fix: each controller should accumulate status changes and write once at the end of reconcile, not after each sub-operation.
