# Conformance Issue Tracker

**294 total fixes** | Round 105 in progress | 31 failures

## Pending deploy (#282-294) — 13 fixes
| # | Fix | Impact |
|---|-----|--------|
| 282 | Status PATCH accepts apply-patch+yaml | 1 test |
| 283 | Preserve container status when removed | 2 tests |
| 284 | Kubelet sync timeouts (10s/30s) | ~12 tests |
| 285 | Aggregated discovery dynamic CRD groups | 2 tests |
| 286 | MicroTime always .000000 | 1 test |
| 287 | generation=1 on creation | 2 tests |
| 288 | fsGroup g+rX not g+rwX | 1 test |
| 289 | Job successPolicy matching indexes only | 1 test |
| 290 | Pod resize status update | 1 test |
| 292 | CRD protobuf all wire types (32/64-bit) | 4 tests |
| 293 | OpenAPI protobuf Accept fix | 2 tests |
| 294 | **CRITICAL** Watch RV timestamp→0 fallback + overflow filter | SS watch + many others |

## Remaining unfixed
| Test | Error |
|------|-------|
| SS watch ordering | Should be fixed by #294 |
| CRD protobuf (x4) | Should be improved by #292 |
| kubectl guestbook/patch | Should be fixed by #293 |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 31 | 441 | ~93% pre-deploy, ~98%+ est post-deploy |
