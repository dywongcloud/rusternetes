# Conformance Issue Tracker

**277 total fixes** | Round 104 IN PROGRESS: 21 failures so far at ~250/441 tests

## What Still Needs Fixing

### Fixed by pending deploy (#270-277) — expect ~11 of 21 to resolve
| Test | Error | Fix |
|------|-------|-----|
| statefulset.go:786 | SS scaling timeout — readiness never persisted | **#270** remove duplicate CAS write, re-read for fresh RV |
| runtime.go:115 | RestartCount=0 expected 2 | **#271** re-read pod for all status writes |
| service.go:3304 | Watch rv=1 missed ADDED event | **#272** treat rv=1 like rv=0 (compacted history) |
| resource_quota.go:1152 | Watch rv=1 missed ResourceQuota | **#272** same fix |
| lifecycle_hook.go:132 | PreStop HTTP hook never executed | **#273** pod deletion calls stop_pod_for |
| job.go:236 | Pod failure policy — pods never ready (15min timeout) | **#270** readiness persistence |
| statefulset.go:2253 | SS rolling update timeout | **#270** readiness persistence |
| init_container.go:440 | Init container — timed out waiting | **#270** readiness persistence |
| watch.go:409 | Configmap watch events missed by label selector | **#276** watch label selector 'in'/'notin' operators |
| downwardapi_volume.go:155 | Label update not reflected in volume | **#277** resync standalone downwardAPI volumes |
| kubectl.go:1130 | dry-run=server actually persisted change | **#277** SSA dry-run check |
| webhook.go:1133 | Webhook deployment not ready (BeforeEach) | **#270** readiness persistence |
| builder.go:97 | kubectl create -f - fails (BeforeEach) | **#247** deployed, may be readiness |

### Still need fixes — 8 remaining failures
| Test | Error | Root cause | Status |
|------|-------|------------|--------|
| aggregated_discovery.go:227 | CRD not in aggregated discovery | **#274** dynamic CRD groups (pending deploy) | Pending |
| util.go:182 | kubectl exec fails (exit code 7) | Exec/networking issue — curl target unreachable | Needs investigation |
| job.go:665 | maxFailedIndexes | **#275** Job controller check (pending deploy) | Pending |
| custom_resource_definition.go:104 | CRD create: context deadline exceeded | CRD protobuf decoder limitation | Hard |
| output.go:263 | EmptyDir file perms 0644 expected 0666 | Docker Desktop virtiofs doesn't support chmod on bind mounts | Platform limitation |
| crd_conversion_webhook.go:318 | Conversion webhook deployment failed | Conversion webhooks not implemented | Hard |
| service.go:1571 | ExternalName DNS resolution fails | CoreDNS doesn't serve CNAME for ExternalName services | Medium |
| crd_publish_openapi.go:366 | CRD create timeout (multiple CRDs same group) | CRD protobuf decoder limitation | Hard |

### Pending deploy (code written, needs rebuild)
| # | Fix | Expected impact |
|---|-----|-----------------|
| 270 | Kubelet readiness: remove duplicate write, re-read pod for fresh RV | ~5-8 timing failures |
| 271 | All pod status writes re-read from storage for fresh resourceVersion | CAS conflicts across all paths |
| 272 | Watch: treat rv=1 like rv=0 — use live cache not compacted etcd | 2+ tests (service, resourcequota) |
| 273 | Pod deletion calls stop_pod_for (preStop lifecycle hooks) | 1 test (lifecycle_hook.go) |
| 274 | Aggregated discovery: dynamically include CRD groups from storage | 1 test |
| 275 | Job controller: maxFailedIndexes check terminates indexed jobs | 1 test |
| 276 | Watch label selector: support 'in', 'notin', '!key' set-based operators | watch.go:409 + related |
| 277 | Downward API: resync standalone volumes on label changes + dryRun fix | 2 tests |

## Progress

| Round | Pass | Fail | Total | Rate | Key changes |
|-------|------|------|-------|------|-------------|
| 97 | ~40 | ~400 | 441 | ~9% | Baseline |
| 98 | | | | 53% | Round 98 fixes |
| 101 | 245 | 196 | 441 | 56% | 76 fixes deployed |
| 103 | 46 | 30 | 76 | 60% | fsGroup, session affinity, IPC sharing |
| 104 | IN PROGRESS | 21 | ~250/441 | ? | #255-269 deployed, #270-277 pending |

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
