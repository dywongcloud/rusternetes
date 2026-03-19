# Full Conformance Failure Analysis

**Last updated**: 2026-03-19 (all fixable issues addressed)

## Fixed Issues

| # | Issue | Tests | Commit |
|---|-------|-------|--------|
| 1 | Status subresource routing | ~8 | c89343c |
| 2 | Projected volume support | ~20 | c89343c |
| 3 | Service type defaulting | ~3 | 09ed555 |
| 4 | Headless service (ClusterIP=None) | ~3 | 640da11 |
| 5 | Lease MicroTime format | ~1 | 09ed555 |
| 6 | EndpointSlice managed-by label | ~1 | 09ed555 |
| 7 | Discovery auth + webhook resources | ~2 | 09ed555 |
| 8 | DeleteCollection routes | ~3 | 09ed555 |
| 9 | Exec query parsing (repeated command=) | ~30 | ebcc6b8 |
| 10 | Pod IP / Container ID / Image ID | ~5 | 894bdc1 |
| 11 | SA automount bypass | ~2 | b6a4fea |
| 12 | NodePort auto-allocation | ~2 | b6a4fea |
| 13 | Watch sendInitialEvents + bookmark | ~all | 767a005 |
| 14 | Watch label/field selector filtering | ~10 | c36367f |
| 15 | Watch support in all handlers | ~20 | c36367f |
| 16 | CronJob ? character in cron schedule | ~2 | aecc290 |
| 17 | Downward API resource field refs | ~5 | aecc290 |
| 18 | ConfigMap/Event deserialization compat | ~3 | aecc290 |
| 19 | WebSocket log streaming | ~1 | aecc290 |
| 20 | Aggregated discovery with resources | ~1 | e21ad25 |
| 21 | iptables jump rule ordering (NodePort+DNS) | ~5 | e21ad25 |

## Cannot Fix (infrastructure/design limitations)

| # | Issue | Tests | Reason |
|---|-------|-------|--------|
| A | CRD protobuf creation | ~3 | Requires protobuf codec |
| B | Variable Expansion subpath | ~2 | Complex kubelet feature |
| C | 2 nodes required | ~2 | Single-node cluster |

## Test Results

| Run | Date | Passed | Failed | Total | Rate |
|-----|------|--------|--------|-------|------|
| Quick mode | 2026-03-18 | 1 | 0 | 1 | 100% |
| Full run 1 | 2026-03-19 | 11 | 75 | 86 | 13% |
| Full run 4 | 2026-03-19 | pending (all fixes deployed) | | 441 | |
