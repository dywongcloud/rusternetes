# Conformance Failure Tracker

**Round 138** | Running | 2026-04-13
**Baseline**: Round 137 = ~380/441 (~86.2%), 61 failures

## Round 138 Failures (23 so far)

### CRD OpenAPI — 5 failures — FIX STAGED ✅ (not yet deployed)
- `crd_publish_openapi.go:77,161,211,285,400`
- **Root cause**: Schema includes empty strings/false booleans K8s omits via Go omitempty
- **Fix**: 86b048a — strip ALL Go omitempty defaults (with test)

### Webhook — 5 failures — TIMING / KUBE-PROXY
- `webhook.go:425,904,1194,2107,2338`
- Pod readiness probe 20s delay + endpoint + iptables window

### Field Validation — 2 failures — FIX STAGED ✅ (not yet deployed)
- `field_validation.go:611` — **Fix**: 858d091 (collect all unknown fields, with tests)
- `field_validation.go:735` — duplicate key detection

### StatefulSet — 2 failures — INVESTIGATING
- `statefulset.go:957` — pod not re-created (port conflict)
- `statefulset.go:1092` — patch timing

### Preemption — 2 failures — DOWNSTREAM of watch
- `preemption.go:268` — watch context canceled prevents observing pod Running
- `preemption.go:877` — failed pod observation expectations

### Service — 2 failures — PARTIALLY FIXED
- `service.go:3459` — service deletion timeout (watch issue)
- `service.go:4291` — NodePort unreachable. **Fix staged**: f80d0c6 (NodePort rules in atomic path)

### DNS — 1 failure — INVESTIGATING
- `dns_common.go:476`

### DaemonSet — 1 failure — INVESTIGATING
- `daemon_set.go:1276`

### Job — 1 failure — FIX STAGED ✅
- `job.go:596` — **Fix**: 31e5e4f — terminating=0 on successPolicy completion

### Service Proxy — 1 failure — INVESTIGATING
- `proxy.go:503` — truncated JSON

### EmptyDir — 1 failure — DinD
- `output.go:263`

## Staged Fixes (not yet deployed)

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
| 138 | TBD (23 so far) | TBD | 441 | TBD |
