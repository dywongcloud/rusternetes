# Conformance Failure Tracker

**Round 138** | Complete | 2026-04-13
**Result**: ~417/441 (~94.6%) — 24 failures, down from 61 in round 137

## Round 138 Failures (24)

### CRD OpenAPI — 5 failures — FIX STAGED ✅
- `crd_publish_openapi.go:77,161,211,285,400`
- **Fix**: 86b048a — strip ALL Go omitempty defaults (with test)

### Webhook — 5 failures — TIMING / WATCH
- `webhook.go:425,904,1194,2107,2338`
- Webhook pod readiness delay + endpoint + iptables timing. Watch fix staged.

### Preemption — 3 failures — FIX STAGED ✅
- `preemption.go:181,268,877` — all have watch context canceled
- **Fix**: 0061469 — reduced channel buffer 8192→16 + bookmark interval 5s→1s

### Field Validation — 2 failures — FIX STAGED ✅
- `field_validation.go:611` — **Fix**: 858d091 (collect all unknowns, with tests)
- `field_validation.go:735` — duplicate key detection

### StatefulSet — 2 failures — MIXED
- `statefulset.go:957` — DinD limitation (containerPort conflicts don't fail in Docker)
- `statefulset.go:1092` — patch timing, downstream of watch

### Service — 2 failures — FIX STAGED ✅
- `service.go:3459` — deletion timeout (watch issue, fix staged)
- `service.go:4291` — NodePort unreachable. **Fix**: f80d0c6

### DNS — 1 failure — INVESTIGATING
- `dns_common.go:476` — container command execution: `pause: syntax error`. agnhost querier container may have incorrect entrypoint/cmd handling with multi-container pods.

### DaemonSet — 1 failure — DOWNSTREAM of watch
- `daemon_set.go:1276` — pods created but not observed as running (watch)

### Job — 1 failure — FIX STAGED ✅
- `job.go:596` — **Fix**: 31e5e4f

### Service Proxy — 1 failure — DOWNSTREAM of watch
- `proxy.go:503` — pod didn't start in time (watch + timing)

### EmptyDir — 1 failure — DinD
- `output.go:263`

## Staged Fixes (for round 139)

| Commit | Fix |
|--------|-----|
| 858d091 | Schema validator collects ALL unknown fields (with tests) |
| 86b048a | OpenAPI strip ALL Go omitempty defaults (with test) |
| 31e5e4f | Job successPolicy terminating=0 |
| f80d0c6 | kube-proxy atomic path: NodePort DNAT rules |
| 0061469 | Watch channel buffer 16 + bookmark 1s |
| 55d52d7 | Status PATCH deep merge — preserve node capacity (with test) |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | ~380 | ~61 | 441 | ~86.2% |
| 138 | ~417 | ~24 | 441 | ~94.6% |
| 139 | TBD | TBD | 441 | TBD |
