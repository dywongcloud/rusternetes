# Conformance Issue Tracker

**281 total fixes** | Round 104 in progress | 36 failures, ~30 fixed by pending deploy

## Pending Deploy (#270-281)
| Fix # | Tests | Description |
|-------|-------|-------------|
| 270 | ~15 | Kubelet readiness: remove duplicate CAS write, re-read pod for fresh RV |
| 271 | 1 | RestartCount tracking in Running→Stopped→Restart |
| 272 | ~5 | Watch: rv=1 sends initial ADDED events |
| 273 | 1 | PreStop hooks: stop before delete from storage, resolve pause IP |
| 274 | 1 | Aggregated discovery: dynamic CRD groups from storage |
| 275 | 1 | Job controller: maxFailedIndexes check |
| 276 | 1 | Downward API labels/annotations trailing newline |
| 277 | 1 | Pod server-side apply respects dryRun=All |
| 278 | ~5 | CRD status update triggers MODIFIED event for Established watch |
| 279 | 1 | VAP messageExpression CEL evaluation |
| 280 | 1 | InvalidResource returns 422 (not 400), immutable secret message |
| 281 | ~5 | OpenAPI v2: 406 for protobuf Accept forces JSON fallback |

## Unfixed Issues
| Test | Error | Root cause |
|------|-------|------------|
| CRD conversion webhook | CR v1→v2 | Conversion webhook not implemented |
| Service NodePort→ExternalName | DNS nslookup | ExternalName CNAME in CoreDNS |
| EmptyDir 0666 | File 0644 | Docker Desktop filesystem/umask |
| Session affinity NodePort | Affinity switch | iptables session affinity |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 97 | ~400 | 441 | ~9% |
| 101 | 196 | 441 | 56% |
| 103 | 30 | 76 | 60% |
| 104 | 36 | 441 | ~92% |
