# Conformance Failure Tracker

**Round 134** | 370/441 (83.9%) | 2026-04-10
**Round 135** | Pending (21 staged fixes) | 2026-04-10

## Root Cause Analysis — Round 134 (71 failures)

### Watch Reliability — ~20 cascade failures (STAGED effdec6)
- `deployment.go:1008,1288`, `rc.go:509,623`, `replica_set.go:232,560`, `runtime.go:115`, `service.go:3459`, `statefulset.go:1092`, `preemption.go:181,268,516,1025`
- **Root cause**: "Watch failed: context canceled" — Connection header prohibited in HTTP/2
- **Fix staged**: effdec6 removes Connection: keep-alive header, uses Transfer-Encoding: chunked
- **K8s ref**: staging/src/k8s.io/apiserver/pkg/endpoints/handlers/watch.go:237

### Webhook Service Readiness — 12 failures (STAGED 46b54c0 + 5c423ba)
- `webhook.go:520,675,904,1269,1334,1400,2107,2132,2491` + 3 EmptyDir cascades
- **Root cause**: Webhook resolution bypassed ClusterIP + denial reason not extracted from status.reason
- **Fix staged**: 46b54c0 resolves via ClusterIP, 5c423ba uses status.reason fallback
- **K8s ref**: staging/src/k8s.io/apiserver/pkg/util/webhook/serviceresolver.go

### CRD OpenAPI — 9 failures (STAGED 047ba6b)
- `crd_publish_openapi.go:77,161,214,253,285,318,366,400,451`
- **Root cause**: serde round-trip lost nested JSONSchemaProps in untagged enum
- **Fix staged**: 047ba6b preserves original JSON in etcd storage

### Field Validation — 4 failures (STAGED 571296a + 5c423ba)
- `field_validation.go:278,462,611,735`
- **Root cause**: CRD schema validation checked cr.spec against top-level schema instead of schema.properties["spec"]
- **Fix staged**: 571296a YAML dup detection, 5c423ba validates spec against spec sub-schema

### DNS — 6 failures
- `dns_common.go:476` (6 occurrences)
- **Root cause**: DNS test pods not starting — container exec runs in pause container, pod proxy can't reach pods
- **Blocked by**: Watch failures (pods don't get status updates), service networking

### Service Networking — 5 failures (STAGED dc42714 + b37a8b8)
- `service.go:768,886,3459`, `proxy.go:271,503`
- **Root cause**: kube-proxy missing FILTER table rules, ClusterIP→Pod forwarding dropped
- **Fix staged**: dc42714 adds KUBE-FORWARD chain in filter table, b37a8b8 reduces sync interval

### Preemption — 4 failures (STAGED 2a6d8d8)
- `preemption.go:181,268,516,1025`
- **Root cause**: Status PATCH shallow merge clobbers node capacity for extended resources
- **Fix staged**: 2a6d8d8 uses deep merge for status PATCH

### Init Container — 2 failures (STAGED d9c9d34)
- `init_container.go:440,565`
- **Root cause**: Kubelet doesn't send intermediate status during init container execution
- **Fix staged**: d9c9d34 adds status updates between init container runs

### CRD Defaulting — 1 failure (STAGED 516922e)
- `custom_resource_definition.go:334`
- **Fix staged**: 516922e applies defaults on GET, f096b77 on LIST

### DaemonSet ControllerRevision — 1 failure (STAGED 73eaccf)
- `daemon_set.go:1276`
- **Fix staged**: 73eaccf sorts JSON keys alphabetically

### Resource Quota — 1 failure (STAGED 776c8fa)
- `resource_quota.go:282`
- **Fix staged**: 776c8fa adds extended resource counting

### EndpointSlice Mirroring — 1 failure (STAGED 6e9a13e)
- `endpointslicemirroring.go:129`
- **Fix staged**: 6e9a13e only skips when service HAS selector

### EmptyDir Volumes — 4 failures (webhook cascade)
- `output.go:263` (4 occurrences)
- **Root cause**: NOT a permissions issue — stale webhook config blocks pod creation
- **Fix**: Webhook service resolution fix (46b54c0) should resolve

### Lifecycle Hooks — 2 failures
- `lifecycle_hook.go:132`, `pre_stop.go:153`
- **Root cause**: Kubelet can't reach pod IPs for HTTP lifecycle hooks (DinD networking)

### Aggregator — 1 failure
- `aggregator.go:359`
- **Root cause**: Extension API server deployment doesn't start

### Service Accounts OIDC — 1 failure
- `service_accounts.go:667`
- **Root cause**: OIDC discovery TLS — pod doesn't trust API server cert

### Host Port — 1 failure
- `hostport.go:219`
- **Root cause**: Host port binding in DinD

### Pod Resize — 1 failure
- `pod_resize.go:857`
- **Root cause**: cgroup changes in DinD

### kubectl — 3 failures
- `kubectl.go:1881` (proxy), `builder.go:97` (scale RC, describe service)
- **Root cause**: kubectl commands fail — scale/describe/proxy

### Service Latency — 1 failure (STAGED 8d5038e)
- `service_latency.go:142`
- **Root cause**: Deserialization fails on missing `ip` field
- **Fix staged**: 8d5038e adds #[serde(default)] to HostAlias/HostIP/PodIP

### Namespace Deletion — 1 failure
- `namespace.go:579`
- **Root cause**: Namespace deleted before controller sets conditions

## Staged Fixes (21 commits, need deploy)

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
| dc42714 | kube-proxy FILTER table KUBE-FORWARD chain | service networking: 5 tests |
| 8d5038e | Pod IP field tolerant deserialization | service_latency:142 |

## Expected Impact of Staged Fixes

Staged fixes should resolve ~40-45 of 71 failures:
- Watch reliability: ~20 tests
- Webhook resolution: ~12 tests
- CRD OpenAPI: 9 tests
- Field validation: 4 tests
- Service networking: ~3-5 tests
- Init container: 2 tests
- Plus individual fixes: ~5 tests

**Projected Round 135**: ~410-415/441 (93-94%)

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 104 | 405 | 36 | 441 | 91.8% |
| 127 | 397 | 44 | 441 | 90.0% |
| 132 | 363 | 78 | 441 | 82.3% |
| 133 | 370 | 71 | 441 | 83.9% |
| 134 | 370 | 71 | 441 | 83.9% |
| 135 | TBD | TBD | 441 | TBD |
