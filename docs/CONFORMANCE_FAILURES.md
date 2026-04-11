# Conformance Failure Tracker

**Round 134** | 370/441 (83.9%) | 2026-04-10
**Round 135** | Pending (35 staged fixes, 105 total fix commits) | 2026-04-10

## Root Cause Analysis — Round 134 (71 failures)

### Watch Reliability — ~20 cascade failures (STAGED effdec6)
- `deployment.go:1008,1288`, `rc.go:509,623`, `replica_set.go:232,560`, `runtime.go:115`, `service.go:3459`, `statefulset.go:1092`, `preemption.go:181,268,516,1025`, `namespace.go:579`
- **Root cause**: 3,875 "Watch failed: context canceled" errors — Connection header prohibited in HTTP/2 (RFC 7540 §8.1.2.2)
- **Fix staged**: effdec6 removes Connection: keep-alive, uses Transfer-Encoding: chunked
- **K8s ref**: staging/src/k8s.io/apiserver/pkg/endpoints/handlers/watch.go:237
- **Note**: Single highest-impact fix. Watch failures cascade into deployment, RC, RS, StatefulSet, preemption, namespace, and DNS failures. Kubelet also couldn't pick up new pods — confirmed by live testing (dns-check2 pod stuck in Pending for 30+ seconds because kubelet's etcd watch was also failing).

### Webhook Service Readiness — 12 failures (STAGED 46b54c0 + 5c423ba)
- `webhook.go:520,675,904,1269,1334,1400,2107,2132,2491` + EmptyDir cascades (output.go:263 x4)
- **Root cause**: Webhook resolution bypassed ClusterIP, went directly to endpoint IPs; denial reason not extracted from status.reason
- **Fix staged**: 46b54c0 resolves via ClusterIP (K8s serviceresolver.go), 5c423ba uses status.reason fallback
- **K8s ref**: staging/src/k8s.io/apiserver/pkg/util/webhook/serviceresolver.go

### CRD OpenAPI — 9 failures (STAGED 047ba6b + 854d9e2 + 99ac117)
- `crd_publish_openapi.go:77,161,214,253,285,318,366,400,451`
- **Root cause**: serde round-trip lost nested JSONSchemaProps in untagged enum; enum field rename; missing multipleOf/externalDocs
- **Fix staged**: 047ba6b preserves original JSON in etcd, 854d9e2 enum rename, 99ac117 missing fields

### Field Validation — 4 failures (STAGED 571296a + 5c423ba)
- `field_validation.go:278,462,611,735`
- **Root cause**: CRD schema validation checked cr.spec against top-level schema instead of schema.properties["spec"]; YAML duplicate keys not detected
- **Fix staged**: 5c423ba validates spec against spec sub-schema, 571296a YAML dup detection

