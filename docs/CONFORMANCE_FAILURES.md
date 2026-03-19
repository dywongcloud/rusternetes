# Full Conformance Failure Analysis

**Last updated**: 2026-03-19 (after fixes round 5 — ALL fixable issues addressed)

## Fixed Issues (30 total)

| # | Issue | Tests | Commit |
|---|-------|-------|--------|
| 1 | Status subresource routing | ~8 | c89343c |
| 2 | Projected volume support | ~20 | c89343c |
| 3 | Service type defaulting | ~3 | 09ed555 |
| 4 | Headless service ClusterIP=None | ~3 | 640da11 |
| 5 | Lease MicroTime format | ~1 | 09ed555 |
| 6 | EndpointSlice managed-by label | ~1 | 09ed555 |
| 7 | Discovery auth + webhook resources | ~2 | 09ed555 |
| 8 | DeleteCollection routes | ~3 | 09ed555 |
| 9 | Exec query parameter parsing | ~30 | ebcc6b8 |
| 10 | Pod IP / Container ID / Image ID | ~5 | 894bdc1 |
| 11 | SA automount bypass | ~2 | b6a4fea |
| 12 | NodePort auto-allocation | ~2 | b6a4fea |
| 13 | Watch sendInitialEvents + bookmark | ~all | 767a005 |
| 14 | Watch label/field selector filtering | ~10 | c36367f |
| 15 | Watch support in all handlers | ~20 | c36367f |
| 16 | CronJob cron ? parsing | ~2 | aecc290 |
| 17 | Downward API resource field refs | ~5 | aecc290 |
| 18 | ConfigMap/Event deserialization | ~3 | aecc290 |
| 19 | WebSocket log streaming | ~1 | aecc290 |
| 20 | Aggregated discovery format | ~1 | 875eecf |
| 21 | Ephemeral container PATCH route | ~1 | 875eecf |
| 22 | NodePort MASQUERADE rules | ~3 | 875eecf |
| 23 | Watch with empty resourceVersion | ~2 | ad10a8b |
| 24 | Status response 409 details | ~1 | ad10a8b |
| 25 | DaemonSet duplicate pod creation | ~1 | ad10a8b |
| 26 | Node update deserialization | ~2 | ad10a8b |
| 27 | Resource name validation | ~1 | ad10a8b |
| 28 | IPAddress API watch/filtering | ~1 | ad10a8b |
| 29 | RoleBinding error messages | ~1 | ad10a8b |
| 30 | List label selector filtering | ~3 | ad10a8b |

## Cannot Fix (infrastructure/design limitations)

| # | Issue | Tests | Reason |
|---|-------|-------|--------|
| A | CRD protobuf creation | ~3 | Need protobuf codec |
| B | Variable Expansion subpath | ~2 | Complex kubelet feature |
| C | 2-node requirement | ~2 | Single-node cluster |
| D | DNS resolution (partial) | ~2 | CoreDNS upstream compat |
