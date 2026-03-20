# Full Conformance Failure Analysis

**Last updated**: 2026-03-20 (round 26 — monitoring, 2 failures so far)

## Current Run (Round 26): 2 failures, tests still running (old code)

### Failures
1. **Chunking compaction** — token never expires (fix committed but not deployed)
2. **StatefulSet scaling** — pods stuck (controller 5s interval, fix committed but not deployed)

### Tests NOT failing (fixes from round 25 working):
- Pod update JSON decode — FIXED (ContainerState deserializer)
- Pod patch resourceVersion — FIXED (clear RV on PATCH)
- PodTemplate lifecycle — FIXED (list filtering)
- ControllerRevision lifecycle — FIXED (list filtering)

## Fixes Committed, Ready for Round 27

1. Subpath validation: reject `..` and absolute paths with CreateContainerError
2. CronJob controller: 10s → 1s reconcile interval
3. StatefulSet controller: 5s → 1s reconcile interval
4. Chunking compaction: 5-minute token expiry with fresh token in 410 response
5. etcd auto-compaction enabled (5m periodic)

## All Fixes Deployed So Far (rounds 1-26)

- 64+ fixes from rounds 1-23
- GC foreground deletion + find_orphans
- Pod resize containerStatus.resources
- JSON decode ContainerState `{}`
- PATCH resourceVersion mismatch
- PodTemplate/ControllerRevision list filtering + watch
