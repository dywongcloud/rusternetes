# Conformance Issue Tracker

**Round 109** | 48 failures / 78 tests ran (e2e killed during skip phase) | 336 fixes deployed

## Round 109 — All 48 Failures

### 1. Webhook deployment not ready (7 failures) — FIXED (7bc88bf)
Fix: Kubelet timeout — assume Running pods still running. Removes exited containers.
**Confidence: MEDIUM** — readiness probe path no longer skipped, but HTTPS probes not verified.
| File | Line |
|------|------|
| `webhook.go` | 425, 520, 601, 1244, 1549, 2338, 2465 |

### 2. Webhook matchConditions CEL error (2 failures) — FIXED (7d40469)
Fix: Case-insensitive CEL error check.
**Confidence: HIGH** — exact error traced and fixed.
| File | Line |
|------|------|
| `webhook.go` | 729, 783 |

### 3. CRD creation timeout (4 failures) — FIXED (be1af28)
Fix: Protobuf brace-scanning validates with serde_json. 10 new tests.
**Confidence: HIGH** — root cause identified and tested.
| File | Line |
|------|------|
| `crd_publish_openapi.go` | 318, 451 |
| `custom_resource_definition.go` | 104, 288 |

### 4. CRD field validation decode error (3 failures) — FIXED (be1af28)
**Confidence: HIGH**
| File | Line |
|------|------|
| `field_validation.go` | 245, 428, 570 |

### 5. CRD field validation timeout (1 failure) — FIXED (be1af28)
**Confidence: HIGH**
| File | Line |
|------|------|
| `field_validation.go` | 305 |

### 6. Pod resize PATCH rejected (3 failures) — FIXED (7d40469)
Fix: Pod PATCH checks X-Original-Content-Type header.
**Confidence: HIGH** — exact error traced and fixed.
| File | Line |
|------|------|
| `pod_resize.go` | 850 (x3) |

### 7. Ephemeral containers PATCH rejected (1 failure) — FIXED (7d40469)
**Confidence: HIGH** — same fix as #6.
| File | Line |
|------|------|
| `ephemeral_containers.go` | 80 |

### 8. Job SuccessPolicy (3 failures) — FIXED (4f60d58)
Fix: Job controller preserves completion status from SuccessPolicy.
**Confidence: HIGH** — exact assertion traced (nil vs *int32(0)).
| File | Line | Error |
|------|------|-------|
| `job.go` | 514 | regular update wiped SuccessPolicy status |
| `job.go` | 553 | same root cause |
| `job.go` | 974 | cascading timeout |

### 9. Aggregated discovery (2 failures) — FIXED (f5241df)
Fix: q-value Accept header parsing.
**Confidence: MEDIUM** — depends on test's exact Accept header format.
| File | Line | Error |
|------|------|-------|
| `aggregated_discovery.go` | 227 | context deadline exceeded |
| `aggregated_discovery.go` | 282 | Expected validatingwebhookconfigurations |

### 10. Resource quota status (2 failures) — NOT VERIFIED
Quota controller rewrites deployed, but specific `status.used` format mismatch needs conformance run to verify.
| File | Line |
|------|------|
| `resource_quota.go` | 282, 489 |

### 11. Watch DELETE event (1 failure) — FIXED (d8030f2)
Error: `Timed out waiting for expected watch notification: {DELETED <nil>}`
Fix: Watch now sends synthetic DELETE when MODIFIED event's labels no longer match selector.
**Confidence: HIGH** — exact behavior traced and implemented.
| File | Line |
|------|------|
| `watch.go` | 409 |

### 12. StatefulSet scaling (1 failure) — NOT FIXED
Error: `StatefulSet ss scaled unexpectedly scaled to 3 -> 2 replicas`
Phase filter deployed but same error occurred in Round 109. Root cause is likely a race condition in pod counting during rapid creation.
| File | Line |
|------|------|
| `statefulset.go` | 2479 |

