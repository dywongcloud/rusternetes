# Performance Optimization Plan

This document catalogs every known architectural inefficiency in rusternetes and lays out a prioritized plan to make it a blazingly fast, resource-efficient alternative to Kubernetes.

## System-Wide Load Profile

Estimated operations per second on an idle cluster (2 nodes, 10 pods):

| Source | etcd ops/sec | Docker API calls/sec | JSON serde ops/sec |
|--------|-------------|----------------------|---------------------|
| 30+ controllers polling every 1-2s | 50-80 | 0 | 100-160 |
| 2 kubelets polling every 1s | 25-30 | 30-40 | 50-60 |
| Scheduler polling every 1s | 3-5 | 0 | 6-10 |
| Kube-proxy every 30s | ~0.1 | 0 | ~0.2 |
| Watch event processing | 5-10 | 0 | 10-20 |
| **TOTAL IDLE** | **~85-130** | **~30-40** | **~170-250** |
| **Conformance (100+ pods)** | **200-400+** | **150-300+** | **500-1000+** |

Real Kubernetes on the same workload: ~5-10 etcd ops/sec (watches, not polls).

---

## Tier 1: Architecture-Level Problems (10x impact)

### 1.1 Pure Polling Architecture — All Controllers

**Problem:** Every controller runs a loop that calls `storage.list()` on ALL resources of its type every 1-2 seconds, regardless of whether anything changed. With 30+ controllers, that's 50-80 etcd range scans per second on an idle cluster.

**Files:**
- `crates/controller-manager/src/main.rs` — spawns all controllers with `sync_interval` (default 1s)
- Every file in `crates/controller-manager/src/controllers/` — each has a `run()` loop with `list()` + `sleep()`

**What real K8s does:** SharedInformer + workqueue. One etcd watch per resource type feeds an in-memory cache. Controllers register event handlers that enqueue only changed resource keys. The reconcile loop processes only changed resources.

**Impact:** Eliminating polling would reduce controller etcd load from ~60 ops/sec to ~2-3 ops/sec (initial list + watch).

### 1.2 Kubelet Docker API Explosion

**Problem:** For each running pod per sync cycle, the kubelet makes 15-20+ Docker API calls:

| Call | Times per pod per cycle | Source |
|------|------------------------|--------|
| `is_pod_running()` → `docker.list_containers()` | 3× | `kubelet.rs:544,597,620` |
| `get_container_statuses()` → `docker.inspect_container()` per container | 5-6× (each inspects N containers) | `kubelet.rs:688,897,1065,1140,1385,1532` |
| `check_liveness()` → `docker.inspect_container()` + probe | 1× per container | `kubelet.rs:1244` |
| `has_terminated_containers()` → `docker.inspect_container()` | 1× per container | `kubelet.rs:1137` |
| Startup probe (duplicate of above) | 1× per container | `runtime.rs:3886` |

With 10 pods × 2 containers = **120+ Docker API calls per sync cycle**.

**Fix:** Cache `is_pod_running()` result (called 3× with same answer). Cache `get_container_statuses()` result (called 5-6× with same answer). Unify startup probe check (done in both `get_container_statuses` and `check_liveness`).

### 1.3 Kube-proxy Flush-and-Rebuild

**Problem:** Every 30 seconds, kube-proxy:
1. Flushes ALL iptables rules (`proxy.rs:29`)
2. Lists all services, endpoints, endpointslices from etcd (3 `storage.list()` calls)
3. Re-creates every iptables rule from scratch

Each iptables operation spawns a new OS process via `Command::new()`. With 100 services × 5 endpoints = 500+ process spawns per cycle. No diffing — always full rebuild even if nothing changed.

**Files:**
- `crates/kube-proxy/src/proxy.rs:25-98` — sync loop
- `crates/kube-proxy/src/iptables.rs:293-428` — rule application

**Fix:** Track last-known state. Diff services/endpoints against previous sync. Only apply iptables changes for what actually changed. Use `iptables-restore` for batch application instead of individual `Command::new()` per rule.

