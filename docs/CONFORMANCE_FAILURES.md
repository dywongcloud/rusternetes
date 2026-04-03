# Conformance Issue Tracker

**Round 123** | IN PROGRESS | ~50/441 conformance tests completed

## Fixes Committed (6 total)

| # | Fix | Tests | Commit |
|---|-----|-------|--------|
| 1 | StatefulSet: exclude terminating pods from status counts | ~5 | 823884f |
| 2 | OpenAPI v3: add x-kubernetes-group-version-kind extensions | ~8 | 7fb8ecd |
| 3 | Job: completion reason CompletionsReached | ~2 | db1a3e5 |
| 4 | OpenAPI v2: use dot-format Content-Type (not @) | ~30 | b3a6772 |
| 5 | RC: only set ReplicaFailure on actual creation errors | ~2 | b3a6772 |
| 6 | Protobuf response encoding middleware | ~6 | 655b38e |

## Failure Categories (from ~50 tests)

| Category | Count | Status |
|----------|-------|--------|
| CRD creation timeout | 5 | Fix #6 (protobuf response) |
| Webhook readiness timeout | 4 | TODO - webhook invocation |
| Connection reset (exec) | 4 | TODO - WebSocket stability |
| Service not reachable | 4 | TODO - kube-proxy/networking |
| OpenAPI MIME error (kubectl) | 3 | Fix #4 (dot-format) |
| StatefulSet readyReplicas | 2 | Fix #1 |
| DNS failures | 2 | TODO - CoreDNS/pod networking |

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
| 123 | ~129+ | ~44+ | 441 | ~74.6% (sonobuoy, in progress) |
