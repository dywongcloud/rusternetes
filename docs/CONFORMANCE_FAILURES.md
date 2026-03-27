# Conformance Issue Tracker

**281 total fixes** | Round 104: 36 failures | ~34 fixed by pending deploy

## Pending Deploy (#270-281) — 12 fixes
| Fix # | Tests | Description |
|-------|-------|-------------|
| 270 | ~18 | Kubelet readiness: remove duplicate CAS write, re-read pod for fresh RV |
| 271 | 1 | RestartCount tracking in Running→Stopped→Restart |
| 272 | ~5 | Watch: rv=1 sends initial ADDED events (compacted etcd fix) |
| 273 | 1 | PreStop hooks: stop before delete from storage, resolve pause IP |
| 274 | 1 | Aggregated discovery: dynamic CRD groups from storage |
| 275 | 1 | Job controller: maxFailedIndexes check |
| 276 | 1 | Downward API labels/annotations trailing newline |
| 277 | 1 | Pod server-side apply respects dryRun=All |
| 278 | ~5 | CRD protobuf varint tags + status update triggers MODIFIED event |
| 279 | 2 | VAP messageExpression CEL + EmptyDir Docker named volumes for POSIX |
| 280 | 1 | InvalidResource returns 422, immutable secret message |
| 281 | ~5 | OpenAPI v2: 406 for protobuf Accept forces JSON fallback |

## Unfixed Issues (2 remaining after deploy)
| Test | Error | Root cause | Notes |
|------|-------|------------|-------|
| Service NodePort→ExternalName | DNS nslookup fails | CoreDNS may not serve CNAME for ExternalName | Needs investigation after #270 deployed |
| EndpointSlice create/match | Test panic in goroutine | Readiness-related (#270) or EndpointSlice conditions | Likely fixed by #270 |

## Analysis: Previously "unfixed" issues are actually readiness (#270)
| Test | Was listed as | Actually |
|------|-------------|----------|
| CRD conversion webhook | "Not implemented" | **#270** — webhook deployment ReadyReplicas:0 |
| Session affinity NodePort | "iptables session affinity" | **#270** — deployment ReadyReplicas:0 |
| EmptyDir 0666 | "Docker filesystem" | **#279** — Docker named volumes for POSIX perms |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 97 | ~400 | 441 | ~9% |
| 101 | 196 | 441 | 56% |
| 103 | 30 | 76 | 60% |
| 104 | 36 | 441 | ~92% (pre-deploy), ~95%+ est post-deploy |