### 1.4 Scheduler Polls ALL Pods + ALL Nodes Every Second

**Problem:** Every cycle, the scheduler:
1. Lists ALL pods across all namespaces (`scheduler.rs:60`)
2. Lists ALL nodes (`scheduler.rs:102`) — even when no pods are pending
3. Lists ALL PriorityClasses (`scheduler.rs:110`)
4. For each candidate node, DRA check lists all ResourceSlices (`scheduler.rs:762`)
5. Affinity checks do O(pods × nodes) nested scans (`advanced.rs:700-744`)

**Files:**
- `crates/scheduler/src/scheduler.rs:55-153`
- `crates/scheduler/src/advanced.rs:700-744`

**Fix:** Watch-driven scheduling queue. Watch pods for `phase=Pending`, watch nodes for changes. Only list on startup. Index pods by node for affinity lookups.

---

## Tier 2: Serialization Overhead (3-5x impact)

### 2.1 `inject_resource_version()` — 2 serde ops per read

**Problem:** Every storage read (get, list item, watch event) goes through `inject_resource_version()` which parses the full JSON into `serde_json::Value`, modifies one field, re-serializes to string, then the caller parses that string into the target type `T`.

**File:** `crates/storage/src/etcd.rs:40-52`

**Cost per operation:**
- `get()`: 3 serde ops (parse→modify→serialize inside inject, then parse→T)
- `list()` per item: same 3 ops. For 1000 pods: **3000 serde operations**
- Watch event: 2 ops (inject only, no final parse)

**Fix:** Insert `,"resourceVersion":"NNN"` directly into the JSON byte buffer after the metadata `{` using a safe byte scanner. Requires fuzz testing to prove correctness. Alternative: use a custom deserializer that injects the field during deserialization.

### 2.2 API Middleware Double-Buffers Request Bodies

**Problem:** Two middleware layers independently buffer the entire request body:
1. `normalize_content_type_middleware` (`middleware.rs:102-230`): buffers up to 10MB, does speculative JSON parsing
2. `log_request_body_middleware` (`middleware.rs:233-296`): buffers the same body again with `to_bytes(body, usize::MAX)`

**Fix:** Merge into a single middleware that reads the body once and shares the bytes.

### 2.3 Watch Handler Deserialize-Reserialize Cycle

**Problem:** Each watch event in the API server does:
1. `serde_json::from_str::<T>(&value)` — deserialize JSON (already from etcd) to typed struct
2. `serde_json::to_string(&k8s_event)` — serialize the wrapper back to JSON

The value is already valid JSON from etcd. The deserialization to T and back to JSON is unnecessary when no transformation is needed.

**File:** `crates/api-server/src/handlers/watch.rs:312-410` (repeated in 4 watch implementations)

**Fix:** Construct the watch event JSON wrapper around the raw etcd JSON string without parsing it. Only deserialize when filtering/transformation is needed.

### 2.4 Filtering Serializes Every Resource

**Problem:** `apply_selectors()` calls `serde_json::to_value(resource)` inside a `.retain()` loop for both field selectors and label selectors.

**File:** `crates/api-server/src/handlers/filtering.rs:25,49`

Listing 1000 pods with both field and label selectors: **2000 to_value() calls**.

**Fix:** Match against the typed struct fields directly instead of converting to Value. Labels are already `HashMap<String, String>` on the struct.

---

## Tier 3: N+1 Query Patterns (2-3x impact)

### 3.1 Controllers Re-list Same Resources Multiple Times Per Cycle

