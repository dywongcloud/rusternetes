# Conformance Failure Tracker

**Round 138** | Running | 2026-04-13
**Baseline**: Round 137 = ~380/441 (~86.2%), 61 failures

## Round 138 Failures (12 so far — ~97% pass rate)

### 1. CRD OpenAPI — 2 failures — INVESTIGATING
- `crd_publish_openapi.go:77,400`
- False extensions stripped (fix working), schemas appear identical but still timeout. May be subtle multi-version or schema ordering difference.

### 2. Field Validation — 1 failure — FIX STAGED ✅
- `field_validation.go:611`
- **Fix**: 858d091 — Validator collects ALL unknown fields. Tests added.

### 3. Webhook — 3 failures — TIMING
- `webhook.go:425,904,1194` — "waiting for webhook configuration to be ready: timed out"
- kube-proxy atomic restore working (77 successful restores). Webhook pod readiness probe (20s delay) + endpoint + iptables creates tight timing window.

### 4. StatefulSet — 1 failure — INVESTIGATING
- `statefulset.go:957` — "Pod ss-0 expected to be re-created at least once"
- Test creates pod with conflicting port. StatefulSet controller should detect failure and recreate.

### 5. DaemonSet — 1 failure — INVESTIGATING
- `daemon_set.go:1276`

### 6. DNS — 1 failure — INVESTIGATING
- `dns_common.go:476`

### 7. Preemption — 1 failure — INVESTIGATING
- `preemption.go:877` — "failed pod observation expectations"

### 8. EmptyDir — 1 failure — DinD
- `output.go:263` — macOS filesystem permissions

### 9. Service Proxy — 1 failure — INVESTIGATING
- `proxy.go:503` — "unexpected end of JSON input"

## Fixes Deployed in Round 138

| Commit | Fix |
|--------|-----|
| 858d091 | Schema validator collects ALL unknown fields (with tests) |
| _(plus 16 from round 137 tracker)_ | |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | ~380 | ~61 | 441 | ~86.2% |
| 138 | TBD (~12 failures so far) | TBD | 441 | ~97% so far |
