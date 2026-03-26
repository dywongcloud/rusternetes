# Conformance Issue Tracker

**Round 97**: 39 FAIL, 0 PASS | **174 fixes** (many NOT deployed in this round)

## ROOT CAUSE: List resourceVersion uses timestamps (fix #174)

List responses used SystemTime::now() as resourceVersion (e.g., 1774536472).
Watch events use etcd mod_revision (e.g., 2883027). When clients use the list
RV to start a watch, the revision spaces don't match and watches fail.

**Fix #174**: List::new() now extracts max RV from items' actual resourceVersions.

## Fixes NOT yet deployed (since round 97 build)

| # | Fix | Impact |
|---|-----|--------|
| 169 | generation=1, ClusterIP alloc, SA token, PodScheduled | 5+ tests |
| 170 | resourceVersion in watch event values | 12+ tests |
| 171 | Endpoints single subset | 1 test |
| 172 | Ensure metadata exists for resourceVersion | 1 test (DRA) |
| 173 | Remove duplicate SA token route (panic fix) | startup crash |
| 174 | **CRITICAL** List RV from items, not timestamps | ALL tests |
| 175 | Immutable returns 403 Forbidden not 400 | 2 tests |

## Investigated but not yet fixed
- core_events.go:135 — Event timestamp microseconds (pipeline normalizes but Go client still fails)
- garbage_collector.go:436 — Orphan propagation needs GC controller
- aggregated_discovery.go:282 — Resource format mismatch (responseKind.group field)

## Round 97 Failures (39 total)

### Watch/timeout (15) — should be fixed by #170 + #174
statefulset.go:786,:1092, rc.go:173,:442,:717, job.go:144,:623,:755,:1251,
deployment.go:238, watch.go:454, proxy.go:271,:503, controller_revision.go:156,
runtimeclass.go:153

### Protobuf/CRD (4) — known limitation
crd_publish_openapi.go:244,:285, builder.go:97 (×2)

### Webhook/aggregator (3)
webhook.go:837,:1194,:1244

### Specific bugs with fixes pending
- service.go:1483 — NodePort/ClusterIP alloc (**FIX #169**)
- service.go:251 — affinity
- service_accounts.go:132,:792 — SA token
- core_events.go:135 — datetime format
- output.go:263 — configmap subpath
- runtime.go:169 — termination message
- runtimeclass.go:297 — runtime class
- configmap_volume.go:547 — immutable 403 (**FIX #175**)
- aggregated_discovery.go:282 — API parse error
- resource_quota.go:102,:209 — quota
- garbage_collector.go:436 — 100 pods
- kubectl.go:1130,:1881 — kubectl
- service_cidrs.go:255 — IPAddress
- pod_resize.go:857 — resize
- validatingadmissionpolicy.go:568 — VAP
