# Full Conformance Failure Analysis

**Last updated**: 2026-03-24 (round 82 building/deploying — 47 fixes)

## Key root causes fixed

| # | Root Cause | Fix | Commit |
|---|-----------|-----|--------|
| 1 | Field selector: missing fields ≠ false | Treat missing as false | `646a407` |
| 2 | Service CIDR: no route for 10.96.0.0/12 | Add route in kube-proxy | `b4f31c2` |
| 3 | API connectivity: ClusterIP not routable | Direct IP + TLS SANs | `b224387`+ |
| 4 | Watch architecture: N×N etcd watches | Watch cache (1 per prefix) | `73c3514` |
| 5 | Protobuf: client sends binary, we reject | Extract JSON from envelope | `2571b32` |
| 6 | Controller interval: 10s too slow | Reduced to 2s | `2ea4199` |
| 7 | Namespace cascade: pods deleted before controllers | Reorder deletion | `1270649` |
| 8 | Watch race: list before subscribe | Subscribe first | `d2e306c` |

## Round 79-81 failures analyzed

| Failure | Count | Root Cause | Fix Status |
|---------|-------|-----------|------------|
| Watch closed (StatefulSet) | 1 | 10s controller interval | **Fixed** (2s) |
| CRD/EndpointSlice protobuf | 3-4 | Binary protobuf body | **Fixed** (extraction) |
| CPU_LIMIT container output | 2 | Divisor returned millicores | **Fixed** |
| Webhook deployment | 1 | TLS + service routing | Partially fixed |
| kubectl exec service | 1 | ClusterIP routing | **Fixed** |
| RS conditions wiped | 1 | Controller overwrites | **Fixed** |
| Status PATCH annotations | 1 | Metadata not merged | **Fixed** |
| Pod resize 404 | 2 | Timing issue | Improved (2s interval) |
| RC rate limiter | 1 | API latency | Improved (2s interval) |
| Lifecycle hook timeout | 1 | Hook execution | Needs investigation |
| Session affinity | 1 | Kube-proxy feature | Not implemented |

## All 47 fixes

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
| Controller interval 2s | `2ea4199`+`ea1b800` |
| Status PATCH metadata merge | `38b44f2` |
