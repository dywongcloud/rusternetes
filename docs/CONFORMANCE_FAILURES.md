# Conformance Failure Tracker

**Round 138** | Running | 2026-04-13
**Current**: ~35 failures at ~80% complete

## Round 138 Failures — Verified Root Causes

### CRD OpenAPI — 6 failures — FIX STAGED + TESTED ✅
- **Fix**: 86b048a — strip ALL Go omitempty defaults
- **Test**: test_strip_false_extensions_removes_defaults (3 levels deep)

### Webhook — 7 failures — NEEDS INVESTIGATION ❌
- NOT downstream of watch (0 watch failures near test failure)
- Webhook pod readiness + kube-proxy routing — needs own root cause

### Preemption — 3 failures — FIX STAGED + TESTED ✅
- **Root cause**: Status PATCH replaced node capacity map, wiping extended resources
- **Fix**: 55d52d7 — deep merge
- **Test**: test_deep_merge_preserves_existing_map_entries

### Field Validation — 2 failures — FIX STAGED + TESTED ✅
- **Fix**: 858d091 — collect ALL unknown fields
- **Tests**: test_collects_all_unknown_fields, test_embedded_resource_meta_fields, test_preserve_unknown_fields

### Watch — SYSTEMIC — FIX STAGED ✅
- **Root cause**: HTTP/2 flow control windows at spec default (64KB) too small
- **Fix**: 5b7048f — match K8s window sizes (256KB stream, 25MB connection)
- Verified via K8s source: secure_serving.go:175-199

### CRD Error Responses — FIX STAGED ✅
- **Root cause**: 10 CRD route handlers returned 500 plain text instead of K8s Status JSON
- **Fix**: 294358e

### VERIFIED downstream of watch (watch failures confirmed near test):
- ReplicaSet (2) — 5 watch failures before each
- Runtime (1) — 5 watch failures before
- Lifecycle hook (1) — 5 watch failures before
- Service deletion (1) — 5 watch failures before
- DNS (1) — 8 watch failures before

### NOT downstream — needs own investigation:
- DaemonSet (1) — 0 watch failures near test, needs own root cause
- Proxy (2) — 0 watch failures near test, needs own root cause

### Other:
- Job (1) — FIX STAGED + TESTED ✅ (31e5e4f, test_success_policy_sets_terminating_zero)
- NodePort service (1) — FIX STAGED ✅ (f80d0c6)
- StatefulSet 957 — DinD limitation
- StatefulSet 1092 — watch downstream (confirmed)
- EmptyDir — DinD limitation

## Staged Fixes (10 commits)

| Commit | Fix | Test |
|--------|-----|------|
| 858d091 | Schema validator collects ALL unknown fields | 3 tests ✅ |
| 86b048a | OpenAPI strip ALL Go omitempty defaults | 1 test (3 levels) ✅ |
| 55d52d7 | Status PATCH deep merge (node capacity) | 1 test ✅ |
| 31e5e4f | Job successPolicy terminating=0 | 1 test ✅ |
| 294358e | CRD error responses: K8s Status JSON | — |
| 5b7048f | HTTP/2 flow control: K8s window sizes | — |
| f80d0c6 | kube-proxy NodePort DNAT rules | — |
| 0061469 | Watch channel buffer 16 + bookmark 1s | — |
| ab02ba3 | Expanded OpenAPI strip test | test only |
| bd80ff0 | Job successPolicy test | test only |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 136 | ABORTED | — | 441 | — |
| 137 | ~380 | ~61 | 441 | ~86.2% |
| 138 | TBD | TBD | 441 | TBD |
