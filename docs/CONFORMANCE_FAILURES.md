# Conformance Issue Tracker

**Round 118** | IN PROGRESS | 113/441 done | 75 passed, 38 failed (66.4%)

## Current Failures — 32 unique

### CRD timeouts (5 tests)
| Test | Status |
|------|--------|
| crd_publish_openapi.go:366 | CRD creation >30s |
| crd_watch.go:72 | CRD creation timeout |
| custom_resource_definition.go:288 | CRD creation timeout |
| field_validation.go:428 | CRD creation timeout |
| field_validation.go (sysctl portion) | FIXED d165195 not deployed |

### Service/network — Docker Desktop (4 tests)
| Test | Status |
|------|--------|
| proxy.go:271 | Service proxy unreachable |
| service.go:4291 | Service unreachable |
| util.go:182 | Service networking |
| output.go:263 | macOS bind mount permissions |

### StatefulSet (3 tests)
| Test | Status |
|------|--------|
| statefulset.go:2479 | FIXED 805c044 (one-at-a-time scale) — not deployed |
| statefulset.go:381 | status.replicas update latency |
| statefulset.go:1092 | Rolling update hash — need deploy + debug |

### Webhook (3 tests)
| Test | Status |
|------|--------|
| webhook.go:1244 | Webhook readiness timeout |
| webhook.go:1631 | Webhook readiness timeout |
| webhook.go:2338 | Webhook readiness timeout |

### Pod latency (5 tests)
| Test | Status |
|------|--------|
| pod_client.go:216 | Pod creation timeout |
| preemption.go:516,1025 | Replicas unavailable |
| replica_set.go:232 | Pod connectivity |
| dns_common.go:476 | Rate limiter exhausted |

### Pending fixes — not deployed (4 tests)
| Test | Fix |
|------|-----|
| sysctl.go:153 | d165195 — report all errors |
| predicates.go:1102 | d165195 — Unschedulable condition |
| limit_range.go:141 | c99e0db — pod defaulting separated |
| expansion.go:419 | 8af3c12 — CreateContainerError preserved |

### Other (8 tests)
| Test | Issue |
|------|-------|
| builder.go:97 | kubectl protobuf (unfixable without real protobuf) |
| job.go:1251 | etcd watch stream ending — FIXED 4991385 keepalive, not deployed |
| init_container.go:565 | Init container failure handling |
| service_accounts.go:151,792 | SA token pod-name — needs TokenRequest API from kubelet |
| kubelet.go:127 | Terminated reason empty (fix deployed but not working) |
| pods.go:600 | WebSocket exec channel order — FIXED 4d7f7e3, not deployed |
| resource_quota.go:489 | Quota status.used not updated |

## Pending Fixes (not deployed)

| Fix | Commit | Tests |
|-----|--------|-------|
| StatefulSet scale-down | 805c044 | 1 |
| Scheduler Unschedulable | d165195 | 1 |
| Sysctl all errors | d165195 | 1 |
| LimitRange separation | c99e0db | 1 |
| CreateContainerError | 8af3c12 | 1 |
| etcd keepalive | 4991385 | ~4 (CRD + job timeouts) |
| WebSocket exec delay | 4d7f7e3 | 1 |
| **Total** | | **~10** |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 116 | 128 | 94 | 222/441 | 57.7% |
| 117 | 89 | 44 | 133/441 | 66.9% |
| 118 | 75 | 38 | 113/441 | 66.4% (in progress) |
