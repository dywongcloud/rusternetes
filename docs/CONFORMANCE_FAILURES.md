# Conformance Issue Tracker

**271 total fixes** | Build clean | Round 104 IN PROGRESS (fixes #255-270 deployed)

## What Still Needs Fixing

### Actively failing in Round 104
| # | Test | Error | Status |
|---|------|-------|--------|
| 270 | statefulset.go:786 | SS scaling timeout — pods never became Ready | **FIXED #270** — duplicate storage write caused CAS conflict, readiness never persisted |
| 271 | runtime.go:115 | RestartCount=0 expected 2 | **FIXED #271** — all pod.clone() writes now re-read from storage for fresh RV |
| — | aggregated_discovery.go:227 | context deadline exceeded 30s | Watch/discovery channel timing — needs investigation |
| — | service.go:3304 | Failed to locate Service via watch | Watch doesn't deliver initial ADDED events — same root cause as watch.go:409 |
| — | util.go:182 | kubectl exec fails (exit code 7) | Exec/attach not fully implemented |
| — | subPathExpr | var-expansion pod CreateContainerError: absolute path | Needs debug logging deployed to trace expanded values |
| — | resource_quota.go:1152 | ResourceQuota watch timeout after /status update | **FIXED #273** — watch rv=1 caused compacted etcd replay loop |
| 272 | lifecycle_hook.go:132 | preStop hook never executed (httpGet not sent) | **FIXED #272** — pod deletion used stop_pod_with_grace_period instead of stop_pod_for |
| — | service.go:3304 | Service watch timeout | **FIXED #273** — same watch rv=1 compaction issue |
| — | job.go:236 | Job pods not ready in 15min | Readiness persistence (#270/#271 pending deploy) |
| — | statefulset.go:2253 | StatefulSet test failure | Readiness persistence (#270/#271 pending deploy) |
| — | job.go:665 | Job test failure | Readiness/timing (#270/#271 pending deploy) |
| — | downwardapi_volume.go:155 | DownwardAPI volume failure | Needs investigation |
| — | custom_resource_definition.go:104 | CRD creation timeout | CRD protobuf decoder issue |
| — | output.go:263 | emptyDir file perms -rw-r--r-- expected -rw-rw-rw- | Docker Desktop macOS bind mount permission limitation |

### Pending deploy (code written, needs rebuild)
| # | Fix | Expected impact |
|---|-----|-----------------|
| 270 | Kubelet readiness: remove duplicate write, re-read pod for fresh RV | ~15 timing failures |
| 271 | All pod status writes re-read from storage for fresh RV | CAS conflicts across all paths |
| 272 | Pod deletion calls stop_pod_for (preStop hooks) instead of force-kill | 1 test (lifecycle_hook.go:132) |
| 273 | Watch rv=1 uses live cache instead of compacted etcd replay | 2+ tests (resource_quota.go, service.go) |

### Previously pending (now deployed in Round 104 build)
| # | Fix | Status |
|---|-----|--------|
| 255 | Kubelet sync interval 2s → 1s | DEPLOYED |
| 256 | SA token JTI credential-id in extra field | DEPLOYED |
| 257 | ValidatingAdmissionPolicy status route | DEPLOYED |
| 258 | MicroTime omits .000000 for whole-second timestamps | DEPLOYED |
| 259 | ResourceClaim AllocationResult.devices field default | DEPLOYED |
| 260 | SS current_revision derived from pod labels, not template | DEPLOYED |
| 261 | Watch resubscribe delay to prevent tight loop on channel close | DEPLOYED |
| 262 | Namespace controller only removes kubernetes finalizer | DEPLOYED |
| 263 | VAP binding must be 2s old before enforcement | DEPLOYED |
| 264 | Reject pods with non-existent RuntimeClass | DEPLOYED |
| 265 | TaintEvictionController: evict pods not tolerating NoExecute taints | DEPLOYED |
| 266 | **CRITICAL** Kubelet writes readiness/container status to storage | DEPLOYED (but had bug — see #270) |
| 267 | Namespace controller recreates kube-root-ca.crt when deleted | DEPLOYED |
| 268 | CRD protobuf decoder: add subresources.status to versions | DEPLOYED |
| 269 | Namespace finalize endpoint | DEPLOYED |

### Known issues from Round 103 (may be fixed by deployed fixes)
| Test | Error | Expected fix |
|------|-------|-------------|
| statefulset.go:786 | SS scaling timeout | #266 + #270 (readiness persistence) |
| deployment.go:769, :520 | Deployment ReadyReplicas:0 | #266 + #270 |
| runtime.go:158 | Container phase never set to Succeeded | #266 + #270 |
| rc.go:173, :717 | RC watch sees stale pod conditions | #266 + #270 |
| pod_client.go:216 | Pod Succeeded never written | #266 + #270 |
| service.go:276 | Service deployment AvailableReplicas:0 | #266 + #270 |
| endpoints.go:526 | Endpoint controller reads stale pod status | #266 + #270 |
| endpointslice.go:798 | Same | #266 + #270 |
| daemon_set.go:980, :1064 | DaemonSet pod status stale | #266 + #270 |
| replica_set.go:203, :738 | RS ReadyReplicas never updated | #266 + #270 |
| downwardapi_volume.go:186 | Volume resync depends on pod status | #266 + #270 |
| service.go:1485, :870 | Service deployment not ready / timeout | #266 + #270 |
| job.go:548 | Job not completed in 900s | #266 + #270 |
| aggregated_discovery.go:336 | context deadline exceeded | #261 (watch resubscribe) |
| crd_publish_openapi.go:161,:285,:451 | CRD create timeout | #268 (CRD protobuf subresources) |
| field_validation.go:305 | CRD create timeout | #268 |
| namespace.go:426 | Failed to add finalizer: 404 | #269 (finalize endpoint) |
| statefulset.go:957 | Pod expected to be re-created | #260 (SS current_revision) |
| statefulset.go:381 | Current revision = update revision | #260 |
| service_accounts.go:667 | SA token timeout 110s | #256 (JTI credential-id) |
| service_accounts.go:792 | timed out | #267 (kube-root-ca.crt) |
| pods.go:600 | Websocket channel 3 before channel 1 | #244 (deployed) |
| runtimeclass.go:64 | Should get forbidden error | #264 (RuntimeClass check) |
| conformance.go:835 | ResourceClaim devices field | #259 (devices default) |
| builder.go:97 | kubectl create -f - fails | #247 (OpenAPI JSON) |

### Code bugs still needing investigation
| Issue | Error | What to do |
|-------|-------|------------|
| watch.go:409 | Watch restart doesn't deliver initial ADDED events | Needs runtime debugging — label update vs list timing |
| subPathExpr | Annotation env var empty on first kubelet sync | Kubelet gets pod from storage before annotation is persisted? Or CAS issue |
| service.go:4291 | affinity-nodeport not reachable | Session affinity / networking |
| util.go:182 | kubectl exec fails | Networking (exec not implemented?) |
| predicates.go:1102 | context deadline exceeded | Scheduling / timing |
| kubectl/logs.go:212 | Webhook deployment not ready | Webhook deployment timing |
| aggregator.go:359 | Extension apiserver deployment not ready | Extension API server not supported |
| lifecycle_hook.go:132 | Timed out after 30s | Lifecycle hook exec timeout |

## Progress

| Round | Pass | Fail | Total | Rate | Key changes |
|-------|------|------|-------|------|-------------|
| 97 | ~40 | ~400 | 441 | ~9% | Baseline |
| 98 | ~53% | | | 53% | Round 98 fixes |
| 101 | 245 | 196 | 441 | 56% | 76 fixes deployed |
| 102 | ~60% | | | 60% | Webhook URL, CRD protobuf, PDB |
| 103 | 46 | 30 | 76 | 60% | fsGroup, session affinity, IPC sharing |
| 104 | IN PROGRESS | | 441 | ? | Fixes #255-270: readiness persistence, taint eviction, CRD subresources |

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
