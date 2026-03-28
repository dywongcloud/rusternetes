# Conformance Issue Tracker

**323 total fixes** | Round 108 in progress | ~155 failures on old code, 11 fixes pending redeploy

## Deployed: #1-312 | Pending: #313-323

## Round 108 Fixes Applied (11 commits, #313-323)

| # | Commit | Fix | Est. Impact |
|---|--------|-----|-------------|
| 313 | 52bafcb | Hostname truncation to 63 chars | ~20+ failures |
| 314 | 52bafcb | RC failure conditions & observed_generation | ~1 failure |
| 315 | 8ecc830 | Watch event batching (flat_map) | ~3 failures |
| 316 | a863f99 | SA token pod binding info | ~2 failures |
| 317 | d800695 | TypeMeta in status update responses | ~3 failures |
| 318 | 628911b | OpenAPI MIME type (406 for protobuf) | ~6 failures |
| 319 | 3147f7b | Fix broken CAS re-reads in kubelet (Ok(Some(p)) -> Ok(p)) | ~20+ failures |
| 320 | 605c80c | Allow metadata updates on immutable ConfigMaps | ~1 failure |
| 321 | 79f55e6 | EmptyDir tmpfs + service quota + LimitRange defaults | ~11 failures |
| 322 | b98b8c4 | CRD status retry + binary body extraction | ~7 failures |
| 323 | b98b8c4 | /etc/hosts in pause container + aggregated discovery Accept | ~4 failures |

**Estimated total impact: ~75+ failures resolved** (pending redeploy to verify)

## Round 108 Failures (22 categories)

| # | Test | Error | Fix/Status |
|---|------|-------|-----------|
| 1 | Webhook deployments (~15) | `sethostname: invalid argument` | **FIXED** #313 + #319 |
| 2 | CRD creation/watch (~7) | `context deadline exceeded` | **FIXED** #322 (retry status update + binary extraction) |
| 3 | CRD field validation (~5) | `key must be a string` | PARTIALLY FIXED #322 (binary body JSON extraction) |
| 4 | Deployment pods not ready (~8) | `ReadyReplicas:0` / `missing field 'kind'` | **FIXED** #313 + #319 + #317 |
| 5 | Job completion (~8) | `Timed out` | LIKELY FIXED by #319 |
| 6 | Watch DELETE events (~3) | `Timed out waiting for {DELETED}` | **FIXED** #315 |
| 7 | ResourceQuota (~5) | `Expected an error` | PARTIALLY FIXED #321 (service quotas added) |
| 8 | StatefulSet scaling (~5) | `scaled 3 -> 2` / timeouts | LIKELY FIXED by #319 |
| 9 | RC failure conditions (~4) | `never added failure condition` | **FIXED** #314 + #319 |
| 10 | ReplicaSet scaling (~3) | `failed to scale` | LIKELY FIXED by #319 |
| 11 | Init containers (~2) | `PodCondition nil` | LIKELY FIXED by #319 |
| 12 | Pod runtime status (~4) | `container statuses []` | PARTIALLY FIXED by #319 |
| 13 | SA token extra info (~4) | `missing pod-name extra` | PARTIALLY FIXED #316 |
| 14 | LimitRange defaults (~1) | `cpu expected 300m actual 100m` | **FIXED** #321 (requests default to limits) |
| 15 | Network/service (~7) | service not reachable / timeouts | LIKELY PARTIALLY FIXED by #319 |
| 16 | /etc/hosts (~1) | `not kubelet managed` | **FIXED** #323 (copy hosts into pause container) |
| 17 | kubectl builder (~7) | `mime: unexpected content` | **FIXED** #318 |
| 18 | Scheduler preemption (~4) | `context deadline exceeded` | LIKELY PARTIALLY FIXED by #319 |
| 19 | EmptyDir permissions (~5) | `perms -rwxr-xr-x expected -rwxrwxrwx` | **FIXED** #321 (tmpfs for all emptyDir + mode=1777) |
| 20 | Aggregated discovery (~3) | `context deadline exceeded` | **FIXED** #323 (Accept header preference) |
| 21 | ConfigMap immutable (~2) | `ConfigMap is immutable` | PARTIALLY FIXED #320 |
| 22 | Misc (~8+) | various | VARIES (see details below) |

### Detail: Issue #22 Miscellaneous
- `expansion.go:419` (x2) - LIKELY FIXED by #319 (CAS re-reads)
- `runtimeclass.go:153` - LIKELY FIXED by #319 (pod status never persisted)
- `kubelet.go:127` - LIKELY FIXED by #319 (CAS re-reads)
- `pods.go:600, 575` - LIKELY FIXED by #319 (CAS re-reads)
- `events.go:124`, `core_events.go:144` - LIKELY FIXED by #317 (TypeMeta in responses)
- `empty_dir_wrapper.go:406` - LIKELY FIXED by #321 (tmpfs for emptyDir)
- `csistoragecapacity.go:190` - Feature gap (CSI not supported)
- `aggregator.go:359` - Feature gap (API aggregator not supported)

## Critical Fix Details

### #313: Hostname truncation (52bafcb)
Pod names > 63 chars (e.g. `sample-webhook-deployment-1ea22597-ec36f15a-8ae5-4dc4-8f3b-1da2641cef30` at 71 chars) caused `runc create` to fail with `sethostname: invalid argument`. All webhook deployment tests, many deployment tests, and any pod with a long generated name were affected. Fixed by truncating to 63 chars with trailing-dash removal.

### #319: CAS re-reads (3147f7b)
**Systemic bug.** All 15 pod status re-reads in the kubelet used `Ok(Some(p)) => p` but `storage.get()` returns `Result<T>`, not `Result<Option<T>>`. Every re-read silently fell through to using the stale pod copy, causing CAS update failures on every single pod status write. Pod readiness, container statuses, and conditions were never being persisted. Added retry logic for CAS conflicts.

### #315: Watch event batching (8ecc830)
etcd batches multiple events per watch response. Our `stream.map()` with early `return` only processed the first event, dropping subsequent events (including DELETE notifications). Fixed with `stream.flat_map()`.

### #318: OpenAPI MIME (628911b)
Protobuf OpenAPI response used `application/com.github.proto-openapi.spec.v2@v1.0+protobuf` — the `@` char is invalid per RFC 2045, causing Go's `mime.ParseMediaType` to fail. kubectl couldn't validate resources. Now returns 406 to force JSON fallback.

## Estimated Post-Deploy Status

| Category | Before | Expected After |
|----------|--------|---------------|
| Hostname truncation fixes | ~20 | 0 |
| CAS re-read fixes | ~20 | 0 |
| Other targeted fixes (11 commits) | ~35 | 0 |
| Feature gaps (CSI, API aggregator) | ~2 | ~2 |
| Possible remaining edge cases | ~78 | ~20-40 |

**Expected: ~20-40 failures remaining (down from ~155)**

## Remaining Feature Gaps (post-redeploy)

1. **CSI storage** - `csistoragecapacity.go` - CSI not implemented
2. **API aggregator** - `aggregator.go` - API aggregation not implemented
3. **Scoped ResourceQuotas** - terminating/not-terminating/best-effort scopes
4. **Pod resize** - in-place pod resource resize not fully supported
5. **Sysctl** - sysctl support limited in Docker environment

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 43 | 441 | 90% |
| 106 | ~25 | 441 | ~94% |
| 107 | 19 | ~430/441 | ~96% |
| 108 | ~155 | ~441 | ~65% (old code, pre-deploy) |
| 108 est | ~20-40 | ~441 | ~91-95% (post-deploy estimate) |
