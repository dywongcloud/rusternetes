# Full Conformance Failure Analysis

**Last updated**: 2026-03-20 (round 23 — chunking fix committed, monitoring)

## Current Run: 4 tests, 1 passed, 3 failed (25% pass rate)
All 64+ fixes deployed. Chunking fix committed but not yet deployed.

## Failures in Current Run

### C1. Chunking continue token (1 test) — FIX COMMITTED (10e40e9)
Now returns 410 Gone for stale tokens (total count changed).
Not deployed yet in current run.

### C2. CronJob "forbid" timeout (1 test)
Test timing issue — CronJob controller works correctly (creates job,
respects Forbid policy) but test hits API rate limits before completing.
May need the test's client rate limiter to be configured.

### C3. StatefulSet rate limiter (1 test)
"client rate limiter Wait returned an error: rate: Wait(n=1) would
exceed context deadline" — test's API polling hits rate limits.
Same root cause as C2 — tests need higher API rate limits.

## API Rate Limit Issue
Both C2 and C3 fail because the e2e test binary's built-in HTTP
client rate limiter (5 QPS, 10 burst by default) is too low for
tests that poll frequently. The `--kube-api-qps` flag doesn't exist
in the e2e binary. Need to investigate if there's another way to
increase rate limits for the test client.

## All Other Issues: FIXED (64+ root causes)
No API server, kubelet, or controller errors observed.
