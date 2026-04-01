# Conformance Issue Tracker

**Round 120** | IN PROGRESS | 9/441 done | 4 passed, 5 failed (44%)

## Current Failures

| # | Test | Error | Root Cause | Status |
|---|------|-------|-----------|--------|
| 1 | statefulset.go:2479 | Scaled unexpectedly | Scale-down doesn't halt on unhealthy pods | Fix committed (readiness check) |
| 2 | builder.go:97 | kubectl protobuf | OpenAPI protobuf encoding not implemented | Known limitation |
| 3 | output.go:263 | Perms 0644 vs 0666 | Docker Desktop virtiofs strips write bits on bind mounts | Platform limitation |
| 4 | crd_publish_openapi.go:244 | CRD timeout 30s | Watch may not deliver MODIFIED event; investigating | Investigating |
| 5 | kubelet.go:127 | Pod terminated 300s timeout | Ready/ContainersReady left True on terminated pod | Fix committed |

## Fixes Committed During This Round (Not Yet Deployed)

17. **StatefulSet readiness check on scale-down** (9b4ba30) — halt scale-down when remaining pods not Ready
18. **Terminated pod conditions** (002eb90) — set Ready/ContainersReady to False when pod terminates

## Deployed Fixes (This Round)

16 fixes from round 119 analysis deployed at start of round 120.

## Known Limitations

- **Bind mount permissions**: Docker Desktop virtiofs strips group/other write bits (~2 tests)
- **kubectl protobuf**: OpenAPI protobuf encoding not implemented (~1 test)

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 119 | ~21 | ~30 | ~51/441 | ~41% (partial, pre-fix baseline) |
| 120 | 4 | 5 | 9/441 | 44% (in progress) |
