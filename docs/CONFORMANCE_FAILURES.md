# Conformance Failure Tracker

**Round 147** | In Progress — 33+ failures so far | 2026-04-16
**Baseline**: Round 146 = 62 failures (85.9%)

## Fixed (pending deploy) — 12 fixes

| Test | Fix # | Root Cause |
|------|-------|-----------|
| chunking.go:194 | 19 | Item count change falsely invalidated pagination tokens |
| init_container.go:235 | 17 | Missed Succeeded path in runtime.rs |
| webhook.go:2491 | 18 | Webhook URL missing ?timeout=Ns query param |
| webhook.go:1481 | 20 | Resource should be "pods" not "pods/attach" |
| webhook.go:2164 | 22 | CR update handler didn't run validating webhooks |
| pods.go:600 | 21 | v1 protocol doesn't use channel 3 status |
| resource_quota.go:302 | 23 | Extended resources (requests.example.com/foo) not checked |
| crd_publish_openapi.go:285+ | 24 | CRD update lost enum field via typed serde round-trip |
| garbage_collector.go:436 | 25+27 | RC finalizer removed before all pods orphaned + GC snapshot-based orphan detection was racy |
| daemon_set.go:1276 | 27 | GC deleted DaemonSet pods (owner existed but missed in snapshot) |
| statefulset.go:957 | 28 | Kubelet didn't check hostPort conflicts at admission |

## Still Failing — no code fix available

| Test | Error | Root Cause |
|------|-------|-----------|
| aggregator.go:359 | deployment not ready | Pod startup failure (Docker 409) — fix 14 deployed but may need more |
| deployment.go:1259, :995 | RS not available | Pod startup failure — downstream of Docker 409 |
| rc.go:538 | pod not reachable | Network issue — pod running but HTTP timeout |
| hostport.go:219 | pod2 timeout | Pod startup failure — Docker 409 |
| output.go:263 | file perms -rw-r--r-- not -rw-rw-rw- | macOS Docker bind mount doesn't support 0666 mode |
| proxy.go:503 | pod didn't start | Pod startup failure |
| service.go:251, :768 | affinity issues | No endpoints — backend pods can't start (Docker 409) |
| service.go:3459 | delete timeout | Watch/timing |
| preemption.go:877 | observation timeout | Scheduler preemption — only 1 of 2 pods created |
| crd_publish_openapi.go:170, :225, :253, :77 | kubectl explain / schema | May need kubectl explain discovery endpoint |

## All Fixes (28 total)

### Deployed in Round 147 (1-16)

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

### Pending Deploy (17-28)

| # | Fix |
|---|-----|
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

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 141 | 368 | 73 | 441 | 83.4% |
| 146 | 379 | 62 | 441 | 85.9% |
| 147 | TBD | 33+ | 441 | ~93% est (in progress, fixes 1-16 deployed) |
