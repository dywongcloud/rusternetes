# Conformance Failure Tracker

**Round 148** | In Progress | 2026-04-16
All 37 fixes deployed.

## Failures (Round 148)

_Tracking as tests run..._

| # | Test | Error | Root Cause | Fix |
|---|------|-------|-----------|-----|

## Progress History

| Round | Pass | Fail | Total | Rate | Fixes |
|-------|------|------|-------|------|-------|
| 141 | 368 | 73 | 441 | 83.4% | — |
| 146 | 379 | 62 | 441 | 85.9% | 1-16 |
| 147 | 398 | 43 | 441 | 90.2% | 1-16 deployed |
| 148 | — | — | 441 | — | 1-37 deployed |

## All Fixes (37)

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
| 25 | RC orphan all-or-nothing |
| 26 | GC debug logging |
| 27 | GC owner re-verification before delete |
| 28 | Kubelet hostPort admission (Phase=Failed) |
| 29 | Scheduler extended resource checking |
| 30 | CRD OpenAPI x-kubernetes-group-version-kind |
| 31 | start_pause_container blocks until running (CRI) |
| 32 | EmptyDir container-local path for POSIX perms |
| 33 | CRD structural pruning |
| 34 | Webhook timeout "deadline" normalization |
| 35 | CRD groups in non-aggregated /apis discovery |
| 36 | Immediate pod status update after restart |
| 37 | App containers share pause network namespace |
| 38 | LIST resourceVersion uses current etcd revision |
