# Full Conformance Failure Analysis

**Last updated**: 2026-03-24 (round 79 — 8/441 done, 1 passed, 7 failed, 43 fixes)

## Root causes identified and fixed

1. **Field selector** (`646a407`): Missing fields = false. Unblocked all tests.
2. **Service CIDR routing** (`b4f31c2`): Route for 10.96.0.0/12.
3. **API connectivity** (`b224387`+`862c286`+`f9c9691`): Direct API IP + TLS SANs.
4. **Watch cache** (`73c3514`): One etcd watch per prefix, broadcast to clients.
5. **Watch race** (`d2e306c`): Subscribe before list.
6. **Namespace cascade** (`1270649`): Delete controllers before pods.
7. **Protobuf extraction** (`2571b32`): Extract JSON from K8s protobuf envelope.

## Round 79 failures (7 out of 8 tests)

- **Watch closed** (1): StatefulSet scaling verification watch closes prematurely
- **CRD protobuf** (2): Fixed by protobuf extraction (not yet deployed)
- **Container output** (1): CPU_LIMIT=2 — downward API fix committed, not deployed
- **Webhook deployment** (1): Pod not ready (service routing + TLS)
- **kubectl exec** (1): Service ClusterIP not reachable from pod
- **ReplicaSet timeout** (1): RS not found in time

## All 43 fixes

| Fix | Commit |
|-----|--------|
| Container logs: search exited containers | `2b1008d` |
| EventList: add ListMeta | `97938e4` |
| gRPC probe | `e738c1f` |
| Scale PATCH | `d335dee` |
| Status PATCH routes | `d335dee` |
| events.k8s.io/v1 apiVersion | `f8a75da` |
| CRD openAPIV3Schema field name | `abd2137` |
| ResourceSlice Kind/apiVersion | `9b21a89` |
| PDB status serde defaults | `9b21a89` |
| PV status phase | `710eee1` |
| metadata.namespace in create handlers | `db40409` |
| camelCase: podIP, hostIP, containerID | `bde38ef` |
| VolumeAttributesClass deletecollection | `bde38ef` |
| OpenAPI /v2 protobuf 406 | `bde38ef` |
| Keep stopped containers for logs | `2c8e1fd` |
| Termination message reading | `c804e57` |
| Init container Waiting state | `b54d541` |
| StatefulSet controller-revision-hash | `7f5c9bc` |
| ServiceAccount token key | `9238eb4` |
| Proxy handler keys | `b4b745c` |
| nonResourceURLs camelCase | `98f0eac` |
| Deployment revision increment | `565c216` |
| EndpointSlice orphan cleanup | `6f79efa` |
| Fail pod start on missing volumes | `5e07c6e` |
| ClusterIP pre-allocation | `4113fe9` |
| KUBERNETES_SERVICE_HOST direct IP | `b224387` |
| KUBERNETES_SERVICE_PORT 6443 | `862c286` |
| TLS cert SANs Docker IPs | `f9c9691` |
| ClusterIP re-allocation | `cd6ab64` |
| Field selector missing = false | `646a407` |
| Watch reconnect from revision | `5edb20d` |
| runAsGroup security context | `bbaa43f` |
| RC clear FailedCreate | `702b107` |
| Service CIDR route | `b4f31c2` |
| iproute2 in kube-proxy | `cec24ff` |
| JOB_COMPLETION_INDEX env var | `f288981` |
| /apis/{group} discovery | `3327567` |
| Downward API hostIP + CPU divisor | `095b407` |
| Watch bookmark send resilience | `0eb215d` |
| Watch cache architecture | `73c3514` |
| Watch race: subscribe before list | `d2e306c` |
| Namespace cascade delete order | `1270649` |
| **Protobuf envelope extraction** | `2571b32` |
