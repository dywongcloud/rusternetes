# Conformance Failure Tracker

## Round 153 Result

**Last checkpoint: 199 passed, 24 failed, 223/441 (89.2%) — still running**

All 24 failures have fixes committed for next run.

## Fixes Ready for Round 154

| Fix | Tests Addressed |
|-----|----------------|
| RS availableReplicas — removed extra phase==Running check | deployment.go:1259 |
| DaemonSet rolling update — K8s maxUnavailable budget | daemon_set.go:494, daemon_set.go:1276 |
| Webhook attach — resource/subResource split + PodAttachOptions | webhook.go:1481 |
| subPath — file-level bind mount for projected volumes | subpath tests |
| EndpointSlice mirroring — mirror manually-created Endpoints | endpointslicemirroring.go:129 |
| Bootstrap kubernetes EndpointSlice | apiserver.go:94 |
| Deployment rollover — proportional replica calculation | deployment.go:1008 |
| CRD OpenAPI — standard properties (metadata, apiVersion, kind) | crd_publish_openapi.go x7 |
| Kube-proxy — watch endpoints + endpointslices, 10s resync | service networking tests |
| Init container status — preserve CrashLoopBackOff state | init_container.go:440 |
| Scheduler preemption — immediate binding after eviction | preemption.go:1025 |

## Known Architectural Limitations

| Issue | Reason |
|-------|--------|
| Pod resize cgroup | Docker cgroup paths differ from K8s |

## Previous Results

| Round | Pass | Fail | Total | Rate | Notes |
|-------|------|------|-------|------|-------|
| 149 | 398 | 43 | 441 | 90.2% | Pre-work-queue baseline |
| 153 | ~199 | ~24 | ~223* | 89.2% | *Still running. All failures have fixes committed. |
| 154 | — | — | — | — | Pending — all fixes applied |
