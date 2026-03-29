# Conformance Issue Tracker

**Round 110** | IN PROGRESS | 41/441 tests completed | 336 fixes deployed

## Round 110 Live Failures (26 failures, 15 passed — 41/441 tests)

| Category | Count | Files | Error |
|----------|-------|-------|-------|
| CRD timeout | 4 | `custom_resource_definition.go`, `crd_publish_openapi.go` | CRD created but Established watch times out |
| kubectl builder | 3 | `builder.go:97` | `proto: cannot parse invalid wire-format data` — fix committed (5da5f98) |
| Webhook not ready | 3 | `webhook.go:839,1194,1244` | webhook deployment pod never reaches Ready |
| StatefulSet | 2 | `statefulset.go:2479,1092` | scaled 3->2 + other |
| Timeout/readiness | 5 | `expansion.go`, `runtime.go`, `daemon_set.go`, `proxy.go`, `util.go` | pods slow to start |
| Pod resize | 1 | `pod_resize.go:857` | PATCH issue |
| DNS | 1 | `dns_common.go:476` | DNS resolution timeout |
| Pod client | 1 | `pod_client.go:216` | ephemeral container |
| Volume perms | 1 | `output.go:263` | permissions |
| SA token | 1 | `service_accounts.go:898` | token rejected — fix committed (4624a26) |
| Job | 1 | `job.go:623` | job issue |
| Resource quota | 1 | `resource_quota.go:282` | quota status |
| Discovery | 1 | `aggregated_discovery.go:282` | discovery timeout |
| Preemption | 1 | `preemption.go:978` | scheduler |

## Fixes committed but not yet deployed
- CRD status update timing: fire all 4 updates without breaking (4624a26)
- TokenRequest defaults (4624a26)
- OpenAPI protobuf envelope wrapping (5da5f98)

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 109 | 48* | 78* | 38%* |
| 110 | 26 | 41/441 | in progress |

*Round 109 incomplete
