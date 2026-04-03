# Conformance Issue Tracker

**Round 124** COMPLETE | 295/441 passed (66.9%) | Regression from 310 (70.3%)

## Round 124 Results

| Category | Failures | Notes |
|----------|----------|-------|
| Services | 13 | Networking/kube-proxy issues |
| AdmissionWebhook | 13 | Webhook readiness + response parsing |
| CRD OpenAPI | 9 | CRD creation timeout |
| DNS | 7 | Pod networking + CoreDNS |
| StatefulSet | 6 | Scale-down + rolling update |
| FieldValidation | 6 | Strict field validation |
| EmptyDir | 5 | Bind mount permissions |
| RC | 5 | ReplicaFailure condition |
| Job | 5 | Indexed job completion |
| Networking | 4 | Pod-to-pod connectivity |
| ReplicaSet | 4 | |
| Deployment | 4 | |
| SchedulerPreemption | 3 | |
| AggregatedDiscovery | 3 | |
| Other | ~49 | Various |

## Fixes Applied in This Round

| # | Fix | Commit |
|---|-----|--------|
| 1 | StatefulSet: exclude terminating pods from status counts | 823884f |
| 2 | OpenAPI v3: x-kubernetes-group-version-kind extensions | 7fb8ecd |
| 3 | Job: reason CompletionsReached | db1a3e5 |
| 4 | OpenAPI v2: dot-format Content-Type (MIME fix reduced from 18 to 1) | b3a6772 |
| 5 | RC: only set ReplicaFailure on actual errors | b3a6772 |
| 6 | Webhook: lenient response parsing | ba0b26f |
| 7 | Scheduler: DisruptionTarget on preemption | d7ef779 |
| 8 | Removed protobuf response middleware (caused wireType crash) | 8965fd5 |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | 310 | 131 | 441 | 70.3% |
| 124 | 295 | 146 | 441 | 66.9% |
