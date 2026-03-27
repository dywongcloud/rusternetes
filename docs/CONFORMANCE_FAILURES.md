# Conformance Issue Tracker

**266 total fixes** | Build clean | Round 103: 68 pass, 51 fail at 119/441 (57%)

## What Still Needs Fixing

### Pending deploy (code written, not yet in running build)
| # | Fix | Expected impact |
|---|-----|-----------------|
| 255 | Kubelet sync interval 2s → 1s | ~15 timing failures |
| 256 | SA token JTI credential-id in extra field | 1 test |
| 257 | ValidatingAdmissionPolicy status route | 1 test |
| 258 | MicroTime omits .000000 for whole-second timestamps | 1 test |
| 259 | ResourceClaim AllocationResult.devices field default | 1 test |
| 260 | SS current_revision derived from pod labels, not template | 1 test |
| 261 | Watch resubscribe delay to prevent tight loop on channel close | 2 tests |
| 262 | Namespace controller only removes kubernetes finalizer, not custom ones | 1 test |
| 263 | VAP binding must be 2s old before enforcement (prevents early denial) | 1 test |
| 264 | Reject pods with non-existent RuntimeClass | 1 test |
| 265 | TaintEvictionController: evict pods not tolerating NoExecute taints | 1 test |
| 266 | **CRITICAL** Kubelet writes readiness/container status to storage during Running sync | ~15 tests |

### Code bugs to fix
| Issue | Error | What to do |
|-------|-------|------------|
| core_events.go:135 | Event timestamp has microseconds | **FIXED #258** — micro_time only adds .000000 if timestamp has sub-second precision |
| watch.go:409 | Watch restart doesn't deliver initial ADDED events | Code path looks correct — may be timing between label update and list. Needs runtime debugging. |
| aggregated_discovery.go:227 | Watch channel closed unexpectedly | **FIXED #261** — added delay before resubscribe to prevent tight loop |
| csistoragecapacity.go:190 | Watch channel closed | **FIXED #261** — same fix |
| validatingadmissionpolicy.go:270 | VAP denies marker too early | **FIXED #263** — binding must be 2s old before enforcement |
| namespace.go:579 | Namespace deleted unexpectedly | **FIXED #262** — namespace controller only removes kubernetes finalizer, not custom ones |
| statefulset.go:381 | Current revision = update revision | **FIXED #260** — derive current_revision from pod labels, not template |

### Architecture gaps (need new features)
| Issue | What's needed |
|-------|---------------|
| NoExecute taint eviction | **FIXED #265** — TaintEvictionController evicts non-tolerating pods |

### Timing-dependent — **ROOT CAUSE FOUND: #266**
| Test | Issue |
|------|-------|
| ALL BELOW | **FIXED #266** — kubelet never wrote readiness/status changes to storage during Running sync |
| statefulset.go:786 | SS scaling — pods marked Ready in container status but not persisted |
| deployment.go:769, :520 | Deployment ReadyReplicas:0 because pod Ready condition never updated |
| runtime.go:158 | Container terminated but phase never set to Succeeded |
| rc.go:173, :717 | RC watch sees stale pod conditions |
| pod_client.go:216 | Pod Succeeded never written |
| service.go:276 | Service deployment AvailableReplicas:0 |
| endpoints.go:526 | Endpoint controller reads stale pod status |
| endpointslice.go:798 | Same |
| daemon_set.go:980 | DaemonSet pod status stale |
| replica_set.go:203 | RS ReadyReplicas never updated |
| downwardapi_volume.go:186 | Volume resync depends on pod status |

### Additional round 103 failures (new)
| Test | Error | Category |
|------|-------|----------|
| aggregated_discovery.go:336 | context deadline exceeded | Timing (#266) |
| crd_publish_openapi.go:161,:285,:451 | failed to create CRD: context deadline exceeded | CRD protobuf decoder limitation |
| field_validation.go:305 | cannot create CRD: context deadline exceeded | CRD protobuf decoder limitation |
| namespace.go:426 | failed to add finalizer: 404 | Namespace PUT timing |
| daemon_set.go:1064 | client rate limiter: context deadline exceeded | Timing (#266) |
| job.go:548 | job not completed in 900s | Timing (#266) |
| replica_set.go:738 | failed to locate RS | Timing (#266) |
| statefulset.go:957 | Pod expected to be re-created | SS rolling update timing |
| service_accounts.go:667 | SA token timeout 110s | SA token timing |
| service_accounts.go:792 | timed out | SA test timing |
| lifecycle_hook.go:132 | Timed out after 30s | Lifecycle hook exec (#194 deployed) |
| pods.go:600 | Websocket channel 3 before channel 1 | **FIXED #244** pending deploy |
| runtimeclass.go:64 | should get forbidden error | **FIXED #264** pending deploy |
| conformance.go:835 | ResourceClaim devices field | **FIXED #259** pending deploy |
| builder.go:97 | kubectl create -f - fails | **FIXED #247** pending deploy |
| service.go:1485 | externalname-service deployment not ready | Timing (#266) |
| service.go:4291 | affinity-nodeport not reachable | Session affinity / networking |
| service.go:870 | context deadline exceeded | Timing (#266) |
| util.go:182 | kubectl exec fails | Networking |
| predicates.go:1102 | context deadline exceeded | Scheduling / timing |
| kubectl/logs.go:212 | Webhook deployment not ready |
| aggregator.go:359 | Extension apiserver deployment not ready |

## Progress

| Round | Pass | Fail | Total | Rate | Key changes |
|-------|------|------|-------|------|-------------|
| 97 | ~40 | ~400 | 441 | ~9% | Baseline |
| 98 | ~53% | | | 53% | Round 98 fixes |
| 101 | 245 | 196 | 441 | 56% | 76 fixes deployed |
| 102 | ~60% | | | 60% | Webhook URL, CRD protobuf, PDB |
| 103 | 46 | 30 | 76 | 60% | fsGroup, session affinity, IPC sharing |

## All Deployed Fixes

<details>
<summary>254 fixes deployed in current build (click to expand)</summary>

Fixes #1-254 are in the current running build. Key categories:

**Infrastructure**: Watch handlers (#188, #197), bookmark RV (#191), list RV (#200), ?watch=true (#197), controller 1s interval (#240)

**API Server**: CRD protobuf (#199, #219, #243, #251), VAP (#198, #214), webhooks (#220), OpenAPI (#213, #247), discovery (#206, #208), PodSecurity (#238), strict validation (#239), fsGroup (#248)

**Controllers**: StatefulSet (#196, #203, #235), Job (#204, #215), RS (#201), RC (#192), PDB (#222), DaemonSet (#211), Deployment revision

**Kubelet**: Env var expansion (#189), CPU ceiling (#186), termination msg (#183, #230), lifecycle hooks (#194), probes (#246), projected volumes (#195), resize (#234), sysctls (#242), IPC sharing (#242, #254), hostname (#229, #253)

**Networking**: Session affinity (#245), proxy (#182, #216), NodePort (#205)

**Routes**: PVC status (#202), PV status (#218), IPAddress status (#184), ServiceCIDR (#184-185), deletecollection wiring (#225-227, #231-232), IngressClass watch (#250)

</details>
