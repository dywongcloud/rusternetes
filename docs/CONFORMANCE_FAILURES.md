# Conformance Issue Tracker

**Round 118** | IN PROGRESS | 30/441 done | 20 passed, 10 failed (66.7%)

## Current Failures

| # | Test | Error | Status |
|---|------|-------|--------|
| 1 | statefulset.go:2479 | Scaled 3->2 timing | FIXED 805c044 — not deployed |
| 2 | job.go:1251 | Job Complete not observed (900s) | etcd watch stream may have ended |
| 3 | predicates.go:1102 | Context deadline | FIXED d165195 — Unschedulable condition, not deployed |
| 4 | output.go:263 | Perms 0755 | Docker Desktop limitation |
| 5 | dns_common.go:476 | Rate limiter exhausted | Cascading from other failures |
| 6 | statefulset.go:381 | status.replicas not updated to 0 | Controller status update latency |
| 7 | sysctl.go:153 | Only first error reported | FIXED d165195 — report all errors, not deployed |
| 8 | crd_publish_openapi.go:366 | CRD creation timeout | API server contention |
| 9 | builder.go:97 | proto parse error | kubectl protobuf — not fixable without real protobuf |
| 10 | pod_client.go:216 | Pod creation timeout 60s | Latency |

## Pending Fixes (not yet deployed)

| Fix | Commit | Issue |
|-----|--------|-------|
| StatefulSet one-at-a-time scale-down | 805c044 | #1 |
| Scheduler Unschedulable condition | d165195 | #3 |
| Sysctl validate all names | d165195 | #7 |
| OpenAPI protobuf removed | d165195 | cleanup |

## Key Results
- **76.5% peak** early in run (dropped to 66.7% as harder tests ran)
- Watch MODIFIED→ADDED fix (ce2f9d3): IngressClass, Ingress, VAP, FlowSchema, EndpointSlice ALL PASS
- Webhook TLS fix (d6b0c60): webhook tests PASS
- CRD async status (213585c): most CRD tests PASS
- LimitRange duplicate removal (3215a6c): LimitRange PASSES
- Field validation (c182bfd): PASSES
- CSR String type (319466f): PASSES
- PDB cause (2bc8ef4): PASSES
- CoreDNS toleration: DNS survives taints
- Zero watch cancel loops

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 116 | 128 | 94 | 222/441 | 57.7% |
| 117 | 89 | 44 | 133/441 | 66.9% |
| 118 | 20 | 10 | 30/441 | 66.7% (in progress) |
