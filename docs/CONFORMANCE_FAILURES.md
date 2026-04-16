# Conformance Failure Tracker

**Round 147** | In Progress — 23 failures at ~200/441 | 2026-04-16
**Round 146 baseline**: 379/441 (85.9%) — 62 failed

## Round 147 Failures (current, 23 unique)

| Test | Error | Fix |
|------|-------|-----|
| chunking.go:194 | pagination token "resource version too old" | Fix 19 (pending) |
| crd_publish_openapi.go:77, :170, :225, :253, :285, :400 | kubectl explain / schema mismatch | Fix 24 (pending) — enum lost in CRD update |
| garbage_collector.go:436 | "expect 100 pods, got 34" — GC deletes orphaned pods | Needs investigation — GC controller orphan handling |
| resource_quota.go:302 | extended resource quota not checked | Fix 23 (pending) |
| webhook.go:1481 | resource "pods/attach" should be "pods" | Fix 20 (pending) |
| webhook.go:2164 | CR update didn't run validating webhooks | Fix 22 (pending) |
| webhook.go:2491 | missing ?timeout=Ns in webhook URL | Fix 18 (pending) |
| init_container.go:235 | PodCondition nil — missed runtime.rs path | Fix 17 (pending) |
| pods.go:600 | v1 protocol got channel 3 status | Fix 21 (pending) |
| output.go:263 | pod not ready ("perms of file" test) | Pod startup / Docker |
| daemon_set.go:1276 | DaemonSet pod count 0 | Needs investigation |
| deployment.go:1259 | RS never had desired availableReplicas | Pod startup / Docker |
| rc.go:538 | pod running but not reachable | Network / kube-proxy |
| hostport.go:219 | pod startup timeout | Pod startup / Docker |
| aggregator.go:359 | sample-apiserver not ready | Pod startup / Docker |
| service.go:251, :768 | session affinity issues | kube-proxy iptables |
| preemption.go:877 | pod observation timeout | Watch / timing |

## Fixes (24 total)

| # | Fix | Deployed | Root Cause |
|---|-----|---------|-----------|
| 1 | RC selector defaulting | R147 | Selector null → pods never matched |
| 2 | Webhook matchConditions | R147 | CEL never evaluated |
| 3 | Webhook timeout "deadline" | R147 | Cause chain not in error |
| 4 | SMP array ordering | R147 | Patch items must come first |
| 5 | Pod Succeeded conditions | R147 | Missing PodInitialized on Succeeded |
| 6 | Defaults after mutation | R147 | SetDefaults only ran once |
| 7 | CRD OpenAPI items unwrap | R147 | Extra {"schema":{}} wrapper |
| 8 | LIST resourceVersion | R147 | Timestamps instead of etcd mod_revisions |
| 9 | Init container restart tracking | R147 | restart_count always 0 |
| 10 | ResourceQuota cpu/memory aliases | R147 | "cpu" vs "requests.cpu" |
| 11 | Exec websocket Success status | R147 | Missing channel 3 status for exit 0 |
| 12 | Docker 409 wait | R147 | Insufficient wait after removal |
| 13 | Attach webhook validation | R147 | Attach handler missing webhook check |
| 14 | Per-pod sync lock | R147 | Concurrent sync_pod → Docker 409 |
| 15 | Skip unchanged status | R147 | Status written every 5s unchanged |
| 16 | Pod resize memory_swap | R147 | Docker rejects memory without memory_swap |
| 17 | Pod conditions runtime.rs | Pending | Missed Succeeded path in runtime.rs |
| 18 | Webhook URL timeout param | Pending | Missing ?timeout=Ns |
| 19 | Pagination staleness | Pending | Item count invalidated tokens |
| 20 | Webhook resource name | Pending | "pods/attach" → "pods" |
| 21 | v1 protocol channel 3 | Pending | v1 doesn't use channel 3 |
| 22 | CR update webhooks | Pending | UPDATE didn't run validating webhooks |
| 23 | Extended resource quota | Pending | requests.example.com/foo not checked |
| 24 | CRD raw JSON storage | Pending | Update handler lost enum via typed struct |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 141 | 368 | 73 | 441 | 83.4% |
| 144 | ~375 | ~60 | 441 | ~85.1% |
| 146 | 379 | 62 | 441 | 85.9% |
| 147 | TBD | 23+ | ~200/441 | ~90% (in progress) |
