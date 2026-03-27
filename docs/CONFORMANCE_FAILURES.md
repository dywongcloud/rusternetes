# Conformance Issue Tracker

**277 total fixes** | Round 104: 15 failures at ~100/441, test still running

## What Still Needs Fixing

### Round 104 failures — categorized
#### Fixed pending deploy (#270-275)
| Test | Error | Fix |
|------|-------|-----|
| statefulset.go:786 | SS scaling timeout | **#270** readiness write CAS fix |
| runtime.go:115 | RestartCount=0 | **#271** restart count tracking |
| statefulset.go (rolling updates) | SS rolling update timeout | **#270** readiness persistence |
| job.go:236 | Pod failure policy — pods never ready | **#270** readiness persistence |
| job.go:665 | maxFailedIndexes | **#275** Job controller check |
| service.go:3304 | Watch rv=1 missed ADDED | **#272** treat rv=1 like rv=0 |
| resource_quota.go:1152 | Watch rv=1 missed ADDED | **#272** treat rv=1 like rv=0 |
| aggregated_discovery.go:227 | CRD not in discovery | **#274** dynamic CRD groups |
| lifecycle_hook.go:132 | PreStop hook never ran | **#273** stop before delete |

#### Still need investigation/fixes
| Test | Error | Root cause |
|------|-------|------------|
| endpointslice test | kubectl exec fails | exec subresource not implemented |
| watch.go:409 | Configmap watch missed events | Watch cache broadcast timing race |
| downwardapi_volume.go:155 | Label update not in volume | Volume resync timing |
| custom_resource_definition.go:104 | CRD create timeout | CRD protobuf decoder |
| CRD conversion webhook | CR v1→v2 conversion | Conversion webhook not implemented |
| output.go:263 | EmptyDir 0666 → 0644 | Docker umask or chmod in mount-tester |

### Pending deploy (code written, needs rebuild)
| # | Fix | Expected impact |
|---|-----|-----------------|
| 270 | Kubelet readiness: remove duplicate write, re-read pod for fresh RV | ~5-8 tests |
| 271 | Kubelet restart count in Running→Stopped→Restart path | 1 test |
| 272 | Watch: treat rv=1 like rv=0 for initial ADDED events | 2+ tests |
| 273 | PreStop hooks: stop containers before deleting from storage | 1 test |
| 274 | Aggregated discovery: dynamic CRD groups from storage | 1 test |
| 275 | Job controller: maxFailedIndexes check | 1 test |
| 276 | Watch label selector: 'in', 'notin', '!key' set-based operators | watch.go:409 + multiple tests |
| 277 | Resync standalone downwardAPI volumes on label changes | 1 test (downwardapi_volume.go:155) |

## Progress

| Round | Pass | Fail | Total | Rate | Key changes |
|-------|------|------|-------|------|-------------|
| 97 | ~40 | ~400 | 441 | ~9% | Baseline |
| 98 | ~53% | | | 53% | Round 98 fixes |
| 101 | 245 | 196 | 441 | 56% | 76 fixes deployed |
| 102 | ~60% | | | 60% | Webhook URL, CRD protobuf, PDB |
| 103 | 46 | 30 | 76 | 60% | fsGroup, session affinity, IPC sharing |
| 104 | IN PROGRESS | 15 | ~100/441 | ~85% est | Fixes #255-269 deployed, #270-275 pending |

## All Deployed Fixes

<details>
<summary>269 fixes deployed in current build (click to expand)</summary>

Fixes #1-269 deployed. Key: kubelet sync 1s (#255), SA JTI (#256), VAP status (#257), MicroTime (#258), ResourceClaim (#259), SS current_revision (#260), watch resubscribe (#261), namespace finalizer (#262), VAP binding age (#263), RuntimeClass check (#264), TaintEviction (#265), readiness writes (#266), kube-root-ca.crt (#267), CRD subresources (#268), namespace finalize (#269).

</details>
