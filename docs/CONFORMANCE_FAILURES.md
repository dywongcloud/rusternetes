# Conformance Issue Tracker

**295 total fixes** | Round 105 in progress | 37 failures

## Pending deploy (#282-295) — 14 fixes
| # | Fix | Impact |
|---|-----|--------|
| 282 | Status PATCH accepts apply-patch+yaml | 1 |
| 283 | Preserve container status when removed | 2 |
| 284 | Kubelet sync timeouts (10s/30s) | ~15 |
| 285 | Aggregated discovery dynamic CRD groups | 2 |
| 286 | MicroTime always .000000 | 1 |
| 287 | generation=1 on creation | 1 |
| 288 | fsGroup g+rX not g+rwX | 1 |
| 289 | Job successPolicy matching indexes | 1 |
| 290 | Pod resize status update | 1 |
| 292 | CRD protobuf all wire types | 4 |
| 293 | OpenAPI always returns JSON | 2 |
| 294 | **CRITICAL** Watch RV timestamp→0 + overflow filter | ~5 |
| 295 | Pod PUT uses stored generation | 2 |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 37 | 441 | ~92% pre-deploy, ~98%+ est post-deploy |
