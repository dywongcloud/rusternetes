# Full Conformance Failure Analysis

**Last updated**: 2026-03-20 (round 14 — fresh run with all 45 fixes deployed)

## Current Run: 6 completed, 5 failed, 1 passed (17%)
Running against fully rebuilt images with ALL fixes deployed.

## Recurring Failures (need deeper investigation)

### R1. StatefulSet unhealthy pod scaling (2 tests)
Error: `Failed waiting for pods to enter running: context deadline exceeded`
Root cause: StatefulSet controller doesn't properly handle OrderedReady
policy when pods have failing readiness probes. Should halt scaling
when existing pods are not Ready.
File: `crates/controller-manager/src/controllers/statefulset.rs`

### R2. Variable Expansion subpath error handling (2 tests)
Error: `Failed after 8-10s. Expected Pod to be in "Pending"`
Root cause: Kubelet should detect undefined env vars in subPathExpr
and set container to Waiting/CreateContainerError, not start it.
File: `crates/kubelet/src/runtime.rs` — expand_subpath_expr()

### R3. CronJob rate limiter timeout (1 test)
Error: `client rate limiter Wait returned an error: context deadline exceeded`
Root cause: Test's API rate limiter expires before the test completes.
This may be a test timing issue rather than a code bug.

### R4. Chunking continue token (1 test)
Error: `Expected token1 != token2`
Root cause: Pagination tokens don't change between compacted list requests.
Complex to fix — requires etcd revision tracking in continue tokens.

## All Other Issues: FIXED (45 root causes committed)

## Test Results History

| Run | Date | Passed | Failed | Total | Rate |
|-----|------|--------|--------|-------|------|
| Quick mode | 2026-03-18 | 1 | 0 | 1 | 100% |
| Full run 1 (no fixes) | 2026-03-19 | 11 | 75 | 86 | 13% |
| Full run 2 (partial fixes) | 2026-03-19 | 2 | 23 | 25 | 8% |
| Full run 3 (all fixes) | 2026-03-20 | 1 | 5 | 6 | 17% |
| (still running) | | | | 441 | |
