# Conformance Issue Tracker

**Round 96**: 31 FAIL, 0 PASS (running) | **168 fixes** (166 deployed + 2 pending)

## ALL Round 96 Failures by Root Cause

### Protobuf body decoding (7 tests) — MUST FIX
CRD/Service created via protobuf. TypeMeta extraction gives minimal JSON missing `spec`.
- custom_resource_definition.go:72 — `missing field spec`
- custom_resource_definition.go:104 — `missing field spec`
- custom_resource_definition.go:288 — `missing field spec`
- crd_publish_openapi.go:318 — `context deadline exceeded` (CRD create timeout)
- crd_publish_openapi.go:366 — `context deadline exceeded`
- field_validation.go:700 — `key must be a string at line 1 column 2`
- service_latency.go:142 — `missing field spec at line 1 column 58`

### Watch timeouts (8 tests) — MUST FIX
Watch-based waiters time out. Events exist but may not reach subscriber.
- statefulset.go:786 — `timed out waiting for the condition`
- rc.go:717 — `timed out waiting for the condition`
- watch.go:409 — `Timed out waiting for expected watch notification`
- daemon_set.go:980 — `failed to locate daemon set`
- replica_set.go:232 — timeout
- lifecycle_hook.go:132 — timeout
- projected_configmap.go:367 — timeout
- builder.go:97 — kubectl timeout (protobuf validation)

### Webhook/aggregator pods not ready (2 tests)
Container starts but deployment shows ReadyReplicas=0.
- webhook.go:1269 — `waiting for webhook configuration to be ready`
- aggregator.go:359 — ReadyReplicas=0

### Pod generation field not set (1 test) — EASY FIX
`metadata.generation` must be set to 1 on creation, incremented on spec change.
- pods.go:556 — `verifying the new pod's generation is 1`

### Downward API resource field (2 tests)
CPU_LIMIT env var not set correctly from resource field ref.
- output.go:282 — `expected CPU_LIMIT=2`
- output.go:263 — pod output mismatch

### PDB status PATCH (1 test) — FIX #167 PENDING
Generic PUT handler can't handle PATCH merge body.
- disruption.go:604 — `the server rejected our request (patch poddisruptionbudgets)`

### Service account token creation (1 test)
- service_accounts.go:898 — `the server rejected our request (post serviceaccounts)`

### Restart count monotonic (1 test) — FIX #168 PENDING
Docker restart count resets on container recreation.
- container_probe.go:1763 — `restart count changed from 1 to 0`

### Resource quota (1 test)
- resource_quota.go:803 — quota counting

### VAP (1 test)
- validatingadmissionpolicy.go:814 — CEL/watch

### Service (2 tests)
- service.go:251 — affinity
- service.go:1444 — `didn't get ClusterIP for non-ExternalName service`

### Scheduling (1 test)
- predicates.go:1247 — `Did not find scheduled condition for pod`

### Job (2 tests)
- job.go:623 — timeout
- job.go:755 — timeout

### Aggregated discovery (1 test)
- aggregated_discovery.go:227 — `failed to decode CRD: missing field spec`

### Pod lifecycle (1 test)
- pod_client.go:216 — pod success timeout

## Priority Fixes

1. **metadata.generation** — set to 1 on create for ALL resources (easy, high impact)
2. **ClusterIP for services** — service.go:1444 missing ClusterIP (easy)
3. **SA token creation** — service_accounts.go:898 rejected (route issue?)
4. **Scheduling condition** — predicates.go:1247 pod condition not set
5. **PDB status PATCH** — fix #167 pending deploy
6. **Restart count** — fix #168 pending deploy
