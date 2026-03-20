# Full Conformance Failure Analysis

**Last updated**: 2026-03-20 (round 16 — 9 tests, 2 passed, 7 failed)

## New Failures Found

### N13. Liveness probe not triggering container restarts (~2 tests)
Error: `expected number of restarts: 5, found restarts: 0`
Container has failing liveness probe but is never restarted.
RestartCount stays at 0. The kubelet's liveness probe handler
must detect threshold exceeded and restart the container.
File: `crates/kubelet/src/kubelet.rs` (liveness check logic)

### N14. ConfigMap list by label selector (~1 test)
Error: `failed to find ConfigMap by label selector`
The configmap list handler doesn't filter by labelSelector.
File: `crates/api-server/src/handlers/configmap.rs`

## Recurring (from previous runs, still present)
- R1: StatefulSet OrderedReady — rate limiter timeout
- R2: Variable Expansion subpath — pod starts instead of failing
- R3: CronJob scheduling — timeout
- R4: Chunking continue tokens

## All 48+ root causes committed. Needs rebuild for latest fixes.
