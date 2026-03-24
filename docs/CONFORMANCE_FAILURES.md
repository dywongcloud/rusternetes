# Full Conformance Failure Analysis

**Last updated**: 2026-03-24 (48 fixes, rebuilding for round 83)

## Critical root causes fixed

| # | Root Cause | Impact | Fix |
|---|-----------|--------|-----|
| 1 | **Container restart storm** | Containers recreated every 2s, flooding watches | `e99730b` |
| 2 | Field selector: missing ≠ false | All tests blocked | `646a407` |
| 3 | Protobuf: client sends binary | CRD/EndpointSlice creation fails | `2571b32` |
| 4 | Service CIDR: no route | Pod-to-service fails | `b4f31c2` |
| 5 | API connectivity: ClusterIP | Pods can't reach API | `b224387`+ |
| 6 | Watch cache: N×N watches | etcd overwhelmed | `73c3514` |
| 7 | Controller interval: 10s | Tests time out | `2ea4199` |
| 8 | Namespace cascade: wrong order | Orphaned pods | `1270649` |
| 9 | Status PATCH: metadata lost | Annotations not merged | `38b44f2` |

## All 48 fixes

| Fix | Commit |
|-----|--------|
| Container logs search | `2b1008d` |
| EventList ListMeta | `97938e4` |
| gRPC probe | `e738c1f` |
| Scale PATCH | `d335dee` |
| Status PATCH routes | `d335dee` |
| events.k8s.io/v1 | `f8a75da` |
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
| StatefulSet revision | `7f5c9bc` |
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
| Namespace cascade | `1270649` |
| Protobuf extraction | `2571b32` |
| RS conditions preserve | `58317e6` |
| Controller interval 2s | `ea1b800` |
| Status PATCH metadata | `38b44f2` |
| Container restart fix | `e99730b` |
| **Watch event: Added vs Modified** | **`00db87c`** |