| Controller | Resource re-listed | Times | Lines |
|------------|-------------------|-------|-------|
| DeploymentController | ReplicaSets | 3× per deployment | `deployment.rs:121,416,538` |
| ReplicaSetController | Pods | 2× per RS | `replicaset.rs:76,140` |
| ReplicationControllerController | Pods | 3× per RC | `replicationcontroller.rs:71,108,188` |
| CronJobController | Jobs | 3× per cronjob | `cronjob.rs:77,158,342` |
| StatefulSetController | Pods | 2× per SS | `statefulset.rs:65,248` |
| DaemonSetController | Pods 2×, Nodes per DS | 2× pods, N× nodes | `daemonset.rs:135,179,282` |
| EndpointSliceController | EndpointSlices | 2× per cycle | `endpointslice.rs:54,151` |

**Fix:** Fetch each resource list once at the top of `reconcile_all()` and pass it to sub-methods. For DaemonSet, list nodes once and share across all DS reconciliations.

### 3.2 Pod CREATE Admission Overhead

A single pod creation triggers 5-6 etcd round-trips:
1. `storage.list()` — LimitRanges in namespace (`pod.rs:125`)
2. `storage.list()` — LimitRanges again in `apply_limit_range()` (`admission.rs:306`)
3. `storage.get()` — Namespace for pod-security check (`pod.rs:285`)
4. `storage.list()` — ResourceQuotas (`admission.rs:176`)
5. `storage.list()` — ALL pods in namespace for usage calculation (`admission.rs:410`)
6. `storage.get()` — ServiceAccount (`admission.rs:778`)

**Fix:** Deduplicate LimitRange reads (listed twice). Cache Namespace/LimitRange lookups (change rarely). Consider tracking quota usage incrementally instead of recalculating from scratch.

---

## Tier 4: Memory & Allocation (1.5-2x impact)

### 4.1 Excessive Pod Cloning

75 `.clone()` calls on full Pod structs in `kubelet.rs` alone. Pod has 453+ fields across nested structs. Most clones modify only 1-2 fields.

Additional clone hotspots:
- `deployment.rs:104,442,512,634` — full Deployment clone 3+ times per reconcile
- `daemonset.rs:229,306` — full Pod clone into HashMap (only needs reference)
- `job.rs:106,127,570` — full Pod clone 3 times

**Fix:** Use `&Pod` references for read paths. Clone only when a modified copy needs to be written to storage.

### 4.2 Watch Cache Memory Growth

- 5000-event ring buffer per resource prefix × ~50 prefixes = potential 250MB+ history (`watch_cache.rs:15,42`)
- `extract_rv()` parses entire JSON tree just to read one integer (`watch_cache.rs:93-98`)
- Unbounded mpsc channels in watch handlers — memory leak with slow clients (`watch.rs:209,605,1879,2053`)

**Fix:** Bound watch channels. Use regex or manual byte scan for resourceVersion extraction. Add TTL or cleanup for old prefixes.

### 4.3 List-Then-Paginate

List handlers load ALL resources into `Vec<T>`, apply selectors, then paginate (`pod.rs:781-800`). Listing page 2 of 10,000 pods loads all 10,000 into memory.

**Fix:** Stream results from storage with limit/offset pushed to etcd. Apply selectors during streaming.

---

## Suggestions Evaluated and Rejected

The following were initially proposed but rejected after deep code audit:

### Kubelet: cache `is_pod_running()` / `get_container_statuses()` — REJECTED
`sync_pod()` mutates container state (start/stop/restart) between calls to these functions. For example, after `start_pod()` at line 663, `get_container_statuses()` at line 688 MUST see the newly started containers. Caching the pre-start result would report containers as not-running. Each call site depends on seeing real-time Docker state.

### Controller N+1: pass list results to sub-methods — REJECTED (most cases)
Controllers CREATE or DELETE child resources between their repeated list calls. DeploymentController creates ReplicaSets between list #1 (line 121) and list #3 (line 540) — the status update needs to see the newly created RS. ReplicaSetController creates/deletes pods between list #1 (line 76) and list #2 (line 140). The re-list is intentional: it re-counts after mutations.

### Watch handler: pass-through JSON — REJECTED
Watch handlers FILTER events using label selectors and field selectors, which require the deserialized typed object (`metadata.labels`, `metadata.namespace`). The handler also does label-change detection (Modified→Deleted when labels no longer match). Raw JSON passthrough is architecturally blocked by the need to inspect object metadata.

