# Full Conformance Failure Analysis

**Last updated**: 2026-03-20 (round 22 — fresh run, ALL 63+ fixes deployed)

## Current Run: 2 tests done, 2 failed, 0 passed
Running on fully rebuilt images with ALL fixes.
No API server or kubelet errors observed.

## Remaining Failures

### C1. Chunking continue token expiry (1 test)
Error: `Expected token1 != token2` at chunking.go:194
The test expects old continue tokens to EXPIRE (return 410 Gone)
after resources are deleted and recreated. Our pagination doesn't
track resource versions in tokens or return 410 for stale tokens.
Fix: Return 410 Gone when a continue token references a resource
version that no longer exists in etcd.
File: `crates/common/src/pagination.rs` and list handlers

### C2. CronJob "forbid" scheduling timeout (1 test)
Error: `Failed to schedule CronJob forbid: context deadline exceeded`
The CronJob controller creates a job but it doesn't complete within
the test timeout. The job pod runs `sleep 300` (5 minutes). The test
expects the first job to be ACTIVE (not completed) while checking
that a second job is NOT scheduled (Forbid policy). The test may
timeout because the polling mechanism hits API rate limits.
File: May be a test timing issue rather than a code bug.

## No Other Errors Observed
- API server: clean (no 4xx/5xx errors)
- Kubelet: clean (no errors)
- Controller manager: only expected warnings
- All 63+ previously identified issues are fixed and deployed
