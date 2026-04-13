# Conformance Failure Tracker

**Round 137** | Running | 2026-04-13
**Baseline**: Round 135 = 373/441 (84.6%), Round 136 = ABORTED (preemption killed e2e)

## Round 137 Failures

_Tracking failures as they are identified from the running conformance tests._

_(none yet — monitoring)_

## Staged for Round 138 (not yet deployed)

| Commit | Fix | K8s Ref |
|--------|-----|---------|
| fb9728d | Preemption — K8s "remove all, reprieve" victim selection | default_preemption.go:233-300 |
| fb9728d | Preemption — proper grace period (not forced 0) | preemption.go:177-219 |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 104 | 405 | 36 | 441 | 91.8% |
| 127 | 397 | 44 | 441 | 90.0% |
| 132 | 363 | 78 | 441 | 82.3% |
| 133 | 370 | 71 | 441 | 83.9% |
| 134 | 370 | 71 | 441 | 83.9% |
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | TBD | TBD | 441 | TBD |
