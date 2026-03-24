# Full Conformance Failure Analysis

**Last updated**: 2026-03-24 (round 74 — 22/441 done, 2 passed, 20 failed)

## Root causes identified and fixed

1. **Field selector** (`646a407`): `spec.unschedulable=false` returned 0 nodes because
   missing JSON fields weren't treated as `false`. This blocked ALL tests.

2. **Service CIDR routing** (`b4f31c2`): No route for 10.96.0.0/12, so pods couldn't
   reach any ClusterIP service. Kube-proxy now adds the route.

3. **API connectivity** (`b224387`+`862c286`+`f9c9691`): Pods used ClusterIP 10.96.0.1
   which wasn't routable from Docker bridge. Fixed with direct API server IP + TLS SANs.

## All 37 fixes this session

| Fix | Commit |
|-----|--------|
| Container logs: search exited containers | `2b1008d` |
| EventList: add ListMeta metadata | `97938e4` |
| gRPC probe: implement health check | `e738c1f` |
| Scale PATCH: accept partial JSON | `d335dee` |
| Status PATCH routes | `d335dee` |
| events.k8s.io/v1: correct apiVersion | `f8a75da` |
| CRD openAPIV3Schema field name | `abd2137` |
| ResourceSlice: set Kind/apiVersion | `9b21a89` |
| PDB status: serde defaults | `9b21a89` |
| PV create: init status with phase | `710eee1` |
| metadata.namespace in create handlers | `db40409` |
| camelCase: podIP, hostIP, containerID, etc | `bde38ef` |
| VolumeAttributesClass deletecollection | `bde38ef` |
| OpenAPI /v2: 406 for protobuf | `bde38ef` |
| Keep stopped containers for logs | `2c8e1fd` |
| Termination message reading | `c804e57` |
| Init container: Waiting for unstarted | `b54d541` |
| StatefulSet: controller-revision-hash | `7f5c9bc` |
| ServiceAccount token: correct key | `9238eb4` |
| Proxy handlers: correct keys | `b4b745c` |
| nonResourceURLs camelCase | `98f0eac` |
| Deployment revision increment | `565c216` |
| EndpointSlice orphan cleanup | `6f79efa` |
| Fail pod start on missing volumes | `5e07c6e` |
| ClusterIP pre-allocation at startup | `4113fe9` |
| KUBERNETES_SERVICE_HOST direct IP | `b224387` |
| KUBERNETES_SERVICE_PORT 6443 | `862c286` |
| TLS cert SANs: Docker bridge IPs | `f9c9691` |
| ClusterIP re-allocation | `cd6ab64` |
| **Field selector: missing = false** | `646a407` |
| Watch reconnect from revision | `5edb20d` |
| runAsGroup security context | `bbaa43f` |
| RC: clear FailedCreate condition | `702b107` |
| Service CIDR route for ClusterIPs | `b4f31c2` |
| iproute2 in kube-proxy container | `cec24ff` |
| JOB_COMPLETION_INDEX env var | `f288981` |
| /apis/{group} discovery endpoint | `3327567` |
| Downward API: hostIP + CPU divisor | `095b407` |

## Current failures (round 74, not yet rebuilt with latest fixes)

### Watch (3) — fix committed, not deployed
### Container output (3) — hostIP/CPU_LIMIT fixes committed
### Webhook deployment (1) — needs service routing
### Controller (2) — RC condition + ResourceQuota
### Networking (2) — service routing fix committed
### Auth (1) — SelfSubjectAccessReview edge case
### Timeouts (8) — various tests timing out
