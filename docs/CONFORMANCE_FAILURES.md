# Full Conformance Failure Analysis

**Last updated**: 2026-03-20 (round 29 starting)

## Round 28 Results: Only 1 failure (chunking, now fixed)

Round 28 had only 1 failure out of all tests that ran before we killed it:
- Chunking compaction: PodTemplate list handler lacked pagination, so continue
  token was never issued. FIXED — added full pagination support.

All other previously-failing tests PASSED:
- CronJob ForbidConcurrent: PASSED (1s reconcile interval)
- StatefulSet scaling: didn't appear (may not have run yet, or passed)
- Variable Expansion subpath: PASSED (CreateContainerError preserved)
- Pod update JSON decode: PASSED
- Pod patch resourceVersion: PASSED
- PodTemplate lifecycle: PASSED (list filtering)
- ControllerRevision lifecycle: PASSED (list filtering)
- GC foreground deletion: PASSED

## Round 29: All 13 fixes deployed

### Complete fix list:
1. GC foreground deletion + find_orphans
2. Pod resize containerStatus.resources
3. JSON decode ContainerState `{}` → None
4. PATCH resourceVersion clear for optimistic concurrency
5. PodTemplate list: Query params, watch, filtering, pagination
6. ControllerRevision list: Query params, watch, filtering
7. Subpath validation: reject `..` and absolute paths
8. CronJob controller: 10s → 1s reconcile
9. StatefulSet controller: 5s → 1s reconcile
10. Chunking compaction: 5-minute token expiry with fresh 410 token
11. etcd auto-compaction: 5m periodic
12. CreateContainerError preserved by sync loop
13. PodTemplate pagination with limit/continue/410 Gone

### Remaining known issues (no fix yet):
- PreStop hook timeout enforcement
- CRD FieldValidation rejection
- ResourceQuota tracking speed
- Services endpoints same port/different protocol
