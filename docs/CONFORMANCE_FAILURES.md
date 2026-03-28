# Conformance Issue Tracker

**291 total fixes** | Round 105 in progress | 31 failures at ~440/441

## Fixed by pending deploy (#282-291) — ~23 failures
| Category | Tests | Fix |
|----------|-------|-----|
| Kubelet sync blocking | Webhook (x5), Deployment (x2), RS, Scheduler (x3), HostPort, SessionAffinity, Service, VarExpansion | **#284** kubelet sync timeouts |
| Events API MicroTime | Events delete collection | **#286** MicroTime .000000 |
| Secret fsGroup perms | Secret defaultMode+fsGroup | **#288** fsGroup g+rX |
| Pod generation | Pod generation (x2) | **#287** generation=1 |
| ResourceClaim SSA | apply-patch+yaml | **#282** YAML content-type |
| Container status | Termination msg, runtime | **#283** preserve status |
| Aggregated discovery | CRD discovery (x2) | **#285** dynamic CRD groups |
| Job success policy | Job succeededIndexes | **#289** count matching only |
| Pod resize | InPlace resize | **#290** resize status |

## Unfixed (~8 failures)
| Test | Error | Root cause |
|------|-------|------------|
| statefulset.go:786 | Watch ordering timeout | Watch cache event delivery |
| CRD OpenAPI (x2) | CRD protobuf timeout | CRD protobuf decoder |
| CRD watch | CRD protobuf timeout | CRD protobuf decoder |
| CRD field selectors | CRD protobuf timeout | CRD protobuf decoder |
| kubectl guestbook | kubectl protobuf OpenAPI | OpenAPI format |
| kubectl patch | kubectl issues | CLI handling |

## Pending deploy (#282-291)
| # | Fix |
|---|-----|
| 282 | Status PATCH accepts apply-patch+yaml |
| 283 | Preserve container status when removed |
| 284 | Kubelet sync timeouts (10s/30s) |
| 285 | Aggregated discovery dynamic CRD groups |
| 286 | MicroTime always .000000 |
| 287 | generation=1 on creation |
| 288 | fsGroup g+rX not g+rwX |
| 289 | Job successPolicy matching indexes only |
| 290 | Pod resize status update |
| 291 | CRD protobuf debug logging |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 31 | ~440/441 | ~93% pre-deploy, ~98% est post-deploy |
