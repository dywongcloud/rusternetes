# Conformance Failure Tracker

## Current Run (Round 153 — release builds, etcd, clean state)

**Status at ~2.5h: 199 passed, 24 failed, 223/441 done (89.2%)**

### All 24 Failures

| # | Test | Fix Status |
|---|------|------------|
| 1 | deployment.go:1259 | **Fixed** (committed, not in this run) — RS availableReplicas removed extra phase check |
| 2 | daemon_set.go:494 | **Fixed** (committed, not in this run) — K8s-style maxUnavailable budget |
| 3 | daemon_set.go:1276 | **Fixed** (committed, not in this run) — same DS rolling update fix |
| 4 | webhook.go:1481 | **Fixed** (committed, not in this run) — resource/subResource split + PodAttachOptions |
| 5 | deployment.go:1008 | Deployment rollover — needs investigation |
| 6 | crd_publish_openapi.go x7 | CRD OpenAPI — fix committed but may need more work |
| 7 | rc.go:538 | RC pod responses timeout — networking/proxy |
| 8 | replica_set.go:232 | RS serve image — networking/proxy |
| 9 | init_container.go:440 | Init container — fix committed but may need more work |
| 10 | lifecycle_hook.go:132 | Lifecycle hook — pod readiness timing |
| 11 | output.go:263 x2 | EmptyDir perms — fix committed |
| 12 | job.go:596 | Job test — needs investigation |
| 13 | hostport.go:219 | HostPort — timing |
| 14 | service_latency.go:145 | Service latency — deployment readiness |
| 15 | service.go:3459 | Service delete timeout |
| 16 | preemption.go:1025 | Preemption timeout |
| 17 | endpointslicemirroring.go:129 | EndpointSlice mirroring — needs investigation |
| 18 | apiserver.go:94 | API server network test — needs investigation |

### Fixes committed but not in this run

- RS availableReplicas: removed extra phase==Running check
- DaemonSet rolling update: K8s-style maxUnavailable budget
- Webhook: resource/subResource split + PodAttachOptions
- subPath: file-level bind mount for projected volumes

## Previous Results

| Round | Pass | Fail | Total | Rate | Notes |
|-------|------|------|-------|------|-------|
| 149 | 398 | 43 | 441 | 90.2% | Pre-work-queue baseline |
| 153 | 199 | 24 | 223* | 89.2% | *Still running. 4 failures already fixed for next run. |
