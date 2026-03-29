# Conformance Issue Tracker

**Round 110** | IN PROGRESS | 13 failures / 22 tests (59% fail rate)

**REGRESSION WARNING**: Round 107 had 19 failures / ~430 tests (~96% pass). We are now at 59% failure rate. Our changes caused regressions.

## Round 110 Live Failures

| # | File | Line | Error |
|---|------|------|-------|
| 1 | `statefulset.go` | 2479 | `scaled 3 -> 2 replicas` |
| 2 | `crd_publish_openapi.go` | 202 | `failed to create CRD: context deadline exceeded` |
| 3 | `custom_resource_definition.go` | 104 | `creating CRD: context deadline exceeded` |
| 4 | `custom_resource_definition.go` | 288 | `creating CRD: context deadline exceeded` |
| 5 | `builder.go` | 97 | `exit status 1` (x2) |
| 6 | `expansion.go` | 419 | pod readiness timeout (127s) |
| 7 | `runtime.go` | 115 | container status timeout (300s) |
| 8 | `proxy.go` | 503 | `context deadline exceeded` |
| 9 | `daemon_set.go` | 473 | timeout |
| 10 | `service_accounts.go` | 898 | `server rejected our request` — TokenRequest API broken |
| 11 | `pod_client.go` | 302 | ephemeral container timeout |
| 12 | `resource_quota.go` | (new) | quota failure |

## Root Cause: Regressions from Our Changes

Since Round 107 (96% pass), we changed 45 files with 7500+ insertions. Key changes that likely caused regressions:

1. **Kubelet 30s per-pod sync timeout** — kills long-running sync operations, preventing pods from completing startup
2. **Kubelet container cleanup on deletion** — adds stop_and_remove_pod overhead during pod deletion
3. **TokenRequest handler rewrite** — changed expiration calculation and response format
4. **Middleware protobuf scanning** — complete rewrite may have edge cases
5. **ServiceAccountClaims struct changes** — added new fields that old tokens can't deserialize

## Action Needed

The priority should be to identify which changes broke passing tests and revert/fix them, not to add more features. The 96% pass rate from Round 107 was the baseline — we need to get back there first.

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 109 | 48* | 78* | 38%* |
| 110 | 13 | 22 | 41% (in progress) |

*Round 109 incomplete
