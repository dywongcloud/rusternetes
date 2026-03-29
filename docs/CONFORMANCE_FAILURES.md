# Conformance Issue Tracker

**Round 110** | IN PROGRESS | 285/441 tests | 181 passed, 104 failed (63.5% pass)

## All Failures by Category

| Category | Count | Fix Status |
|----------|-------|------------|
| Webhook readiness | 8 | Fix committed — scheme lowercase, no_proxy, custom headers |
| CRD timeout | 7 | Fix committed — synchronous status update before response |
| Network/service | 6 | Fix committed — deployment status aggregation for readiness |
| Job | 6 | Fix committed — suspend, deadline, preserve completion status |
| Preemption/scheduler | 6 | Fix committed — deployment status for availableReplicas |
| kubectl builder | 5 | Fix committed — protobuf envelope wrapping |
| StatefulSet | 4 | Fix committed — partition, parallel policy, rolling update guard |
| Field validation | 4 | Fix committed — dotted paths, combined error messages |
| Pod resize | 3 | Fix committed — PATCH content-type X-Original-Content-Type |
| SA token | 3 | Fix committed — projected token with pod binding, audience relaxation |
| ReplicaSet | 3 | Fix committed — deployment status aggregation |
| RC | 3 | Fix committed — CAS retry on condition clear |
| Deployment | 3 | Fix committed — full status aggregation, TypeMeta injection |
| Aggregated discovery | 3 | Fix committed — correct group field in resource entries |
| Runtime status | 2 | Fix committed — CAS re-reads, container status persistence |
| DNS | 2 | Fix committed — CoreDNS pod readiness, endpoints populated |
| Proxy | 2 | Fix committed — deployment readiness for service backends |
| Pod client | 2 | Fix committed — ephemeral container PATCH content-type |
| Volume perms | 2 | Fix committed — tmpfs mode=1777 for all emptyDir |
| Watch | 1 | Fix committed — synthetic ADDED for label-filtered objects |
| Service latency | 1 | Fix committed — selector always serialized (not skip_serializing) |
| LimitRange | 1 | Fix committed — default request fallback to limits |
| Events | 1 | Fix committed — field selector on event list |
| Sysctl | 1 | Fix committed — Forbidden error type for unsafe sysctls |
| Hostport | 1 | Fix committed — hostIP binding from pod spec |
| Secrets volume | 1 | Fix committed — deletion handling in volume resync |
| /etc/hosts | 1 | Fix committed — tar upload to pause container |
| Namespace | 1 | Fix committed — cascade finalization |
| Resource quota | 1 | Fix committed — scoped quotas, status calculation |
| Aggregator | 1 | Fix committed — deployment status for readiness |
| kubectl logs | 1 | Fix committed — no trailing newline |
| RuntimeClass | 1 | Fix committed — CAS re-reads for pod status |
| DaemonSet | 2 | Fix committed — ControllerRevision creation, terminal pod handling |
| Lifecycle hook | 1 | Fix committed — preStop HTTP hook execution |
| Kubelet | 1 | Fix committed — CAS re-reads, readiness persistence |
| Exec util | 1 | Fix committed — WebSocket exec channel protocol |
| Endpoints | 1 | Fix committed — endpoint controller timing |
| Node pods | 2 | Fix committed — SPDY channel ordering, pod lifecycle |
| Expansion | 1 | Fix committed — CAS re-reads for env var expansion pods |
| Controller revision | 1 | Fix committed — ControllerRevision count from DaemonSet |

**All 104 failures have fix commits. Need rebuild + redeploy to verify.**

## Fixes Committed (not deployed)

10 fix commits targeting all 104 failures. Key fixes:
- CRD: Synchronous status update eliminates watch race condition
- Webhook: Scheme lowercase + no_proxy for readiness probes
- Deployment: Full status aggregation with availableReplicas
- SA token: Projected tokens include pod binding info
- Field validation: Dotted paths + combined error messages
- Watch: Synthetic ADDED/DELETED for label selector changes
- Service: Selector always serialized for deserialization compatibility

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 110 | 104 | 285/441 | 63.5% (in progress) |
