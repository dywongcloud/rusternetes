# Conformance Issue Tracker

**296 total fixes** | Round 105 in progress | ~40 failures

## Pending deploy (#282-296) — 15 fixes
| # | Fix | Impact |
|---|-----|--------|
| 282 | Status PATCH accepts apply-patch+yaml | 1 |
| 283 | Preserve container status when removed | 3 |
| 284 | Kubelet sync timeouts (10s/30s) | ~20 |
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
| 296 | Job successPolicy all-indexes waits for all completions | 1 |

## Remaining issues after deploy (~3)
| Test | Error | Status |
|------|-------|--------|
| ExternalName DNS | nslookup fails after type change | CoreDNS/DNS issue |
| Subpath configmap existing file | Pod doesn't reach Succeeded | Container phase transition (#283/#284) |
| VAP validate Deployment | CEL messageExpression | #279 pending deploy |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | ~40 | 441 | ~91% pre-deploy, ~99% est post-deploy |
