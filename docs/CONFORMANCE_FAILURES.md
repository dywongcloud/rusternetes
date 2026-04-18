# Conformance Failure Tracker

## Known Issues

| # | Component | Issue | Severity | Notes |
|---|-----------|-------|----------|-------|
| A | job controller | Creates duplicate pods for same Job | Medium | Saw `Created pod for Job cronjob-3097/forbid-1776548915 (1/1)` twice at 5s apart. Per-resource key dedup should prevent this but the job's pod creation may race with watch event re-enqueue. Need expectations tracking (like K8s `expectations.CreationObserved`). |
| B | serviceaccount controller | `Resource already exists` errors on token secrets | Low | Race between bootstrap SA creation and controller SA creation. Controller should check existence before create, or ignore AlreadyExists. |
| C | endpoints/endpointslice | Pod readiness changes not detected until 30s resync | Medium | Controllers watch services but not pods. K8s uses cross-resource watches: pod events → look up owning service via label selector → enqueue service key. Without this, endpoint updates lag pod readiness by up to 30s. |
| D | daemonset controller | Node additions not detected until 30s resync | Medium | Controller watches daemonsets but not nodes. K8s DaemonSet controller watches nodes and enqueues affected DaemonSets on node add/remove/taint changes. |
| E | scheduler | No cross-resource watch for pod→node | Low | Scheduler uses sentinel pattern (reconcile_all). K8s scheduler has per-pod keys with node informer for preemption. Current design works but is less efficient. |
| H | serviceaccount + namespace | SA controller fights namespace deletion — keeps recreating SAs in terminating namespaces | Critical | Namespace controller deletes resources, SA controller sees namespace exists and creates default SA + token secret, namespace controller retries and finds more resources. Each cycle ~1s, repeats 3-5 times per namespace deletion. Fix: SA controller must skip namespaces with deletionTimestamp set. |
| F | garbage collector | 30s scan interval is the bottleneck for pod deletion | High | Controllers set deletionTimestamp but actual deletion waits for GC's 30s scan cycle. Scaling down 3 StatefulSet pods takes 90s. In K8s, the API server handles graceful deletion directly — GC only handles orphans. Either reduce GC interval to 5s, add watch-based GC, or have controllers/kubelet delete directly after grace period. |
| G | controllers | Status writes trigger same-resource watch events causing feedback loops | High | Controllers write `.status` back to the same storage key they watch. In K8s, status goes through `/status` subresource which doesn't trigger the main informer watch. Need to either: separate status storage path, filter status-only watch events, or have controllers compare before writing. |
