# Full Conformance Failure Analysis

**Last updated**: 2026-03-24 (round 74 in progress — tests running, 6 done, 0 passed)

## How to run conformance tests
```bash
docker compose build && docker compose up -d
bash scripts/cleanup-sonobuoy.sh
bash scripts/run-conformance.sh
bash scripts/conformance-progress.sh    # monitor progress
```

## Root cause found: field selector broke all tests

The e2e suite's `SynchronizedBeforeSuite` lists nodes with field selector
`spec.unschedulable=false`. Our field selector treated missing JSON fields
as non-matching, returning 0 nodes. Every test was skipped. Fixed in `646a407`.

## All fixes this session (29 code fixes)

| Fix | Commit |
|-----|--------|
| Container logs: search exited containers | `2b1008d` |
| EventList: add ListMeta metadata | `97938e4` |
| gRPC probe: implement health check | `e738c1f` |
| Scale PATCH: accept partial JSON | `d335dee` |
| VolumeAttachment + ResourceQuota status PATCH routes | `d335dee` |
| Pagination tests: fix ContinuationToken fields | `c93a3be` |
| events.k8s.io/v1: correct apiVersion | `f8a75da` |
| CRD openAPIV3Schema field name | `abd2137` |
| ResourceSlice: set Kind/apiVersion | `9b21a89` |
| PDB status: serde defaults | `9b21a89` |
| PV create: init status with phase | `710eee1` |
| metadata.namespace in create handlers | `db40409` |
| camelCase: podIP, hostIP, containerID, etc | `bde38ef` |
| VolumeAttributesClass deletecollection route | `bde38ef` |
| OpenAPI /v2: 406 for protobuf | `bde38ef` |
| Keep stopped containers for logs | `2c8e1fd` |
| Termination message reading | `c804e57` |
| Init container: Waiting for unstarted | `b54d541` |
| StatefulSet: controller-revision-hash label | `7f5c9bc` |
| ServiceAccount token: correct storage key | `9238eb4` |
| Proxy handlers: correct storage keys | `b4b745c` |
| nonResourceURLs camelCase | `98f0eac` |
| Deployment revision increment | `565c216` |
| EndpointSlice orphan cleanup | `6f79efa` |
| Fail pod start on missing volumes | `5e07c6e` |
| ClusterIP pre-allocation at startup | `4113fe9` |
| KUBERNETES_SERVICE_HOST direct IP + TLS SANs | `b224387`+`862c286`+`f9c9691` |
| ClusterIP re-allocation for existing services | `cd6ab64` |
| **Field selector: missing fields = false** | **`646a407`** |

## Round 74 failures (in progress)

### Watch issues (2 failures)
- Watch closed before UntilWithoutRetry timeout
- Timed out waiting for watch notification (ConfigMap MODIFIED)
- Root cause: watch stream reconnection loses events during the gap

### Container output (1 failure)
- expected "0" in container output — got empty/wrong output
- Likely: container exits before Docker captures stdout

### Webhook deployment (1 failure)
- sample-webhook-deployment never becomes ready (0 available)
- Pod crashes or can't serve HTTPS

### Controller issues (1 failure)
- RC FailedCreate condition never cleared after pod creation succeeds
- "unable to create pods: only 0 of 2 desired replicas are available"

### kubectl/networking (1 failure)
- kubectl exec inside pod fails — curl to service times out
