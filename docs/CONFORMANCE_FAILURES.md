# Conformance Issue Tracker

**Round 110** | IN PROGRESS | 265/441 tests | 168 passed, 97 failed (63.4% pass)

## All Failures by Category (97 total)

| Category | Count | Fix Status |
|----------|-------|------------|
| Webhook readiness | 8 | Fix committed (scheme lowercase, no_proxy) |
| Network/service | 6 | Deployment readiness timeout |
| Job | 6 | Fix committed (suspend, deadline, preserve completion) |
| Preemption/scheduler | 6 | Pod readiness + scheduling timeout |
| kubectl builder | 5 | Fix committed (protobuf envelope) |
| CRD timeout | 7 | Fix committed (synchronous status update) |
| StatefulSet | 4 | Fix committed (partition, parallel policy) |
| Field validation | 4 | Fix committed (dotted paths, combined errors) |
| Pod resize | 3 | Fix committed (PATCH content-type) |
| SA token | 3 | Fix committed (projected token with pod binding) |
| ReplicaSet | 3 | Scaling timeout |
| RC | 3 | Fix committed (CAS retry, condition clear) |
| Deployment | 3 | Fix committed (status aggregation, TypeMeta) |
| Aggregated discovery | 3 | Fix committed (group field in responses) |
| Runtime status | 2 | Container status timeout |
| DNS | 2 | Resolution timeout |
| Proxy | 2 | Service proxy timeout |
| Pod client | 2 | Ephemeral container |
| Volume perms | 2 | tmpfs mode=1777 |
| Watch | 1 | Fix committed (synthetic ADDED for label match) |
| Service latency | 1 | Fix committed (selector always serialized) |
| LimitRange | 1 | Fix committed (default request fallback) |
| Events | 1 | Fix committed (field selector, event list) |
| Sysctl | 1 | Fix committed (Forbidden error type) |
| Hostport | 1 | Fix committed (hostIP binding) |
| Secrets volume | 1 | Fix committed (deletion handling) |
| /etc/hosts | 1 | Fix committed (tar upload to pause) |
| Namespace | 1 | Fix committed (finalization) |
| Resource quota | 1 | Quota status format |
| Aggregator | 1 | Deployment readiness |
| kubectl logs | 1 | Fix committed (trailing newline) |
| RuntimeClass | 1 | Pod status timeout |
| DaemonSet | 1 | Timeout |
| Lifecycle hook | 1 | NEW — need to investigate |
| Kubelet | 1 | Pod status |
| Exec util | 1 | NEW — exec into container |
| Endpoints | 1 | NEW — endpoint creation |
| Node pods | 1 | Pod lifecycle |
| Expansion | 1 | Env var timeout |

## Fixes Committed (not deployed — need rebuild)

8 fix commits targeting ~60 of 97 failures. Remaining ~37 are mostly timeout/readiness issues that depend on kubelet and Docker Desktop performance.

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 110 | 97 | 265/441 | 63.4% (in progress) |
