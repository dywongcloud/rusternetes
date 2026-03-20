# Full Conformance Failure Analysis

**Last updated**: 2026-03-20 (round 28 starting — all fixes deployed)

## Round 28: All Known Fixes Deployed

### Fixes in this round:
1. GC foreground deletion + find_orphans (round 25)
2. Pod resize containerStatus.resources (round 25)
3. JSON decode ContainerState `{}` → None (round 25)
4. PATCH resourceVersion mismatch (round 25)
5. PodTemplate list: Query params, watch, label/field selector filtering (round 25)
6. ControllerRevision list: Query params, watch, filtering (round 25)
7. Subpath validation: reject `..` and absolute paths (round 26)
8. CronJob controller: 10s → 1s reconcile interval (round 26)
9. StatefulSet controller: 5s → 1s reconcile interval (round 26)
10. Chunking compaction: 5-minute token expiry with fresh token in 410 (round 27)
11. etcd auto-compaction: 5m periodic (round 27)
12. CreateContainerError preserved: sync loop no longer overrides with Running (round 28)

### Known remaining issues (may or may not be fixed):
- PreStop hook timeout (kubelet doesn't enforce timeout on lifecycle handlers)
- CRD FieldValidation (creation rejected for unknown reason)
- ResourceQuota tracking (controller may be slow)
- Services endpoints same port/different protocol

## Previous Rounds Summary
- Round 25: 12 failures (down from 15)
- Round 26: 4 failures (only known issues, all with fixes committed)
- Round 27: 2 failures so far (chunking + variable expansion, fixes committed)
- Round 28: deploying all fixes

## All historical fixes: 64+ from rounds 1-23, plus 12 new fixes above
