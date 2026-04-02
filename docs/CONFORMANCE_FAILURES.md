# Conformance Issue Tracker

**Round 121** | COMPLETE | Round 122 ready with 17 additional fixes

## KEY FIX: OpenAPI v2 Protobuf Encoding (dcedd60)

The Go client-go library requests OpenAPI v2 via protobuf. Our server returned raw
JSON, causing "proto: cannot parse invalid wire-format data" errors. This broke:
- kubectl validation (8 tests)
- CRD informer initialization (~12 tests — WaitForEstablishedCRD never polled)
- Webhook informer initialization (~13 tests)
- 12K watch "context canceled" errors from broken informers

Now properly wraps JSON in K8s protobuf wire format (magic + envelope).

## Fixes for Round 122 (17 committed, not yet deployed)

| # | Fix | Tests | Commit |
|---|-----|-------|--------|
| 28 | OpenAPI v2 protobuf wire format | ~33 | dcedd60 |
| 29 | CRD watch history replay | ~12 | 5cd32b0 |
| 30 | PriorityClassName → priority value | ~7 | fa65ed7 |
| 31 | Namespace pod termination ordering | ~1 | 313085f |
| 32 | Exec WebSocket channel flush | ~1 | c742a89 |
| 33 | SA node-uid bound tokens | ~3 | d883860 |
| 34 | Scheduler preemption + decimal CPU | ~7 | d883860 |
| 35 | VAP validation actions (422) | ~2 | d883860 |
| 36 | ConfigMap optional volume cleanup | ~1 | d883860 |
| 37 | DaemonSet rolling update | ~2 | 15f5ff9 |
| 38 | Pod proxy port parsing + root | ~2 | 15f5ff9 |
| 39 | SubPath env var expansion | ~2 | 15f5ff9 |
| 40 | LabelSelector Default + serde | ~1 | befccde |
| 41 | Events v1→core field mapping | ~1 | 942c382 |
| 42-44 | Endpoint, kube-proxy, proxy fixes | ~3 | 15f5ff9 |

## Remaining Unfixed (~8 tests)

| Issue | Tests | Root Cause |
|-------|-------|-----------|
| /etc/hosts | 1 | Docker overrides bind mount; need extra_hosts |
| Pod resize | 1 | cgroup cpu.weight reading |
| Lifecycle hooks | 1 | Watch stability (should improve with protobuf fix) |
| PreStop | 1 | PreStop hook validation |
| Sysctl | 1 | Watch stability |
| CSI storage capacity | 1 | Watch stability |
| Logs | 1 | kubectl --since flag line count |
| Aggregator | 1 | Extension API server pod not becoming Ready |

## Known Limitations (~14 tests)

| Issue | Tests | Reason |
|-------|-------|--------|
| Bind mount permissions | 6 | Docker Desktop virtiofs strips write bits |
| kubectl protobuf | 8 | Should be fixed by protobuf encoding |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | ~308 | ~133 | 441 | ~70% (same fixes as R120) |
| 122 | — | — | 441 | PENDING (protobuf + 16 fixes) |
