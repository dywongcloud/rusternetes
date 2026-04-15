# Conformance Failure Tracker

**Round 143** | Complete — 372/441 (84.4%) | 2026-04-15

## Round 143 Failures (69 total)

### Webhook — 18 failures
- `webhook.go:425,520,601,675,904,1194,1244,1269,1334,1549,1631,2032,2107(x3),2338,2465`
- Still "No route to host" or "connection refused" to ClusterIP

### CRD OpenAPI — 9 failures
- `crd_publish_openapi.go:77,170,211,267,285,318,366,400,451`

### EmptyDir/Volumes — 7 failures
- `output.go:263` (x5), `output.go:282` (x2)

### DNS — 6 failures
- `dns_common.go:476` (x6)

### Service — 5 failures
- `service.go:768,3459,4291(x3)`

### Apps — 10 failures
- `deployment.go:995,1259`
- `statefulset.go:957,1092`
- `replica_set.go:232,560`
- `rc.go:509,623`
- `daemon_set.go:1276`
- `init_container.go:233`

### Network — 3 failures
- `proxy.go:271,503`
- `hostport.go:219`

### Other — 11 failures
- `service_latency.go:145`
- `preemption.go:877`
- `resource_quota.go:290`
- `aggregator.go:359`
- `garbage_collector.go:436`
- `runtime.go:115`
- `pod_resize.go:857`
- `init_container.go:440`
- `secrets_volume.go:337`
- `pod_client.go:236`
- `pre_stop.go:153`

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 135 | 373 | 68 | 441 | 84.6% |
| 137 | ~380 | ~61 | 441 | ~86.2% |
| 141 | 368 | 73 | 441 | 83.4% |
| 142 | 372 | 69 | 441 | 84.4% |
| 143 | 372 | 69 | 441 | 84.4% |
