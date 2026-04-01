# Conformance Issue Tracker

**Round 119** | IN PROGRESS | 8/441 done | 6 passed, 2 failed (75.0%)
Zero watch failures. All round 118 fixes deployed.

## Current Failures (Round 119)

| # | Test | Error | Status |
|---|------|-------|--------|
| 1 | statefulset.go:2479 | Scaled 3->2 timing race | Known — scale-down fix deployed but timing still tight |
| 2 | pod_client.go:216 | Pod creation timeout 60s | Pod startup latency |

## Deployed Fixes This Round

56 fixes deployed including:
- etcd gRPC keepalive (4991385)
- ConfigMap webhook pipeline (fac86a3)
- SA token bound in Secret volume (0a30348)
- ResourceQuota enforcement in all controllers (7985cf9)
- CrashLoopBackOff exponential backoff (fa0122b)
- Stale webhook config cleanup on NS delete (88f9c37)
- Deployment direct pod counting (36ff92b)
- Namespace two-cycle finalizer removal (2a0ff37)
- DaemonSet updatedNumberScheduled fix (9451c4e)
- Volume refresh dir creation + key deletion (9451c4e)
- Event reportingController alias (2d6a5e1)
- StatefulSet scale-down one-at-a-time (805c044)
- Scheduler Unschedulable condition (d165195)
- Sysctl all errors reported (d165195)
- LimitRange pod defaulting separation (c99e0db)
- CreateContainerError status preserved (8af3c12)
- WebSocket exec channel delay (4d7f7e3)

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 119 | 6 | 2 | 8/441 | 75.0% (in progress) |
