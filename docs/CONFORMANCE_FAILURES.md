# Full Conformance Failure Analysis

**Last updated**: 2026-03-21 (round 43 — exec timeout fix deployed)

## Critical Issue Found: kubectl exec hanging
The kubelet exec handler's output stream collection hung indefinitely
because bollard's Docker exec stream didn't close after command completion.
FIX DEPLOYED: Added 30-second timeout to exec output collection.

The exec hang was causing the entire test suite to stall on any test
that uses kubectl exec (StatefulSet probe manipulation, etc.).

## 33 fixes deployed in round 43:
1-31: Previous fixes (GC, pod resize, JSON decode, PATCH RV, list filtering,
      subpath, controller intervals, chunking, CreateContainerError retry,
      pagination, CronJob status, 410 Expired, kubelet 2s sync, RV consistency,
      remainingItemCount nil, Ready=False conditions, readOnlyRootFs,
      observedGeneration, StatefulSet readyReplicas, CSIDriver delete,
      DaemonSet status RV, container CMD/Entrypoint, CRD body parsing,
      status sub-routes, watch transient errors, CRD serde, ephemeral containers)
32. Kubelet exec: always use attached mode for start_exec
33. Kubelet exec: 30s timeout on output collection to prevent hanging
