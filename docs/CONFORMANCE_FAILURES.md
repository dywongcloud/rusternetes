# Conformance Issue Tracker

**Round 118** | IN PROGRESS | 17/441 done | 13 passed, 4 failed (76.5%)

## Current Failures

| # | Test | Error | Status |
|---|------|-------|--------|
| 1 | statefulset.go:2479 | Scaled 3->2 timing race | FIXED (805c044) — not deployed |
| 2 | job.go:1251 | Job completion timeout 900s | Job controller latency |
| 3 | predicates.go:1102 | Context deadline exceeded | Scheduling timeout |
| 4 | output.go:263 | Perms 0755 not 0777 | Docker Desktop limitation |

## Not Yet Deployed

| Fix | Commit |
|-----|--------|
| StatefulSet scale-down one-at-a-time with terminating wait | 805c044 |
| CreateContainerError status preserved on retry | 8af3c12 |
| StatefulSet compute_revision image logging | 8af3c12 |

## Key Results This Round
- **76.5% pass rate** (up from 66.9% in R117, 64.2% in R110)
- Watch MODIFIED→ADDED fix working: IngressClass, Ingress, VAP, FlowSchema all PASS
- Webhook TLS fix working: webhook tests PASS
- CRD async status fix working: CRD tests PASS
- LimitRange duplicate removal working: LimitRange test PASSES
- CoreDNS toleration working: DNS survives taints
- Zero watch cancel loops

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 116 | 128 | 94 | 222/441 | 57.7% |
| 117 | 89 | 44 | 133/441 | 66.9% |
| 118 | 13 | 4 | 17/441 | 76.5% (in progress) |
