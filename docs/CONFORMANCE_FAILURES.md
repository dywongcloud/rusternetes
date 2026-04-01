# Conformance Issue Tracker

**Round 118** | IN PROGRESS | 103/441 done | 67 passed, 36 failed (65.0%)

## Current Failures — 28 unique

### CRD creation timeouts (4 tests)
- crd_publish_openapi.go:366, crd_watch.go:72, custom_resource_definition.go:288, field_validation.go:428
- Root cause: CRD creation takes >30s under load
- Potential fix: reduce CRD handler overhead

### Service/network (Docker Desktop limitations) (3 tests)
- proxy.go:271 — service proxy unreachable
- util.go:182 — service networking
- dns_common.go:476 — rate limiter from informer retries

### StatefulSet (3 tests)
- statefulset.go:2479 — timing race (FIXED 805c044, not deployed)
- statefulset.go:381 — status.replicas not updated
- statefulset.go:1092 — rolling update not triggering

### Webhook (3 tests)
- webhook.go:1244, 1631, 2338 — webhook service reachability/TLS

### Pod startup/latency (3 tests)
- pod_client.go:216 — pod creation timeout
- preemption.go:1025, 516 — replicas unavailable
- replica_set.go:232 — pod connectivity

### Pending fix (not deployed) (5 tests)
- sysctl.go:153 — FIXED d165195 (report all errors)
- predicates.go:1102 — FIXED d165195 (Unschedulable condition)
- limit_range.go:141 — FIXED c99e0db (separate pod defaulting from LimitRange)
- builder.go:97 — kubectl protobuf (not fixable without real protobuf)
- expansion.go:419 — CreateContainerError status (FIXED 8af3c12)

### Other (7 tests)
- job.go:1251 — etcd watch stream ending (Complete not observed)
- output.go:263 — Docker Desktop permissions
- service_accounts.go:151,792 — SA token pod-name extra info
- pods.go:600 — WebSocket exec channel ordering (status before stdout)
- init_container.go:565 — init container failure handling

## Pending Fixes (not deployed)

| Fix | Commit | Tests |
|-----|--------|-------|
| StatefulSet one-at-a-time scale-down | 805c044 | statefulset.go:2479 |
| Scheduler Unschedulable condition | d165195 | predicates.go:1102 |
| Sysctl validate all names | d165195 | sysctl.go:153 |
| LimitRange pod defaulting separation | c99e0db | limit_range.go:141 |
| CreateContainerError preserved | 8af3c12 | expansion.go:419 |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 116 | 128 | 94 | 222/441 | 57.7% |
| 117 | 89 | 44 | 133/441 | 66.9% |
| 118 | 64 | 34 | 98/441 | 65.3% (in progress) |
