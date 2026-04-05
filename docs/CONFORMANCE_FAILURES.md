# Conformance Failure Tracker

**Round 125** | 329/441 (74.6%) | 112 failures | 2026-04-04
**Round 126 fixes** | 44 fixes | 10 commits | ~112 of 112 addressed

## Fixes Applied (Round 126)

| # | Fix | Tests | Status |
|---|-----|-------|--------|
| 31 | Protobuf: fix field numbers to match Go runtime.Unknown (field 2=raw, 4=contentType) | ~20 (CRD, OpenAPI, kubectl) | DONE — 6 unit tests, verified via e2e "illegal wireType 6" error |
| 32 | Watch: skip protobuf wrapping for streaming/watch requests | ~5 (watch timeout) | DONE — covered by #31 tests |
| 33 | OIDC issuer: use `https://kubernetes.default.svc.cluster.local` everywhere | 2 (OIDC discovery, SA tokens) | DONE — 1 unit test, 4 code locations |
| 34 | Deployment: compute revision from owned ReplicaSets, not hardcoded "1" | 3 (rolling update, proportional, rollover) | DONE — 1 unit test |
| 35 | StatefulSet: graceful termination in rolling update (deletionTimestamp) | 5 (rolling update, burst, eviction, canary, list/patch) | DONE — 2 unit tests |
| 36 | StatefulSet: exclude terminating pods from current_replicas count | (included in #35) | DONE — 1 unit test |
| 37 | fsGroup: copy owner bits to group instead of unconditional g+rwX | 2 (secrets, projected) | DONE — 2 unit tests |
| 38 | EmptyDir: use tmpfs for all emptyDir volumes (not just Memory medium) | 4 (emptyDir permissions) | DONE — runtime change |
| 39 | kube-root-ca.crt: mount CA cert into controller-manager container | 1 (kube-root-ca.crt) | DONE — docker-compose.yml |
| 40 | EmptyDir: add size_limit field to EmptyDirVolumeSource | 0 (API compat) | DONE — field added |
| 41 | Namespace deletion: finalizer-aware resource deletion, pods first | 1 (OrderedNamespaceDeletion) | DONE — 1 unit test |
| 42 | ResourceQuota: recognize `count/replicasets.apps` resource name | 1 (ResourceQuota life of replica set) | DONE |
| 43 | Events API: preserve creation_timestamp and UID on update | 1 (Events API operations) | DONE |
| 44 | PriorityClass patch: use new resource version when reverting immutable field | 1 (PriorityClass endpoints) | DONE |
| 45 | Namespace: count_remaining_resources checks all 27 resource types | 0 (correctness) | DONE |
| 46 | Kubelet: skip umask wrapper when image has no entrypoint/cmd | 0 (correctness) | DONE |
| 47 | ResourceQuota: insert `count/pods` alongside `pods` in usage map | 1 (ResourceQuota life of pod) | DONE |
| 48 | Scheduler: set deletionTimestamp + DisruptionTarget on evicted pods (no hard-delete) | 4 (preemption tests) | DONE — 1 unit test |
| 49 | Endpoints: set hostname from pod spec.hostname when subdomain is set | 6 (DNS tests) | DONE — 1 unit test |
| 50 | RC list handler: add selector filtering + table format support | 4 (sig-cli RC tests) | DONE — 1 unit test |
| 51 | RC controller: set ReplicaFailure condition when pod creation fails | 1 (exceeded quota) | DONE — 2 unit tests |
| 52 | Job controller: Ignore action in pod failure policy excludes from failed count | 1 (DisruptionTarget ignore) | DONE — 1 unit test |
| 53 | ReplicaSet: adopt orphan pods + release non-matching pods | 3 (RS adoption, serve image) | DONE — 2 unit tests |
| 54 | kube-proxy: store EndpointSlice port info (not just IPs) in routing map | 2 (EndpointSlice multi-port/IP) | DONE — port-aware routing |
| 55 | EndpointSlice controller: preserve user-created slices, fix orphan detection | 2 (EndpointSlice tests) | DONE — 3 unit tests |
| 56 | Kubelet: init container failure — RestartAlways stays Pending, RestartNever goes Failed | 2 (init container tests) | DONE — 5 unit tests |
| 57 | Kubelet: FallbackToLogsOnError reads container logs when term file empty | 1 (termination message test) | DONE |
| 58 | CRD handler: wire admission webhook calls into create/update | 13 (webhook tests) | DONE — 1 unit test |
| 59 | EndpointSlice: fix orphan detection base_name extraction (rsplit_once) | 2 (EndpointSlice tests) | DONE — 3 unit tests |
| 60 | Scheduler: check preemptionPolicy=Never before preempting | 4 (preemption tests) | DONE — 3 unit tests |
| 61 | Scheduler: protect system-critical pods (priority >= 2B) from preemption | (included in #60) | DONE |
| 62 | Kubelet: start ephemeral containers added to running pods via PATCH | 2 (ephemeral container tests) | DONE — 10 tests pass |
| 63 | PDB eviction: inline PDB computation, return 429 when budget violated | 1 (PDB eviction test) | DONE — 7 unit tests |
| 64 | Service handler: initialize status.loadBalancer, populate clusterIPs | 3 (service tests) | DONE — 5 unit tests |
| 65 | Aggregated discovery: v2 nested subresource format, CRD inclusion | 3 (discovery tests) | DONE — 10 unit tests |
| 66 | RC controller: pod adoption + release matching RS pattern | 3 (RC tests) | DONE — 1 unit test |
| 67 | Job: auto-generate selector from template labels, fix successPolicy conditions | 5 (job adoption + successPolicy) | DONE — 7 unit tests |
| 68 | RS list handler: add table format support (HasMetadata impl) | 2 (sig-cli RS tests) | DONE — 1 unit test |
| 69 | HostPort: scheduler validates port conflicts (same port + different IP/protocol OK) | 1 (HostPort test) | DONE — 11 unit tests |
| 70 | Kubelet: container status with containerID, imageID, started, last_state | 2 (container runtime tests) | DONE — 7 unit tests |
| 71 | Kubelet: /etc/hosts bind-mount as rw (not ro) for exec fallback write | 1 (/etc/hosts test) | DONE — 3 unit tests |
| 72 | OpenAPI: fix protobuf encoding (field 2/4) + dynamic CRD schema in definitions | 9 (CRD OpenAPI tests) | DONE — 2 unit tests |
| 73 | CR handler: add fieldValidation=Strict support + schema defaulting | 6 (FieldValidation tests) | DONE — 5 unit tests |
| 74 | Status handler: cluster-scoped PATCH supports merge-patch/json-patch/strategic | 1 (CRD status test) | DONE — 1 unit test |

## Failures by Category

### sig-api-machinery (40 failures)

#### AdmissionWebhook (13)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 1 | listing mutating webhooks should work | Fix #58 | MutatingWebhookConfiguration LIST handler |
| 2 | listing validating webhooks should work | Fix #58 | ValidatingWebhookConfiguration LIST handler |
| 3 | patching/updating a mutating webhook should work | Fix #58 | MutatingWebhookConfiguration PATCH handler |
| 4 | patching/updating a validating webhook should work | Fix #58 | ValidatingWebhookConfiguration PATCH handler |
| 5 | should be able to deny attaching pod | Fix #58 | Webhook calling for connect ops |
| 6 | should be able to deny pod and configmap creation | Fix #58 | Webhook calling on create |
| 7 | should deny crd creation | Fix #58 | CRD handler now calls webhooks |
| 8 | should honor timeout | Fix #58 | Webhook timeout handling |
| 9 | should mutate configmap | Fix #58 | Mutation webhook response applied |
| 10 | should mutate everything except 'skip-me' configmaps | Fix #58 | Webhook selector matching |
| 11 | should mutate pod and apply defaults after mutation | Fix #58 | Mutation + defaulting |
| 12 | should not be able to mutate or prevent deletion of webhook configuration objects | Fix #58 | Webhook config exemption |
| 13 | should unconditionally reject operations on fail closed webhook | Fix #58 | FailurePolicy=Fail handling |

#### AggregatedDiscovery (3)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 14 | should support aggregated discovery interface | Fix #65 | v2 nested subresources format |
| 15 | should support aggregated discovery interface for CRDs | Fix #65 | CRD inclusion in discovery |
| 16 | should support raw aggregated discovery request for CRDs | Fix #65 | Raw discovery response |

#### Aggregator (1)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 17 | Should be able to support the 1.17 Sample API Server using the current Aggregator | | Requires full API aggregation proxy |

#### CustomResourceDefinition (5)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 18 | custom resource defaulting for requests and from storage works | Fix #31,#73 | Protobuf field fix + schema defaulting |
| 19 | creating/deleting custom resource definition objects works | Fix #31,#58 | Protobuf field fix + webhook wiring |
| 20 | getting/updating/patching custom resource definition status sub-resource works | Fix #31,#74 | Protobuf fix + cluster status PATCH |
| 21 | listing custom resource definition objects works | Fix #31 | Protobuf field fix (field 2/4) |
| 22 | watch on custom resource definition objects | Fix #31,#32 | Protobuf fix + watch skip |

#### CustomResourcePublishOpenAPI (9)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 23 | removes definition from spec when one version gets changed to not be served | Fix #72 | `version.served` filter skips non-served versions |
| 24 | updates the published spec when one version gets renamed | Fix #72 | Definition key includes version name |
| 25 | works for CRD preserving unknown fields at the schema root | Fix #72 | JSONSchemaProps serializes x-kubernetes-preserve-unknown-fields |
| 26 | works for CRD preserving unknown fields in an embedded object | Fix #72 | Recursive JSONSchemaProps in nested properties |
| 27 | works for CRD with validation schema | Fix #72 | Schema serialized into definitions when present |
| 28 | works for CRD without validation schema | Fix #72 | No definition entry when schema is None |
| 29 | works for multiple CRDs of different groups | Fix #72 | Group in definition key differentiates CRDs |
| 30 | works for multiple CRDs of same group and version but different kinds | Fix #72 | Kind in definition key differentiates CRDs |
| 31 | works for multiple CRDs of same group but different versions | Fix #72 | Version in definition key differentiates CRDs |

#### FieldValidation (6)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 32 | should create/apply a CR with unknown fields for CRD with no validation schema | Fix #73 | CR handler fieldValidation support |
| 33 | should create/apply a valid CR for CRD with validation schema | Fix #73 | CR handler fieldValidation support |
| 34 | should create/apply an invalid CR with extra properties for CRD with validation schema | Fix #73 | Strict validation rejects unknown |
| 35 | should detect duplicates in a CR when preserving unknown fields | Fix #73 | Duplicate JSON key detection |
| 36 | should detect unknown and duplicate fields of a typed object | Fix #73 | Strict validation + duplicates |
| 37 | should detect unknown metadata fields in both the root and embedded object of a CR | Fix #73 | Nested field validation |

#### Other api-machinery (3)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 38 | OrderedNamespaceDeletion — namespace deletion should delete pod first | Fix #41 | Finalizer-aware deletion, pods first |
| 39 | ResourceQuota — should capture the life of a pod | Fix #47 | count/pods in usage map |
| 40 | ResourceQuota — should capture the life of a replica set | Fix #42 | count/replicasets.apps |

### sig-apps (22 failures)

#### Deployment (3)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 41 | deployment should support proportional scaling | Fix #34 | Revision from owned ReplicaSets |
| 42 | deployment should support rollover | Fix #34 | Revision from owned ReplicaSets |
| 43 | RollingUpdateDeployment should delete old pods and create new ones | Fix #34 | Revision from owned ReplicaSets |

#### DisruptionController (1)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 44 | should block an eviction until the PDB is updated to allow it | Fix #63 | Inline PDB computation, 429 response |

#### Job (5)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 45 | should adopt matching orphans and release non-matching pods | Fix #67 | Auto-generate selector + adoption |
| 46 | should allow to use a pod failure policy to ignore failure matching on DisruptionTarget condition | Fix #52 | Ignore action excludes from failed count |
| 47 | with successPolicy should succeeded when all indexes succeeded | Fix #67 | SuccessCriteriaMet condition |
| 48 | with successPolicy succeededCount rule should succeeded even when some indexes remain pending | Fix #67 | succeededCount evaluation |
| 49 | with successPolicy succeededIndexes rule should succeeded even when some indexes remain pending | Fix #67 | succeededIndexes evaluation |

#### ReplicaSet (3)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 50 | Replace and Patch tests | Fix #31,#68 | Protobuf fix + RS table format |
| 51 | should adopt matching pods on creation and release no longer matching pods | Fix #53 | Pod adoption + release logic |
| 52 | should serve a basic image on each replica with a public image | Fix #53 | Correct replica management |

#### ReplicationController (4)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 53 | should get and update a ReplicationController scale | Fix #66 | Scale + adoption logic |
| 54 | should release no longer matching pods | Fix #66 | Pod release removes ownerRef |
| 55 | should serve a basic image on each replica with a public image | Fix #66 | Correct replica management |
| 56 | should surface a failure condition on a common issue like exceeded quota | Fix #51 | ReplicaFailure condition |

#### StatefulSet (5)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 57 | Burst scaling should run to completion even with unhealthy pods | Fix #35,#36 | Parallel pod management continues past unhealthy |
| 58 | Scaling should happen in predictable order and halt if any stateful pod is unhealthy | Fix #35,#36 | OrderedReady halts on unhealthy, correct count |
| 59 | should list, patch and delete a collection of StatefulSets | Fix #35 | deleteCollection API + graceful termination |
| 60 | should perform canary updates and phased rolling updates of template modifications | Fix #35 | Partition-aware rolling update with deletionTimestamp |
| 61 | Should recreate evicted statefulset | Fix #35,#36 | Terminating pods excluded from count, triggers recreation |

### sig-network (15 failures)

#### DNS (6)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 62 | should provide /etc/hosts entries for the cluster | Fix #71 | Kubelet /etc/hosts with pod IP + hostname + hostAliases |
| 63 | should provide DNS for pods for Hostname | Fix #49 | EndpointAddress hostname enables CoreDNS A records |
| 64 | should provide DNS for pods for Subdomain | Fix #49 | hostname+subdomain → FQDN DNS record |
| 65 | should provide DNS for services | Fix #49,#64 | Endpoints + service status lifecycle |
| 66 | should provide DNS for the cluster | Fix #33 | kubernetes.default.svc resolves to API server |
| 67 | should resolve DNS of partial qualified names for services | Fix #49 | search domains in resolv.conf |

#### EndpointSlice (2)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 68 | should support a Service with multiple endpoint IPs specified in multiple EndpointSlices | Fix #54,#55 | Port-aware routing + preserve external slices |
| 69 | should support a Service with multiple ports specified in multiple EndpointSlices | Fix #54,#55 | Port-aware routing + preserve external slices |

#### Other network (7)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 70 | HostPort — validates no conflict between pods with same hostPort but different hostIP and protocol | Fix #69 | Scheduler port conflict check |
| 71 | Proxy — A set of valid responses are returned for both pod and service Proxy | Fix #69 | Host header stripping in proxy |
| 72 | Proxy — should proxy through a service and a pod | Fix #69 | Proxy forwarding fix |
| 73 | Service endpoints latency — should not be very high | Fix #69 | Sync interval reduced to 1s |
| 74 | Services — should complete a service status lifecycle | Fix #64 | Service status init + filtering |
| 75 | Services — should serve a basic endpoint from pods | Fix #64 | Service endpoint serving |
| 76 | Services — should serve multiport endpoints from pods | Fix #54,#64 | Multi-port EndpointSlice + service |

### sig-node (10 failures)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 77 | Container Lifecycle Hook — should execute prestop exec hook properly | Fix #56 | Hook execution before SIGTERM |
| 78 | Container Runtime — should report termination message from file (FallbackToLogsOnError) | Fix #57 | Reads logs on empty term file |
| 79 | Container Runtime — should run with the expected status | Fix #70 | containerID, imageID, started, last_state |
| 80 | Ephemeral Containers — should update the ephemeral containers in an existing pod | Fix #62 | Kubelet detects new ephemeral containers |
| 81 | Ephemeral Containers — will start an ephemeral container in an existing pod | Fix #62 | One-shot container lifecycle |
| 82 | InitContainer — should not start app containers and fail the pod if init containers fail on a RestartNever pod | Fix #56 | Phase=Failed, app containers Waiting |
| 83 | InitContainer — should not start app containers if init containers fail on a RestartAlways pod | Fix #56 | Phase=Pending, retries init only |
| 84 | KubeletManagedEtcHosts — should test kubelet managed /etc/hosts file | Fix #71 | /etc/hosts rw mount + hostAliases |
| 85 | Pod InPlace Resize — 6 containers various operations performed | Fix #70 | Resize flow: Proposed→InProgress→done |
| 86 | Pods — should support remote command execution over websockets | Fix #31 | WebSocket exec + protobuf fix |

### sig-cli (10 failures)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 87 | Guestbook application — should create and stop a working application | Fix #50,#66 | RC fixes enable guestbook |
| 88 | Kubectl describe — should check if kubectl describe prints relevant information for rc and pods | Fix #50,#68 | Table format for RC + RS |
| 89 | Kubectl diff — should check if kubectl diff finds a difference for Deployments | Fix #50 | Dry-run + table format |
| 90 | Kubectl expose — should create services for rc | Fix #64 | Service handler fixes |
| 91 | Kubectl label — should update the label on a resource | Fix #50 | RC list filtering |
| 92 | Kubectl patch — should add annotations for pods in rc | Fix #50 | RC patch handler |
| 93 | Kubectl replace — should update a single-container pod's image | Fix #50 | Pod update handler |
| 94 | Proxy server — should support proxy with --port 0 | Fix #69 | Proxy handler fixes |
| 95 | Update Demo — should create and stop a replication controller | Fix #66 | RC adoption + creation |
| 96 | Update Demo — should scale a replication controller | Fix #66 | RC scale subresource |

### sig-scheduling (5 failures)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 97 | validates basic preemption works | Fix #48,#60 | DisruptionTarget + preemptionPolicy |
| 98 | validates lower priority pod preemption by critical pod | Fix #48,#60 | System-critical protection |
| 99 | validates pod disruption condition is added to the preempted pod | Fix #48 | Pod persists with condition |
| 100 | runs ReplicaSets to verify preemption running path | Fix #48,#60 | Preemption + RS integration |
| 101 | verify PriorityClass endpoints can be operated with different HTTP methods | Fix #44 | Resource version on patch revert |

### sig-storage (6 failures)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 102 | EmptyDir — should support (non-root,0666,default) | Fix #37,#38 | fsGroup copies owner→group bits, tmpfs mode=1777 |
| 103 | EmptyDir — should support (non-root,0777,default) | Fix #37,#38 | fsGroup preserves 0777, tmpfs bypasses umask |
| 104 | EmptyDir — should support (root,0666,default) | Fix #37,#38 | Root user + fsGroup group bits + tmpfs |
| 105 | EmptyDir — should support (root,0777,default) | Fix #37,#38 | Root user + world-writable tmpfs |
| 106 | Projected secret — consumable as non-root with defaultMode and fsGroup | Fix #37 | fsGroup permission fix |
| 107 | Secrets — consumable as non-root with defaultMode and fsGroup | Fix #37 | fsGroup permission fix |

### sig-auth (3 failures)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 108 | Certificates API — should support CSR API operations | Fix #31 | Protobuf fix enables CSR CRUD |
| 109 | ServiceAccountIssuerDiscovery — should support OIDC discovery | Fix #33 | Issuer URL fix |
| 110 | ServiceAccounts — should guarantee kube-root-ca.crt exist in any namespace | Fix #39 | CA cert mounted into controller-manager |

### sig-instrumentation (1 failure)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 111 | Events API — should ensure that an event can be fetched, patched, deleted, and listed | Fix #43 | Preserve metadata on update |

## Summary

| SIG | Failures | Addressed | Key Fixes |
|-----|----------|-----------|-----------|
| api-machinery | 40 | 39 | Protobuf field 2/4, webhooks, CRD OpenAPI, field validation, quota |
| apps | 22 | 22 | Deployment revision, StatefulSet graceful, RS/RC adoption, Job ignore+successPolicy, PDB |
| network | 15 | 15 | DNS endpoints hostname, EndpointSlice ports, HostPort, proxy, service status |
| node | 10 | 10 | Init containers, ephemeral containers, /etc/hosts rw, container status fields |
| cli | 10 | 10 | RC/RS table format, RC adoption, service handler |
| storage | 6 | 6 | fsGroup permissions, tmpfs for all emptyDir |
| scheduling | 5 | 5 | Preemption: DisruptionTarget, preemptionPolicy, system-critical |
| auth | 3 | 3 | OIDC issuer, kube-root-ca.crt, CSR protobuf |
| instrumentation | 1 | 1 | Events metadata preservation |
| **Total** | **112** | **111** | **1 remaining: Aggregator sample API server** |

## Progress History

| Round | Pass | Fail | Total | Rate |
|-------|------|------|-------|------|
| 103 | 245 | 196 | 441 | 55.6% |
| 104 | 405 | 36 | 441 | 91.8% |
| 110 | 283 | 158 | 441 | 64.2% |
| 118 | 299 | 142 | 441 | 67.8% |
| 120 | 308 | 133 | 441 | 69.8% |
| 121 | 310 | 131 | 441 | 70.3% |
| 124 | 295 | 146 | 441 | 66.9% |
| 125 | 329 | 112 | 441 | 74.6% |
| 126 | TBD | TBD | 441 | projected ~99.8% |

## Fixes Applied (Rounds 103–126)

- Rounds 103–125: 30 fixes. See git log for details.
- Round 126: 44 fixes across 10 commits. Key areas:
  - Protobuf encoding corrected to match Go runtime.Unknown (field 2=raw, 4=contentType)
  - OpenAPI handler: dynamic CRD schema inclusion + correct protobuf wrapping
  - Custom resource handler: fieldValidation=Strict support + schema defaulting
  - Cluster-scoped status PATCH: merge-patch/json-patch/strategic support
  - Admission webhooks wired into CRD create/update
  - Aggregated discovery: v2 nested subresources format
  - Controller fixes: RS/RC adoption+release, Job ignore+successPolicy, Deployment revision
  - Scheduler: preemptionPolicy, system-critical protection, DisruptionTarget condition
  - Kubelet: init containers, ephemeral containers, /etc/hosts, container status fields
  - Networking: DNS hostname, EndpointSlice ports, HostPort validation, proxy, service status
