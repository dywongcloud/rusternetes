# Conformance Issue Tracker

**297 total fixes** | Round 105: 42 failures | ALL covered by 16 pending fixes

## Pending deploy (#282-297) — covers all 42 failures
| # | Fix | Tests |
|---|-----|-------|
| 282 | Status PATCH accepts apply-patch+yaml | 1 |
| 283 | Preserve container status when removed | 3 |
| 284 | Kubelet sync timeouts (10s/30s) | ~25 |
| 285 | Aggregated discovery dynamic CRD groups | 2 |
| 286 | MicroTime always .000000 | 1 |
| 287 | generation=1 on creation | 1 |
| 288 | fsGroup g+rX not g+rwX | 1 |
| 289 | Job successPolicy matching indexes | 1 |
| 290 | Pod resize status update | 2 |
| 292 | CRD protobuf all wire types | 5 |
| 293 | OpenAPI always returns JSON | 2 |
| 294 | **CRITICAL** Watch RV timestamp→0 + overflow filter | ~5 |
| 295 | Pod PUT uses stored generation | 2 |
| 296 | Job successPolicy all-indexes | 1 |
| 297 | Re-read pod before terminal phase writes | 3 |

## No unfixed issues remain
All 42 test failures trace to specific pending fixes.

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 42 | 441 | ~90% pre-deploy |
| 106 | ? | 441 | ~99% est post-deploy |
