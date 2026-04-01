# Conformance Issue Tracker

**Round 117** | IN PROGRESS | 133/441 done | 89 passed, 44 failed (66.9%)

## Pending Fixes (for next deploy)

| Fix | Commit | Tests Fixed |
|-----|--------|-------------|
| Watch MODIFIED→ADDED label selector | ce2f9d3 | 6 (IngressClass, Ingress, VAP×2, FlowSchema, EndpointSlice) |
| Duplicate LimitRange removal | 3215a6c | 1 (LimitRange) |
| Webhook TLS self-signed certs | d6b0c60 | 4 (webhook×4) |
| CRD async status update | 213585c | 3 (CRD×3) |
| Field validation format | c182bfd | 1 |
| kubectl Content-Type | b2f9538 | 1 |
| CSR condition String type | 319466f | 1 |
| PDB DisruptionBudget cause | 2bc8ef4 | 1 |
| etcd compaction 1m→10m | 10bc590 | 1 (watch.go:223) |
| CreateContainerError status preserved | 8af3c12 | 1 (expansion.go:419) |
| StatefulSet revision logging | 8af3c12 | diagnostic |
| **Total** | | **~20 tests** |

## Remaining Failures After Deploy

### Docker Desktop limitations (cannot fix)
| Test | Issue |
|------|-------|
| service.go:1450,1571,4291 | iptables DNAT bypassed by userspace networking |
| output.go:263 | macOS bind mount umask (0755 not 0777) |
| pre_stop.go:153 | Requires service networking |
| hostport.go:219 | Docker Desktop doesn't support hostIP-specific port binding |

### Pod startup latency / rate limiting
| Test | Issue |
|------|-------|
| rc.go:538,623 | Client rate limiter exhausted from informer retries |
| replica_set.go:232,738 | Pod connectivity / timed out |
| wait.go:63 | Rate limiter |
| preemption.go:1025 | Replicas not available in time |
| dns_common.go:476 | Rate limiter from informer retries |

### StatefulSet rolling update
| Test | Issue |
|------|-------|
| statefulset.go:957,1092 | Template hash unchanged after patch — logging added (8af3c12) |
| statefulset.go:2479 | Timing race at scale boundary |

### Complex test requirements
| Test | Issue |
|------|-------|
| aggregator.go:359 | Sample API server pod needs TLS certs + etcd sidecar |
| job.go:974 | Pod adopt/release — CAS retry added (2bc8ef4) |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 116 | 128 | 94 | 222/441 | 57.7% |
| 117 | 89 | 44 | 133/441 | 66.9% (in progress) |
| 118 (projected) | ~109 | ~24 | ~133 | ~82% |
