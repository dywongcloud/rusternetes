# Conformance Issue Tracker

**Round 110** | IN PROGRESS | 39/441 tests completed | 336 fixes deployed

## Round 110 Live Failures (24 failures, 15 passed)

| File | Count | Error |
|------|-------|-------|
| `custom_resource_definition.go` | 3 | CRD creation timeout (30s) |
| `builder.go` | 3 | kubectl create/apply exit status 1 |
| `statefulset.go` | 2 | scaled 3->2 replicas |
| `webhook.go` | 2 | webhook config not ready timeout |
| `crd_publish_openapi.go` | 1 | CRD creation timeout |
| `expansion.go` | 1 | pod readiness timeout |
| `runtime.go` | 1 | container status timeout (300s) |
| `pod_resize.go` | 1 | pod resize PATCH issue |
| `daemon_set.go` | 1 | DaemonSet timeout |
| `proxy.go` | 1 | proxy test timeout |
| `util.go` | 1 | network util timeout |
| `pod_client.go` | 1 | ephemeral container |
| `output.go` | 1 | volume permissions |
| `service_accounts.go` | 1 | SA token rejected |
| `job.go` | 1 | job issue |
| `resource_quota.go` | 1 | quota status |
| `aggregated_discovery.go` | 1 | discovery timeout |
| `preemption.go` | 1 | scheduler preemption |

## Fixes committed (not yet deployed)
- CRD: Fire 4 status updates without breaking (4624a26)
- TokenRequest: Default derive on spec for protobuf resilience (4624a26)
- ContainerPort: Added host_ip field to test (4624a26)

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 109 | 48* | 78* | 38%* |
| 110 | 24 | 39/441 | in progress |

*Round 109 incomplete
