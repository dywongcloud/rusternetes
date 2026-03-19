# Full Conformance Failure Analysis

**Last updated**: 2026-03-19 (round 9 — live monitoring + exec proxy)

## Current Run Status (running against pre-exec-proxy images)
- Tests completed: 19 of 441
- Passed: 2, Failed: 17
- Pass rate: 11% (but most fixes haven't been deployed yet)

## Fixed Issues (27 root causes, all committed)

| # | Issue | Tests | Commit |
|---|-------|-------|--------|
| 1-15 | (see previous entries) | ~150 | various |
| 16 | CronJob ? parsing | ~2 | aecc290 |
| 17 | Downward API resource field refs | ~5 | aecc290 |
| 18 | ConfigMap/Event deserialization | ~3 | aecc290 |
| 19 | WebSocket log streaming | ~1 | aecc290 |
| 20 | Aggregated discovery format | ~1 | 875eecf |
| 21 | Ephemeral container PATCH route | ~1 | 875eecf |
| 22 | NodePort MASQUERADE rules | ~3 | 875eecf |
| 23 | Watch empty RV + status details | ~3 | ad10a8b |
| 24 | DaemonSet dupes + node deser + validation | ~4 | ad10a8b |
| 25 | Protobuf 406 + SubPathExpr + GC + preemption | ~9 | 6d0788a |
| 26 | Pod initial Pending phase + probe IPs | ~7 | ad78f7e |
| 27 | Exec proxy to kubelet (Option A architecture) | ~30 | 5ec75ef |

## Known Failures Observed in Current Run

### F1. Watch MODIFIED events not delivered (2+ tests)
ConfigMap mutations don't trigger MODIFIED watch events.
Root cause: etcd watch may not detect key modifications reliably,
or the watch stream filters are too aggressive.

### F2. CronJob scheduling still failing (2 tests)
CronJob tests timeout. The cron ? fix is deployed but the controller
may have other issues (job creation, schedule evaluation timing).

### F3. DNS resolution (1 test)
Pods can't resolve cluster DNS names. CoreDNS integration issue.

### F4. Conformance node check (1 test)
"should have at least two untainted nodes" — nodes might have taints
or the node info isn't complete enough.

### F5. Pod activeDeadlineSeconds (1 test)
Pod with activeDeadlineSeconds should be terminated after deadline.
Kubelet doesn't enforce this.

### F6. Watch restart after close (1 test)
Watch stream doesn't properly handle restart with new resourceVersion.

## Not Yet Deployed (committed but needs rebuild)
- Exec proxy to kubelet (5ec75ef)
- All fixes need fresh docker image builds

## Notes for Next Session
- Rebuild ALL images before next conformance run
- Watch event delivery for MODIFIED is a key systemic issue
- CronJob controller needs debugging
- activeDeadlineSeconds needs kubelet implementation
