# Conformance Issue Tracker

**283 total fixes** | Round 105 in progress | 13 failures at 23/441 (52.2%)

## Round 105 Failures
| Test | Error | Root cause | Fix |
|------|-------|------------|-----|
| statefulset.go:786 | Watch ordering timeout | Watch cache delivery timing | Needs investigation |
| preemption.go:1025 | RS availableReplicas=0 | Scheduler preemption not implemented | Needs scheduler work |
| conformance.go:888 | apply-patch+yaml rejected | Status handler content-type | **FIXED #282** |
| aggregated_discovery.go:227 | CRD not in discovery | Dynamic CRD groups reverted | Needs re-implementation |
| crd_publish_openapi.go:244 | CRD OpenAPI timeout | CRD protobuf decoder | Needs investigation |
| output.go:282 | Secret defaultMode+fsGroup perms | fsGroup file permissions | Needs fsGroup fix |
| pods.go:556 | Pod generation mismatch | Generation field handling | Needs investigation |
| job.go:548 | Job success policy | Job completion timing | Needs investigation |
| builder.go:97 (x2) | kubectl -f - protobuf OpenAPI | OpenAPI protobuf format | **FIXED #281** pending deploy |
| runtime.go:162 | Empty container statuses | Container removed before inspect | **FIXED #283** pending deploy |
| webhook.go:1133 | Webhook deployment not ready | Webhook pod startup timing | Readiness-related |
| expansion.go:345 | subPathExpr pod not running | Annotation empty on first sync | Needs subPathExpr fix |

## Pending deploy
| # | Description |
|---|------------|
| 282 | Status PATCH accepts apply-patch+yaml for SSA |
| 283 | Preserve container status when Docker container removed |

## Progress
| Round | Fail | Pass | Total | Rate |
|-------|------|------|-------|------|
| 101 | 196 | 245 | 441 | 56% |
| 104 | 36 | ~405 | 441 | 92% |
| 105 | 13 | 12 | 23/441 | 52.2% (in progress) |
