# Full Conformance Failure Analysis

**Last updated**: 2026-03-20 (round 15 — R1/R2 fixes committed, not deployed)

## Status
- 47 root cause categories fixed and committed
- Fresh run with 45 fixes: 6 tests done, 1 passed, 5 failed
- R1 (StatefulSet OrderedReady) and R2 (SubPathExpr error) just committed
- R3 (CronJob rate limit) and R4 (Chunking tokens) still open

## Remaining Open Issues

### R3. CronJob rate limiter timeout (1 test)
Test's API rate limiter expires. May need `--kube-api-qps` and `--kube-api-burst`
flags to increase rate limits.

### R4. Chunking continue token semantics (1 test)
Continue tokens don't change between compacted list requests.
Requires etcd revision tracking in pagination tokens.

### N10. ValidatingAdmissionPolicy (2 tests) — skipped (needs CEL)
### N11. Field validation strict mode (3 tests) — skipped (needs strict parsing)

## All fixes need image rebuild + redeploy to take effect.
