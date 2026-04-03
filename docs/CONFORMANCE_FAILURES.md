# Conformance Issue Tracker

**Round 124** | 295/441 (66.9%) | Fixes pending redeploy

## Fixes Since Round 124 (4 new, 14 total this session)

| # | Fix | Est. Tests | Commit |
|---|-----|-----------|--------|
| 10 | Exec WebSocket: 500ms delay before close | ~20 | 24ca36b |
| 11 | OpenAPI v3: schemas for all 47 resource types | ~6 | 79f4f4a |
| 12 | Targeted protobuf response for protobuf requests | ~13 | c859496 |
| 13 | Recreate deployment: wait for old pods to terminate | ~2 | 140048a |

## All Fixes This Session

| # | Fix | Commit |
|---|-----|--------|
| 1 | StatefulSet: exclude terminating pods from status | 823884f |
| 2 | OpenAPI v3: x-kubernetes-group-version-kind extensions | 7fb8ecd |
| 3 | Job: reason CompletionsReached | db1a3e5 |
| 4 | OpenAPI v2: dot-format Content-Type | b3a6772 |
| 5 | RC: ReplicaFailure only on actual errors | b3a6772 |
| 6 | Protobuf response middleware (added then removed) | 655b38e → 8965fd5 |
| 7 | Webhook: lenient response parsing | ba0b26f |
| 8 | Scheduler: DisruptionTarget on preemption | d7ef779 |
| 9 | cargo fmt | 7cba226 |
| 10-13 | See above | |

## Remaining Failures Analysis (from round 124 evidence)

| Category | Count | Root Cause | Fixable? |
|----------|-------|-----------|---------|
| Exec connection reset | ~20 | WebSocket close too early | Fix #10 |
| CRD creation timeout | ~13 | Client expects protobuf response | Fix #12 |
| AdmissionWebhook | 13 | Webhook response parsing + readiness | Fix #7 + needs more |
| CRD OpenAPI | 9 | CRD creation timeout | Fix #12 |
| DNS | 7 | Exec reset + CoreDNS networking | Partial |
| FieldValidation | 6 | Missing OpenAPI schemas | Fix #11 |
| StatefulSet | 6 | Scale-down + rolling update + patch | Fix #1 + needs more |
| EmptyDir | 5 | Docker Desktop virtiofs perms | Platform limit |
| RC | 5 | Condition clearing + lifecycle | Fix #5 + needs more |
| Job | 5 | Orphan adoption + success policy timing | Partial |
| Service type transitions | 5 | kube-proxy iptables timing | Intermittent |
| Networking Pods | 4 | Exec connection reset | Fix #10 |
| Deployment | 4 | Recreate + rolling update + scaling | Fix #13 + needs more |
| ReplicaSet | 4 | Status patch overwritten by controller | Needs fix |
| SchedulerPreemption | 4 | DisruptionTarget + preemption path | Fix #8 + needs more |
| AggregatedDiscovery | 3 | CRD timeout blocks discovery | Fix #12 |
| Pod InPlace Resize | 5 | Exec connection reset reading cgroups | Fix #10 |
| Kubectl (various) | 7 | OpenAPI MIME (fixed) + validation | Fix #4 |
| Other | ~20 | Various (init containers, watchers, etc.) | Mixed |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | 310 | 131 | 441 | 70.3% |
| 124 | 295 | 146 | 441 | 66.9% |
