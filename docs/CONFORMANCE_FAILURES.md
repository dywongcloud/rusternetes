# Conformance Issue Tracker

**190 fixes** | 24 pending deploy | Build clean, all unit tests pass

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

## Round 98 results (in progress)

8 passed, 7 failed so far (15/441 done)

## Active failures (round 98)

| Test | Error | Root Cause |
|------|-------|------------|
| output.go:263 | `FOOBAR=$(FOO);;$(BAR)` not expanded | **FIXED #189** — expand `$(VAR)` in env values using prior env vars |
| crd_publish_openapi.go:161 | `failed to decode CRD: missing field 'spec'` | **FIXED #190** — return 415 for native protobuf; client retries with JSON |
| field_validation.go:570 | `key must be a string at line 1 column 2` | **FIXED #190** — same protobuf issue |
| validatingadmissionpolicy.go:120 | wait for marker timeout | Watch events or VAP controller issue |
| runtimeclass.go:153 | timeout | Still failing despite watch handler fix (#180) — may need kube-root-ca.crt |
| statefulset.go:786 | timed out scaling | StatefulSet controller timing/watch |
| statefulset.go:2253 | timed out | StatefulSet readiness probe timing |

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