### Kube-proxy: diff before apply — DEFERRED (risky)
The flush-and-rebuild pattern is self-healing: process restarts, failed iptables commands, and partial state all recover on the next cycle. A diff approach would need to handle stale in-memory state after restart, partial failures, and deleted services. The complexity/risk outweighs the benefit given kube-proxy only syncs every 30 seconds.

---

## Execution Order (Revised After Audit)

### Phase 1: Proven Safe, High Impact

1. **Remove `log_request_body_middleware`** — purely diagnostic, gated by debug log level (which is off in production). Buffers entire request body a second time for no benefit at info level. Safe to remove entirely.
2. **Deduplicate LimitRange reads** in pod admission — listed twice (pod.rs:125 and admission.rs:306). Pure waste, no correctness concern.
3. **Filtering: struct-level matching** — `apply_selectors()` calls `to_value()` per resource. Labels are already `HashMap<String, String>` on the struct. Match directly instead of serializing to Value. No correctness concern — same filtering logic, just avoids the intermediate serialization.
4. **Bound watch channels** — replace `unbounded_channel()` with bounded channel + backpressure (`watch.rs:209,605`). Prevents memory leak with slow clients. Standard practice, no behavioral change for well-behaved clients.
5. **Watch cache: targeted RV extraction** — replace full JSON parse in `extract_rv()` (`watch_cache.rs:93-98`) with a simple string search for `"resourceVersion":"NNN"`. Safe because the format is controlled by our own `inject_resource_version()`.

### Phase 2: Moderate Risk, Big Wins

6. **Scheduler: watch-driven queue** — watch pods for `phase=Pending` instead of listing ALL pods every second. List nodes only when a pending pod exists. Significant latency reduction.
7. **Reduce Pod cloning in kubelet** — many clones are in read-only paths where `&Pod` would suffice. Must audit each clone individually; some are needed for storage writes.
8. **Cache admission lookups** — namespace and LimitRange data change rarely. Cache with short TTL (e.g., 5 seconds). Must invalidate on write.

### Phase 3: Highest Impact, Highest Risk

9. **SharedInformer for controllers** — watch-driven cache + workqueue. This is the single biggest possible win (eliminates 30+ list calls/sec) but also the most complex. Requires careful cache coherency, watch reconnection handling, and extensive integration testing. The prior CachedStorage attempt was reverted for exactly these reasons.
10. **Streaming pagination** — push limit/offset to storage layer instead of loading all resources into memory.
11. **Fast resourceVersion injection** — byte-level insertion instead of full JSON parse. Requires fuzz testing to prove correctness across all resource types.

---

## Changes Already Implemented

The following optimizations have already been applied:

- **Removed global mutex** on etcd client — concurrent gRPC access (`storage/src/etcd.rs`)
- **Eliminated redundant GET after PUT/CREATE** — mod_revision from txn response (`storage/src/etcd.rs`)
- **Safe `inject_resource_version`** — single parse→modify→reserialize (was double) (`storage/src/etcd.rs`)
- **Watch-driven kubelet** — hybrid watch+poll, immediate reaction to pod changes (`kubelet/src/kubelet.rs`)
- **Eliminated double pod list** in kubelet sync_loop (`kubelet/src/kubelet.rs`)
- **Release build profile** — LTO, codegen-units=1, opt-level=3, strip symbols (`Cargo.toml`)
- **Reduced log levels** — debug→info in docker-compose (`docker-compose.yml`)
- **etcd tuning** — 1m compaction, higher quota (`docker-compose.yml`)
- **Faster scheduler polling** — 2s→1s default (`scheduler/src/main.rs`)
- **Removed Dockerfile build throttle** — CARGO_BUILD_JOBS=2 removed (`Dockerfile.*`)
- **Made NamespaceController/NetworkPolicyController generic** over Storage trait
