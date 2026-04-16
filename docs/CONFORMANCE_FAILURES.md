# Conformance Failure Tracker

**Round 147** | In Progress — 10 failures at ~100/441 (90.2%) | 2026-04-16
**Round 146 baseline**: 379/441 (85.9%) — 62 failed

## Round 147 Failures (so far)

| Test | Error | Status |
|------|-------|--------|
| chunking.go:194 | pagination token "resource version too old" | Fix 19 pending deploy |
| crd_publish_openapi.go:225 | kubectl explain can't find CRD resource | Needs investigation |
| crd_publish_openapi.go:400 | CRD OpenAPI schema | Needs investigation |
| garbage_collector.go:436 | "expect 100 pods, got 34" — GC deletes orphaned pods | Needs investigation |
| webhook.go:2491 | error missing ?timeout=Ns in URL | Fix 18 pending deploy |
| init_container.go:235 | PodCondition nil — missed Succeeded path in runtime.rs | Fix 17 pending deploy |
| rc.go:538 | pod running but not reachable over network | Needs investigation |
| hostport.go:219 | pod startup timeout | Needs investigation |
| service.go:251 | session affinity not working | Needs investigation |
| aggregator.go:359 | sample-apiserver deployment not ready | Needs investigation |

## Fixes Applied

| # | Fix | Root Cause |
|---|-----|-----------|
| 1 | RC selector defaulting | K8s defaults selector from template labels; ours was null |
| 2 | Webhook matchConditions | CEL conditions never evaluated |
| 3 | Webhook timeout "deadline" | Cause chain not in error message |
| 4 | SMP array ordering | Patch items must come first |
| 5 | Pod Succeeded conditions | Missing PodInitialized on Succeeded |
| 6 | Defaults after mutation | K8s runs SetDefaults twice |
| 7 | CRD OpenAPI items unwrap | Extra {"schema":{}} wrapper |
| 8 | LIST resourceVersion | Timestamps instead of etcd mod_revisions (1123 watch failures) |
| 9 | Init container restart tracking | restart_count always 0 |
| 10 | ResourceQuota cpu/memory aliases | "cpu" vs "requests.cpu" |
| 11 | Exec websocket Success status | Missing channel 3 status for exit 0 |
| 12 | Docker 409 wait | Insufficient wait after container removal |
| 13 | Attach webhook validation | Attach handler missing webhook check |
| 14 | Per-pod sync lock | Concurrent sync_pod → Docker 409 races |
| 15 | Skip unchanged status | Status written every 5s even when unchanged |
| 16 | Pod resize memory_swap | Docker rejects memory update without memory_swap |
| 17 | Pod conditions runtime.rs path | Missed Succeeded path in runtime.rs (fix 5 only covered kubelet.rs) |
| 18 | Webhook URL timeout param | Missing ?timeout=Ns (test checks URL in error) |
| 19 | Pagination staleness | Item count change invalidated tokens; K8s only uses timeout |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 141 | 368 | 73 | 441 | 83.4% |
| 143 | 372 | 69 | 441 | 84.4% |
| 144 | ~375 | ~60 | 441 | ~85.1% |
| 146 | 379 | 62 | 441 | 85.9% |
| 147 | ~400 | ~10 | ~100/441 | 90.2% (in progress) |
