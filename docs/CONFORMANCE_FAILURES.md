# Conformance Issue Tracker

**Round 109** | 48 failures / 78 tests ran (e2e killed during skip phase) | 328 fixes deployed

## Round 109 — All 48 Failures

### 1. Webhook deployment not ready (7 failures) — FIXED (7bc88bf)
Error: `waiting for webhook configuration to be ready: timed out waiting for the condition`
Fix: Kubelet timeout fix — when Docker API times out checking Running pods, assume still running instead of skipping readiness path. Also removes exited containers to prevent Docker overhead.
| File | Line |
|------|------|
| `webhook.go` | 425, 520, 601, 1244, 1549, 2338, 2465 |

### 2. Webhook matchConditions CEL error (2 failures) — FIXED (7d40469)
Error: `matchConditions[0].expression: compilation failed: No such key: metadata`
Fix: Case-insensitive comparison for CEL errors. "No such key" (uppercase) wasn't matching "no such key" check.
| File | Line |
|------|------|
| `webhook.go` | 729, 783 |

### 3. CRD creation timeout (4 failures)
Error: `failed to create CRD: context deadline exceeded` / `creating CustomResourceDefinition: context deadline exceeded`
| File | Line |
|------|------|
| `crd_publish_openapi.go` | 318, 451 |
| `custom_resource_definition.go` | 104, 288 |

### 4. CRD field validation decode error (3 failures)
Error: `cannot create crd failed to decode CRD: key must be a string at line 1 column 2`
Binary body (protobuf/CBOR) can't be parsed as JSON.
| File | Line |
|------|------|
| `field_validation.go` | 245, 428, 570 |

### 5. CRD field validation timeout (1 failure)
Error: `cannot create crd context deadline exceeded`
| File | Line |
|------|------|
| `field_validation.go` | 305 |

### 6. Pod resize PATCH rejected (3 failures) — FIXED (7d40469)
Error: `failed to patch pod for resize: Unsupported content type: application/json`
Fix: Pod PATCH handler now checks X-Original-Content-Type header before content-type.
| File | Line |
|------|------|
| `pod_resize.go` | 850 (x3) |

### 7. Ephemeral containers PATCH rejected (1 failure) — FIXED (7d40469)
Error: `Failed to patch ephemeral containers: Unsupported content type: application/json`
Fix: Same as #6 — pod PATCH handler shared by ephemeral containers endpoint.
| File | Line |
|------|------|
| `ephemeral_containers.go` | 80 |

### 8. Job SuccessPolicy (3 failures) — FIXED (4f60d58)
Fix: Job controller now preserves completion status from SuccessPolicy instead of overwriting on next reconcile.
| File | Line | Error |
|------|------|-------|
| `job.go` | 514 | Expected 0, got nil — regular update wiped SuccessPolicy status |
| `job.go` | 553 | Same root cause |
| `job.go` | 974 | context deadline exceeded — cascading from status not set |

### 9. Aggregated discovery (2 failures) — FIXED (f5241df)
Fix: q-value preference parsing for Accept header. Returns aggregated when q-value is higher.
| File | Line | Error |
|------|------|-------|
| `aggregated_discovery.go` | 227 | context deadline exceeded |
| `aggregated_discovery.go` | 282 | Expected validatingwebhookconfigurations to be present |

### 10. Resource quota status (2 failures)
Quota `status.used` doesn't match expected values.
| File | Line |
|------|------|
| `resource_quota.go` | 282, 489 |

### 11. Watch DELETE event (1 failure)
Error: `Timed out waiting for expected watch notification: {DELETED <nil>}`
| File | Line |
|------|------|
| `watch.go` | 409 |

### 12. StatefulSet scaling (1 failure)
Error: `StatefulSet ss scaled unexpectedly scaled to 3 -> 2 replicas`
| File | Line |
|------|------|
| `statefulset.go` | 2479 |

### 13. /etc/hosts not kubelet-managed (1 failure)
Error: Docker default `/etc/hosts` used instead of kubelet-managed one.
| File | Line |
|------|------|
| `kubelet_etc_hosts.go` | 143 |

### 14. Init container timeout (1 failure)
Error: `timed out waiting for the condition`
| File | Line |
|------|------|
| `init_container.go` | 440 |

### 15. Service latency decode error (1 failure) — FIXED (already deployed)
Error: `failed to decode: missing field 'selector' at line 1 column 493`
Fix: ServiceSpec has Default derive and `#[serde(default)]` on selector — already in deployed code.
| File | Line |
|------|------|
| `service_latency.go` | 142 |

### 16. Network service (2 failures)
| File | Line | Error |
|------|------|-------|
| `service.go` | 1571 | context deadline exceeded |
| `service.go` | 4291 | service not reachable within 2m0s |

### 17. DNS resolution (1 failure)
Error: `context deadline exceeded`
| File | Line |
|------|------|
| `dns_common.go` | 476 |

### 18. EndpointSlice (1 failure)
Error: `Error fetching EndpointSlice: context deadline exceeded`
| File | Line |
|------|------|
| `endpointslice.go` | 798 |

### 19. Hostport (1 failure)
Error: `The phase of Pod pod2 is Failed which is unexpected`
| File | Line |
|------|------|
| `hostport.go` | 219 |

### 20. Scheduler predicates (1 failure)
Error: `context deadline exceeded`
| File | Line |
|------|------|
| `predicates.go` | 1102 |

### 21. Container runtime status (1 failure)
Error: expected container count mismatch
| File | Line |
|------|------|
| `runtime.go` | 115 |

### 22. Secrets volume (1 failure)
Error: `Error reading file /etc/secret-volumes/delete/data-1`
| File | Line |
|------|------|
| `secrets_volume.go` | 374 |

### 23. EmptyDir volume permissions (1 failure)
Error: file permissions mismatch
| File | Line |
|------|------|
| `output.go` | 263 |

### 24. kubectl API parse (1 failure) — FIXED (f91637a)
Error: `Failed to parse /api output : unexpected end of JSON input`
Fix: OpenAPI v2 now returns JSON instead of 406 for protobuf Accept header.
| File | Line |
|------|------|
| `kubectl.go` | 1881 |

### 25. kubectl builder (1 failure) — FIXED (f91637a)
Error: `exit status 1` — kubectl create with validation failed
Fix: Same as #24 — OpenAPI validation works now.
| File | Line |
|------|------|
| `builder.go` | 97 |

### 26. Pod lifecycle (2 failures) — FIXED (7d40469 + 7bc88bf)
Fix: Ephemeral container PATCH now uses correct content-type. Pod readiness path no longer skipped on Docker timeout.
| File | Line | Error |
|------|------|-------|
| `pods.go` | 575 | expected 2 containers, got different count |
| `pod_client.go` | 302 | ephemeral container timeout |

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 109 | 48* | 78* | 38%* |

*incomplete — e2e container killed during skip phase, only 78/441 tests ran
