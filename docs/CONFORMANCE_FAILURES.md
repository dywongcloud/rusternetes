# Conformance Issue Tracker

**172 fixes total** | All unit tests pass | Pending deploy

## Fixes since round 96 deploy (10 new)

| # | Fix | Tests |
|---|-----|-------|
| 163 | Protobuf: extract TypeMeta instead of 415 | 10 CRD |
| 164 | ServiceCIDR /status route + SSA annotation | 2 |
| 166 | initial-events-end bookmark always sent | framework |
| 167 | PDB status PATCH via generic handler | 1 |
| 168 | Restart count monotonic | 1 |
| 169 | generation=1, ClusterIP, SA token, PodScheduled | 5+ |
| 170 | **CRITICAL** resourceVersion in watch events | 12+ |
| 171 | Endpoints single subset | unit test |
| 172 | Ensure metadata exists for resourceVersion | 1 (DRA) |

## Expected results after deploy

### Should PASS (~22 tests from round 96 failures)
- 12 watch timeouts (fix #170)
- 5+ specific bugs (fixes #164-169, #172)

### Will still FAIL — protobuf (10 tests)
K8s CRD client uses native protobuf serialization. The raw field in the
Unknown protobuf envelope contains protobuf-encoded CRD data, NOT JSON.
Our middleware correctly identifies this but can't decode it without the
CRD protobuf schema. The `--kube-api-content-type=application/json` flag
only affects the e2e main client, not the apiextensions client.

These tests were ALWAYS failing across all rounds (not a regression):
- custom_resource_definition.go:72, :104, :288
- crd_publish_openapi.go:318, :366
- field_validation.go:570, :700
- aggregated_discovery.go:227, :336
- service_latency.go:142

### Will still FAIL — needs post-deploy debugging (5 tests)
- webhook.go:861, :1269 — webhook pod readiness
- aggregator.go:359 — aggregator pod readiness
- output.go:282 — CPU_LIMIT=1 vs expected 2
- output.go:263 — configmap subpath

### Will still FAIL — missing features (3 tests)
- pod_resize.go:857 — in-place pod resource resize (not implemented)
- resource_quota.go:803 — quota timing (controller interval)
- validatingadmissionpolicy.go:814 — depends on CRD creation (protobuf)
