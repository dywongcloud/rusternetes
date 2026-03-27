# Conformance Issue Tracker

**277 total fixes** | Round 104: 19 failures, ~200/441 tests run

## What Still Needs Fixing

### Fixed by pending deploy (#270-277) — 11 of 19 failures
| Test | Error | Fix |
|------|-------|-----|
| statefulset.go:786 | SS scaling timeout | **#270** readiness write CAS fix |
| runtime.go:115 | RestartCount=0 | **#271** restart count tracking |
| aggregated_discovery.go:227 | CRD not in discovery | **#274** dynamic CRD groups |
| service.go:3304 | Watch rv=1 missed ADDED | **#272** treat rv=1 like rv=0 |
| resource_quota.go:1152 | Watch rv=1 missed ADDED | **#272** treat rv=1 like rv=0 |
| lifecycle_hook.go:132 | PreStop hook never ran | **#273** stop before delete |
| job.go:236 | Pod failure policy — pods never ready | **#270** readiness persistence |
| statefulset.go (rolling) | SS rolling update timeout | **#270** readiness persistence |
| job.go:665 | maxFailedIndexes | **#275** Job controller check |
| init_container.go:440 | Init container fail — timeout | **#270** readiness persistence |
| kubectl.go:1130 | dry-run=server persisted | **#277** dry-run in SSA path |

### Still need fixes — 8 of 19 failures
| Test | Error | Root cause | Complexity |
|------|-------|------------|-----------|
| endpointslice test | kubectl exec → curl fails | Exec works but curl target unreachable | Medium |
| watch.go:409 | Configmap watch events missed | Watch cache broadcast timing | Hard |
| downwardapi_volume.go:155 | Label update not in volume | Volume resync works but may be readiness (#270) | Low |
| custom_resource_definition.go:104 | CRD create timeout | CRD protobuf decoder (#268) | Medium |
| output.go:263 | EmptyDir 0666→0644 | Docker Desktop filesystem/umask | Platform |
| CRD conversion webhook | CR v1→v2 | Not implemented | Hard |
| service NodePort→ExternalName | DNS nslookup fails | CoreDNS/ExternalName CNAME | Medium |
| CRD OpenAPI | Multiple CRDs same group | CRD OpenAPI schema generation | Medium |

### Pending deploy
| # | Fix | Impact |
|---|-----|--------|
| 270 | Kubelet readiness: remove duplicate write, re-read pod for fresh RV | ~5 tests |
| 271 | Kubelet restart count in Running→Stopped→Restart path | 1 test |
| 272 | Watch: treat rv=1 like rv=0 for initial ADDED events | 2 tests |
| 273 | PreStop hooks: stop before delete + resolve pod IP from pause | 1 test |
| 274 | Aggregated discovery: dynamic CRD groups from storage | 1 test |
| 275 | Job controller: maxFailedIndexes check | 1 test |
| 276 | Downward API labels/annotations trailing newline | 1 test |
| 277 | Pod server-side apply respects dryRun=All | 1 test |

## Progress

| Round | Pass | Fail | Total | Rate | Key changes |
|-------|------|------|-------|------|-------------|
| 97 | ~40 | ~400 | 441 | ~9% | Baseline |
| 98 | | | | 53% | Round 98 fixes |
| 101 | 245 | 196 | 441 | 56% | 76 fixes deployed |
| 103 | 46 | 30 | 76 | 60% | fsGroup, session affinity, IPC sharing |
| 104 | ~181 | 19 | ~200/441 | ~90% est | #255-269 deployed, #270-277 pending |
