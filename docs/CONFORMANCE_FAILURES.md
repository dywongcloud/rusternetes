# Conformance Issue Tracker

**Round 123** | IN PROGRESS | Tests still running

## Fixes Applied

| # | Fix | Tests | Status |
|---|-----|-------|--------|
| 1 | StatefulSet: exclude terminating pods from status counts | ~5 | Committed (823884f) |
| 2 | OpenAPI v3: add x-kubernetes-group-version-kind extensions | ~8 | Committed (7fb8ecd) |
| 3 | OpenAPI v2: use MIME-parseable Content-Type header | ~8 | Committed (7fb8ecd) |
| 4 | Job: completion reason CompletionsReached | ~2 | Committed (db1a3e5) |

## Current Failures (11 of ~441 tests completed, all failed)

| Test | Root Cause | Fix |
|------|-----------|-----|
| StatefulSet Burst scaling | readyReplicas counted terminating pods | Fix #1 |
| StatefulSet Scaling predictable | readyReplicas counted terminating pods | Fix #1 |
| Kubectl scale RC | OpenAPI v3 missing GVK / v2 bad MIME | Fix #2 + #3 |
| Job indexed completions | Reason "Completed" vs "CompletionsReached" | Fix #4 |
| Services multiport endpoints | Endpoints subset wrong port mapping | TODO - investigate port resolution |
| StatefulSet list/patch/delete | Timeout waiting on patch condition | TODO - investigate |
| Service status lifecycle | Timeout deleting service | TODO - investigate |
| EmptyDir shared volumes | Exec connection reset by peer | TODO - WebSocket/exec stability |
| DNS configurable nameservers | Exec connection reset by peer | TODO - WebSocket/exec stability |
| CRD listing | CRD creation timeout | TODO - investigate CRD handler |
| CRD publish OpenAPI | CRD creation timeout | TODO - investigate CRD handler |

## Known Platform Limitations (~6 tests)

| Issue | Tests | Reason |
|-------|-------|--------|
| Bind mount permissions | 6 | Docker Desktop virtiofs strips write bits |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | 310 | 131 | 441 | 70.3% |
| 123 | — | — | 441 | IN PROGRESS (0 passed so far) |
