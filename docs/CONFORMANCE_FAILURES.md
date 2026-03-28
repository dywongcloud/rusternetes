# Conformance Issue Tracker

**290 total fixes** | Round 105 in progress | 28 failures at ~400/441

## Pending deploy (#282-290) — 9 fixes
| # | Fix | Tests |
|---|-----|-------|
| 282 | Status PATCH accepts apply-patch+yaml for SSA | 1 |
| 283 | Preserve container status when Docker container removed | 2 |
| 284 | Kubelet sync timeouts (10s/30s) prevent 4min blocking | ~12 |
| 285 | Aggregated discovery includes dynamic CRD groups | 2 |
| 286 | MicroTime always includes .000000 microseconds | 1 |
| 287 | set_initial_generation always sets generation=1 | 1 |
| 288 | fsGroup chmod uses g+rX not g+rwX | 1 |
| 289 | Job successPolicy counts only matching indexes | 1 |
| 290 | Pod resize updates status with allocatedResources | 1 |

## Remaining unfixed (~6 failures)
| Test | Error | Root cause |
|------|-------|------------|
| statefulset.go:786 | Watch ordering timeout | Watch cache event delivery |
| CRD OpenAPI/watch (x3) | CRD protobuf timeout | CRD protobuf decoder |
| kubectl guestbook/patch (x2) | kubectl protobuf OpenAPI | OpenAPI format |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 28 | ~400/441 | IN PROGRESS — ~22 fixed by pending deploy |
