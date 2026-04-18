# Conformance Failure Tracker

## Known Issues

| # | Component | Issue | Severity | Notes |
|---|-----------|-------|----------|-------|
| A | job controller | Creates duplicate pods for same Job | Medium | Saw `Created pod for Job cronjob-3097/forbid-1776548915 (1/1)` twice at 5s apart. Per-resource key dedup should prevent this but the job's pod creation may race with watch event re-enqueue. Need expectations tracking (like K8s `expectations.CreationObserved`). |
| B | serviceaccount controller | `Resource already exists` errors on token secrets | Low | Race between bootstrap SA creation and controller SA creation. Controller should check existence before create, or ignore AlreadyExists. |
| C | endpoints/endpointslice | Pod readiness changes not detected until 30s resync | Medium | Controllers watch services but not pods. K8s uses cross-resource watches: pod events → look up owning service via label selector → enqueue service key. Without this, endpoint updates lag pod readiness by up to 30s. |
| D | daemonset controller | Node additions not detected until 30s resync | Medium | Controller watches daemonsets but not nodes. K8s DaemonSet controller watches nodes and enqueues affected DaemonSets on node add/remove/taint changes. |
| E | scheduler | No cross-resource watch for pod→node | Low | Scheduler uses sentinel pattern (reconcile_all). K8s scheduler has per-pod keys with node informer for preemption. Current design works but is less efficient. |
