# Full Conformance Failure Analysis

**Last updated**: 2026-03-21 (round 32 starting)

## Round 31 Results: 2 failures
1. StatefulSet scaling — rate limiter timeout (15+ min test)
2. Chunking compaction — 410 reason was "Gone" not "Expired" (FIXED)

## Round 32: 16 fixes deployed

### Fix list:
1. GC foreground deletion + propagation policy
2. GC find_orphans (ALL owners gone)
3. Pod resize containerStatus.resources
4. JSON decode ContainerState `{}` → None
5. PATCH resourceVersion clear
6. PodTemplate list: Query params, watch, filtering
7. ControllerRevision list: Query params, watch, filtering
8. Subpath validation: reject `..` and absolute paths
9. CronJob controller: 10s → 1s reconcile
10. StatefulSet controller: 5s → 1s reconcile
11. Chunking token expiry (5 min) with fresh token in 410
12. etcd auto-compaction (5m periodic)
13. CreateContainerError preserved + retry on sync loop
14. PodTemplate pagination (limit/continue/410 Gone)
15. CronJob status.active with ObjectReferences
16. 410 reason: "Expired" not "Gone" (for IsResourceExpired)

### Remaining known issues:
- StatefulSet scaling test times out (15+ min, may be test-client rate limit)
- PreStop hook timeout enforcement
- CRD FieldValidation
