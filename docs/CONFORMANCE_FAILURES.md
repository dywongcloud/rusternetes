# Conformance Issue Tracker

**Round 117** | IN PROGRESS | 128/441 done | 87 passed, 41 failed (68.0%)

## Not Yet Deployed Fixes (17 expected additional passes)

| Fix | Commit | Tests |
|-----|--------|-------|
| Watch MODIFIED→ADDED label selector bug | ce2f9d3 | IngressClass, Ingress, VAP(2), FlowSchema, EndpointSlice = 6 |
| Duplicate LimitRange removal | 3215a6c | LimitRange = 1 |
| Webhook TLS accept self-signed | d6b0c60 | webhook(4) = 4 |
| CRD async status update | 213585c | CRD(3) = 3 |
| Field validation duplicate format | c182bfd | field_validation = 1 |
| kubectl Content-Type | b2f9538 | builder = 1 |
| CSR condition String type | 319466f | certificates = 1 |
| PDB DisruptionBudget cause | 2bc8ef4 | disruption = 1 |

## Current Failures — 41 total, 38 unique locations

### Fixed by pending deploys (~17 tests)
- ingressclass.go:375, ingress.go:232, validatingadmissionpolicy.go:814,270
- flowcontrol.go:661, endpointslice.go:409
- field_validation.go:105, builder.go:97, certificates.go:372
- crd_publish_openapi.go:161,77,451
- webhook.go:601,1194,1334,1631
- limit_range.go:162, disruption.go:372

### Docker Desktop limitations (~5 tests)
- service.go:1450,1571,4291 — iptables DNAT bypassed by userspace networking
- output.go:263 — macOS bind mount umask
- pre_stop.go:153 — service networking required

### Pod latency / rate limiting (~8 tests)
- hostport.go:219 — init container failed (hostPort conflict)
- rc.go:538,623 — rate limiter exhausted
- replica_set.go:232,738 — pod connectivity / timed out
- wait.go:63 — rate limiter
- preemption.go:1025 — replicas unavailable
- dns_common.go:476 — rate limiter exhausted

### StatefulSet rolling update (~3 tests)
- statefulset.go:957,1092 — template hash not changing after patch
- statefulset.go:2479 — timing race

### Other (~5 tests)
- job.go:974 — adopt/release CAS (fix pending: 2bc8ef4)
- watch.go:223 — second watch notification not delivered
- expansion.go:419 — subpath CreateContainerError not observed by test
- aggregator.go:359 — sample API server pod not ready

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 116 | 128 | 94 | 222/441 | 57.7% |
| 117 | 87 | 41 | 128/441 | 68.0% (in progress) |
| 117+deploy | ~104 | ~24 | ~128/441 | ~81% (projected) |
