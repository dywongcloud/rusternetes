# Conformance Issue Tracker

**Round 110** | IN PROGRESS | 336 fixes deployed

## Round 110 Live Failures

| # | File | Error | Status |
|---|------|-------|--------|
| 1 | `statefulset.go:2479` | `scaled 3 -> 2 replicas` | Rolling update guard deployed but still failing |
| 2 | `crd_publish_openapi.go:202` | `failed to create CRD: context deadline exceeded` | Protobuf fix deployed but still failing |
| 3 | `builder.go:97` | `exit status 1` — kubectl create failed | OpenAPI JSON fix deployed but still failing |
| 4 | `expansion.go:419` | env var expansion timeout | Likely pod readiness timeout |

**4 failures / 8 tests so far (50% fail rate)**

Tests still running — monitoring.

## Observations
- StatefulSet (#1): Rolling update guard fix (72d2973) IS in this build but didn't help. Need deeper investigation.
- CRD (#2): Protobuf fix (be1af28) IS in this build but CRDs still timing out. Protobuf conversion may still produce incomplete JSON for some CRD types.
- kubectl (#3): OpenAPI fix (f91637a) IS in this build. The builder.go error is from kubectl applying a YAML file, possibly a different code path than what we fixed.
- Expansion (#4): Pod waiting for readiness — may resolve once kubelet syncs catch up.

## Kubelet Issue Found
```
WARN kubelet::kubelet: Timeout syncing pod coredns (30s), skipping to next pod
WARN kubelet::kubelet: Transient error syncing pod sonobuoy: Docker responded with status code 500
```
The 30s per-pod timeout is too aggressive — Docker inspect responses are large and slow on Docker Desktop. Sequential pod syncing with 30s timeout per pod causes 2.5+ minute sync cycles with 5+ pods.

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 109 | 48* | 78* | 38%* |
| 110 | 4 | 8 | 50% (in progress) |

*Round 109 incomplete — e2e killed during skip phase
