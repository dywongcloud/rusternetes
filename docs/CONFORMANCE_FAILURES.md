# Conformance Issue Tracker

**Round 123** | IN PROGRESS (old code) | Fixes pending redeploy

## Fixes Committed (9 total, not yet deployed)

| # | Fix | Est. Tests | Commit |
|---|-----|-----------|--------|
| 1 | StatefulSet: exclude terminating pods from status | ~5 | 823884f |
| 2 | OpenAPI v3: x-kubernetes-group-version-kind extensions | ~8 | 7fb8ecd |
| 3 | Job: reason CompletionsReached (not Completed) | ~2 | db1a3e5 |
| 4 | OpenAPI v2: dot-format Content-Type (K8s canonical) | ~18 | b3a6772 |
| 5 | RC: only set ReplicaFailure on actual errors | ~2 | b3a6772 |
| 6 | Protobuf response encoding middleware | ~6 | 655b38e |
| 7 | Webhook: lenient response parsing fallback | ~6 | ba0b26f |
| 8 | Scheduler: DisruptionTarget condition on preemption | ~2 | d7ef779 |
| 9 | cargo fmt | — | 7cba226 |

## Current Run (old code, ~60/441 conformance tests)

Pass rate ~74.6% (from sonobuoy progress). Higher than round 121 (70.3%).

## Known Platform Limitations (~6 tests)

| Issue | Tests | Reason |
|-------|-------|--------|
| Bind mount permissions | 6 | Docker Desktop virtiofs strips write bits |
| Pod-to-pod networking | ~4 | Docker Desktop iptables DNAT limitations |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | 310 | 131 | 441 | 70.3% |
| 123 | ~74.6% | — | 441 | IN PROGRESS (old code) |
