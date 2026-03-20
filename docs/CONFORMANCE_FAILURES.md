# Full Conformance Failure Analysis

**Last updated**: 2026-03-20 (round 17 — 40 tests, 6 passed, 34 failed)

## Current run on partially-updated images
Many failures are from undeployed fixes. Need full rebuild.

## New Issue Found

### N15. Init container waiting reason "PodInitializing" vs "ContainerCreating"
Error: `container should have reason PodInitializing, got ContainerCreating`
When init containers are running, regular containers should show
Waiting reason "PodInitializing" not "ContainerCreating".
File: `crates/kubelet/src/kubelet.rs` or `runtime.rs`

## All 49 fixes committed. Need rebuild + redeploy.

## Recurring (mostly from undeployed fixes)
- StatefulSet rate limiter — fixed with qps/burst increase
- Variable Expansion subpath — R2 fix committed
- Chunking continue tokens — complex, skip for now
- CronJob scheduling — timing issue
- Pod volume timeouts — fixes committed but not deployed
- Liveness restart count — fix committed but not deployed
