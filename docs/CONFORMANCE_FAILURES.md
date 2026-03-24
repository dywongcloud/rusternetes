# Full Conformance Failure Analysis

**Last updated**: 2026-03-24 (round 85 — 9/441 done, **4 PASSED**, 4 failed, 51 fixes)

## Critical root causes fixed

| # | Root Cause | Impact | Fix | Commit |
|---|-----------|--------|-----|--------|
| 1 | Container restart storm | Containers recreated every 2s, floods watches | Check state before recreating | `e99730b` |
| 2 | Watch Added vs Modified | All Put events sent as MODIFIED | Check prev_kv for new keys | `00db87c` |
| 3 | Field selector missing=false | All tests blocked | Treat missing as false | `646a407` |
| 4 | Protobuf body rejection | CRD/EndpointSlice creation fails | Extract JSON from envelope | `2571b32` |
| 5 | Service CIDR no route | Pod-to-service fails | Add route in kube-proxy | `b4f31c2` |
| 6 | API ClusterIP not routable | Pods can't reach API | Direct IP + TLS SANs | `b224387`+ |
| 7 | Watch N×N explosion | etcd overwhelmed | Watch cache (1 per prefix) | `73c3514` |
| 8 | Controller 10s interval | Tests time out waiting | Reduced to 2s | `ea1b800` |
| 9 | Namespace cascade order | Orphaned pods block cleanup | Delete controllers first | `1270649` |

## Round 84 active failures (11 tests, 10 unique types)

### 1. Watch closed before timeout (StatefulSet)
- **Test**: StatefulSet scaling order verification
- **Symptom**: Watch for pod events closes before test verifies scaling order
- **Root cause**: Container restart storm was a RED HERRING — events controller emits "Created container" events every 2s (incrementing count), not actual container recreations. The kubelet only starts containers 4 times total. The watch close is caused by HTTP/2 stream management — client-go sends RST_STREAM which causes tx.send() to fail.
- **Confirmed**: Bookmark interval reduced to 15s (no more "context canceled" spam). Container restarts reduced to 4 from 25+. But watch still closes.
- **Research needed**: HTTP/2 stream lifecycle management. May need to use a different streaming mechanism or implement server-sent events.

### 2. IntOrString serialization (Deployment)
- **Test**: Deployment rollback/scaling with percentage maxUnavailable/maxSurge
- **Symptom**: `invalid value for IntOrString: invalid type: string is not a percentage`
- **Root cause**: `maxUnavailable` stored as `Option<String>`. Integer value `1` stored as string `"1"`, serialized back as `"maxUnavailable": "1"`. Go client sees string "1" which isn't a percentage and fails.
- **Fix needed**: Change `maxUnavailable`/`maxSurge` to `Option<serde_json::Value>` to preserve original int/string type. In progress — build error in controller-manager needs fixing.

### 3. Service not reachable (EndpointSlice test)
- **Test**: Service endpoint connectivity
- **Symptom**: `nc: connect to endpoint-test2 (10.96.0.3) port 80 (tcp) failed: Connection refused`
- **Root cause**: Service ClusterIP routing works (route added) but no backend pod is listening on port 80. The pod may not have started, or the service→endpoint mapping is wrong.
- **Research needed**: Check if endpoints controller creates proper endpoints for the service. Check if pods are actually running and listening.

### 4. Watch condition timeout (ReplicationController lifecycle)
- **Test**: RC lifecycle — watch for status updates
- **Symptom**: `Wait until condition with watch events should not return an error: timed out`
- **Root cause**: Likely same as #1 — watch events being dropped or not delivered properly. May also be affected by Added vs Modified fix.
- **Research needed**: Check if RC status updates generate proper watch events.

### 5. Container output "[1-9]" (Downward API projected volume)
- **Test**: Projected volume with memory/CPU limits
- **Symptom**: Expected a number like "134217728" (memory in bytes) but got wrong value
- **Root cause**: Downward API volume resource field handling. CPU divisor fix (`095b407`) deployed but memory divisor may also need fixing.
- **Research needed**: Check `get_container_resource_value` for memory resource fields. Verify divisor handling for memory (should return bytes with divisor "1").

