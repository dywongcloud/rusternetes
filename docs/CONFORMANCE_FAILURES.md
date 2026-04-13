# Conformance Failure Tracker

**Round 138** | Running | 2026-04-13
**Result so far**: 24 failures (~94.6%) — down from 61 in round 137

## Round 138 Failures (24)

### CRD OpenAPI — 5 failures — FIX STAGED ✅ (not deployed this round)
- `crd_publish_openapi.go:77,161,211,285,400`
- **Fix**: 86b048a — strip ALL Go omitempty defaults (with test)

### Webhook — 5 failures — TIMING
- `webhook.go:425,904,1194,2107,2338`
- Pod readiness 20s delay + endpoint + iptables timing window

### Preemption — 3 failures — INVESTIGATING
- `preemption.go:181,268,877`
- Watch context canceled + pod observation timeouts

### Field Validation — 2 failures — FIX STAGED ✅ (not deployed this round)
- `field_validation.go:611` — **Fix**: 858d091
- `field_validation.go:735` — duplicate key detection

### StatefulSet — 2 failures — INVESTIGATING
- `statefulset.go:957` — port conflict test
- `statefulset.go:1092` — patch timing

### Service — 2 failures — PARTIALLY FIXED
- `service.go:3459` — deletion timeout (watch)
- `service.go:4291` — NodePort unreachable. **Fix staged**: f80d0c6

### DNS — 1 failure — INVESTIGATING
- `dns_common.go:476` — container command execution issue (pause shell error)

### DaemonSet — 1 failure — INVESTIGATING
- `daemon_set.go:1276`

### Job — 1 failure — FIX STAGED ✅
- `job.go:596` — **Fix**: 31e5e4f

### Service Proxy — 1 failure — INVESTIGATING
- `proxy.go:503` — truncated JSON response

### EmptyDir — 1 failure — DinD
- `output.go:263`

## Staged Fixes (for round 139)

| Commit | Fix |
|--------|-----|
| 858d091 | Schema validator collects ALL unknown fields (with tests) |
| 86b048a | OpenAPI strip ALL Go omitempty defaults (with test) |
| 31e5e4f | Job successPolicy terminating=0 |
| f80d0c6 | kube-proxy atomic path: NodePort DNAT rules |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | ~380 | ~61 | 441 | ~86.2% |
| 138 | ~417 | ~24 | 441 | ~94.6% |
