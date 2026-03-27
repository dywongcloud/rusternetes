# Conformance Issue Tracker

**235 total fixes** | Build clean, all unit tests pass

## Current Status

- **Round 101 COMPLETE** (deployed fixes #1-216): **245 pass, 196 fail (55.6%)**
- **Pending deploy** (fixes #217-227): 11 fixes targeting ~35+ additional passes → projected **~280/441 (63%)**
- **Progress**: ~40 passes (round 97) → 245 passes (round 101) — **6x improvement**

## Deployed Fixes (in current running build)

Fixes #1-216 are deployed and active in round 101.

### Critical infrastructure fixes
| # | Fix | Impact |
|---|-----|--------|
| 170 | resourceVersion in watch events from etcd mod_revision | ALL tests |
| 174 | List RV from items, not timestamps | ALL tests |
| 191 | Bookmark resourceVersion: use current etcd revision, not "0" | many tests (eliminated test hangs) |
| 200 | List resourceVersion: always set (fallback "1"), never empty | many tests |
| 197 | ?watch=true support on 21 list handlers across 12 files | many tests |
| 188 | 23 missing watch handlers + routes | many tests |

### API server fixes
| # | Fix |
|---|-----|
| 199 | Decode K8s native protobuf to JSON for CRD creation |
| 214 | VAP variables: CEL Map variable (eliminated VAP test stalling) |
| 198 | VAP: evaluate spec.variables before validations |
| 206 | API group discovery endpoint /apis/{group}/ |
| 208 | Aggregated discovery: autoscaling v1+v2 |
| 213 | OpenAPI v3 root document + per-group-version endpoints |
| 207 | ServiceAccount username doubled fix |
| 187 | CRD status: Established + NamesAccepted conditions |
| 179 | CEL matchConditions validation in webhooks |
| 175 | Immutable returns 403 Forbidden |
| 177 | Aggregated discovery responseKind.group empty |
| 202 | PVC status subresource route |
| 184 | IPAddress status route + ServiceCIDR Ready condition |
| 210 | ResourceClaim status PATCH uses generic handler |
| 212 | EndpointSlice ports accepts null |
| 205 | Service update handler allocates NodePorts |
| 216 | Service proxy root path route |
| 182 | Proxy double-slash path fix |
| 209 | Namespace kubernetes finalizer on create |

### Controller fixes
| # | Fix |
|---|-----|
| 196 | StatefulSet: availableReplicas = readyReplicas |
| 203 | StatefulSet: currentRevision vs updateRevision in rolling updates |
| 204 | Job podFailurePolicy FailJob action |
| 215 | Job controller adopts orphaned pods |
| 192 | ResourceQuota: count replicationcontrollers + resourcequotas |
| 201 | RC controller: count ready replicas properly |
| 211 | DaemonSet pods get controller-revision-hash label |

### Kubelet/runtime fixes
| # | Fix |
|---|-----|
| 189 | Env var $(VAR) expansion in container env values |
| 186 | CPU/memory downward API: ceiling division |
| 183 | Termination message bind-mount + host-file read |
| 194 | Lifecycle hook exec: 30s timeout (was infinite) |
| 195 | Projected volume resync: items, downwardAPI, stale file deletion |
| 193 | RuntimeClass list ?watch=true |

## Pending Deploy (not in current build)

| # | Fix | Impact |
|---|-----|--------|
| 217 | Generic count-based ResourceQuota admission for service create | 1 test |
| 218 | PersistentVolume status subresource route | 1 test |
| 219 | CRD protobuf decoder: fix names field numbers (kind=4, listKind=5, not 3,4) | 5+ tests |
| 220 | Webhook URL resolution: resolve .svc names to endpoint IPs from storage | 13+ tests |
| 221 | Pod create sets QoS class (BestEffort/Burstable/Guaranteed) | 1+ tests |
| 222 | PDB status: set observedGeneration (was None, test waits for it) | 5 tests |
| 223 | SA token: add system:authenticated group | 1 test |
| 224 | Immutable ConfigMap/Secret returns 422 Invalid (not 403 Forbidden) | 2 tests |
| 225 | FlowSchema deletecollection handler + route | 1 test |
| 226 | EndpointSlice deletecollection route (handler existed but not wired) | 1 test |
| 227 | ResourceClaimTemplate deletecollection route (handler existed but not wired) | 1 test |
| 228 | Event k8s_time serializer: Time format without microseconds (was MicroTime) | 1 test |
| 229 | Docker container hostname set to pod hostname (not container ID) | 1 test |
| 230 | Termination message: don't fall through to docker cp when host file exists but is empty | 1 test |
| 231 | Wire deletecollection for HPA v1/v2, RC, ControllerRevision, ResourceClaim | 5 tests |
| 232 | Wire deletecollection for VolumeSnapshot, CSIStorageCapacity | 2 tests |
| 233 | Watch: don't filter MODIFIED events by label selector (fixes label change watches) | 4 tests |
| 234 | Pod resize: re-read pod from storage in Running sync to get updated spec resources | 4 tests |
| 235 | StatefulSet rolling update: delete old-revision pods one at a time | 2 tests |

## Remaining Unfixed Issues

### Environment limitations (cannot fix in code)
| Issue | Details |
|-------|---------|
| File permissions (emptyDir/secret) | Docker umask 0022 strips group/other write bits. Tests expect 0666/0777 but get 0644/0755. |
| Service session affinity | iptables `recent` module not available in Docker Desktop LinuxKit VM |

### Needs architectural work
| Issue | Details |
|-------|---------|
| Watch label selector re-evaluation | **FIXED #233** — don't filter MODIFIED by label selector so clients see label changes |
| kubectl stdin validation | kubectl `--validate` uses OpenAPI schema. Our schema may be incomplete for some resource types. |
| PodSecurity admission | No pod security admission controller — pods that violate namespace policy aren't rejected |
| StatefulSet rolling update | **FIXED #235** — deletes old-revision pods one at a time for rolling update |
| Strict field validation | Server doesn't detect duplicate JSON fields when fieldValidation=Strict |

### Controller timing / watch delivery
| Issue | Details |
|-------|---------|
| StatefulSet scaling timeout | statefulset.go:786 — pods created but readiness probe transition slow |
| ReplicaSet locate/scale | replica_set.go:738,:560,:232 — RS watch event delivery timing |
| PDB processing timeout | client rate limiter exceeded — too many API calls |
| Webhook not ready | webhook.go:425,:1269 — webhook service endpoint timing |

### Needs investigation
| Issue | Details |
|-------|---------|
| runtime.go:169 | Termination message: expected empty but got "DONE" — **FIXED #230** don't fall through to docker cp when host file exists |
| service_accounts.go:898 | SA token test failure — needs debugging |
| resource_quota.go:142 | Quota enforcement for service creation (fix #217 pending) |
| Job completion timeout | job.go:958 — pod adoption timing |
| Pod resize status | Cgroup values not updating after resize |
| Network connectivity | proxy.go:271, network/util.go:182 — service proxy / endpoint resolution |
| Websocket exec channel | Got status (ch3) before stdout (ch1) — protocol ordering |
| Strict decoding | duplicate field detection for fieldValidation=Strict |
| RS availableReplicas | RS controller timing — pods not counted as available fast enough |
| Deployment revision | Rolling update revision annotation timing |
| Namespace PUT 404 | Timing — namespace not created yet when test adds finalizer |
| Pod count 72/100 | Scheduling capacity with 2 nodes, test pods from other tests consuming slots |

## All Fixes by Session

<details>
<summary>Fixes #169-178 (pre-session)</summary>

| # | Fix |
|---|-----|
| 169 | generation=1, ClusterIP, SA token, PodScheduled |
| 170 | resourceVersion in watch events |
| 171 | Endpoints single subset |
| 172 | Ensure metadata for resourceVersion |
| 173 | Remove duplicate SA token route (panic) |
| 174 | List RV from items, not timestamps |
| 175 | Immutable returns 403 Forbidden |
| 176 | RC orphan handling + DaemonSet ControllerRevision |
| 177 | Aggregated discovery responseKind.group empty |
| 178 | In-place pod resize via Docker update_container |
</details>

<details>
<summary>Fixes #179-218 (this session — 40 fixes)</summary>

| # | Fix |
|---|-----|
| 179 | CEL matchConditions validation in webhook create handlers |
| 180-181 | RuntimeClass + ResourceQuota watch handlers + routes |
| 182 | Proxy double-slash path fix |
| 183 | Termination message bind-mount + host-file read |
| 184-185 | IPAddress/ServiceCIDR status + watch |
| 186 | CPU/memory downward API ceiling division |
| 187 | CRD status Established + NamesAccepted |
| 188 | 23 missing watch handlers + routes |
| 189 | Env var $(VAR) expansion |
| 190 | Protobuf 415 (superseded by #199) |
| 191 | **CRITICAL** bookmark RV: 0 → current etcd revision |
| 192 | ResourceQuota counts RCs + RQs |
| 193 | RuntimeClass list ?watch=true |
| 194 | Lifecycle hook exec 30s timeout |
| 195 | Projected volume resync |
| 196 | StatefulSet availableReplicas |
| 197 | ?watch=true on 21 list handlers |
| 198 | VAP variable evaluation |
| 199 | CRD protobuf decoder |
| 200 | List RV fallback "1" |
| 201 | RC ready replica count |
| 202 | PVC status route |
| 203 | StatefulSet revision tracking |
| 204 | Job podFailurePolicy |
| 205 | Service NodePort on update |
| 206 | API group discovery /apis/{group}/ |
| 207 | SA username doubled fix |
| 208 | Autoscaling v1+v2 discovery |
| 209 | Namespace kubernetes finalizer |
| 210 | ResourceClaim status PATCH |
| 211 | DaemonSet controller-revision-hash label |
| 212 | EndpointSlice ports null |
| 213 | OpenAPI v3 per-group endpoints |
| 214 | **CRITICAL** VAP CEL Map variables |
| 215 | Job pod adoption |
| 216 | Service proxy root path |
| 217 | ResourceQuota admission for services |
| 218 | PV status route |
</details>
