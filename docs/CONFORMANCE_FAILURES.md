# Conformance Issue Tracker

**265 total fixes** | Build clean | Round 103: 64 pass, 47 fail at 111/441 (57%)

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

### Timing-dependent (improved by #255, may still fail)
| Test | Issue |
|------|-------|
| statefulset.go:786 | SS scaling — pods created but readiness probe slow |
| deployment.go:769, :520 | Deployment replicas not reaching ready |
| runtime.go:158 | Container not transitioning to Succeeded |
| rc.go:173, :717 | RC watch condition / status timeout |
| pod_client.go:216 | Pod not reaching Succeeded |
| service.go:276 | Service deployment not reaching available |
| endpoints.go:526 | Endpoint fetch rate limited |
| endpointslice.go:798 | EndpointSlice fetch rate limited |
| daemon_set.go:980 | DaemonSet locate timeout |
| replica_set.go:203 | RS pod not becoming ready |
| downwardapi_volume.go:186 | Volume value not updating fast enough |
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
