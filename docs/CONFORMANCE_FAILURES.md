# Conformance Issue Tracker

**298 total fixes** | Round 106 IN PROGRESS | 5 failures so far (down from 43 in Round 105!)

## Deployed Fixes
Fixes #1-297 deployed in current build. Round 105 fixes #282-297 now active.

## Round 106 Failures (5 so far)

### New issues to fix
| Test | Error | Status |
|------|-------|--------|
| statefulset.go:786 | SS scaled unexpectedly 3→2 + readiness probe timeout=0 | **FIXING #298** — probe timeout=0 treated as 0s not 1s default |
| CRD FieldSelectors | CRD protobuf creation timeout | CRD protobuf decoder issue |
| ResourceQuota terminating scopes | ResourceQuota with terminating scopes | NEW — needs investigation |
| kubectl replace | Update single-container pod image | NEW — needs investigation |
| Proxy v1 | Pod/service proxy responses | NEW — needs investigation |

## Pending deploy
| # | Fix |
|---|-----|
| 298 | Probe timeout_seconds=0 treated as default 1s (not 0s) |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 43 | 441 | 90% |
| 106 | 5 | ~50/441 | ~90% so far (IN PROGRESS) |
