# Conformance Issue Tracker

**194 fixes** | 28 pending deploy | Build clean, all unit tests pass

## Pending deploy fixes (since round 97)

| # | Fix | Impact |
|---|-----|--------|
| 169 | generation=1, ClusterIP, SA token, PodScheduled | 5+ tests |
| 170 | **CRITICAL** resourceVersion in watch events | 12+ tests |
| 171 | Endpoints single subset | 1 test |
| 172 | Ensure metadata for resourceVersion | 1 test |
| 173 | Remove duplicate SA token route (panic) | startup |
| 174 | **CRITICAL** List RV from items, not timestamps | ALL tests |
| 175 | Immutable returns 403 Forbidden | 2 tests |
| 176 | RC orphan handling + DaemonSet ControllerRevision | 2 tests |
| 177 | Aggregated discovery responseKind.group empty | 1 test |
| 178 | In-place pod resize via Docker update_container | 1 test |
| 179 | CEL matchConditions validation in webhook create handlers | 1 test |
| 180 | RuntimeClass watch handler + route | 2 tests |
| 181 | ResourceQuota watch handlers + routes (ns + all) | 2 tests |
| 182 | Proxy double-slash path fix (node/service/pod) | 1 test |
| 183 | Termination message bind-mount + host-file read | 1 test |
| 184 | IPAddress status route + ServiceCIDR Ready condition | 1 test |
| 185 | ServiceCIDR + IPAddress watch handlers + routes | 1 test |
| 186 | CPU/memory downward API: ceiling division (not floor) | 2 tests |
| 187 | CRD status: Established + NamesAccepted conditions on create | 4 tests |
| 188 | Add 23 missing watch handlers + routes (CRD, webhooks, VAP, PDB, RBAC, storage, etc.) | many tests |
| 189 | Env var `$(VAR)` expansion in container env values | 1+ tests |
| 190 | Return 415 for native protobuf bodies (CRD client retries with JSON) | 3+ tests |
| 191 | **CRITICAL** Fix bookmark resourceVersion: 0 → use current etcd revision | many tests |
| 192 | ResourceQuota: count replicationcontrollers + resourcequotas | 1 test |
| 193 | RuntimeClass list handler supports `?watch=true` query param | 1 test |
| 194 | Lifecycle hook exec handler: 30s timeout (was infinite) | 1 test |

## Round 98 results (in progress)

8 passed, 7 failed so far (15/441 done)

## Active failures (round 98)

| Test | Error | Root Cause |
|------|-------|------------|
| output.go:263 | `FOOBAR=$(FOO);;$(BAR)` not expanded | **FIXED #189** — expand `$(VAR)` in env values using prior env vars |
| crd_publish_openapi.go:161 | `failed to decode CRD: missing field 'spec'` | **FIXED #190** — return 415 for native protobuf; client retries with JSON |
| field_validation.go:570 | `key must be a string at line 1 column 2` | **FIXED #190** — same protobuf issue |
| validatingadmissionpolicy.go:120 | wait for marker timeout | Watch events or VAP controller issue |
| runtimeclass.go:153 | timeout | **FIXED #193** — list handler now supports ?watch=true query param |
| statefulset.go:786 | timed out scaling | StatefulSet controller timing/watch |
| statefulset.go:2253 | timed out | StatefulSet readiness probe timing |
| projected_configmap.go:367 | Error reading projected configmap file | Projected volume ConfigMap data not available after update |
| projected_downwardapi.go:155 | timeout | Projected downward API volume issue |
| service.go:251 | affinity timeout | Session affinity iptables recent module |
| lifecycle_hook.go:132 | Timed out after 30s | **FIXED #194** — exec handler had no timeout (blocked forever) |
| **bookmark resourceVersion: 0** | Watch bookmarks sent with RV "0" | **FIXED #191** — initialize with current etcd revision, not "0". All 4 watch functions fixed. |
| resource_quota.go:422 | missing replicationcontrollers, resourcequotas in status.used | **FIXED #192** — added RC + RQ counting to quota controller |
| output.go:263 (2nd) | env var output wrong | May be fixed by #189 (env var expansion) — needs redeploy |

## Previously fixed (deployed in round 98)

| Test | Fix |
|------|-----|
| output.go:263,:282 (CPU) | **#186** — ceiling division |
| runtime.go:169 | **#183** — termination msg bind-mount |
| webhook.go:837 | **#179** — CEL validation |
| runtimeclass.go (watch) | **#180** — watch handler |
| resource_quota.go | **#181** — watch handlers |
| service_cidrs.go | **#184-185** — status route + watch |
| kubectl.go:1881 | **#182** — proxy path fix |
| CRD conditions | **#187** — Established+NamesAccepted |
| 23 watch handlers | **#188** — all resource types |
