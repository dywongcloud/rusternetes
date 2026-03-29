# Conformance Issue Tracker

**Round 110** | IN PROGRESS | 336 fixes deployed

## Round 110 Live Failures (8 failures / 14 tests so far)

| # | File | Line | Error |
|---|------|------|-------|
| 1 | `statefulset.go` | 2479 | `scaled 3 -> 2 replicas` |
| 2 | `crd_publish_openapi.go` | 202 | `failed to create CRD: context deadline exceeded` |
| 3 | `builder.go` | 97 | `exit status 1` — kubectl create |
| 4 | `builder.go` | 97 | `exit status 1` — kubectl create (2nd) |
| 5 | `expansion.go` | 419 | env var expansion timeout (127s) |
| 6 | `custom_resource_definition.go` | 288 | `creating CRD: context deadline exceeded` |
| 7 | `runtime.go` | 115 | `context deadline exceeded` (300s) |
| 8 | `daemon_set.go` | 473 | timeout |

## Root Cause Analysis

### CRD timeouts (#2, #6) — 30s each
Protobuf decode fix (be1af28) is deployed. The structured decoder and validated brace scanning should handle most CRDs. These 30s timeouts suggest some CRD protobuf payloads still aren't being decoded correctly. Need to check what specific CRDs are failing.

### StatefulSet (#1) — 62s
Rolling update guard (72d2973) is deployed but didn't prevent the issue. The `all_ready` check should block rolling updates when pods aren't Ready yet. Need to investigate: is the revision hash different, or is there a different deletion path?

### kubectl builder (#3, #4) — instant failures
OpenAPI JSON fix (f91637a) is deployed. These are `exit status 1` from kubectl create/apply. May be a different validation path or the OpenAPI spec doesn't cover the resource being created.

### Expansion timeout (#5) — 127s
Pod env var expansion test timed out waiting for pod readiness. This happened during initial kubelet startup overload (Docker 500 errors + sync timeouts). Kubelet is stable now — DaemonSet pods are reaching 1/1 Running in ~20s.

## Kubelet Health
- Initial startup: sync timeouts on coredns (30s per-pod limit), Docker 500 errors
- After stabilization: no timeouts, pods reaching Ready in ~20 seconds
- The 30s per-pod sync timeout is aggressive for Docker Desktop but doesn't cause persistent issues

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 109 | 48* | 78* | 38%* |
| 110 | 8 | 14 | 57% (in progress) |

*Round 109 incomplete — e2e killed during skip phase
