# Conformance Failure Tracker

**Round 147** | In Progress — 29 failures so far | 2026-04-16
**Baseline**: Round 146 had 62 failures (85.9%)

## Fixed (pending next deploy)

These have code fixes committed but need `docker compose build` + redeploy.

| Test | Error | Fix # | Root Cause |
|------|-------|-------|-----------|
| chunking.go:194 | pagination token expired | 19 | Item count change falsely invalidated tokens |
| init_container.go:235 | PodCondition nil | 17 | Missed Succeeded path in runtime.rs |
| init_container.go:440 | exit code / restart tracking | 9 | restart_count always 0, last_state always None |
| webhook.go:2491 | missing ?timeout=Ns | 18 | Webhook URL missing timeout query param |
| webhook.go:1481 | "pods/attach" resource | 20 | Should be "pods" not "pods/attach" |
| webhook.go:2164 | CR update not denied | 22 | CR update handler didn't run validating webhooks |
| pods.go:600 | channel 3 on v1 protocol | 21 | v1 protocol doesn't use channel 3 status |
| resource_quota.go:302 | extended resource not checked | 23 | requests.example.com/foo not enforced |
| garbage_collector.go:436 | 100 pods → 34 | 25 | Orphan finalizer removed before all pods orphaned |
| crd_publish_openapi.go:285+ | schema mismatch / explain fails | 24 | CRD update lost enum field via typed serde |

## Still Failing (no fix yet)

| Test | Error | Category | Notes |
|------|-------|----------|-------|
| crd_publish_openapi.go:170, :225, :253, :77 | kubectl explain / schema | CRD OpenAPI | Fix 24 addresses enum loss; remaining may be kubectl explain discovery or residual schema mismatch |
| aggregator.go:359 | deployment not ready | Pod startup | Sample-apiserver pod fails to start |
| deployment.go:1259, :995 | RS not available / rollover | Pod startup | Pods not becoming Ready |
| daemon_set.go:1276 | 0 pods on node | DaemonSet | Only 1 eligible node found (expects 2) |
| statefulset.go:957 | pod not re-created | StatefulSet | Controller not deleting/recreating pods |
| rc.go:538 | pod not reachable | Network | Pod running but HTTP requests timeout |
| hostport.go:219 | pod2 timeout | Pod startup | Container start failure |
| output.go:263 | pod not ready | Pod startup | File permission test pod fails |
| proxy.go:503 | pod didn't start | Pod startup | Pod timeout |
| service.go:251, :768, :3459 | affinity / unreachable / delete timeout | kube-proxy | Session affinity iptables, endpoint routing |
| preemption.go:877 | observation timeout | Scheduling | Watch-dependent, may be timing |

## All Fixes (25 total)

### Deployed in Round 147 (fixes 1-16)

| # | Fix | What it fixed |
|---|-----|--------------|
| 1 | RC selector defaulting | rc.go:623, gc.go:436 — selector was null |
| 2 | Webhook matchConditions | webhook.go:932, :2222, :2164 — CEL never evaluated |
| 3 | Webhook timeout "deadline" | webhook.go:1400 — cause chain not in error |
| 4 | SMP array ordering | statefulset.go:1092 — patch items must come first |
| 5 | Pod Succeeded conditions | init_container.go:235 — missing PodInitialized |
| 6 | Defaults after mutation | webhook.go:1352 — SetDefaults only ran once |
| 7 | CRD OpenAPI items unwrap | CRD tests — extra {"schema":{}} wrapper |
| 8 | LIST resourceVersion | Systemic — timestamps instead of etcd mod_revisions |
| 9 | Init container restart tracking | init_container.go:440 — restart_count always 0 |
| 10 | ResourceQuota cpu/memory aliases | resource_quota.go:290 — "cpu" vs "requests.cpu" |
| 11 | Exec websocket Success status | exec_util.go:113 — missing channel 3 for exit 0 |
| 12 | Docker 409 wait | Pod startup — insufficient wait after removal |
| 13 | Attach webhook validation | webhook.go:1481 — attach had no webhook check |
| 14 | Per-pod sync lock | Pod startup — concurrent sync_pod races |
| 15 | Skip unchanged status | builder.go:97 — status written every 5s unchanged |
| 16 | Pod resize memory_swap | pod_resize.go:857 — Docker rejects without memory_swap |

### Pending Deploy (fixes 17-27)

| # | Fix | What it fixes |
|---|-----|--------------|
| 17 | Pod conditions runtime.rs | init_container.go:235 — missed Succeeded path |
| 18 | Webhook URL timeout param | webhook.go:2491 — missing ?timeout=Ns |
| 19 | Pagination staleness | chunking.go:194 — item count invalidated tokens |
| 20 | Webhook resource name | webhook.go:1481 — "pods/attach" → "pods" |
| 21 | v1 protocol channel 3 | pods.go:600 — v1 doesn't use channel 3 |
| 22 | CR update webhooks | webhook.go:2164 — UPDATE had no validating webhooks |
| 23 | Extended resource quota | resource_quota.go:302 — custom resources not checked |
| 24 | CRD raw JSON storage | CRD tests — update handler lost enum via typed struct |
| 25 | RC orphan all-or-nothing | gc.go:436 — finalizer removed before all pods orphaned |
| 26 | GC debug logging | DaemonSet/GC — log metadata failures and orphan reasons |
| 27 | GC owner re-verification | DaemonSet/GC — re-read owner from storage before deleting orphan |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 141 | 368 | 73 | 441 | 83.4% |
| 144 | ~375 | ~60 | 441 | ~85.1% |
| 146 | 379 | 62 | 441 | 85.9% |
| 147 | TBD | 29+ | 441 | ~93% est (in progress, fixes 1-16 deployed) |
