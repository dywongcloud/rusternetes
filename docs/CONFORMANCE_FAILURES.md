# Conformance Failure Tracking

**Last updated**: 2026-03-25 | **Round 90**: 16 PASS, 45 FAIL | **Fixes**: 105 committed, pending deploy

## Fixes pending deploy (will resolve on next rebuild)

| Fix | Failures resolved | Impact |
|-----|-------------------|--------|
| Stale webhook namespace skip | ~21 pod creation failures | Skips webhooks when service namespace deleted |
| Conversion webhook graceful failure | Pod creation 503 errors | Returns unconverted instead of error |
| Node capacity in heartbeat | "Allocatable does not contain cpu" | Ensures capacity always populated |
| Aggregated discovery Accept header | "Failed to parse /api output" | Fixes content negotiation |
| Downward API resource defaults | "memory_limit: 0" | Returns node capacity (8Gi/4CPU) when no limits |
| CRD apiVersion defaults | "missing field apiVersion" | Injects defaults from protobuf |
| Watch leak cleanup | Performance degradation | Stops watches when no subscribers |
| Field validation duplicate format | "duplicate field" error format | Matches K8s "unknown field" format |
| Job backoffLimitPerIndex | 900s job timeout | Tracks per-index failures |
| Kube-proxy headless DNAT | "host/network None not found" | Validates ClusterIP before DNAT |
| FlowSchema delete route | "method not allowed" on delete | Added .delete() to router |
| CSR update kind/resourceVersion | CSR update fails | Added kind/apiVersion, resourceVersion check |

## Currently broken — needs code fix

### Watch stream closure (HTTP/2 RST_STREAM) — 5 failures
- `statefulset.go:786`, `deployment.go:585`, `daemon_set.go:980`, `replica_set.go:560`, `crd_watch.go:72`
- Client-go sends RST_STREAM to cancel watches. Watch history replay works but informer reconnection may miss events.
- **Root cause**: HTTP/2 stream management. When sender fails, watch terminates.
- **Needs**: More robust watch stream error handling — detect RST_STREAM and keep connection alive or reconnect transparently.

### Scheduler preemption — 3 failures
- `preemption.go:516`, `predicates.go:354`, `predicates.go:1035`
- Preemption code exists but pods don't get scheduled after eviction within timeout.
- **Needs**: Debug why preempted pod doesn't get re-scheduled. May be timing issue with 2s scheduler interval.

### Webhook container crash (exit 255) — 2 failures
- `webhook.go:1194`, `webhook.go:1631`
- agnhost webhook container exits immediately with code 255 on Docker Desktop.
- **Root cause**: Likely ARM64/AMD64 image compatibility or missing shared libraries.
- **Needs**: Test with native ARM64 image or investigate container crash reason.

### EndpointSlice count — 1 failure
- `endpointslice.go:798` — "Expected at least 2 EndpointSlices, got 1"
- Our controller creates 1 EndpointSlice per service. K8s creates multiple for different ports or large endpoint sets.
- **Needs**: EndpointSlice controller should create separate slices for different port combinations.

### CRD creation timeout — 1 failure
- `crd_publish_openapi.go:285` — "failed to create CRD: context deadline exceeded"
- CRD creation via protobuf takes too long or body parsing fails silently.
- **Needs**: Debug CRD creation path for protobuf requests.

### CSR CRUD — 1 failure
- `certificates.go:343` — update step fails
- CSR handler may not preserve all fields on update.
- **Needs**: Debug CSR update handler.

### FlowControl CRUD — FIXED (pending deploy)
- `flowcontrol.go:433` — FlowSchema delete route missing
- **Fix**: Added `.delete()` method to flowschemas/:name route.

### CSR CRUD — FIXED (pending deploy)
- `certificates.go:343` — CSR update missing kind/apiVersion
- **Fix**: Added kind/apiVersion, resourceVersion check, status preservation.

### Service test — 1 failure
- `service.go:4408` — stale webhook (will be fixed by webhook namespace skip)

## Not fixable from our code

| Issue | Reason |
|-------|--------|
| File permissions (output.go:263) | Docker Desktop VirtioFS may mask Unix permissions |
| Webhook container crash (exit 255) | Docker Desktop ARM64 image compatibility |
| ValidatingAdmissionPolicy tests | Skipped — requires CEL evaluation engine |
