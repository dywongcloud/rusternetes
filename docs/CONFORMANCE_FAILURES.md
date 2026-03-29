# Conformance Issue Tracker

**Round 110** | IN PROGRESS | 52/441 tests completed | 336 fixes deployed

## Round 110 Live Failures (33 failures, 19 passed — 52/441 tests)

| Category | Count | Error |
|----------|-------|-------|
| CRD timeout | 4 | CRD created but Established watch times out at 30s |
| kubectl builder | 3 | `proto: cannot parse invalid wire-format data` — fix committed |
| Webhook not ready | 3 | webhook deployment pod readiness timeout |
| StatefulSet | 2 | scaled 3->2, SS ordering |
| Service accounts | 2 | token request rejected |
| Network/service | 2 | service not reachable |
| Timeout/startup | 7 | expansion, runtime, daemon_set, proxy, util, DNS, runtimeclass |
| Pod resize | 1 | PATCH issue |
| Pod lifecycle | 2 | pod_client, pods |
| Volume perms | 1 | output.go |
| Job | 1 | job issue |
| RC | 1 | replication controller |
| Resource quota | 1 | quota status |
| Discovery | 1 | aggregated discovery |
| LimitRange | 1 | limit range |
| Preemption | 1 | scheduler |

## Fixes committed but not yet deployed
- CRD: Fire 4 status updates without breaking early (4624a26)
- TokenRequest: Default derive on spec (4624a26)
- OpenAPI: Protobuf envelope wrapping for kubectl (5da5f98)

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 109 | 48* | 78* | 38%* |
| 110 | 33 | 52/441 | in progress |

*Round 109 incomplete