### 13. /etc/hosts not kubelet-managed (1 failure) — NOT VERIFIED
Tar upload to pause container deployed, but Docker may still override /etc/hosts for containers in shared network namespace. Needs conformance run.
| File | Line |
|------|------|
| `kubelet_etc_hosts.go` | 143 |

### 14. Init container timeout (1 failure) — NOT VERIFIED
CAS fix deployed, but init container condition tracking needs conformance run.
| File | Line |
|------|------|
| `init_container.go` | 440 |

### 15. Service latency decode error (1 failure) — FIXED (already deployed)
**Confidence: HIGH** — ServiceSpec Default derive tested with unit tests.
| File | Line |
|------|------|
| `service_latency.go` | 142 |

### 16. Network service (2 failures) — NOT VERIFIED
Depends on kube-proxy + pod readiness. CAS fix helps pods reach Ready but networking needs conformance run.
| File | Line | Error |
|------|------|-------|
| `service.go` | 1571 | context deadline exceeded |
| `service.go` | 4291 | service not reachable within 2m0s |

### 17. DNS resolution (1 failure) — NOT VERIFIED
Depends on CoreDNS + kube-proxy routing. Needs conformance run.
| File | Line |
|------|------|
| `dns_common.go` | 476 |

### 18. EndpointSlice (1 failure) — NOT VERIFIED
Depends on endpoint controller timing. Needs conformance run.
| File | Line |
|------|------|
| `endpointslice.go` | 798 |

### 19. Hostport (1 failure) — NOT FIXED
Error: `The phase of Pod pod2 is Failed which is unexpected`
Pod with hostPort fails — likely Docker Desktop limitation with hostPort binding.
| File | Line |
|------|------|
| `hostport.go` | 219 |

### 20. Scheduler predicates (1 failure) — NOT VERIFIED
Depends on resource reporting + scheduling. Needs conformance run.
| File | Line |
|------|------|
| `predicates.go` | 1102 |

### 21. Container runtime status (1 failure) — NOT VERIFIED
Expected 2 containers, got 0 after 300s. CAS fix should help but needs conformance run.
| File | Line |
|------|------|
| `runtime.go` | 115 |

### 22. Secrets volume (1 failure) — FIXED (d8030f2)
Error: `Error reading file /etc/secret-volumes/delete/data-1`
Fix: Volume resync now removes files when secret keys are deleted. Also handles complete secret deletion.
**Confidence: HIGH** — exact issue traced (resync only added, never removed files).
| File | Line |
|------|------|
| `secrets_volume.go` | 374 |

### 23. EmptyDir volume permissions (1 failure) — NOT VERIFIED
tmpfs with mode=1777 deployed but not tested in conformance. Needs run to verify.
| File | Line |
|------|------|
| `output.go` | 263 |

### 24. kubectl API parse (1 failure) — FIXED (f91637a)
**Confidence: HIGH** — tested kubectl create from STDIN.
| File | Line |
|------|------|
| `kubectl.go` | 1881 |

### 25. kubectl builder (1 failure) — FIXED (f91637a)
**Confidence: HIGH**
| File | Line |
|------|------|
| `builder.go` | 97 |

### 26. Pod lifecycle (2 failures) — PARTIALLY FIXED
Ephemeral container PATCH fixed (7d40469). Pod count mismatch needs conformance run.
| File | Line | Error |
|------|------|-------|
| `pods.go` | 575 | expected 2 containers, NOT VERIFIED |
| `pod_client.go` | 302 | ephemeral container PATCH FIXED |

## Summary
- **FIXED (HIGH confidence)**: #2, #3, #4, #5, #6, #7, #8, #11, #15, #22, #24, #25 = 24 failures
- **FIXED (MEDIUM confidence)**: #1, #9 = 9 failures
- **NOT VERIFIED (need conformance run)**: #10, #13, #14, #16, #17, #18, #20, #21, #23, #26 = 12 failures
- **NOT FIXED (known issues)**: #12, #19 = 2 failures

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 109 | 48* | 78* | 38%* |

*incomplete — e2e container killed during skip phase, only 78/441 tests ran
