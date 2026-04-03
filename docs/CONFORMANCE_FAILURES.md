# Conformance Issue Tracker

**Round 124** COMPLETE | 295/441 passed (66.9%)

## All 146 Failures by Category

| Category | Count | Root Cause |
|----------|-------|-----------|
| Services | 13 | Exec connection reset, kube-proxy iptables, service type changes |
| AdmissionWebhook | 13 | Webhook response parsing, webhook readiness timeout |
| CRD PublishOpenAPI | 9 | CRD creation timeout (client-go polling) |
| DNS | 7 | Exec connection reset, CoreDNS pod networking |
| StatefulSet | 6 | Scale-down readyReplicas, rolling update patch, burst scaling |
| FieldValidation | 6 | Strict/unknown field validation not implemented |
| EmptyDir | 5 | Docker Desktop virtiofs bind mount permissions |
| ReplicationController | 5 | ReplicaFailure condition, scale/lifecycle |
| Job | 5 | Indexed completion, pod failure policy, disruption |
| Networking Pods | 4 | Exec connection reset (intra-pod, node-pod) |
| ReplicaSet | 4 | Scale, lifecycle, adoption |
| Deployment | 4 | Rolling update, rollback, proportional scaling |
| SchedulerPreemption | 4 | DisruptionTarget condition, preemption path |
| CRD resources | 4 | CRD creation timeout, status sub-resource |
| AggregatedDiscovery | 3 | Discovery API format not implemented |
| ServiceAccounts | 3 | Token validation, projected volume |
| Pod InPlace Resize (guaranteed) | 3 | Exec connection reset reading cgroups |
| Pod InPlace Resize (burstable) | 2 | Exec connection reset reading cgroups |
| InitContainer | 2 | Init container lifecycle |
| Ephemeral Containers | 2 | Ephemeral container support |
| Lifecycle Hook | 2 | PostStart HTTP hook, preStop |
| Proxy | 2 | kubectl proxy, service proxy |
| EndpointSlice | 2 | Multiport endpoints, port mapping |
| Kubectl Update Demo | 2 | kubectl create validation (MIME) |
| ResourceQuota | 2 | Quota enforcement |
| SchedulerPredicates | 2 | NodeSelector, node affinity |
| Kubectl (various) | 7 | diff, describe, expose, label, patch, replace, guestbook |
| Node (various) | 5 | Variable expansion, sysctls, pods, kubelet, container runtime |
| Storage (various) | 2 | Secrets permissions, projected secret |
| Other (1 each) | 7 | Events API, HostPort, LimitRange, Certificates, DisruptionController, DaemonSet, Watchers, OrderedNamespaceDeletion, Aggregator |

## Root Cause Summary

| Root Cause | Est. Tests | Fixable? |
|-----------|-----------|---------|
| Exec WebSocket connection reset | ~20 | Yes - exec error stream handling |
| CRD creation timeout | ~13 | Yes - client-go response format |
| Webhook response parsing | ~13 | Partially fixed (ba0b26f) |
| FieldValidation not implemented | ~6 | Yes - strict field validation |
| Docker Desktop bind mount perms | ~5 | No - platform limitation |
| kubectl MIME (1 remaining) | ~2 | Mostly fixed (b3a6772) |
| AggregatedDiscovery format | ~3 | Yes - discovery API |
| Service type transitions | ~4 | Yes - service controller |
| StatefulSet rolling update | ~3 | Yes - patch handling |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | 310 | 131 | 441 | 70.3% |
| 124 | 295 | 146 | 441 | 66.9% |
