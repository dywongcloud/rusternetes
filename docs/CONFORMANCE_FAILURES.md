# Conformance Issue Tracker

**320 total fixes** | Round 108 in progress | ~155 failures on old code, 8 fixes pending redeploy

## Deployed: #1-312 | Pending: #313-320

## Round 108 Fixes Applied (8 commits, #313-320)

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

**Estimated total impact: ~55+ failures resolved** (pending redeploy to verify)

## Round 108 Failures (22 categories)

| # | Test | Error | Fix/Status |
|---|------|-------|-----------|
| 1 | Webhook deployments (~15) | `sethostname: invalid argument` | **FIXED** #313 + #319 |
| 2 | CRD creation/watch (~7) | `context deadline exceeded` | NOT FIXED (protobuf decode) |
| 3 | CRD field validation (~5) | `key must be a string` | NOT FIXED (protobuf/CBOR body) |
| 4 | Deployment pods not ready (~8) | `ReadyReplicas:0` / `missing field 'kind'` | **FIXED** #313 + #319 + #317 |
| 5 | Job completion (~8) | `Timed out` | LIKELY FIXED by #319 |
| 6 | Watch DELETE events (~3) | `Timed out waiting for {DELETED}` | **FIXED** #315 |
| 7 | ResourceQuota (~5) | `Expected an error` | NOT FIXED (service/scoped quotas) |
| 8 | StatefulSet scaling (~5) | `scaled 3 -> 2` / timeouts | LIKELY FIXED by #319 |
| 9 | RC failure conditions (~4) | `never added failure condition` | **FIXED** #314 + #319 |
| 10 | ReplicaSet scaling (~3) | `failed to scale` | LIKELY FIXED by #319 |
| 11 | Init containers (~2) | `PodCondition nil` | LIKELY FIXED by #319 |
| 12 | Pod runtime status (~4) | `container statuses []` | PARTIALLY FIXED by #319 |
| 13 | SA token extra info (~4) | `missing pod-name extra` | PARTIALLY FIXED #316 |
| 14 | LimitRange defaults (~1) | `cpu expected 300m actual 100m` | NOT FIXED |
| 15 | Network/service (~7) | service not reachable / timeouts | LIKELY PARTIALLY FIXED by #319 |
| 16 | /etc/hosts (~1) | `not kubelet managed` | NOT FIXED (Docker overrides bind mount) |
| 17 | kubectl builder (~7) | `mime: unexpected content` | **FIXED** #318 |
| 18 | Scheduler preemption (~4) | `context deadline exceeded` | LIKELY PARTIALLY FIXED by #319 |
| 19 | EmptyDir permissions (~5) | `perms -rwxr-xr-x expected -rwxrwxrwx` | NOT FIXED (no tmpfs) |
| 20 | Aggregated discovery (~3) | `context deadline exceeded` | NOT FIXED (API not implemented) |
| 21 | ConfigMap immutable (~2) | `ConfigMap is immutable` | PARTIALLY FIXED #320 |
| 22 | Misc (~8+) | various | VARIES (see details below) |

### Detail: Issue #22 Miscellaneous
- `expansion.go:419` (x2) - LIKELY FIXED by #319
- `runtimeclass.go:153` - NOT FIXED (RuntimeClass not supported)
- `kubelet.go:127` - LIKELY FIXED by #319
- `pods.go:600, 575` - LIKELY FIXED by #319
- `events.go:124`, `core_events.go:144` - NOT FIXED (Events API field issues)
- `empty_dir_wrapper.go:406` - NOT FIXED (emptyDir wrapper)
- `csistoragecapacity.go:190` - NOT FIXED (CSI not supported)
- `aggregator.go:359` - NOT FIXED (API aggregator not supported)

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
| Other targeted fixes | ~15 | 0 |
| Remaining unfixed | ~100 | ~100 |

**Expected: ~100 failures remaining (down from ~155)**

## Priority Order for Remaining Fixes (post-redeploy)

1. **CRD Creation/Watch** (#2) - 7 failures, protobuf decode issue
2. **CRD Field Validation** (#3) - 5 failures, protobuf/CBOR body decode
3. **ResourceQuota Admission** (#7) - 5 failures, service/scoped quotas
4. **EmptyDir Permissions** (#19) - 5 failures, tmpfs support needed
5. **Aggregated Discovery** (#20) - 3 failures, API not implemented
6. **Network/Service** (#15) - remaining pure networking issues
7. **Everything else** - individual feature gaps

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 104 | 36 | 441 | 92% |
| 105 | 43 | 441 | 90% |
| 106 | ~25 | 441 | ~94% |
| 107 | 19 | ~430/441 | ~96% |
| 108 | ~155 | ~441 | ~65% (old code, pre-deploy) |
| 108 est | ~100 | ~441 | ~77% (post-deploy estimate) |
