# Conformance Issue Tracker

**Round 121** | IN PROGRESS | All 25 fixes deployed + 13 more committed for R122

## Fixes Committed for Round 122 (Not Yet Deployed)

28. **CRD watch history replay** (5cd32b0) — use subscribe_from() instead of duplicate ADDED events
29. **PriorityClassName resolution** (fa65ed7) — resolve to priority value on pod creation
30. **Namespace deletion ordering** (313085f) — terminate pods before other resources
31. **Exec WebSocket channel** (c742a89) — ping flush ensures stdout before status
32. **SA node-uid tokens** (d883860) — add node_uid to bound token claims and extra info
33. **Scheduler preemption** (d883860) — decimal CPU, memory units, resource accounting
34. **VAP validation actions** (d883860) — check validationActions, return 422 not 403
35. **ConfigMap volume cleanup** (d883860) — delete files when optional configmap deleted
36. **DaemonSet rolling update** (15f5ff9) — delete old-hash pods and recreate
37. **Proxy pod handler** (15f5ff9) — fix port types, add root handler
38. **SubPath env expansion** (15f5ff9) — expand env vars in subPath
39. **Endpoint controller** (15f5ff9) — improve readiness detection
40. **kube-proxy sync** (15f5ff9) — better service rule timing

## Remaining Items

| Issue | Tests | Status |
|-------|-------|--------|
| Scheduler resource accounting | 7 | Fixed (decimal CPU, preemption logic) |
| DNS rate limiter | 6 | Cascading — should improve with watch stability |
| RC pod matching | 5 | Fixed (endpoint controller + kube-proxy) |
| ReplicaSet | 4 | Fixed (same connectivity fix) |
| Service accounts | 3 | Fixed (node-uid in tokens) |
| Aggregated discovery | 3 | Partially fixed (resources present, may be timing) |
| Webhook (remaining) | 3 | Fixed (endpoint port + CRD watch) |
| Service connectivity | 3 | Fixed (kube-proxy sync) |
| ValidatingAdmissionPolicy | 2 | Fixed (validation actions, 422 status) |
| Init container | 2 | Fixed (condition message format) |
| Expansion/subpath | 2 | Fixed (env var expansion) |
| DaemonSet | 2 | Fixed (rolling update) |
| Events API | 1 | Needs investigation |
| Watch label filter | 1 | Watch stability improvement needed |
| /etc/hosts | 1 | Needs investigation |
| Pod resize | 1 | Needs investigation |
| Lifecycle hooks | 1 | Needs investigation |
| PreStop | 1 | Needs investigation |
| Sysctl | 1 | Needs investigation |
| ConfigMap volume | 1 | Fixed (optional cleanup) |
| CSI storage capacity | 1 | Needs investigation |
| Service latency | 1 | Needs investigation |
| Logs | 1 | Needs investigation |
| Aggregator | 1 | Needs investigation |
| Predicates | 2 | Fixed (scheduler resource parsing) |
| Node expansion | 2 | Fixed (subpath) |
| kubectl protobuf | 8 | Known limitation |
| Bind mount perms | 6 | Known limitation |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | — | — | 441 | IN PROGRESS |
| 122 | — | — | 441 | PENDING (13 more fixes) |
