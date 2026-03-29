# Conformance Issue Tracker

**Round 110** | IN PROGRESS | 336 fixes deployed

## Round 110 Live Failures (10 failures / 17 tests — 41% fail rate)

| # | File | Line | Error |
|---|------|------|-------|
| 1 | `statefulset.go` | 2479 | `scaled 3 -> 2 replicas` |
| 2 | `crd_publish_openapi.go` | 202 | `failed to create CRD: context deadline exceeded` |
| 3 | `custom_resource_definition.go` | 104 | `creating CustomResourceDefinition: context deadline exceeded` |
| 4 | `custom_resource_definition.go` | 288 | `creating CustomResourceDefinition: context deadline exceeded` |
| 5 | `builder.go` | 97 | `exit status 1` (x2) |
| 6 | `expansion.go` | 419 | pod startup timeout |
| 7 | `runtime.go` | 115 | `Pod didn't start within time out period` |
| 8 | `proxy.go` | 503 | `context deadline exceeded` |
| 9 | `daemon_set.go` | 473 | (new — need error detail) |

## By Root Cause

### CRD protobuf decode (3 failures: #2, #3, #4)
All 30-second timeouts. Protobuf fix deployed but some CRD payloads still not converting correctly.

### Pod startup timeout (3 failures: #6, #7, #8)
Pods not starting fast enough. Happened during initial kubelet startup overload. Kubelet stable now — new pods reach Ready in ~20s.

### StatefulSet rolling update (1 failure: #1)
Rolling update guard deployed but not preventing the issue.

### kubectl validation (2 failures: #5)
OpenAPI JSON fix deployed but kubectl create from STDIN still failing for some resources.

### DaemonSet (1 failure: #9)
New — need to investigate.

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 109 | 48* | 78* | 38%* |
| 110 | 10 | 17 | 41% (in progress) |

*Round 109 incomplete
