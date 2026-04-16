# Conformance Failure Tracker

**Round 147** | 398/441 (90.2%) — 43 failed | 2026-04-16
**Round 146** | 379/441 (85.9%) — 62 failed

## Issues Still Needing Fixes

| # | Test | Error | Root Cause |
|---|------|-------|-----------|
| 1 | rc.go:538 | pod running but HTTP timeout | Pod-to-pod network routing — likely downstream of Docker pause timing (fix 31) |
| 2 | proxy.go:271 | "Unable to reach service through proxy" | Service endpoints not ready — downstream of pod startup (fix 31) |
| 3 | pre_stop.go:153 | "validating pre-stop: timed out" | Test uses pod proxy which needs working pod networking |

## Issues Fixed But Not Yet Deployed (next round should pass)

| Test | Fix # | What was wrong |
|------|-------|---------------|
| webhook.go:1481 | 20 | Resource sent as "pods/attach" instead of "pods" |
| webhook.go:2164 | 22 | CR update handler didn't run validating webhooks |
| webhook.go:1400 | 34 | Timeout error missing "deadline" (reqwest says "operation timed out") |
| webhook.go:2491 | 18+34 | Missing ?timeout=Ns in URL + missing "deadline" |
| webhook.go:2222 | 33 | CRD structural pruning not implemented — unknown fields persisted |
| crd_publish_openapi.go (9 tests) | 24+30+35 | CRD update lost enum, missing GVK extension, CRD groups missing from /apis discovery |
| garbage_collector.go:436 | 25+27 | Orphan finalizer removed before all pods orphaned + GC snapshot was racy |
| resource_quota.go:302 | 23 | Extended resources (requests.example.com/foo) not checked |
| chunking.go:194 | 19 | Item count change invalidated pagination tokens |
| daemon_set.go:1276 | 27 | GC deleted DaemonSet pods (owner not in snapshot) |
| statefulset.go:957 | 28 | Kubelet didn't reject pods with hostPort conflicts |
| init_container.go:235 | 17 | Missed Succeeded path in runtime.rs |
| init_container.go:440 | 9 | restart_count always 0, last_state always None |
| pods.go:600 | 21 | v1 websocket protocol doesn't use channel 3 |
| preemption.go:877 | 29 | Scheduler didn't check extended resources (fakecpu) |
| output.go:263 (4 tests) | 32 | EmptyDir on host bind mount lost POSIX permissions |

## Issues Expected to Be Fixed by Docker Timing Fix (31)

These all fail because pods can't start — the pause container isn't fully running before app containers try to join its network namespace.

| Test | Error |
|------|-------|
| aggregator.go:359 | sample-apiserver deployment not ready |
| deployment.go:1259, :995 | RS never had desired availableReplicas |
| replica_set.go:232 | pod responses timeout |
| hostport.go:219 | pod2 startup timeout |
| proxy.go:503 | pod didn't start |
| service_latency.go:145 | deployment not ready |
| service.go:251 (3 tests) | session affinity — no endpoints (pods not starting) |
| service.go:768 | service not reachable — no endpoints |
| service.go:3459 | service delete timeout |

## All Fixes (35 total)

| # | Fix | Deployed |
|---|-----|---------|
| 1 | RC selector defaulting from template labels | R147 |
| 2 | Webhook matchConditions CEL evaluation | R147 |
| 3 | Webhook timeout "deadline" in error message | R147 |
| 4 | SMP array ordering (patch items first) | R147 |
| 5 | Pod Succeeded conditions (PodInitialized=True) | R147 |
| 6 | Defaults after mutation (SetDefaults twice) | R147 |
| 7 | CRD OpenAPI items unwrap | R147 |
| 8 | LIST resourceVersion (etcd mod_revision not timestamp) | R147 |
| 9 | Init container restart tracking | R147 |
| 10 | ResourceQuota cpu/memory aliases | R147 |
| 11 | Exec websocket Success status on channel 3 | R147 |
| 12 | Docker 409 wait (stop before remove, 500ms) | R147 |
| 13 | Attach webhook validation | R147 |
| 14 | Per-pod sync lock (prevent concurrent sync_pod) | R147 |
| 15 | Skip unchanged status writes | R147 |
| 16 | Pod resize memory_swap | R147 |
| 17 | Pod conditions runtime.rs Succeeded path | Pending |
| 18 | Webhook URL timeout param (?timeout=Ns) | Pending |
| 19 | Pagination staleness (remove item count check) | Pending |
| 20 | Webhook resource "pods" not "pods/attach" | Pending |
| 21 | v1 protocol skip channel 3 status | Pending |
| 22 | CR update validating webhooks | Pending |
| 23 | Extended resource quota checking | Pending |
| 24 | CRD raw JSON storage (preserve enum) | Pending |
| 25 | RC orphan all-or-nothing (retry if any PATCH fails) | Pending |
| 26 | GC debug logging | Pending |
| 27 | GC owner re-verification before delete | Pending |
| 28 | Kubelet hostPort admission (reject with Phase=Failed) | Pending |
| 29 | Scheduler extended resource checking | Pending |
| 30 | CRD OpenAPI x-kubernetes-group-version-kind extension | Pending |
| 31 | start_pause_container blocks until running (CRI semantics) | Pending |
| 32 | EmptyDir uses container-local path for POSIX permissions | Pending |
| 33 | CRD structural pruning (remove unknown fields) | Pending |
| 34 | Webhook timeout error normalized to include "deadline" | Pending |
| 35 | CRD groups in non-aggregated API discovery (/apis) |
| 36 | Immediate pod status update after container restart | Pending |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 141 | 368 | 73 | 441 | 83.4% |
| 146 | 379 | 62 | 441 | 85.9% |
| 147 | 398 | 43 | 441 | 90.2% |
