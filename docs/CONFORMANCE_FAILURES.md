# Full Conformance Failure Analysis

**Last updated**: 2026-03-21 (round 41 starting — 31 fixes deployed)

## Round 40: 1 failure (StatefulSet watch closed)
Watch stream breaks on empty etcd responses. FIXED — watch now
continues on transient errors instead of breaking.

## All 31 fixes deployed:
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
23. StatefulSet readyReplicas: check Ready condition
24. CSIDriver deletecollection route
25. DaemonSet/status: clear resourceVersion
26. Container command→Entrypoint, args→Cmd
27. CRD: manual body parsing + x-kubernetes-* serde
28. PLC/FlowSchema/CRD: /status sub-resource routes
29. Watch: continue on transient errors
30. CRD JSONSchemaProps: x-kubernetes-validations
31. Ephemeral container support in kubelet
