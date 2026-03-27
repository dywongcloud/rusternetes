# Conformance Issue Tracker

**277 total fixes** | Round 104 in progress | 30 failures so far

## Round 104 Failures

### FIXED — pending deploy (need rebuild + redeploy)
| Fix # | Test | Error | How fixed |
|-------|------|-------|-----------|
| 270 | statefulset.go:786 — SS scaling | Readiness never persisted | Remove duplicate CAS write, re-read pod for fresh RV |
| 270 | statefulset.go — rolling updates | SS rolling update timeout | Same readiness fix |
| 270 | job.go:236 — pod failure policy | Pods never Ready | Same readiness fix |
| 270 | init_container.go:440 | Init container timeout | Same readiness fix |
| 270 | webhook.go:1133 — admission webhook | Webhook deployment not ready | Same readiness fix |
| 271 | runtime.go:115 — container exit status | RestartCount=0 | Track RestartCount in restart path |
| 272 | service.go:3304 — service status | Watch rv=1 missed ADDED event | Treat rv=1 like rv=0 |
| 272 | resource_quota.go:1152 | Watch rv=1 missed ADDED event | Same watch fix |
| 273 | lifecycle_hook.go:132 — preStop hook | Hook never executed | Stop containers before deleting from storage |
| 274 | aggregated_discovery.go:227 | CRD not in discovery | Dynamic CRD groups in aggregated discovery |
| 275 | job.go:665 — maxFailedIndexes | Job not terminated | Job controller checks maxFailedIndexes |
| 276 | downwardapi_volume.go:155 | Label update not in volume | Trailing newline in labels file |
| 277 | kubectl.go:1130 — dry-run | Dry-run persisted pod | Dry-run check in server-side apply path |

### UNFIXED — need new code
| Test | Error | Root cause | Priority |
|------|-------|------------|----------|
| watch.go:409 — configmap watchers | Watch events missed | Watch cache broadcast timing race | HIGH |
| watch — specific RV | Watch from specific RV fails | etcd history replay too slow | HIGH |
| CRD listing (custom_resource_definition.go:104) | CRD create: context deadline | CRD watch for Established condition | HIGH |
| CRD OpenAPI (crd_publish_openapi.go:366) | CRD create: context deadline | Same CRD Established condition | HIGH |
| FieldValidation CRD | CRD create: context deadline | Same CRD Established condition | HIGH |
| CRD conversion webhook | CR v1→v2 conversion | Conversion webhook not implemented | HARD |
| service NodePort→ExternalName | DNS nslookup fails | ExternalName CNAME in CoreDNS | MEDIUM |
| endpointslice — kubectl exec+curl | curl target unreachable | Service routing via kube-proxy | MEDIUM |
| emptyDir 0666 (output.go:263) | File perms 0644 not 0666 | Docker Desktop filesystem/umask | PLATFORM |
| kubectl label (BeforeEach) | Deployment not ready | Readiness (#270) or kubectl issue | LOW |
| kubectl create -f (builder.go:97) | OpenAPI protobuf parse | OpenAPI response not valid protobuf | MEDIUM |
| job — locally restarted | Job completion with restarts | Readiness (#270) or job tracking | LOW |
| statefulset.go:1092 | Image mismatch | SS rolling update image | LOW |

### Summary of pending fixes
| Fix # | Component | Description |
|-------|-----------|-------------|
| 270 | kubelet | Readiness: remove duplicate write, re-read pod for fresh RV |
| 271 | kubelet | RestartCount tracking in Running→Stopped→Restart path |
| 272 | api-server | Watch: treat rv=1 like rv=0 for initial ADDED events |
| 273 | kubelet | PreStop hooks: stop containers before deleting from storage; resolve pause IP |
| 274 | api-server | Aggregated discovery: dynamic CRD groups from storage |
| 275 | controller-manager | Job controller: maxFailedIndexes check |
| 276 | kubelet | Downward API labels/annotations trailing newline |
| 277 | api-server | Pod server-side apply respects dryRun=All |

## Progress

| Round | Pass | Fail | Total | Rate | Key changes |
|-------|------|------|-------|------|-------------|
| 97 | ~40 | ~400 | 441 | ~9% | Baseline |
| 98 | | | | 53% | Round 98 fixes |
| 101 | 245 | 196 | 441 | 56% | 76 fixes deployed |
| 103 | 46 | 30 | 76 | 60% | fsGroup, session affinity |
| 104 | ? | 30 | 441 | ~93% est | #255-269 deployed, #270-277 pending |

## Deployed Fixes

Fixes #1-269 deployed in current build. #270-277 written but not deployed.
