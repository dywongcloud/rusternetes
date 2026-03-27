# Conformance Issue Tracker

**246 total fixes** | Build clean, all unit tests pass

### Session summary: 68 fixes (#179-246)

## Current Status

- **Round 102 in progress** (deployed fixes #1-229): 42 pass, 27 fail at 69/441 (**60%**)
- **Pending deploy** (fixes #230-246): 17 fixes
- **Progress**: ~40 passes (round 97) → 245/441 (round 101, 56%) → 60% (round 102, 69 done)

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
| 236 | Sysctls: apply pod sysctls to application containers (not just pause) | 1 test |
| 237 | Webhook matchConditions: type-check CEL with admission context variables | 1 test |
| 238 | PodSecurity admission: reject privileged/hostNamespace pods in baseline/restricted namespaces | 1 test |
| 239 | Strict field validation: detect duplicate JSON keys | 1 test |
| 240 | Reduce controller sync interval from 2s to 1s for faster status updates | timing tests |
| 241 | TokenRequest audiences field accepts null (deserialize_null_default) | 1 test |
| 242 | Share IPC namespace with pause container (fixes sysctl tests) | 1 test |
| 243 | CRD protobuf decoder: extract version names from versions array | 1+ tests |
| 244 | Websocket exec: send empty stdout before status on channel 3 | 1 test |
| 245 | Session affinity: per-endpoint chains with recent module for proper DNAT+mark | 3 tests |
| 246 | HTTP probe: accept 200-399 as success (was 2xx only) | timing tests |

## Remaining Unfixed Issues

### Environment limitations (cannot fix in code)
| Issue | Details |
|-------|---------|
| File permissions (emptyDir/secret) | Docker umask 0022 strips group/other write bits. Tests expect 0666/0777 but get 0644/0755. May need fsGroup implementation. |
| Service session affinity | **FIXED #245** — restructured to use per-endpoint chains with recent module |

### Needs architectural work
| Issue | Details |
|-------|---------|
| Watch label selector re-evaluation | **FIXED #233** — don't filter MODIFIED by label selector so clients see label changes |
| kubectl stdin validation | kubectl `--validate` uses OpenAPI schema. Our schema may be incomplete for some resource types. |
| PodSecurity admission | **FIXED #238** — basic PSA: reject privileged/hostNS pods in baseline/restricted namespaces |
| StatefulSet rolling update | **FIXED #235** — deletes old-revision pods one at a time for rolling update |
| Strict field validation | **FIXED #239** — detect duplicate JSON keys at top level |

### Controller timing / watch delivery
| Issue | Details |
|-------|---------|
| StatefulSet scaling timeout | Improved by #240 (1s interval), #246 (probe 200-399). May need dedicated probe timer. |
| ReplicaSet locate/scale | Improved by #240 (1s interval). RS available count should be faster. |
| PDB processing timeout | Improved by #222 (observedGeneration), #240 (1s interval). |
| Webhook not ready | Improved by #220 (URL resolution), #240 (1s interval). Still 2-3 failures. |

### Needs investigation
| Issue | Details |
|-------|---------|
| runtime.go:169 | **FIXED #230** — don't fall through to docker cp when host file exists |
| service_accounts.go:898 | **FIXED #241** — TokenRequest audiences null handling |
| resource_quota.go:142 | **FIXED #217** — quota admission for services |
| Job completion timeout | **FIXED #215** — pod adoption |
| Pod resize status | **FIXED #234** — re-read fresh spec in Running sync |
| Network connectivity | **FIXED #216,#220** — proxy root path + webhook URL resolution |
| Websocket exec channel | **FIXED #244** — send empty stdout before status |
| Strict decoding | **FIXED #239** — duplicate JSON key detection |
| RS availableReplicas | Improved by #240 (1s interval). Timing-dependent. |
| Deployment revision | Timing — revision annotation set during reconcile |
| Namespace PUT 404 | Timing — namespace not created yet when test adds finalizer |
| Pod count 72/100 | Scheduling capacity — need more node capacity or fewer concurrent tests |

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
