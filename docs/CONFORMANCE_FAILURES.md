# Conformance Issue Tracker

**170 fixes total** | Pending deploy with critical watch fix

## CRITICAL FIX #170: Watch events missing resourceVersion

Root cause of 0-pass regression in round 96. Watch events from etcd did NOT
include metadata.resourceVersion in the JSON values (resourceVersion is only
added during get/list, not stored in etcd). The watch cache fell back to
chrono timestamps for revision tracking, creating mixed revision spaces
that confused the K8s client watch reconnection.

Fix: inject etcd mod_revision as metadata.resourceVersion into every watch
event value at the etcd storage layer, for both regular and from_revision watches.

## Fixes pending deploy (since round 96 deploy)

| # | Fix | Tests affected |
|---|-----|----------------|
| 163 | Protobuf: extract TypeMeta instead of 415 | 10 CRD/protobuf |
| 164 | ServiceCIDR /status route + SSA last-applied annotation | 2 |
| 166 | initial-events-end bookmark always sent | framework hang |
| 167 | PDB status PATCH via generic handler | 1 |
| 168 | Restart count monotonic (max docker + prev) | 1 |
| 169 | metadata.generation=1 on create, ClusterIP alloc, SA token, PodScheduled | 5+ |
| 170 | **CRITICAL** resourceVersion in watch event values | 12+ watch timeouts |

## Round 96 Failures (40 total, 0 passes)

### Watch timeouts (12) — should be fixed by #170
statefulset.go:786, rc.go:717, daemon_set.go:980, replica_set.go:232,
lifecycle_hook.go:132, pod_client.go:216, projected_configmap.go:367,
watch.go:409, job.go:623, :755, runtime.go:158, proxy.go:271

### Protobuf body (10) — partially addressed by #163
custom_resource_definition.go:72, :104, :288,
crd_publish_openapi.go:318, :366, field_validation.go:570, :700,
aggregated_discovery.go:227, :336, service_latency.go:142

### Webhook/aggregator (3)
webhook.go:861 (invalid matchConditions not rejected),
webhook.go:1269 (webhook not ready), aggregator.go:359

### Specific bugs (15) — many fixed by #164-169
- output.go:282 — CPU_LIMIT downward API
- output.go:263 — configmap subpath
- container_probe.go:1763 — restart count (**FIX #168**)
- disruption.go:604 — PDB PATCH (**FIX #167**)
- pods.go:556 — generation (**FIX #169**)
- service_accounts.go:898 — SA token (**FIX #169**)
- service.go:1444, :1483 — ClusterIP (**FIX #169**)
- predicates.go:1247 — PodScheduled (**FIX #169**)
- service.go:251 — affinity
- pod_resize.go:857 — resize
- conformance.go:696 — resourceVersion empty
- validatingadmissionpolicy.go:814 — VAP
- resource_quota.go:803 — quota
- builder.go:97 — kubectl protobuf
