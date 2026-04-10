# Conformance Failure Tracker

**Round 133** | 370/441 (83.9%) | 2026-04-10
**Round 134** | Running (deployed with 42 fixes from this session) | 2026-04-10

## What's Deployed in Round 134

All fixes through commit `f096b77` (43 commits). Key fixes:
- Node labels, SA token resync, CRD OpenAPI/watch/webhooks
- JWT kubernetes.io claims, namespace conditions, deployment template matching
- Aggregated discovery shortNames, webhook namespace selectors/TLS/subresource matching
- Scheduler extended resources, YAML parsing, JSONSchemaProps omitempty
- CR unknown fields, CRD defaults on read, empty vec/map skip
- ObjectMeta null name tolerance, EndpointAddress ip null tolerance
- CSR status PATCH metadata, root CA reconciliation, ephemeral-storage capacity
- CRD server-side apply, endpoint slice empty selectors

## Staged for Next Deploy (2 commits after round 134 build)

| Commit | Fix | Tests |
|--------|-----|-------|
| d9c9d34 | Init container intermediate status updates | init_container:440,565 |
| f096b77 | CRD LIST defaults on read | custom_resource_definition tests |

## Remaining Unfixed Issues

| # | Test | Root Cause | Difficulty |
|---|------|------------|-----------|
| 1 | `webhook.go:520,675,904,1269,1334,1400,2107` | Webhook service pod startup too slow for 30s timeout | Infrastructure — pod startup timing in Docker |
| 2 | `service.go:768,886,3459` | kube-proxy iptables rules not routing correctly | Infrastructure — kube-proxy networking |
| 3 | `proxy.go:271,503` | Service proxy can't reach backend pods | Infrastructure — ClusterIP routing |
| 4 | `deployment.go:995,1259` | Watch connection drops cause timeouts | Watch reliability |
| 5 | `rc.go:509,623` | Watch cascade / pod startup timeout | Watch reliability |
| 6 | `replica_set.go:232,560` | Watch cascade / pod startup timeout | Watch reliability |
| 7 | `statefulset.go:957` | Pod deletion timing between controller and kubelet | Controller/kubelet coordination |
| 8 | `dns_common.go:476` | Container exec runs /pause not shell | Container image/exec issue |
| 9 | `daemon_set.go:1276` | ControllerRevision Match — byte-level comparison | Go vs Rust JSON key ordering |
| 10 | `aggregator.go:359` | Extension API server deployment can't start | Infrastructure — pod networking |
| 11 | `hostport.go:219` | Host port binding in container-in-container Docker | DinD limitation |
| 12 | `field_validation.go:735` | YAML duplicate key detection | Needs custom YAML parser |
| 13 | `pod/output.go:263` | File permissions 0755 vs 0777 | Docker umask (0022) |
| 14 | `pod_resize.go:857` | cgroup changes in container-in-container | DinD limitation |
| 15 | `kubectl.go:1881` | kubectl proxy curl fails | Proxy/networking |
| 16 | `endpointslice.go:135` | Orphan cleanup rate limiter cascade | Client rate limiting |
| 17 | `endpointslicemirroring.go:129` | Mirroring timing | Controller timing |
| 18 | `resource_quota.go:282` | Pod scheduling with extended resources | Extended resource in quota |
| 19 | `service_accounts.go:667` | OIDC discovery TLS verification | Self-signed CA trust |
| 20 | `runtime.go:115` | Container restart watch cascade | Watch reliability |
| 21 | `custom_resource_definition.go:334` | CRD defaulting on read | FIXED 516922e (needs deploy) |

## All Fix Commits This Session (44)

| # | Commit | Fix |
|---|--------|-----|
| 1 | c10e449 | Node labels |
| 2 | 3136c2a | SA token resync |
| 3 | f34bd51 | CRD OpenAPI x-kubernetes booleans |
| 4 | 6edb6be | CRD webhooks |
| 5 | 323d9dc | Container restart volumes |
| 6 | db4855b | JWT kubernetes.io claims |
| 7 | c5ad02d | Namespace condition logging |
| 8 | d26e2ef | Namespace condition CAS retry |
| 9 | f7dfb20 | CRD watch support |
| 10 | c4d3fa7 | Job successPolicy ready=0 |
| 11 | eb07e78 | Pod PATCH metadata.name |
| 12 | f50d364 | Pod logs ephemeral containers |
| 13 | 8dbedb5 | EndpointAddress ip default |
| 14 | 77f4e6f | CRD type defaults |
| 15 | 176b2cd | CSR status PATCH metadata |
| 16 | af5e245 | Webhook TLS CA bundle |
| 17 | c4bda95 | Root CA ConfigMap reconciliation |
| 18 | c2a0dd8 | EndpointSlice empty selectors |
| 19 | 967b1fd | Node ephemeral-storage capacity |
| 20 | f1e00db | Webhook namespaceSelector |
| 21 | 7ae38d7 | Webhook error lowercase |
| 22 | 5b19baf | CRD PATCH server-side apply |
| 23 | b2ba5cf | Deployment template matching |
| 24 | 06d3a40 | Discovery shortNames |
| 25 | faf427c | JSONSchemaProps omitempty strings/bools |
| 26 | 0b22923 | CustomResource unknown fields |
| 27 | 182b280 | Namespace finalization timing |
| 28 | 2332cf4 | ObjectMeta null name |
| 29 | 5ff70c7 | CRD PATCH YAML parsing |
| 30 | b5e457c | EndpointAddress ip null |
| 31 | 09bcebe | Webhook resource/subresource matching |
| 32 | 4e442e8 | Scheduler extended resources |
| 33 | 0347108 | JSONSchemaProps empty vec/map skip |
| 34 | 516922e | CRD GET defaults on read |
| 35 | f096b77 | CRD LIST defaults on read |
| 36 | d9c9d34 | Init container intermediate status |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 104 | 405 | 36 | 441 | 91.8% |
| 127 | 397 | 44 | 441 | 90.0% |
| 132 | 363 | 78 | 441 | 82.3% |
| 133 | 370 | 71 | 441 | 83.9% |
| 134 | TBD | TBD | 441 | TBD |
