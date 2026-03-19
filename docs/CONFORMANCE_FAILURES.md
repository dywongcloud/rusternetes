# Full Conformance Failure Analysis

**Last updated**: 2026-03-19 (round 6 — protobuf, subpath, GC, preemption)

## Fixed Issues (25 root causes)

| # | Issue | Tests | Commit |
|---|-------|-------|--------|
| 1 | Status subresource routing | ~8 | c89343c |
| 2 | Projected volume support | ~20 | c89343c |
| 3 | Service type defaulting + headless | ~6 | 09ed555, 640da11 |
| 4 | Lease MicroTime format | ~1 | 09ed555 |
| 5 | EndpointSlice managed-by label | ~1 | 09ed555 |
| 6 | Discovery auth + webhook + aggregated | ~3 | 09ed555, e21ad25 |
| 7 | DeleteCollection routes | ~3 | 09ed555 |
| 8 | Exec query parsing | ~30 | ebcc6b8 |
| 9 | Pod IP / Container ID / Image ID | ~5 | 894bdc1 |
| 10 | SA automount bypass + NodePort alloc | ~4 | b6a4fea |
| 11 | Watch sendInitialEvents + selectors | ~all | 767a005, c36367f |
| 12 | Watch support in all handlers | ~20 | c36367f |
| 13 | CronJob ? + downwardAPI + ConfigMap/Event | ~10 | aecc290 |
| 14 | WebSocket log streaming | ~1 | aecc290 |
| 15 | NodePort MASQUERADE + ephemeral route | ~4 | 875eecf |
| 16 | Watch empty RV + status details | ~3 | ad10a8b |
| 17 | DaemonSet dupes + node deser + validation | ~4 | ad10a8b |
| 18 | IPAddress API + RoleBinding errors | ~2 | ad10a8b |
| 19 | Protobuf 406 fallback (CRD tests) | ~3 | 6d0788a |
| 20 | SubPathExpr variable expansion | ~2 | 6d0788a |
| 21 | GC replicationcontrollers scan | ~2 | 6d0788a |
| 22 | Scheduler preemption eviction | ~2 | 6d0788a |

## Remaining (infrastructure only)

| # | Issue | Tests | Reason |
|---|-------|-------|--------|
| A | 2 nodes required | ~2 | Single-node cluster |

## Test Results

| Run | Date | Passed | Failed | Total | Rate |
|-----|------|--------|--------|-------|------|
| Quick mode | 2026-03-18 | 1 | 0 | 1 | 100% |
| Full run 1 | 2026-03-19 | 11 | 75 | 86 | 13% |
| Full run 5 | pending | — | — | 441 | — |
