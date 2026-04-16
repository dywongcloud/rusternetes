# Conformance Failure Tracker

**Round 147** | Complete — 398/441 passed (90.2%) — 43 failed | 2026-04-16
**Baseline**: Round 146 = 62 failures (85.9%)

## Still Failing (43 tests) — Round 147

| # | Test | Category |
|---|------|----------|
| 1 | webhook.go:1481 — deny attaching pod | Webhook admission |
| 2 | webhook.go:2164 — deny CR creation/update/deletion | Webhook admission |
| 3 | webhook.go:1400 — deny pod and configmap creation | Webhook admission |
| 4 | webhook.go:2491 — honor timeout | Webhook admission |
| 5 | webhook.go:2222 — mutate CR with pruning | Webhook admission |
| 6 | aggregator.go:359 — Sample API Server | Aggregator proxy |
| 7 | crd_publish_openapi.go:451 — removes def when version not served | CRD OpenAPI |
| 8 | crd_publish_openapi.go:400 — updates spec when version renamed | CRD OpenAPI |
| 9 | crd_publish_openapi.go:225 — preserving unknown fields at root | CRD OpenAPI |
| 10 | crd_publish_openapi.go:253 — preserving unknown in embedded | CRD OpenAPI |
| 11 | crd_publish_openapi.go:77 — CRD with validation schema | CRD OpenAPI |
| 12 | crd_publish_openapi.go:170 — CRD without validation schema | CRD OpenAPI |
| 13 | crd_publish_openapi.go:285 — multiple CRDs different groups | CRD OpenAPI |
| 14 | crd_publish_openapi.go:366 — same group/version different kinds | CRD OpenAPI |
| 15 | crd_publish_openapi.go:318 — same group different versions | CRD OpenAPI |
| 16 | garbage_collector.go:436 — orphan pods from RC | GC |
| 17 | resource_quota.go:302 — capture life of a pod | ResourceQuota |
| 18 | chunking.go:194 — continue after compaction | Pagination |
| 19 | daemon_set.go:1276 — RollingUpdate pod update | DaemonSet |
| 20 | deployment.go:1259 — proportional scaling | Deployment |
| 21 | deployment.go:995 — rollover | Deployment |
| 22 | replica_set.go:232 — basic image on each replica | ReplicaSet |
| 23 | rc.go:538 — basic image on each replica | ReplicationController |
| 24 | statefulset.go:957 — recreate evicted statefulset | StatefulSet |
| 25 | hostport.go:219 — no conflict different hostIP/protocol | HostPort |
| 26 | proxy.go:503 — valid responses for pod and service Proxy | Proxy |
| 27 | proxy.go:271 — proxy through service and pod | Proxy |
| 28 | service_latency.go:145 — endpoint latency not very high | Service |
| 29 | service.go:251 — switch session affinity NodePort | Service |
| 30 | service.go:251 — switch session affinity ClusterIP | Service |
| 31 | service.go:3459 — service status lifecycle | Service |
| 32 | service.go:251 — session affinity for NodePort | Service |
| 33 | service.go:768 — serve basic endpoint from pods | Service |
| 34 | runtime.go:129 — container exit expected status | Node/Runtime |
| 35 | init_container.go:235 — invoke init on RestartNever | Init container |
| 36 | init_container.go:440 — not start app if init fails RestartAlways | Init container |
| 37 | pods.go:600 — remote command over websockets | Exec |
| 38 | pre_stop.go:153 — call prestop when killing pod | Node lifecycle |
| 39 | preemption.go:877 — preemption running path | Scheduling |
| 40 | output.go:263 — EmptyDir (non-root,0666,default) | EmptyDir perms |
| 41 | output.go:263 — EmptyDir (non-root,0777,default) | EmptyDir perms |
| 42 | output.go:263 — EmptyDir (root,0666,default) | EmptyDir perms |
| 43 | output.go:263 — EmptyDir (root,0777,default) | EmptyDir perms |

## All Fixes (31 total, all deployed in Round 147)

| # | Fix |
|---|-----|
| 1 | RC selector defaulting from template labels |
| 2 | Webhook matchConditions CEL evaluation |
| 3 | Webhook timeout "deadline" in error message |
| 4 | SMP array ordering (patch items first) |
| 5 | Pod Succeeded conditions (PodInitialized=True) |
| 6 | Defaults after mutation (SetDefaults twice) |
| 7 | CRD OpenAPI items unwrap |
| 8 | LIST resourceVersion (etcd mod_revision not timestamp) |
| 9 | Init container restart tracking |
| 10 | ResourceQuota cpu/memory aliases |
| 11 | Exec websocket Success status on channel 3 |
| 12 | Docker 409 wait (stop before remove, 500ms) |
| 13 | Attach webhook validation |
| 14 | Per-pod sync lock (prevent concurrent sync_pod) |
| 15 | Skip unchanged status writes |
| 16 | Pod resize memory_swap |
| 17 | Pod conditions runtime.rs Succeeded path |
| 18 | Webhook URL timeout param (?timeout=Ns) |
| 19 | Pagination staleness (remove item count check) |
| 20 | Webhook resource "pods" not "pods/attach" |
| 21 | v1 protocol skip channel 3 status |
| 22 | CR update validating webhooks |
| 23 | Extended resource quota checking |
| 24 | CRD raw JSON storage (preserve enum) |
| 25 | RC orphan all-or-nothing (retry if any PATCH fails) |
| 26 | GC debug logging |
| 27 | GC owner re-verification before delete |
| 28 | Kubelet hostPort admission (reject with Phase=Failed) |
| 29 | Scheduler extended resource checking |
| 30 | CRD OpenAPI x-kubernetes-group-version-kind extension |
| 31 | start_pause_container blocks until running (CRI RunPodSandbox semantics) |
| 32 | EmptyDir uses container-local path for POSIX permission support |
| 33 | CRD structural pruning (remove unknown fields after webhook mutation) |
| 34 | Webhook timeout error normalized to include "deadline" |
| 35 | CRD groups in non-aggregated API discovery (/apis) |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 141 | 368 | 73 | 441 | 83.4% |
| 146 | 379 | 62 | 441 | 85.9% |
| 147 | 398 | 43 | 441 | 90.2% (all 31 fixes deployed) |
