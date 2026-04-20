# Conformance Failure Tracker

## Current Run (Round 153 — release builds, etcd, clean state)

**Status at 57min: 42 passed, 4 failed, 46/441 done (91.3%)**

### Failures

| # | Test | Duration | Error | Root Cause | Fix Status |
|---|------|----------|-------|------------|------------|
| 1 | deployment.go:1259 | 304s | RS availableReplicas not converging | Extra phase==Running check in is_pod_available(). K8s only checks Ready + minReadySeconds + not-terminating. | **Fixed** (committed d7162b4, not in this run) |
| 2 | daemon_set.go:494 | 9s | DS rollback without restarts | DS rolling update missing revision tracking — no updateRevision/currentRevision in status, no hash snapshot for rollout. | Needs fix |
| 3 | webhook.go:1481 | 20s | kubectl attach denied — broken pipe | Attach webhook GVR uses pods/attach subresource. Pre-existing. | Pre-existing |
| 4 | daemon_set.go:1276 | 8s | DS rolling update pod count | Same root cause as #2 — DS revision tracking incomplete. | Needs fix |

### Fixes committed but not in this run

- RS availableReplicas: removed extra phase==Running check (d7162b4)

### New failure discovered during run

| # | Test | Error | Root Cause |
|---|------|-------|------------|
| 5 | subpath projected volume | mount-tester sees configmap content instead of written file | subPath mount binds entire directory instead of specific file. Docker bind mount needs file-level targeting for projected volume subPaths. |

## Known Architectural Limitations

| Issue | Reason |
|-------|--------|
| Pod resize cgroup | Docker cgroup paths differ from K8s |
| HostPort conflict | Timing-dependent |

## Previous Results

| Round | Pass | Fail | Total | Rate | Notes |
|-------|------|------|-------|------|-------|
| 149 | 398 | 43 | 441 | 90.2% | Pre-work-queue baseline |
| 152 | 266 | 42 | 308* | 86.4% | *Killed by external restart |
| 153 | 42 | 4 | 46 | 91.3% | Running — already above baseline |
