# Conformance Issue Tracker

**278 total fixes** | Round 104 in progress | 34 failures, ~25+ fixed by pending deploy (#270-278)

## Round 104 Failures (31 so far)

### FIXED — pending deploy
| Fix # | Tests affected | Description |
|-------|---------------|-------------|
| 270 | ~13 tests | Kubelet readiness write: remove duplicate CAS write, re-read pod. Fixes SS scaling, rolling updates, job timeouts, init containers, webhook deployments, endpoints |
| 271 | 1 test | RestartCount tracking in Running→Stopped→Restart |
| 272 | 4 tests | Watch rv=1 sends initial ADDED events. Fixes service, resourcequota, configmap watchers |
| 273 | 1 test | PreStop hooks: stop before delete from storage |
| 274 | 1 test | Aggregated discovery: dynamic CRD groups |
| 275 | 1 test | Job maxFailedIndexes check |
| 276 | 1 test | Downward API labels trailing newline |
| 277 | 1 test | Pod SSA respects dryRun=All |
| 278 | 5 tests | CRD status update triggers MODIFIED event for Established watch |

### UNFIXED — need new code
| Test | Error | Root cause |
|------|-------|------------|
| CRD conversion webhook | CR v1→v2 | Conversion webhook not implemented |
| Service NodePort→ExternalName | DNS nslookup fails | ExternalName CNAME in CoreDNS |
| EmptyDir 0666 | File 0644 not 0666 | Docker Desktop filesystem/umask |
| kubectl create -f (protobuf) | OpenAPI proto parse | kubectl requires protobuf OpenAPI v2 |

### Pending deploy summary
| Fix # | Component | Description |
|-------|-----------|-------------|
| 270 | kubelet | Readiness: remove duplicate write, re-read pod for fresh RV |
| 271 | kubelet | RestartCount in restart path |
| 272 | api-server | Watch: rv=1 treated like rv=0 |
| 273 | kubelet | PreStop: stop before delete, resolve pause IP |
| 274 | api-server | Aggregated discovery: dynamic CRD groups |
| 275 | controller-manager | Job: maxFailedIndexes |
| 276 | kubelet | Downward API: trailing newline |
| 277 | api-server | Pod SSA: dryRun=All |
| 278 | api-server | CRD: status update after create |

## Progress

| Round | Pass | Fail | Total | Rate | Key changes |
|-------|------|------|-------|------|-------------|
| 97 | ~40 | ~400 | 441 | ~9% | Baseline |
| 101 | 245 | 196 | 441 | 56% | 76 fixes deployed |
| 103 | 46 | 30 | 76 | 60% | fsGroup, session affinity |
| 104 | ~410 | 31 | 441 | ~93% | #255-269 deployed, #270-278 pending |
