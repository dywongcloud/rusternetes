# Conformance Issue Tracker

**Round 110** | IN PROGRESS | 90/441 tests | 36 passed, 54 failed (40% pass)

## Round 110 Live Failures by Category

| Category | Count | Fix Status |
|----------|-------|------------|
| kubectl builder | 4 | Fix committed (5da5f98) — protobuf envelope wrapping |
| CRD timeout | 5 | Fix committed (4624a26) — status update timing |
| Webhook not ready | 3 | Fix committed (7266a9e) — scheme lowercase + no_proxy |
| Network/service | 4 | Timeout — pods may not be reachable fast enough |
| StatefulSet | 2 | Rolling update guard not sufficient |
| SA tokens | 2 | Fix committed (f65ab7b) — TokenRequest resilience |
| Pod resize | 2 | PATCH content-type fix deployed |
| RC | 2 | Fix committed (f65ab7b) — CAS retry on condition clear |
| Pod client | 2 | Ephemeral container |
| Volume perms | 2 | tmpfs mode=1777 deployed |
| Proxy | 2 | Timeout |
| CRD field validation | 1 | Protobuf scanning fix deployed |
| Expansion | 1 | Pod startup timeout |
| Runtime | 1 | Container status timeout |
| DaemonSet | 1 | Timeout |
| DNS | 1 | Resolution timeout |
| Job | 1 | SuccessPolicy |
| LimitRange | 1 | Fix committed (f65ab7b) — default request fallback |
| Quota | 1 | Status mismatch |
| Preemption | 1 | Scheduler |
| Predicates | 1 | Scheduler |
| ReplicaSet | 1 | Scaling |
| RuntimeClass | 1 | Timeout |
| Events | 1 | Event format |
| Discovery | 1 | Aggregated discovery |
| Node pods | 1 | Pod lifecycle |

## Fixes committed (not yet deployed — need rebuild)
| Commit | Fix |
|--------|-----|
| 4624a26 | CRD: 4 status updates, TokenRequest defaults |
| 5da5f98 | OpenAPI: protobuf envelope for kubectl |
| f65ab7b | RC CAS retry, SA token validation, LimitRange defaults |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 109 | 48* | 78* | 38%* |
| 110 | 54 | 90/441 | 40% pass (in progress) |

*Round 109 incomplete
