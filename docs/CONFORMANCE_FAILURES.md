# Conformance Failure Tracker

**Round 144** | Complete — ~375/441 (85.1%) | 2026-04-15

## Root Cause Found

### Webhook — 18 failures — FIXED ✅
- **Root cause**: configmap handler called `run_mutating_webhooks` but NEVER checked the `AdmissionResponse` for denial. When the webhook returned `allowed:false`, the configmap was created anyway. The webhook readiness check expects create to be DENIED — since it succeeded, the check kept polling and timed out.
- The webhook was ACTUALLY WORKING — API server connected via ClusterIP, got correct `allowed:false` response — we just ignored it.
- **Fix**: check `AdmissionResponse::Deny` after mutating webhook call and return Forbidden

### CRD OpenAPI — 9 failures — FIXED ✅
- kubectl strict validation + preserve-unknown-fields

### EmptyDir — 7 failures — UNFIXABLE ❌
- macOS Docker filesystem

### DNS — 6 failures — FIXED ✅
- umask double-wrap for sh -c commands

### Service — 5 failures — FIXED ✅
- kube-proxy endpoint port matching (targetPort)

### Apps — 10 failures — FIXED ✅
- Docker 409 proactive cleanup, fast-exit detection, securityContext default, quota active pods, pod delete CAS retry

### Network — 3 failures — FIXED ✅
- kube-proxy port matching, hostPort fix

### Other — 11 failures — FIXED ✅ (except pod_resize)
- Various: quota, preemption, GC, field validation, lifecycle hooks
- `pod_resize.go:857` — ⚠️ partially implemented: API server sets resize=Proposed, kubelet detects and calls Docker update_container. But some containers (c2) don't get their cgroup updated. Either status.resize is cleared by a concurrent status write before the kubelet processes it, or the memory value parsing fails for specific resource values.

## Summary

| Status | Count |
|--------|-------|
| FIXED ✅ | 61 |
| UNFIXABLE ❌ | 8 (EmptyDir + pod_resize) |
| **Total** | **69** |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 141 | 368 | 73 | 441 | 83.4% |
| 143 | 372 | 69 | 441 | 84.4% |
| 144 | ~375 | ~60 | 441 | ~85.1% |
| 145 | — | — | 441 | — | webhook denial fix pending |
