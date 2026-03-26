# Conformance Issue Tracker

**172 fixes total** | All unit tests pass | Pending deploy

## Fixes since round 96 deploy

| # | Fix | Tests affected |
|---|-----|----------------|
| 163 | Protobuf: extract TypeMeta instead of 415 | 10 CRD/protobuf |
| 164 | ServiceCIDR /status route + SSA last-applied annotation | 2 |
| 166 | initial-events-end bookmark always sent | framework hang |
| 167 | PDB status PATCH via generic handler | 1 |
| 168 | Restart count monotonic (max docker + prev) | 1 |
| 169 | metadata.generation=1, ClusterIP alloc, SA token, PodScheduled | 5+ |
| 170 | **CRITICAL** resourceVersion in watch event values | 12+ watch timeouts |
| 171 | Endpoints: single subset with ready + notReady | 1 (unit test) |
| 172 | Ensure metadata exists in storage create | 1 (DRA resourceVersion) |

## Round 96 Failures — Status

### Fixed by pending deploys (22+)
- **Watch timeouts (12)** — fixed by #170 (resourceVersion injection)
- **PDB PATCH (1)** — fixed by #167
- **Restart count (1)** — fixed by #168
- **Generation (1)** — fixed by #169
- **ClusterIP (2)** — fixed by #169
- **SA token (1)** — fixed by #169
- **PodScheduled (1)** — fixed by #169
- **DRA resourceVersion (1)** — fixed by #172
- **ServiceCIDR status (1)** — fixed by #164
- **SSA annotation (1)** — fixed by #164

### Protobuf body (10) — partially addressed
CRD/protobuf bodies contain native protobuf, not JSON. TypeMeta extraction
gives minimal JSON missing spec. K8s client ignores --kube-api-content-type
for CRD operations. These tests were ALWAYS failing (not a regression).
- custom_resource_definition.go:72, :104, :288
- crd_publish_openapi.go:318, :366
- field_validation.go:570, :700
- aggregated_discovery.go:227, :336
- service_latency.go:142

### Webhook/aggregator (3) — needs post-deploy debugging
Container starts but pod status may not transition to Ready.
Fix #161 (kubelet sync) should help. Need deployed testing to verify.
- webhook.go:861, :1269
- aggregator.go:359

### Remaining specific issues (5)
- output.go:282 — CPU_LIMIT=1 instead of 2. Need to debug with deployed code.
- output.go:263 — configmap subpath timeout
- service.go:251 — session affinity (iptables recent module deployed)
- pod_resize.go:857 — pod resize state verification
- resource_quota.go:803 — quota counting timeout
- validatingadmissionpolicy.go:814 — VAP CEL
- builder.go:97 — kubectl protobuf (related to CRD protobuf issue)
