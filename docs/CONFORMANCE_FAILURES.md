# Full Conformance Failure Analysis

**Last updated**: 2026-03-24 (round 79 in progress — 42 fixes deployed)

## Root causes identified and fixed

1. **Field selector** (`646a407`): `spec.unschedulable=false` returned 0 nodes.
   Missing JSON fields weren't treated as `false`. Blocked ALL tests from running.

2. **Service CIDR routing** (`b4f31c2`): No route for 10.96.0.0/12.
   Pods couldn't reach any ClusterIP service.

3. **API connectivity** (`b224387`+`862c286`+`f9c9691`): ClusterIP 10.96.0.1
   not routable from Docker bridge. Fixed with direct API server IP + TLS SANs.

4. **Watch architecture** (`73c3514`): Each client created a separate etcd watch.
   Implemented watch cache: one etcd watch per prefix, broadcast to all clients.

5. **Watch race condition** (`d2e306c`): Listed resources before subscribing to
   events, missing events created between list and subscribe.

6. **Namespace cascade delete** (`1270649`): Deleted pods before controllers,
   causing controllers to recreate pods during cleanup. Tests stalled waiting
   for orphaned pod cleanup.

## All 42 fixes this session

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
| ClusterIP re-allocation for existing services | `cd6ab64` |
| Field selector: missing = false | `646a407` |
| Watch reconnect from revision | `5edb20d` |
| runAsGroup security context | `bbaa43f` |
| RC: clear FailedCreate condition | `702b107` |
| Service CIDR route for ClusterIPs | `b4f31c2` |
| iproute2 in kube-proxy container | `cec24ff` |
| JOB_COMPLETION_INDEX env var | `f288981` |
| /apis/{group} discovery endpoint | `3327567` |
| Downward API: hostIP default + CPU divisor | `095b407` |
| Don't break watch on bookmark send failure | `0eb215d` |
| Watch cache: one etcd watch per prefix | `73c3514` |
| Watch race fix: subscribe before list | `d2e306c` |
| Namespace cascade: delete controllers before pods | `1270649` |
