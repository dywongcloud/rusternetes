# Full Conformance Failure Analysis

**Last updated**: 2026-03-21 (round 39 starting — 23 fixes deployed)

## Round 38: Only 1 failure (StatefulSet readyReplicas)
Previous 4 failures reduced to 1 thanks to Ready=False conditions fix.
The remaining failure: StatefulSet readyReplicas counting phase instead
of Ready condition. FIXED in round 39.

## All 23 fixes deployed:
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
11. Chunking token expiry (5 min) with fresh token
12. etcd auto-compaction (5m periodic)
13. CreateContainerError preserved + retry on sync
14. PodTemplate pagination (limit/continue/410)
15. CronJob status.active ObjectReferences
16. 410 reason: "Expired" not "Gone"
17. Kubelet sync interval: 10s → 2s
18. Pagination: consistent resourceVersion across pages
19. Last page remainingItemCount: nil not 0
20. Pod conditions: Ready=False when probes fail
21. readOnlyRootFilesystem in Docker HostConfig
22. observedGeneration from metadata.generation
23. StatefulSet readyReplicas: check Ready condition not phase
