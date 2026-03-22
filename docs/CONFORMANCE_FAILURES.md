# Full Conformance Failure Analysis

**Last updated**: 2026-03-22 (round 56 completed — est. ~150/441 failures)

## Session Summary

Started this session with tests unable to complete (exec hanging).
Deployed 43+ conformance fixes across 53 commits. The full 441-test
conformance suite now runs to completion.

### Key fixes deployed:
1. GC foreground deletion + propagation policy
2. Pod resize containerStatus.resources
3. JSON decode ContainerState `{}` → None
4. PATCH resourceVersion clear
5. PodTemplate/ControllerRevision list filtering + watch + pagination
6. Subpath validation (reject `..`, absolute, backticks)
7. CronJob/StatefulSet controller 1s intervals
8. Chunking token expiry (5min) with fresh token + Expired reason
9. etcd auto-compaction
10. CreateContainerError preserved + retry
11. CronJob status.active ObjectReferences
12. Kubelet sync 2s interval
13. Pagination consistent resourceVersion + nil remainingItemCount
14. Pod conditions Ready=False when probes fail
15. readOnlyRootFilesystem
16. observedGeneration
17. StatefulSet readyReplicas check Ready condition
18. CSIDriver deletecollection + VolumeAttachment status routes
19. DaemonSet status clear RV
20. Container command→Entrypoint, args→Cmd
21. CRD manual body parsing + x-kubernetes-* serde
22. PLC/FlowSchema/CRD status sub-resource routes
23. Watch transient error handling
24. Ephemeral container support
25. **WebSocket exec with v5.channel.k8s.io** (breakthrough fix)
26. Direct Docker execution (bypass kubelet proxy)
27. Exec stream 1s timeout + inspect_exec
28. DeviceClass kind/apiVersion
29. hostIPs in pod status

### Remaining failure categories:
- Container exec output (volume content not visible via exec)
- File permissions on mounted volumes
- CRD creation (decode errors)
- Watch stream reliability
- Webhook deployment readiness
- Resource limits in cgroups
- Node schedulability during tests

### Progress track:
- Round 25: 12 failures in first 50 tests
- Round 28: 1 failure (chunking)
- Round 39: 13 failures (API/controller gaps)
- Round 52: 54 failures (first full run attempt)
- Round 53: ~155 failures (first completed run)
- Round 56: ~150 failures (completed, ~66% pass rate)
