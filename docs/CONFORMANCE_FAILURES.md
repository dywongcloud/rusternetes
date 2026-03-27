# Conformance Issue Tracker

**280 total fixes** | Round 104 in progress | 36+ failures

## Round 104 Failures

### FIXED — pending deploy (#270-280)
| Fix # | Tests affected | Description |
|-------|---------------|-------------|
| 270 | ~15 tests | Kubelet readiness write: remove duplicate CAS write, re-read pod |
| 271 | 1 test | RestartCount tracking in Running→Stopped→Restart |
| 272 | ~5 tests | Watch rv=1 sends initial ADDED events |
| 273 | 1 test | PreStop hooks: stop before delete from storage |
| 274 | 1 test | Aggregated discovery: dynamic CRD groups |
| 275 | 1 test | Job maxFailedIndexes check |
| 276 | 1 test | Downward API labels trailing newline |
| 277 | 1 test | Pod SSA respects dryRun=All |
| 278 | ~5 tests | CRD status update triggers MODIFIED event |
| 279 | 1 test | VAP messageExpression CEL evaluation |
| 280 | 1 test | InvalidResource returns 422, immutable secret message |

### UNFIXED — need new code
| Test | Error | Root cause |
|------|-------|------------|
| CRD conversion webhook | CR v1→v2 | Conversion webhook not implemented |
| Service NodePort→ExternalName | DNS nslookup fails | ExternalName CNAME in CoreDNS |
| EmptyDir 0666 | File 0644 not 0666 | **FIXED #279** — Docker named volumes for POSIX perms |
| kubectl create -f (protobuf) | OpenAPI proto parse | kubectl requires protobuf OpenAPI v2 |
| Session affinity NodePort | Affinity switch fails | iptables session affinity |
| EndpointSlice create/match | Panic in goroutine | EndpointSlice condition check |

## Progress

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 97 | ~40 | ~400 | 441 | ~9% |
| 101 | 245 | 196 | 441 | 56% |
| 103 | 46 | 30 | 76 | 60% |
| 104 | ? | 36+ | 441 | ~92% est |
