# Conformance Issue Tracker

**Round 110** | IN PROGRESS | 259/441 tests | 163 passed, 96 failed (62.9% pass)

## Fixes committed (not yet deployed)

| Commit | Fix | Target Failures |
|--------|-----|-----------------|
| 4624a26 | CRD sync status update, TokenRequest defaults | 5 CRD + 2 SA |
| 5da5f98 | OpenAPI protobuf envelope for kubectl | 4 builder |
| f65ab7b | RC CAS retry, LimitRange defaults | 2 RC + 1 LR |
| 7266a9e | Webhook probe: scheme lowercase + no_proxy | 5+ webhook |
| 829ce94 | CRD sync, SS partition, Job suspend, events, logs tail, sysctl | 15+ |
| 0da0e57 | Field validation improvements | 2 |
| fba0a62 | Watch synthetic ADDED for label-filtered objects | 1 watch |
| cde918d | Deployment status aggregation, namespace finalization, event field selectors, SA token projection, TypeMeta | 10+ |

**~40 failures targeted by committed fixes**

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 109 | 48* | 78* | 38%* |
| 110 | 96 | 259/441 | 62.9% (in progress) |

*Round 109 incomplete
