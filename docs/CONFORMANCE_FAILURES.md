# Conformance Issue Tracker

**Round 110** | IN PROGRESS | 336 fixes deployed

## Round 110 Live Failures (6 failures / 11 tests so far)

| # | File | Line | Duration | Error |
|---|------|------|----------|-------|
| 1 | `statefulset.go` | 2479 | 62s | `scaled 3 -> 2 replicas` |
| 2 | `crd_publish_openapi.go` | 202 | 30s | `failed to create CRD: context deadline exceeded` |
| 3 | `builder.go` | 97 | 0.5s | `exit status 1` — kubectl create |
| 4 | `builder.go` | 97 | 0.6s | `exit status 1` — kubectl create (2nd occurrence) |
| 5 | `expansion.go` | 419 | 127s | env var expansion — pod readiness timeout |
| 6 | `custom_resource_definition.go` | 288 | 30s | `creating CustomResourceDefinition: context deadline exceeded` |

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
| 110 | 6 | 11 | 45% (in progress, early) |

*Round 109 incomplete — e2e killed during skip phase
