# Conformance Issue Tracker

**Round 110** | IN PROGRESS | 336 fixes deployed

## Round 110 Live Failures (13 failures / 22 tests — 59% fail rate)

| # | File | Count | Error |
|---|------|-------|-------|
| 1 | `statefulset.go:2479` | 1 | `scaled 3 -> 2 replicas` — ss-0 readiness probe slow, OrderedReady can't scale |
| 2 | `crd_publish_openapi.go` | 1 | CRD created but Established watch times out at 30s |
| 3 | `custom_resource_definition.go` | 2 | `creating CRD: context deadline exceeded` — same root cause as #2 |
| 4 | `builder.go:97` | 2 | `exit status 1` — kubectl create/apply YAML validation |
| 5 | `expansion.go:419` | 1 | pod readiness timeout (127s) |
| 6 | `runtime.go:115` | 1 | container status timeout (300s) |
| 7 | `daemon_set.go:473` | 1 | DaemonSet pod startup timeout |
| 8 | `proxy.go` | 1 | proxy test failure |
| 9 | `pod_client.go` | 1 | ephemeral container issue |
| 10 | `service_accounts.go` | 1 | SA token issue |
| 11 | `resource_quota.go` | 1 | quota status mismatch |

## Key Findings

### Pods DO become Ready quickly (2 seconds!)
Pods without readiness probes reach Ready=True within 2 seconds of creation. The CAS fix and kubelet improvements ARE working. Example from logs: pod created at 16:24:31, Ready=True at 16:24:33.

### StatefulSet bottleneck: readiness probe latency
The SS test uses `readinessProbe: httpGet /localhost.crt port 80`. With OrderedReady policy, ss-1 can't be created until ss-0 is Ready. The kubelet's sequential sync means the readiness probe may not be checked every second — Docker API latency causes sync cycles to take 5-10 seconds with many pods.

### CRD timeout: Established watch not receiving MODIFIED event
CRDs are created successfully (logs confirm). But the client watching for Established condition times out at 30s. The background status update (50ms/200ms/1000ms retry) should generate a MODIFIED event, but the watch may not be subscribed in time.

### kubectl builder: YAML validation path
The `exit status 1` errors come from kubectl applying YAML files. Our OpenAPI v3 spec doesn't define all resource types, so kubectl falls back to v2 validation which may also fail for some resources.

## Progress
| Round | Fail | Total | Rate |
|-------|------|-------|------|
| 107 | 19 | ~430 | ~96% |
| 108 | 178 | 441 | 60% |
| 109 | 48* | 78* | 38%* |
| 110 | 13 | 22 | 59% (in progress) |

*Round 109 incomplete
