# Full Conformance Failure Analysis

**Last updated**: 2026-03-20 (round 25 running, fixes for round 26 ready)

## Current Run (Round 25): 3 failures so far, tests still running

### Failures in Current Run

1. **CronJob ForbidConcurrent** — rate limiter timeout (controller 10s interval too slow)
   FIX READY: Reduced CronJob reconcile interval from 10s to 1s.

2. **Variable Expansion subpath** — kubelet doesn't validate subpath
   FIX READY: Added subpath validation before volume lookup. Rejects `..`
   path traversal and absolute paths with CreateContainerError.

3. **StatefulSet scaling** — rate limiter timeout (controller 5s interval)
   FIX READY: Reduced StatefulSet reconcile interval from 5s to 1s.

## Fixes Ready for Round 26 (not yet deployed)

1. CronJob controller interval: 10s → 1s
2. StatefulSet controller interval: 5s → 1s
3. Subpath validation: reject `..` and absolute paths in subPathExpr/subPath

## Fixes Already Deployed in Round 25

1. JSON decode `lastState:{}` — custom deserializer for empty ContainerState
2. PATCH resourceVersion mismatch — clear RV for PATCH operations
3. PodTemplate list — added Query params, watch, label/field selector filtering
4. ControllerRevision list — same as PodTemplate
5. GC foreground deletion — propagationPolicy + foregroundDeletion finalizer
6. GC find_orphans — only orphan when ALL owners gone
7. Pod resize containerStatus.resources — populated from container spec

## Previous Fixes (64+ from earlier rounds)
All API server, kubelet, and controller fixes from rounds 1-23.

## Tests Previously Failing, Now Expected to Pass

- Pod update JSON decode (F8) — fixed by ContainerState deserializer
- Pod patch resourceVersion (F10) — fixed by clearing RV on PATCH
- PodTemplate lifecycle (F5) — fixed by adding list filtering
- ControllerRevision lifecycle (F9) — fixed by adding list filtering
- GC foreground deletion (C4) — fixed by propagation policy support
- Pod InPlace Resize (C5) — fixed by populating resources fields
- Chunking continue token (C1) — fixed in earlier commit
