# Conformance Issue Tracker

**274 total fixes** | Build clean | Round 104: 13 failures at ~50/441 tests run

## What Still Needs Fixing

### Round 104 failures (13 so far, test still running)
| # | Test | Error | Status |
|---|------|-------|--------|
| 270 | statefulset.go:786 | SS scaling timeout — readiness never persisted | **FIXED #270** pending deploy |
| 271 | runtime.go:115 | RestartCount never incremented | **FIXED #271** pending deploy |
| 274 | aggregated_discovery.go:227 | CRD not in aggregated discovery | **FIXED #274** pending deploy — added dynamic CRD groups |
| 272 | service.go:3304 | Watch rv=1 missed service ADDED event | **FIXED #272** pending deploy — treat rv=1 like rv=0 |
| — | endpointslice test | kubectl exec fails | kubectl exec not fully implemented |
| 272 | resource_quota.go:1152 | Watch rv=1 missed ResourceQuota | **FIXED #272** pending deploy |
| 273 | lifecycle_hook.go:132 | PreStop HTTP hook never executed | **FIXED #273** pending deploy — stop before delete from storage |
| — | job.go:548 | Pod failure policy — DisruptionTarget | Job controller issue |
| — | statefulset.go (rolling updates) | SS rolling update timeout | Readiness persistence (#270) |
| — | job.go:665 | maxFailedIndexes termination | Job controller — indexed job tracking |
| — | downwardapi_volume.go:155 | Label update not reflected in volume | Volume resync timing |
| — | custom_resource_definition.go:104 | CRD create: context deadline exceeded | CRD protobuf issue (#268) |
| — | output.go:263 | EmptyDir file perms: expected 0666, got 0644 | Docker container umask 0022 |

### Pending deploy (code written, needs rebuild)
| # | Fix | Expected impact |
|---|-----|-----------------|
| 270 | Kubelet readiness: remove duplicate write, re-read pod for fresh RV | ~15 timing failures |
| 271 | Kubelet restart count: track RestartCount in Running→Stopped→Restart path | 1 test |
| 272 | Watch: treat rv=1 like rv=0 for initial ADDED events | 2+ tests (service, resourcequota) |
| 273 | PreStop hooks: stop containers before deleting from storage + resolve pod IP from pause container | 1 test |
| 274 | Aggregated discovery: dynamically include CRD groups from storage | 1 test |

### Code bugs still needing investigation
| Issue | Error | What to do |
|-------|-------|------------|
| kubectl exec | Exec subresource returns error | Need full exec websocket/SPDY implementation |
| EmptyDir 0666 | File mode 0644 instead of 0666 | Docker umask issue — need to set umask in container or use chmod |
| Job maxFailedIndexes | Job not terminated when failed indexes exceed limit | Job controller needs maxFailedIndexes tracking |
| Job pod failure policy | DisruptionTarget condition handling | Job controller needs pod failure policy implementation |
| Label update volume | Downward API volume not updated on label change | Volume resync needs to check pod labels |
| CRD create timeout | Creating CRD via protobuf times out | CRD protobuf decoder may need improvement |
| SS rolling update | StatefulSet rolling update timeout | Readiness persistence (#270) should fix |

### Previously deployed fixes (Round 104 build)
Fixes #255-269 deployed. Key: kubelet sync 1s (#255), SA JTI (#256), VAP status (#257), MicroTime (#258), ResourceClaim (#259), SS current_revision (#260), watch resubscribe (#261), namespace finalizer (#262), VAP binding age (#263), RuntimeClass check (#264), TaintEviction (#265), readiness writes (#266), kube-root-ca.crt (#267), CRD subresources (#268), namespace finalize (#269).

## Progress

| Round | Pass | Fail | Total | Rate | Key changes |
|-------|------|------|-------|------|-------------|
| 97 | ~40 | ~400 | 441 | ~9% | Baseline |
| 98 | ~53% | | | 53% | Round 98 fixes |
| 101 | 245 | 196 | 441 | 56% | 76 fixes deployed |
| 102 | ~60% | | | 60% | Webhook URL, CRD protobuf, PDB |
| 103 | 46 | 30 | 76 | 60% | fsGroup, session affinity, IPC sharing |
| 104 | IN PROGRESS | 13 | ~50/441 | ? | Fixes #255-269 deployed, #270-274 pending |

## All Deployed Fixes

<details>
<summary>269 fixes deployed in current build (click to expand)</summary>

Fixes #1-269 are in the current running build. Key categories:

**Infrastructure**: Watch handlers (#188, #197), bookmark RV (#191), list RV (#200), ?watch=true (#197), controller 1s interval (#240, #255)

**API Server**: CRD protobuf (#199, #219, #243, #251, #268), VAP (#198, #214, #257, #263), webhooks (#220), OpenAPI (#213, #247), discovery (#206, #208), PodSecurity (#238), strict validation (#239), fsGroup (#248), namespace finalize (#269)

**Controllers**: StatefulSet (#196, #203, #235, #260), Job (#204, #215), RS (#201), RC (#192), PDB (#222), DaemonSet (#211), Deployment revision, TaintEviction (#265), Namespace (#262, #267)

**Kubelet**: Env var expansion (#189), CPU ceiling (#186), termination msg (#183, #230), lifecycle hooks (#194), probes (#246), projected volumes (#195), resize (#234), sysctls (#242), IPC sharing (#242, #254), hostname (#229, #253), readiness writes (#266), sync interval (#255)

**Networking**: Session affinity (#245), proxy (#182, #216), NodePort (#205)

**Authentication**: SA token (#256), JTI credential-id

**Routes**: PVC status (#202), PV status (#218), IPAddress status (#184), ServiceCIDR (#184-185), deletecollection wiring (#225-227, #231-232), IngressClass watch (#250)

**Common**: MicroTime (#258), ResourceClaim devices (#259), RuntimeClass check (#264)

</details>