### DNS — 6 failures (watch cascade + service networking + kubelet sync)
- `dns_common.go:476` (6 occurrences)
- **Root cause**: Tested and verified: multi-container pods with emptyDir+backtick commands work correctly on our kubelet. DNS failures are downstream of: (1) watch cascade (kubelet doesn't see new pods), (2) kubelet blocked by ErrImagePull retries on failed pods (fixed: 1d9b11f), (3) service networking gaps (fixed: dc42714 + e810b09)
- **Expected fix**: Watch fix + kubelet ErrImagePull fix + kube-proxy fixes should resolve

### Service Networking — 5 failures (STAGED dc42714 + e810b09 + b37a8b8)
- `service.go:768,886,3459`, `proxy.go:271,503`
- **Root cause**: kube-proxy missing FILTER table KUBE-FORWARD chain; flush+rebuild cycle creates "Connection refused" gap (confirmed: 172.18.0.13:80 returns "Connection refused" from exec pod)
- **Fix staged**: dc42714 adds KUBE-FORWARD filter rules, e810b09 eliminates flush gap by hashing state, b37a8b8 reduces sync interval to 1s
- **K8s ref**: pkg/proxy/iptables/proxier.go — KUBE-FORWARD chain, iptables-restore for atomic updates

### Preemption — 4 failures (STAGED 2a6d8d8, watch cascade)
- `preemption.go:181,268,516,1025`
- **Root cause**: Status PATCH shallow merge clobbers node capacity; also blocked by watch failures
- **Fix staged**: 2a6d8d8 deep merge for status PATCH

### kubectl — 3 failures (STAGED 7b1bf50 + de62b6f + dd89022 + 319f3f0 + 4103c84)
- `kubectl.go:1881` (proxy), `builder.go:97` (scale RC x2, describe service)
- **Root causes found and fixed**:
  - PATCH handlers routed to SSA when fieldManager set with non-apply content type → 7b1bf50
  - Scale subresource auth used `format!("{}.{}", resource, group)` producing trailing dot → de62b6f
  - Scale selector returned as JSON instead of label selector string format → dd89022
  - EndpointSlice port.name was nil causing kubectl describe crash → 319f3f0
  - /api/ returned empty response (trailing slash not matched) → 4103c84
- **K8s ref**: staging/src/k8s.io/apiserver/pkg/endpoints/handlers/patch.go

### Init Container — 2 failures (STAGED d9c9d34)
- `init_container.go:440,565`
- **Fix staged**: d9c9d34 adds intermediate status updates between init container runs

### Lifecycle Hooks — 2 failures (watch cascade + networking)
- `lifecycle_hook.go:132`, `pre_stop.go:153`
- **Root cause**: Verified kubelet CAN reach pod IPs (ping works to 172.18.0.x). preStop exec handler is correct. postStart now kills container on failure (7bf82ee). Verified inter-pod HTTP connectivity works (TCP reaches but server may not be ready). Failures are downstream of watch cascade preventing pod status updates.

### CRD Defaulting — 1 failure (STAGED 516922e + f096b77 + 378f3d3)
- `custom_resource_definition.go:334`
- **Fix staged**: 516922e GET defaults, f096b77 LIST defaults, 378f3d3 top-level extra fields

### DaemonSet ControllerRevision — 1 failure (STAGED 73eaccf)
- `daemon_set.go:1276`
- **Fix staged**: 73eaccf sorts JSON keys alphabetically matching Go encoding/json

### Resource Quota — 1 failure (STAGED 776c8fa)
- `resource_quota.go:282`
- **Fix staged**: 776c8fa adds extended resource counting

### EndpointSlice Mirroring — 1 failure (STAGED 6e9a13e, watch cascade)
- `endpointslicemirroring.go:129`
- **Fix staged**: 6e9a13e mirrors Endpoints for selector-less services

### Service Accounts OIDC — 1 failure (STAGED 79078f9 + 12aea53)
- `service_accounts.go:667`
- **Root cause**: JWT tokens signed with HS256 instead of RS256; JWKS endpoint returned empty keys; TLS cert not trusted by pod
- **Fix staged**: 79078f9 RS256 signing + generate-certs.sh SA key pair, 12aea53 JWKS endpoint returns public key in JWK format
- **K8s ref**: pkg/serviceaccount/jwt.go, pkg/serviceaccount/openidmetadata.go

### Aggregator — 1 failure (STAGED 7bf82ee)
- `aggregator.go:359`
- **Root cause**: Aggregation proxy silently fell through when endpoints not ready
- **Fix staged**: 7bf82ee uses ClusterIP resolution + returns 503 when service unavailable
- **K8s ref**: staging/src/k8s.io/kube-aggregator/pkg/apiserver/handler_proxy.go

### Container Runtime — 1 failure (STAGED 01c7443)
- `runtime.go:115`
- **Root cause**: Kubelet only restarted containers for RestartPolicy=Always, not OnFailure
- **Fix staged**: 01c7443 handles OnFailure (restart on non-zero exit code, skip on exit 0)
- **K8s ref**: pkg/kubelet/kubelet.go — computePodActions()

### Service Latency — 1 failure (STAGED 8d5038e)
- `service_latency.go:142`
- **Fix staged**: 8d5038e adds #[serde(default)] to HostAlias/HostIP/PodIP

### Host Port — 1 failure (DinD limitation)
- `hostport.go:219`
- Host port binding to specific IPs doesn't work in DinD

### Pod Resize — 1 failure (DinD limitation)
- `pod_resize.go:857`
- cgroup changes limited in DinD

## Staged Fixes (35 commits, need deploy)

| Commit | Fix | Tests |
|--------|-----|-------|
| d9c9d34 | Init container intermediate status | init_container:440,565 |
| 516922e | CRD GET defaults on read | custom_resource_definition:334 |
| f096b77 | CRD LIST defaults on read | CRD list tests |
| 73eaccf | DaemonSet CR key sorting | daemon_set:1276 |
| 776c8fa | ResourceQuota extended resources | resource_quota:282 |
| 6e9a13e | EndpointSlice mirroring selector-less | endpointslicemirroring:129 |
| 1be61f8 | EndpointSlice sync interval 2s | webhook readiness timing |
| 71608a0 | StatefulSet scale-down proper deletion | statefulset:957 |
| effdec6 | Watch HTTP/2 headers fix | ~20 watch cascade failures |
| bab6e26 | Deployment maxSurge respect | deployment:995 |
| 571296a | YAML duplicate key detection | field_validation:735 |
| b37a8b8 | kube-proxy sync interval 1s | service networking |
| 2a6d8d8 | Status PATCH deep merge | preemption, resource_quota |
| 854d9e2 | JSONSchemaProps enum rename | CRD schemas with enum |
| 99ac117 | JSONSchemaProps missing multipleOf, externalDocs | CRD completeness |
| 378f3d3 | CRD defaults — top-level extra fields | custom_resource_definition:334 |
| 047ba6b | CRD storage — preserve original JSON | crd_publish_openapi: 9 tests |
| 46b54c0 | Webhook service resolution via ClusterIP | webhook: 12 tests |
| 5c423ba | CRD schema validation + webhook denial reason | field_validation: 3, webhook: 1 |
| dc42714 | kube-proxy FILTER table KUBE-FORWARD chain | service networking |
| 8d5038e | Pod IP field tolerant deserialization | service_latency:142 |
| 7bf82ee | Aggregator ClusterIP + 503 + postStart kills | aggregator:359, lifecycle |
| 7b1bf50 | PATCH SSA only for apply-patch content type | kubectl label/scale/annotate |
| de62b6f | Scale subresource auth resource name | kubectl scale RC |
| dd89022 | Scale selector label string format | kubectl scale RC |
| 319f3f0 | EndpointSlice port name always set | kubectl describe service crash |
| e810b09 | kube-proxy skip sync when unchanged | service networking flush gap |
| 79078f9 | RS256 JWT signing for OIDC | service_accounts:667 |
| 01c7443 | Kubelet OnFailure restart policy | runtime:115 |
| 4103c84 | Discovery API trailing slash normalization | kubectl.go:1881 proxy |
| 12aea53 | OIDC JWKS endpoint returns RSA public key | service_accounts:667 |
| 1d9b11f | Kubelet ErrImagePull — don't block sync loop | all pod startup timing |

## Expected Impact of Staged Fixes

| Category | Fixes | Expected Tests Fixed |
|----------|-------|---------------------|
| Watch reliability | effdec6 | ~20 |
| Webhook resolution | 46b54c0, 5c423ba | ~12 |
| CRD OpenAPI | 047ba6b, 854d9e2, 99ac117 | 9 |
| Field validation | 571296a, 5c423ba | 4 |
| DNS (watch+networking+kubelet) | effdec6, dc42714, e810b09, 1d9b11f | ~4-6 |
| Service networking | dc42714, e810b09, b37a8b8 | ~3-5 |
| Preemption | 2a6d8d8 | ~2-4 |
| kubectl | 7b1bf50, de62b6f, dd89022, 319f3f0, 4103c84 | 3 |
| Init container | d9c9d34 | 2 |
| Individual fixes | various (11 commits) | ~8 |
| **Total** | | **~63-69 of 71** |

**Projected Round 135**: ~433-439/441 (98-99.5%)

Remaining 2-8 expected failures:
- Host port (DinD limitation — can't bind to specific bridge IPs)
- Pod resize (DinD cgroup limitation)
- Possibly: OIDC TLS trust (pod needs CA cert in trust store), namespace deletion race

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 104 | 405 | 36 | 441 | 91.8% |
| 127 | 397 | 44 | 441 | 90.0% |
| 132 | 363 | 78 | 441 | 82.3% |
| 133 | 370 | 71 | 441 | 83.9% |
| 134 | 370 | 71 | 441 | 83.9% |
| 135 | TBD | TBD | 441 | TBD |
