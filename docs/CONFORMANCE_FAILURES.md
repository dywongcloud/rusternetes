# Conformance Failure Tracker

**Round 140** | Running (43 min, 0 watch failures!) | 2026-04-14

## Round 140 Failures (8 so far)

### CRD OpenAPI — 2 failures — INVESTIGATING
- `crd_publish_openapi.go:161,253`

### Webhook — 2 failures — INVESTIGATING
- `webhook.go:1631,2107`

### EmptyDir — 2 failures — DinD
- `output.go:263` (x2) — macOS filesystem permissions

### Service Latency — 1 failure — INVESTIGATING
- `service_latency.go:145` — deployment not ready

### DaemonSet — 1 failure — INVESTIGATING
- `daemon_set.go:1276`

## Key Metrics
- **Watch failures: 0** (down from 3012 in round 138!)
- HTTP/2 flow control fix completely eliminated watch context canceled
- Lease-based heartbeat preventing node NotReady

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 137 | ~380 | ~61 | 441 | ~86.2% |
| 138 | TERMINATED | ~35+ | 441 | — |
| 140 | TBD (8 failures at 43min) | TBD | 441 | TBD |
