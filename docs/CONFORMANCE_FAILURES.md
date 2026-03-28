# Conformance Issue Tracker

**282 total fixes** | Round 105 in progress | 11 failures at ~200/441

## Round 105 Failures
| Test | Error | Root cause |
|------|-------|------------|
| statefulset.go:786 | Watch event ordering timeout | Watch cache delivery timing |
| preemption.go:1025 | RS availableReplicas=0 | Scheduler preemption not implemented |
| conformance.go:888 | ResourceClaim apply-patch+yaml | **FIXED #282** — status handler accepts YAML |
| aggregated_discovery.go:227 | CRD not in discovery | Dynamic CRD groups (#274 reverted) |
| crd_publish_openapi.go:244 | CRD OpenAPI timeout | CRD protobuf decoder |
| output.go:282 | Secret defaultMode+fsGroup perms | fsGroup file permissions |
| pods.go:556 | Pod generation mismatch | Pod generation field issue |
| job.go:548 | Job success policy | Job completion timing |
| builder.go:97 | kubectl -f - protobuf OpenAPI | OpenAPI protobuf format |
| runtime.go:162 | Empty container statuses | Container exits before status write |
| webhook.go:1133 | Webhook deployment not ready | Webhook pod startup timing |

## Pending deploy
| # | Description |
|---|------------|
| 282 | Status PATCH accepts apply-patch+yaml for SSA |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 101 | 196 | 441 | 56% |
| 103 | 30 | 76 | 60% |
| 104 | 36 | 441 | 92% |
| 105 | 11 | ~200/441 | ~95% (in progress) |