### 6. Container output "1" (Downward API volume)
- **Test**: CPU request downward API volume
- **Symptom**: Expected "1" (1 core) in cpu_request file but got wrong value
- **Root cause**: Same as #5. The CPU divisor fix changes default to return cores, but volume path may use different logic.
- **Research needed**: Verify projected volume and downward API volume both use `get_container_resource_value`.

### 7. CSINode deletecollection (fixed, not deployed)
- **Fix**: Added `.delete()` route for CSINode collection endpoint
- **Commit**: `a9cdc55`

### 8. Webhook configuration denied
- **Test**: ValidatingWebhookConfiguration admission
- **Symptom**: "create validatingwebhookconfiguration should have been denied by the api-server"
- **Root cause**: Our API server doesn't enforce validating admission webhooks. When a webhook is configured to deny certain requests, our server allows them anyway.
- **Research needed**: Check if `admission_webhook.rs` validates incoming requests against configured ValidatingWebhookConfigurations.

### 9. Context deadline exceeded (Scheduler predicate, ×2)
- **Test**: Scheduler predicate tests
- **Symptom**: Pod scheduling times out
- **Root cause**: Scheduler may be too slow to schedule pods, or the node affinity/anti-affinity predicates aren't implemented properly.
- **Research needed**: Check scheduler implementation for predicate evaluation.

### 10. Watch "context canceled" spam
- **Observed**: `retrywatcher.go:169] "Watch failed" err="context canceled"` every second
- **Impact**: Causes watch-dependent tests to fail (deployments, RC, etc.)
- **Root cause**: Client-go's retry watcher keeps restarting because our watch stream closes. This may be caused by HTTP/2 stream management issues.
- **Research needed**: Check if our streaming response properly handles HTTP/2 flow control. May need to send periodic data to keep the stream alive.

## All 49 fixes committed

| Fix | Commit |
|-----|--------|
| Container logs search | `2b1008d` |
| EventList ListMeta | `97938e4` |
| gRPC probe | `e738c1f` |
| Scale PATCH | `d335dee` |
| Status PATCH routes | `d335dee` |
| events.k8s.io/v1 | `f8a75da` |
| CRD openAPIV3Schema | `abd2137` |
| ResourceSlice Kind | `9b21a89` |
| PDB status defaults | `9b21a89` |
| PV status phase | `710eee1` |
| metadata.namespace | `db40409` |
| camelCase abbreviations | `bde38ef` |
| VolumeAttributesClass delete | `bde38ef` |
| OpenAPI protobuf 406 | `bde38ef` |
| Container retention | `2c8e1fd` |
| Termination message | `c804e57` |
| Init container Waiting | `b54d541` |
| StatefulSet revision | `7f5c9bc` |
| SA token key | `9238eb4` |
| Proxy handler keys | `b4b745c` |
| nonResourceURLs | `98f0eac` |
| Deployment revision | `565c216` |
| EndpointSlice cleanup | `6f79efa` |
| Fail on missing volumes | `5e07c6e` |
| ClusterIP pre-allocation | `4113fe9` |
| K8S_SERVICE_HOST | `b224387` |
| K8S_SERVICE_PORT | `862c286` |
| TLS SANs | `f9c9691` |
| ClusterIP re-allocation | `cd6ab64` |
| Field selector | `646a407` |
| Watch reconnect | `5edb20d` |
| runAsGroup | `bbaa43f` |
| RC FailedCreate | `702b107` |
| Service CIDR route | `b4f31c2` |
| iproute2 | `cec24ff` |
| JOB_COMPLETION_INDEX | `f288981` |
| /apis/{group} | `3327567` |
| Downward API | `095b407` |
| Watch bookmark resilience | `0eb215d` |
| Watch cache | `73c3514` |
| Watch subscribe-before-list | `d2e306c` |
| Namespace cascade | `1270649` |
| Protobuf extraction | `2571b32` |
| RS conditions preserve | `58317e6` |
| Controller interval 2s | `ea1b800` |
| Status PATCH metadata | `38b44f2` |
| Container restart fix | `e99730b` |
| Watch Added vs Modified | `00db87c` |
| CSINode deletecollection | `a9cdc55` |
