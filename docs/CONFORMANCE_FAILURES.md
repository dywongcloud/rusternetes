# Conformance Failure Tracker

**Round 138** | Running | 2026-04-13
**Baseline**: Round 137 = ~380/441 (~86.2%), 61 failures

## Round 138 Failures (20 so far at ~half complete)

### CRD OpenAPI — 5 failures — FIX STAGED ✅ (not yet deployed)
- `crd_publish_openapi.go:77,161,211,285,400`
- **Root cause**: Schema includes empty strings and false booleans that K8s omits via Go omitempty
- **Fix**: 86b048a — strip ALL Go omitempty defaults (with test). Not deployed in this round.

### Webhook — 4 failures — TIMING
- `webhook.go:425,904,1194,2338` — readiness timeout
- kube-proxy atomic restore working. Pod readiness probe 20s delay + endpoint + iptables creates tight window.

### Field Validation — 2 failures — FIX STAGED ✅ (not yet deployed)
- `field_validation.go:611` — **Fix**: 858d091 (collect all unknown fields)
- `field_validation.go:735` — duplicate key detection

### StatefulSet — 2 failures — NEEDS FIX ❌
- `statefulset.go:957` — pod not re-created (port conflict test)
- `statefulset.go:1092` — patch timing

### DNS — 1 failure — INVESTIGATING
- `dns_common.go:476`

### DaemonSet — 1 failure — INVESTIGATING
- `daemon_set.go:1276`

### Job — 1 failure — NEEDS FIX ❌
- `job.go:596` — Job successPolicy not implemented. Test expects SuccessCriteriaMet condition.

### Preemption — 1 failure — INVESTIGATING
- `preemption.go:877`

### Service — 1 failure — INVESTIGATING
- `service.go:4291`

### Service Proxy — 1 failure — INVESTIGATING
- `proxy.go:503` — truncated JSON response

### EmptyDir — 1 failure — DinD
- `output.go:263`

## Staged Fixes (not yet deployed in round 138)

| Commit | Fix |
|--------|-----|
| 858d091 | Schema validator collects ALL unknown fields (with tests) |
| 86b048a | OpenAPI strip ALL Go omitempty defaults (with test) |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | ~380 | ~61 | 441 | ~86.2% |
| 138 | TBD (~20 failures at half) | TBD | 441 | TBD |
