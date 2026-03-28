# Conformance Issue Tracker

**288 total fixes** | Round 105 in progress | 25 failures

## Fixed by pending deploy (#282-288)
| Test | Fix |
|------|-----|
| Events API delete collection (events.go:217) | **#286** MicroTime microseconds |
| Secret fsGroup perms (output.go:282) | **#288** fsGroup g+rX not g+rwX |
| Pod generation (pods.go:556) | **#287** generation=1 on creation |
| ResourceClaim apply-patch (conformance.go:888) | **#282** YAML content-type |
| Container status empty (runtime.go:162) | **#283** preserve status |
| Termination msg FallbackToLogsOnError | **#283** same fix |
| Aggregated discovery CRDs (x2) | **#285** dynamic CRD groups |
| Variable expansion (expansion.go:345) | **#284** kubelet sync timeout |
| Webhook BeforeEach (x3) | **#284** kubelet sync timeout |
| Scheduler preemption (x3) | **#284** kubelet sync timeout |
| HostPort conflicts | **#284** kubelet sync timeout |
| Session affinity NodePort | **#284** kubelet sync timeout |
| Deployment lifecycle | **#284** kubelet sync timeout |
| Scheduler predicates | **#284** kubelet sync timeout |

## Still unfixed (~5 failures)
| Test | Error | Root cause |
|------|-------|------------|
| statefulset.go:786 | Watch ordering timeout | Watch cache event delivery |
| CRD OpenAPI (x2) | CRD protobuf timeout | CRD protobuf decoder |
| CRD watch | Watch events | CRD watch delivery |
| kubectl guestbook/patch (x2) | kubectl protobuf OpenAPI | OpenAPI format |
| Job success policy | Job completion | Job tracking |
| Pod InPlace Resize | Container resize | Resize implementation |

## Pending deploy (#282-288)
| # | Fix |
|---|-----|
| 282 | Status PATCH accepts apply-patch+yaml |
| 283 | Preserve container status when removed |
| 284 | Kubelet sync timeouts (10s/30s) |
| 285 | Aggregated discovery dynamic CRD groups |
| 286 | MicroTime always .000000 |
| 287 | generation=1 on creation |
| 288 | fsGroup g+rX not g+rwX |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 25 | ~300/441 | IN PROGRESS — ~20 fixed by pending deploy |
