# Conformance Issue Tracker

**Round 94**: 16 FAIL, 0 PASS (running) | **165 fixes** (162 deployed + 3 pending) | Watch bookmark "0" bug found

## Round 94 Failures (14 so far)

| # | Test | Error | Root Cause | Status |
|---|------|-------|-----------|--------|
| 1 | cronjob.go:110 | stale CronJob data | Old etcd entries from prev run | Need clean start |
| 2 | statefulset.go:786 | timed out | Watch stream timeout | Watch reliability issue |
| 3 | output.go:263 | perms -rwxrwxrwx expected | Volume file permissions wrong | Volume defaultMode |
| 4 | custom_resource_definition.go:72 | protobuf 415 rejected | 415 doesn't work, client doesn't retry | **FIX #163 PENDING** TypeMeta extraction |
| 5 | runtime.go:169 | termination message empty | Docker cp from stopped container | Termination msg |
| 6 | service_cidrs.go:177 | resource not found | Missing /status route | **FIX #164 PENDING** |
| 7 | builder.go:97 (×2) | kubectl create protobuf | Protobuf body can't be parsed | Related to #4 |
| 8 | deployment.go:585 | can't locate deployment | Watch timeout after status update | Watch reliability |
| 9 | proxy.go:503 | pod timeout | Pod didn't become Ready in time | Kubelet/watch |
| 10 | preemption.go:268 | can't schedule | Scheduler preemption not working | Scheduler bug |
| 11 | field_validation.go:700 | CRD decode error | Protobuf body | Related to #4 |
| 12 | expansion.go:419 | container failure wait | subPathExpr pod condition | Kubelet |
| 13 | namespace.go:339 | missing applied annotation | SSA doesn't set last-applied-configuration | **FIX #164 PENDING** |

## Critical Issues

### Watch stream reliability (causes timeouts: #2, #8, #9)
Pods DO start and become Ready (deployment became Available in 6s).
But watch-based waiters time out because the watch stream closes prematurely.
The K8s client's retrywatcher gets "context canceled" and can't reconnect reliably.
This blocks ~30% of all tests that use watches to wait for conditions.

### Protobuf handling (#4, #7, #11)
K8s Go client sends protobuf-encoded bodies for CRDs and other built-in types.
Our middleware can't extract JSON from pure-protobuf payloads.
Fix #163 extracts TypeMeta from the protobuf envelope to construct minimal JSON.
This won't fully solve CRD creation but gives better errors than 415.

### Scheduler preemption (#10)
"No suitable node found" even for basic pods with priority classes.
The scheduler's resource accounting or preemption logic has a bug.

## Fixes Pending Deploy (2)

| # | Fix | Impact |
|---|-----|--------|
| 163 | Protobuf: extract TypeMeta instead of 415 | 4+ tests (CRD/kubectl) |
| 164 | ServiceCIDR status route + SSA last-applied-configuration | 2 tests |

## Previous Fixes (deployed in round 94)

162 fixes deployed including:
- CRITICAL: resourceVersion uses etcd mod_revision (not timestamps)
- CRITICAL: Kubelet doesn't set Failed on transient sync errors
- Watch history via etcd watch_from_revision
- All 25 deletecollection routes registered
- Node addresses + nodeInfo in heartbeat
- Secret/ConfigMap immutability
- Namespace phase/Terminating
- PVC deserialization defaults
- PATCH dry-run, deployment Recreate, init container restart
- Session affinity, job successPolicy, sysctls
- And 150+ other fixes
