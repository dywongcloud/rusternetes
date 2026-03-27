# Conformance Issue Tracker

**278 total fixes** | Round 104 IN PROGRESS: 30 failures (27 unique) at ~400/441 tests

## What Still Needs Fixing

### Expected to fix with pending deploy (#270-278) — ~18 of 27 unique failures
| Test | Error | Fix |
|------|-------|-----|
| statefulset.go:786 | SS scaling timeout — readiness | **#270** CAS conflict fix |
| statefulset.go:2253 | SS rolling update timeout | **#270** readiness |
| statefulset.go:1092 | SS test timeout | **#270** readiness |
| runtime.go:115 | RestartCount=0 expected 2 | **#271** re-read pod for status writes |
| service.go:3304 | Watch rv=1 missed ADDED event | **#272** treat rv=1 like rv=0 |
| resource_quota.go:1152 | Watch rv=1 missed ResourceQuota | **#272** same |
| lifecycle_hook.go:132 | PreStop HTTP hook never executed | **#273** stop_pod_for on deletion |
| aggregated_discovery.go:227 | CRD not in discovery | **#274** dynamic CRD groups |
| job.go:236 | Pod failure policy — pods never ready | **#270** readiness |
| job.go:665 | maxFailedIndexes | **#275** Job controller check |
| job.go:817 | Job pods not ready | **#270** readiness |
| watch.go:409 (x2) | Configmap watch label selector | **#276** 'in'/'notin' operators |
| downwardapi_volume.go:155 | Label update not in volume | **#277** resync standalone downwardAPI |
| kubectl.go:1130 | dry-run=server persisted | **#277** SSA dry-run check |
| init_container.go:440 | Init container timeout | **#270** readiness |
| webhook.go:1133 (x2) | Webhook deployment not ready | **#270** readiness |
| endpointslice.go:798 | EndpointSlice rate limiter timeout | **#270** readiness |
| custom_resource_definition.go:104 | CRD create timeout (complex) | **#278** protobuf varint tags |
| custom_resource_definition.go:288 | CRD create timeout | **#278** protobuf varint tags |
| field_validation.go:428 | CRD field validation fail | **#278** protobuf varint tags |
| field_validation.go:700 | CRD decode error | **#278** protobuf varint tags |
| crd_publish_openapi.go:366 | CRD OpenAPI timeout | **#278** protobuf varint tags |

### Need additional fixes (~5 remaining failures)
| Test | Error | Root cause | Difficulty |
|------|-------|------------|-----------|
| util.go:182 | kubectl exec fails (exit code 7) | Exec networking — curl unreachable | Medium |
| output.go:263 | EmptyDir file perms 0644 expected 0666 | Docker Desktop virtiofs chmod | Platform |
| crd_conversion_webhook.go:318 | Conversion webhook deployment | Not implemented | Hard |
| service.go:1571 | ExternalName DNS resolution fails | CoreDNS CNAME for ExternalName | Medium |
| builder.go:97 | kubectl create -f - fails | kubectl stdin handling | Medium |

### Pending deploy (code written, needs rebuild)
| # | Fix | Expected impact |
|---|-----|-----------------|
| 270 | Kubelet readiness: remove duplicate CAS write, re-read for fresh RV | ~8 tests |
| 271 | All pod status writes re-read from storage for fresh RV | CAS conflicts |
| 272 | Watch: treat rv=1 like rv=0 — use live cache not compacted etcd | 2 tests |
| 273 | Pod deletion calls stop_pod_for (preStop lifecycle hooks) | 1 test |
| 274 | Aggregated discovery: dynamically include CRD groups | 1 test |
| 275 | Job controller: maxFailedIndexes check | 1 test |
| 276 | Watch label selector: 'in', 'notin', '!key' set-based operators | 2 tests |
| 277 | Downward API volume resync + SSA dryRun + trailing newline | 3 tests |
| 278 | CRD protobuf decoder: multi-byte varint tag support | 4-5 tests |

## Progress

| Round | Pass | Fail | Total | Rate | Key changes |
|-------|------|------|-------|------|-------------|
| 97 | ~40 | ~400 | 441 | ~9% | Baseline |
| 98 | | | | 53% | Round 98 fixes |
| 101 | 245 | 196 | 441 | 56% | 76 fixes deployed |
| 103 | 46 | 30 | 76 | 60% | fsGroup, session affinity, IPC |
| 104 | IN PROGRESS | 30 | ~400/441 | ~93% est | #255-269 deployed, #270-278 pending |

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
