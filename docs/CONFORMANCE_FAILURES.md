# Conformance Failure Tracker

**Round 135** | Running | 2026-04-11
**Round 134** | 370/441 (83.9%) | 2026-04-10

## Round 135 — New Failures

_Tracking failures as they appear. 38 fixes deployed since round 134._

### Preemption — `predicates.go:1041` (3 occurrences)
- Pod stuck in Pending/Unschedulable — critical pod can't preempt
- **Root cause**: Scheduler preemption only checked cpu/memory, not extended resources
- **Fix committed**: e1f4bd0 handles ALL resource types in preemption (for next deploy)

### Webhook — `webhook.go:520`
- "waiting for webhook configuration to be ready: timed out"
- Webhook service readiness check still timing out

### CRD OpenAPI — `crd_publish_openapi.go:285`
- Schema mismatch — investigating

### DNS — `dns_common.go:476`
- "client rate limiter Wait returned an error: context deadline exceeded"

### EndpointSlice Mirroring — `endpointslicemirroring.go:202`
- "Did not find matching EndpointSlice" — mirrored slices not cleaned up when source Endpoints deleted
- **Fix committed**: 361752a — cleanup mirrored slices + recognize mirroring-controller label

### Service Networking — `service.go:886`
- Multiport service unreachable via ClusterIP — kube-proxy timing

### CRD OpenAPI — `crd_publish_openapi.go:214,285`
- Schema items lost during typed deserialization
- **Fix committed**: 0188c3c — use raw JSON for CRD schemas in OpenAPI handler



## Deployed Fixes (38 commits since round 134)

| Commit | Fix |
|--------|-----|
| effdec6 | Watch HTTP/2 — remove Connection: keep-alive header |
| 46b54c0 | Webhook service resolution via ClusterIP |
| 5c423ba | CRD schema validation spec sub-schema + webhook denial reason |
| 047ba6b | CRD storage — preserve original JSON |
| 854d9e2 | JSONSchemaProps enum rename |
| 99ac117 | JSONSchemaProps missing multipleOf, externalDocs |
| 378f3d3 | CRD defaults — top-level extra fields |
| 571296a | YAML duplicate key detection |
| 2a6d8d8 | Status PATCH deep merge |
| dc42714 | kube-proxy FILTER table KUBE-FORWARD chain |
| e810b09 | kube-proxy skip sync when state unchanged |
| b37a8b8 | kube-proxy sync interval 1s |
| d9c9d34 | Init container intermediate status |
| 516922e | CRD GET defaults on read |
| f096b77 | CRD LIST defaults on read |
| 73eaccf | DaemonSet CR key sorting |
| 776c8fa | ResourceQuota extended resources |
| 6e9a13e | EndpointSlice mirroring selector-less |
| 1be61f8 | EndpointSlice sync interval 2s |
| 71608a0 | StatefulSet scale-down proper deletion |
| bab6e26 | Deployment maxSurge respect |
| 7bf82ee | Aggregator ClusterIP + 503 + postStart kills container |
| 7b1bf50 | PATCH SSA only for apply-patch content type |
| de62b6f | Scale subresource auth resource name |
| dd89022 | Scale selector label string format |
| 319f3f0 | EndpointSlice port name always set |
| 8d5038e | Pod IP field tolerant deserialization |
| 79078f9 | RS256 JWT signing for OIDC |
| 12aea53 | OIDC JWKS endpoint returns RSA public key |
| 01c7443 | Kubelet OnFailure restart policy |
| 4103c84 | Discovery API trailing slash normalization |
| 1d9b11f | Kubelet ErrImagePull — don't block sync loop |
| 5709a1f | Job terminating count + deleteCollection Status |
| ed474ac | deleteCollection returns Status JSON body |
| 1fe7e06 | Node DaemonEndpoints kubelet port 10250 |
| 6031dac | generate-certs.sh SA key before early exit |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 104 | 405 | 36 | 441 | 91.8% |
| 127 | 397 | 44 | 441 | 90.0% |
| 132 | 363 | 78 | 441 | 82.3% |
| 133 | 370 | 71 | 441 | 83.9% |
| 134 | 370 | 71 | 441 | 83.9% |
| 135 | TBD | TBD | 441 | TBD |
