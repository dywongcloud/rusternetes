# Conformance Issue Tracker

**Round 117** | IN PROGRESS | 113/441 done | 75 passed, 38 failed (66.4%)

## Current Failures (Round 117) — 33 unique

### Will be fixed on next deploy (8 tests)
| Test | Error | Fix |
|------|-------|-----|
| ingressclass.go:375 | ADDED not MODIFIED | ce2f9d3 — label selector bug |
| ingress.go:232 | ADDED not MODIFIED | ce2f9d3 |
| validatingadmissionpolicy.go:814,270 | ADDED not MODIFIED | ce2f9d3 |
| flowcontrol.go:661 | ADDED not MODIFIED | ce2f9d3 |
| endpointslice.go:409 | ADDED not MODIFIED | ce2f9d3 |
| field_validation.go:105 | Wrong error format | c182bfd — duplicate vs unknown |
| builder.go:97 | MIME error | b2f9538 — Content-Type |
| crd_publish_openapi.go:161,77 | CRD timeout | 213585c — async status update |
| certificates.go:372 | Unknown variant | 319466f — CSR String type |

### Docker Desktop limitations (5 tests)
| Test | Error | Reason |
|------|-------|--------|
| service.go:1450,1571,4291 | Service unreachable | iptables DNAT bypassed by Docker userspace networking |
| output.go:263 | Perms 0755 not 0777 | macOS bind mount umask |
| pre_stop.go:153 | Timed out | Service networking |

### StatefulSet rolling update (3 tests)
| Test | Error | Status |
|------|-------|--------|
| statefulset.go:957,1092 | Pod not re-created | Template hash comparison — needs deploy + debug |
| statefulset.go:2479 | Scaled 3->2 | Timing race |

### Webhook service reachability (3 tests)
| Test | Error | Status |
|------|-------|--------|
| webhook.go:601,1334,1631 | Timed out | Webhook service not reachable from API server |

### Pod startup/latency (5 tests)
| Test | Error | Status |
|------|-------|--------|
| hostport.go:219 | Pod not starting | Latency |
| rc.go:538,623 | Unavailable replicas | Pod startup |
| replica_set.go:232,738 | Failed/timed out | Pod startup/informer |
| wait.go:63 | Rate limiter | Client rate limiting |
| preemption.go:1025 | Replicas unavailable | Pod readiness |

### Other (4 tests)
| Test | Error | Status |
|------|-------|--------|
| job.go:974 | Pod not released | Job adopt/release CAS — 2bc8ef4 |
| disruption.go:372 | Wrong cause format | PDB DisruptionBudget — 2bc8ef4 |
| watch.go:223 | No 2nd notification | Watch delivery |
| expansion.go:419 | Container didn't fail | Subpath validation |

## Not Yet Deployed Fixes

| Fix | Commit | Tests Fixed |
|-----|--------|-------------|
| **Watch MODIFIED→ADDED** | ce2f9d3 | 6 tests |
| Field validation format | c182bfd | 1 test |
| Content-Type | b2f9538 | 1 test |
| CRD async status | 213585c | 2 tests |
| CSR String type | 319466f | 1 test |
| PDB cause + Job CAS | 2bc8ef4 | 2 tests |
| Watch param logging | dd468e2 | diagnostics |

**Total: ~13 tests expected to be fixed on next deploy**

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 116 | 128 | 94 | 222/441 | 57.7% |
| 117 | 71 | 34 | 105/441 | 67.6% (in progress) |
