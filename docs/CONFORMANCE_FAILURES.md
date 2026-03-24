# Full Conformance Failure Analysis

**Last updated**: 2026-03-24 (rebuilding with 45 fixes for round 81)

## Key root causes fixed

1. **Field selector** (`646a407`): Missing fields = false
2. **Service CIDR routing** (`b4f31c2`): Route for 10.96.0.0/12
3. **API connectivity** (`b224387`+`f9c9691`): Direct IP + TLS SANs
4. **Watch cache** (`73c3514`): One etcd watch per prefix
5. **Protobuf extraction** (`2571b32`): JSON from K8s protobuf envelope
6. **Controller interval** (`2ea4199`): Reduced from 10s to 1s
7. **Namespace cascade** (`1270649`): Delete controllers before pods

## Round 79 failure analysis (18 failures, 1 pass)

| # | Failure | Fix | Status |
|---|---------|-----|--------|
| 1 | Watch closed (StatefulSet) | Controller 10s→1s | Fixed, not deployed |
| 2 | CRD protobuf (×2) | Protobuf extraction | Fixed, not deployed |
| 3 | EndpointSlice protobuf | Protobuf extraction | Fixed, not deployed |
| 4 | CPU_LIMIT=2 env var | CPU divisor fix | Fixed, not deployed |
| 5 | cpu_limit file = 1 | CPU divisor fix | Fixed, not deployed |
| 6 | Webhook deployment | Volume + service routing | Partially fixed |
| 7 | kubectl exec curl | Service CIDR route | Deployed |
| 8 | RS conditions wiped | Preserve conditions | Fixed, not deployed |
| 9 | Pod resize 404 (×2) | Timing — 1s interval helps | Fixed, not deployed |
| 10 | RC rate limiter | 1s interval helps | Fixed, not deployed |
| 11 | Lifecycle hook 30s | Implementation exists | Needs investigation |
| 12 | Session affinity | Kube-proxy feature | Not yet implemented |
| 13 | CRD deadline | Protobuf extraction | Fixed, not deployed |

**Expected improvement**: 10 of 13 failure types have committed fixes.
With 1s controller interval, tests should complete ~10x faster.

## All 45 fixes

| Fix | Commit |
|-----|--------|
| Container logs: search exited | `2b1008d` |
| EventList ListMeta | `97938e4` |
| gRPC probe | `e738c1f` |
| Scale PATCH | `d335dee` |
| Status PATCH routes | `d335dee` |
| events.k8s.io/v1 apiVersion | `f8a75da` |
| CRD openAPIV3Schema | `abd2137` |
| ResourceSlice Kind | `9b21a89` |
| PDB status defaults | `9b21a89` |
| PV status phase | `710eee1` |
| metadata.namespace | `db40409` |
| camelCase abbreviations | `bde38ef` |
| VolumeAttributesClass delete | `bde38ef` |
| OpenAPI protobuf 406 | `bde38ef` |
| Container retention | `2c8e1fd` |
| Termination message | `c804e57` |
| Init container Waiting | `b54d541` |
| StatefulSet revision hash | `7f5c9bc` |
| SA token key | `9238eb4` |
| Proxy handler keys | `b4b745c` |
| nonResourceURLs | `98f0eac` |
| Deployment revision | `565c216` |
| EndpointSlice cleanup | `6f79efa` |
| Fail on missing volumes | `5e07c6e` |
| ClusterIP pre-allocation | `4113fe9` |
| K8S_SERVICE_HOST | `b224387` |
| K8S_SERVICE_PORT | `862c286` |
| TLS SANs | `f9c9691` |
| ClusterIP re-allocation | `cd6ab64` |
| Field selector | `646a407` |
| Watch reconnect | `5edb20d` |
| runAsGroup | `bbaa43f` |
| RC FailedCreate | `702b107` |
| Service CIDR route | `b4f31c2` |
| iproute2 | `cec24ff` |
| JOB_COMPLETION_INDEX | `f288981` |
| /apis/{group} | `3327567` |
| Downward API | `095b407` |
| Watch bookmark resilience | `0eb215d` |
| Watch cache | `73c3514` |
| Watch subscribe-before-list | `d2e306c` |
| Namespace cascade order | `1270649` |
| Protobuf extraction | `2571b32` |
| RS conditions preserve | `58317e6` |
| **Controller interval 1s** | `2ea4199` |
