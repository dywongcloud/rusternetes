# Full Conformance Failure Analysis

**Last updated**: 2026-03-21 (round 36 in progress)

## Round 36: 2 failures so far (both StatefulSet), tests still running

### F1. StatefulSet burst scaling — HTTP PROBE CONNECTIVITY
"Failed waiting for pods to enter running: client rate limiter"
Root cause: HTTP readiness probes aren't being executed. The kubelet
never checks probes for StatefulSet pods. Without readiness probe
passing, pods stay not-Ready, StatefulSet controller won't scale up
(OrderedReady), test times out.
Investigation: check_probe/check_http_probe functions exist but
are never called during the sync loop for these pods. May be a
network connectivity issue (kubelet can't reach pod IP) or probe
execution is being skipped.

### F2. StatefulSet predictable scaling — SAME ROOT CAUSE
Same HTTP probe issue as F1.

## Fixes deployed in round 36:
1-16: All previous fixes
17. Kubelet sync interval: 10s → 2s
18. Pagination: consistent resourceVersion across pages
19. Last page remainingItemCount: nil not 0

## Tests now PASSING (confirmed in round 36):
- All non-StatefulSet tests that have run
- Chunking test hasn't run yet in this round

## Known remaining issues to investigate:
- HTTP probe execution for pods (may affect many tests)
- Chunking compaction (may be fixed by RV + nil fixes)
