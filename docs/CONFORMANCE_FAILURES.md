# Full Conformance Failure Analysis

**Last updated**: 2026-03-20 (round 18 — 50 tests, 6 passed, 44 failed = 12%)

## Errors from ALL Containers

### API Server Errors
1. **500 on StatefulSet scale subresource** — `GET /apis/apps/v1/.../statefulsets/ss/scale` returns 500
2. **500 on events list with label selector** — `GET /api/v1/events?labelSelector=...` returns 500
3. **400 on pod update** — `PUT /api/v1/.../pods/pod-update-...` returns 400 (deserialization)
4. **422 on configmap create** — `POST /api/v1/.../configmaps` returns 422
5. **422 on ReplicaSet create** — `POST /apis/apps/v1/.../replicasets` returns 422
6. **422 on CSINode create** — `POST /apis/storage.k8s.io/v1/csinodes` returns 422
7. **409 on DaemonSet status update** — repeated resourceVersion conflicts
8. **406 on CRD create** — protobuf body (repeated, can't fix without protobuf)

### Kubelet Errors
9. **"Unknown volume type for volume podinfo"** — downwardAPI volume with `items[].mode` field not handled
10. **resourceVersion conflicts on pod status update** — kubelet updates conflict with other writers

### Controller Manager Errors
11. **GC dependency cycle** — "Cycle detected in ownership chain: default -> default" (repeated)

### E2E Test Failures (unique causes)
12. **Scale subresource path routing** — `Wrong number of path arguments for Path. Expected 5 but got 2`
13. **Init container exit status** — `second init container should have failed` but shows exitCode 0
14. **Deployment revision not set** — `deployment doesn't have the required revision set`
15. **ExternalName service ClusterIP** — `unexpected Spec.ClusterIP (10.96.0.3) for ExternalName`
16. **Event timestamp parsing** — `parsing time "2017-09-19T13:49:16Z"` as MicroTime
17. **Namespace deletion** — `namespace was deleted unexpectedly` (namespace controller)
18. **grpc message too large** — `decoded message length too large: found 8358184 bytes, limit 4194304`
19. **Pod update rejected** — `the server rejected our request for an unknown reason (put pods)`
20. **Field validation strict mode** — `error missing unknown/duplicate field`
21. **Aggregated discovery** — `Failed to find /apis/`
22. **Networking** — `failed, 2 out of 2 connections failed`
23. **Sysctl** — test at sysctl.go:104 failing
24. **Taint-based eviction** — test at taints.go:489 failing
25. **Controller revision** — `error waiting for controllerrevisions to be created`
26. **EndpointSlice mirroring** — `Did not find matching EndpointSlice`
27. **Preemption** — scheduling/preemption.go BeforeEach failing
28. **CronJob scheduling** — both concurrent and forbid tests timing out
29. **Downward API volumes** — multiple pods timing out (volume "podinfo" unknown type)
30. **Projected volumes** — configmap/secret projected pods timing out

## Already Fixed but Not in This Run
These were committed AFTER the images were built:
- Liveness probe restart count tracking (8e3dce5)
- Init container PodInitializing reason (e294ffa)

## Root Cause Analysis

### MUST FIX (highest impact):
- **#9 downwardAPI volume "podinfo" with items[].mode** — This causes ~10 pod timeout failures. The volume has items with `mode` field that our handler doesn't support.
- **#12 Scale subresource path** — StatefulSet scale GET fails with wrong path args
- **#15 ExternalName ClusterIP** — Service update to ExternalName doesn't clear ClusterIP
- **#13 Init container exit status** — Init containers showing exitCode 0 when they should fail
- **#14 Deployment revision** — Deployment controller doesn't set revision annotation
- **#16 Event timestamp** — Event creation fails on non-MicroTime timestamps

### SHOULD FIX:
- **#11 GC cycle detection** — False positive cycle detection
- **#17 Namespace deletion** — Namespace controller deletes too aggressively
- **#19 Pod update rejected** — PUT pod deserialization issue
- **#26 EndpointSlice mirroring** — Custom endpoints not mirrored to EndpointSlices
- **#25 Controller revision** — DaemonSet controller revisions not created

### COMPLEX/SKIP:
- **#8 CRD protobuf** — needs protobuf codec
- **#20 Field validation strict** — needs strict JSON parsing mode
- **#18 grpc message too large** — etcd limit on large list responses
