# Conformance Issue Tracker

**283 total fixes** | Round 105 in progress | 16 failures at ~100/441

## Round 105 Failures (16 unique)

### Kubelet sync blocking (4 failures)
| Test | Error | Root cause |
|------|-------|------------|
| expansion.go:345 | Pod not running in 2min | Kubelet sync blocked 4min on slow Docker op |
| webhook.go:1133 (x2) | Webhook deployment not ready | Same kubelet blocking issue |
| runtime.go:162 | Empty container statuses | Container removed before status capture — **FIXED #283** |

### Scheduler preemption (2 failures)
| Test | Error | Root cause |
|------|-------|------------|
| preemption.go:1025 | RS availableReplicas=0 | Scheduler preemption not implemented |
| preemption.go:268 | Critical pod preemption | Same — preemption not implemented |

### CRD/API machinery (4 failures)
| Test | Error | Root cause |
|------|-------|------------|
| aggregated_discovery.go:227 | CRD not in discovery | Dynamic CRD groups code was reverted |
| crd_publish_openapi.go:244 | CRD OpenAPI timeout | CRD protobuf decoder |
| crd_watch.go:72 | CRD watch test | CRD watch event delivery |
| conformance.go:888 | apply-patch+yaml rejected | **FIXED #282** — status accepts YAML |

### kubectl/CLI (2 failures)
| Test | Error | Root cause |
|------|-------|------------|
| builder.go:97 | kubectl -f - protobuf OpenAPI | kubectl expects protobuf OpenAPI v2 |
| kubectl patch | Guestbook app | kubectl create/apply issues |

### Node/Pod (2 failures)
| Test | Error | Root cause |
|------|-------|------------|
| pods.go:556 | Pod generation mismatch | Generation field handling |
| job.go:548 | Job success policy | Job completion tracking |

### Storage/Volume (1 failure)
| Test | Error | Root cause |
|------|-------|------------|
| output.go:282 | Secret defaultMode+fsGroup | fsGroup file permissions |

### StatefulSet (1 failure)
| Test | Error | Root cause |
|------|-------|------------|
| statefulset.go:786 | Watch ordering timeout | Watch cache event delivery timing |

## Pending deploy
| # | Description |
|---|-------------|
| 282 | Status PATCH accepts apply-patch+yaml for SSA |
| 283 | Preserve container status when Docker container removed |

## Progress
| Round | Fail | Pass | Done | Total | Rate |
|-------|------|------|------|-------|------|
| 101 | 196 | 245 | 441 | 441 | 56% |
| 104 | 36 | ~405 | 441 | 441 | 92% |
| 105 | 16 | ? | ~100 | 441 | IN PROGRESS |
