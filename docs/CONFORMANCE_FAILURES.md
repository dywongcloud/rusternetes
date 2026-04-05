# Conformance Failure Tracker

**Round 125** | 329/441 (74.6%) | 112 failures | 2026-04-04

## Fixes in Progress (Round 126)

| # | Fix | Tests | Status |
|---|-----|-------|--------|
| 31 | Protobuf: fix field numbers (3/5 not 2/4) + wrap response for any Accept protobuf | ~31 (CRD, FieldValidation, OpenAPI, kubectl) | DONE — 6 unit tests pass |
| 32 | Watch: skip protobuf wrapping for streaming/watch requests | ~5 (watch timeout) | DONE — covered by #31 tests |
| 33 | OIDC issuer: use `https://kubernetes.default.svc.cluster.local` everywhere | 2 (OIDC discovery, SA tokens) | DONE — 1 unit test, updated in 4 locations (kubelet x3, authentication handler x1), discovery endpoint already correct |
| 34 | Deployment: compute revision from owned ReplicaSets, not hardcoded "1" | 3 (rolling update, proportional, rollover) | DONE — 1 unit test |
| 35 | StatefulSet: graceful termination in rolling update (deletionTimestamp, not direct delete) | 5 (rolling update, burst, eviction, canary, list/patch) | DONE — 2 unit tests |
| 36 | StatefulSet: exclude terminating pods from current_replicas count | (included in #35) | DONE — 1 unit test |
| 37 | fsGroup: copy owner bits to group instead of unconditional g+rwX | 2 (secrets, projected) | DONE — 2 unit tests |
| 38 | EmptyDir: use tmpfs for all emptyDir volumes (not just Memory medium) | 4 (emptyDir permissions) | DONE — runtime change, no separate test |
| 39 | kube-root-ca.crt: mount CA cert into controller-manager container | 1 (kube-root-ca.crt) | DONE — docker-compose.yml volume mount added |
| 40 | EmptyDir: add size_limit field to EmptyDirVolumeSource | 0 (API compat) | DONE — field added to struct |
| 41 | Namespace deletion: finalizer-aware resource deletion | 1 (OrderedNamespaceDeletion) | DONE — 1 unit test, pods deleted first with grace period |
| 42 | ResourceQuota: recognize `count/replicasets.apps` resource name | 1 (ResourceQuota life of replica set) | DONE — added alongside existing `count/replicasets` |
| 43 | Events API: preserve creation_timestamp and UID on update | 1 (Events API operations) | DONE — event handler preserves existing metadata |
| 44 | PriorityClass patch: use new resource version when reverting immutable field | 1 (PriorityClass endpoints) | DONE — prevents stale resource version error |
| 45 | Namespace: count_remaining_resources checks all resource types (not just 4) | 0 (correctness) | DONE — matches deletion resource_types list |
| 46 | Kubelet: skip umask wrapper when image has no entrypoint/cmd | 0 (correctness) | DONE — prevents empty `exec` crash |
| 47 | ResourceQuota: insert `count/pods` alongside `pods` in usage map | 1 (ResourceQuota life of pod) | DONE — matches pattern used by all other resource types |
| 48 | Scheduler: don't hard-delete evicted pods — set deletionTimestamp + DisruptionTarget only | 4 (preemption tests) | DONE — 1 unit test |
| 49 | Endpoints: set hostname from pod spec when subdomain is set | 6 (DNS tests) | DONE — 1 unit test |
| 50 | RC list handler: add selector filtering + table format support | 4 (sig-cli RC tests) | DONE — 1 unit test |
| 51 | RC controller: set ReplicaFailure condition when pod creation fails | 1 (exceeded quota) | DONE — 1 unit test |
| 52 | Job controller: Ignore action in pod failure policy should not count toward failed | 1 (DisruptionTarget ignore) | DONE — 1 unit test |

**Projected impact**: ~57 of 112 failures addressed

## Failures by Category

### sig-api-machinery (40 failures)

#### AdmissionWebhook (13)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 1 | listing mutating webhooks should work | | |
| 2 | listing validating webhooks should work | | |
| 3 | patching/updating a mutating webhook should work | | |
| 4 | patching/updating a validating webhook should work | | |
| 5 | should be able to deny attaching pod | | |
| 6 | should be able to deny pod and configmap creation | | |
| 7 | should deny crd creation | | |
| 8 | should honor timeout | | |
| 9 | should mutate configmap | | |
| 10 | should mutate everything except 'skip-me' configmaps | | |
| 11 | should mutate pod and apply defaults after mutation | | |
| 12 | should not be able to mutate or prevent deletion of webhook configuration objects | | |
| 13 | should unconditionally reject operations on fail closed webhook | | |

#### AggregatedDiscovery (3)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 14 | should support aggregated discovery interface | | |
| 15 | should support aggregated discovery interface for CRDs | | |
| 16 | should support raw aggregated discovery request for CRDs | | |

#### Aggregator (1)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 17 | Should be able to support the 1.17 Sample API Server using the current Aggregator | | |

#### CustomResourceDefinition (5)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 18 | custom resource defaulting for requests and from storage works | Fix #31 | Protobuf fix should resolve |
| 19 | creating/deleting custom resource definition objects works | Fix #31 | Protobuf fix should resolve |
| 20 | getting/updating/patching custom resource definition status sub-resource works | Fix #31 | Protobuf fix should resolve |
| 21 | listing custom resource definition objects works | Fix #31 | Protobuf fix should resolve |
| 22 | watch on custom resource definition objects | Fix #31,#32 | Protobuf + watch fix |

#### CustomResourcePublishOpenAPI (9)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 23 | removes definition from spec when one version gets changed to not be served | Fix #31 | Protobuf fix should resolve |
| 24 | updates the published spec when one version gets renamed | Fix #31 | Protobuf fix should resolve |
| 25 | works for CRD preserving unknown fields at the schema root | Fix #31 | Protobuf fix should resolve |
| 26 | works for CRD preserving unknown fields in an embedded object | Fix #31 | Protobuf fix should resolve |
| 27 | works for CRD with validation schema | Fix #31 | Protobuf fix should resolve |
| 28 | works for CRD without validation schema | Fix #31 | Protobuf fix should resolve |
| 29 | works for multiple CRDs of different groups | Fix #31 | Protobuf fix should resolve |
| 30 | works for multiple CRDs of same group and version but different kinds | Fix #31 | Protobuf fix should resolve |
| 31 | works for multiple CRDs of same group but different versions | Fix #31 | Protobuf fix should resolve |

#### FieldValidation (6)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 32 | should create/apply a CR with unknown fields for CRD with no validation schema | Fix #31 | Protobuf fix should resolve |
| 33 | should create/apply a valid CR for CRD with validation schema | Fix #31 | Protobuf fix should resolve |
| 34 | should create/apply an invalid CR with extra properties for CRD with validation schema | Fix #31 | Protobuf fix should resolve |
| 35 | should detect duplicates in a CR when preserving unknown fields | Fix #31 | Protobuf fix should resolve |
| 36 | should detect unknown and duplicate fields of a typed object | Fix #31 | Protobuf fix should resolve |
| 37 | should detect unknown metadata fields in both the root and embedded object of a CR | Fix #31 | Protobuf fix should resolve |

#### Other api-machinery (3)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 38 | OrderedNamespaceDeletion — namespace deletion should delete pod first | Fix #41 | Finalizer-aware deletion, pods first |
| 39 | ResourceQuota — should capture the life of a pod | Fix #47 | count/pods missing from usage map |
| 40 | ResourceQuota — should capture the life of a replica set | Fix #42 | count/replicasets.apps recognition |

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
| 44 | should block an eviction until the PDB is updated to allow it | | PDB eviction logic needed |

#### Job (4)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 45 | should adopt matching orphans and release non-matching pods | | |
| 46 | should allow to use a pod failure policy to ignore failure matching on DisruptionTarget condition | Fix #52 | Ignore action excludes from failed count |
| 47 | with successPolicy should succeeded when all indexes succeeded | | |
| 48 | with successPolicy succeededCount rule should succeeded even when some indexes remain pending | | |
| 49 | with successPolicy succeededIndexes rule should succeeded even when some indexes remain pending | | |

#### ReplicaSet (3)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 50 | Replace and Patch tests | | |
| 51 | should adopt matching pods on creation and release no longer matching pods | | |
| 52 | should serve a basic image on each replica with a public image | | |

#### ReplicationController (4)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 53 | should get and update a ReplicationController scale | | |
| 54 | should release no longer matching pods | | |
| 55 | should serve a basic image on each replica with a public image | | |
| 56 | should surface a failure condition on a common issue like exceeded quota | Fix #51 | ReplicaFailure condition on quota error |

#### StatefulSet (5)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 57 | Burst scaling should run to completion even with unhealthy pods | Fix #35,#36 | Graceful termination + replicas count |
| 58 | Scaling should happen in predictable order and halt if any stateful pod is unhealthy | Fix #35,#36 | Graceful termination + replicas count |
| 59 | should list, patch and delete a collection of StatefulSets | Fix #35,#36 | Graceful termination + replicas count |
| 60 | should perform canary updates and phased rolling updates of template modifications | Fix #35,#36 | Graceful termination + replicas count |
| 61 | Should recreate evicted statefulset | Fix #35,#36 | Graceful termination + replicas count |

### sig-network (15 failures)

#### DNS (6)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 62 | should provide /etc/hosts entries for the cluster | Fix #49 | Endpoints hostname from pod spec |
| 63 | should provide DNS for pods for Hostname | Fix #49 | Endpoints hostname from pod spec |
| 64 | should provide DNS for pods for Subdomain | Fix #49 | Endpoints hostname from pod spec |
| 65 | should provide DNS for services | Fix #49 | Endpoints hostname from pod spec |
| 66 | should provide DNS for the cluster | Fix #49 | Endpoints hostname from pod spec |
| 67 | should resolve DNS of partial qualified names for services | Fix #49 | Endpoints hostname from pod spec |

#### EndpointSlice (2)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 68 | should support a Service with multiple endpoint IPs specified in multiple EndpointSlices | | |
| 69 | should support a Service with multiple ports specified in multiple EndpointSlices | | |

#### Other network (7)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 70 | HostPort — validates no conflict between pods with same hostPort but different hostIP and protocol | | |
| 71 | Proxy — A set of valid responses are returned for both pod and service Proxy | | |
| 72 | Proxy — should proxy through a service and a pod | | |
| 73 | Service endpoints latency — should not be very high | | |
| 74 | Services — should complete a service status lifecycle | | |
| 75 | Services — should serve a basic endpoint from pods | | |
| 76 | Services — should serve multiport endpoints from pods | | |

### sig-node (10 failures)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 77 | Container Lifecycle Hook — should execute prestop exec hook properly | | |
| 78 | Container Runtime — should report termination message from file (FallbackToLogsOnError) | | |
| 79 | Container Runtime — should run with the expected status | | |
| 80 | Ephemeral Containers — should update the ephemeral containers in an existing pod | | |
| 81 | Ephemeral Containers — will start an ephemeral container in an existing pod | | |
| 82 | InitContainer — should not start app containers and fail the pod if init containers fail on a RestartNever pod | | |
| 83 | InitContainer — should not start app containers if init containers fail on a RestartAlways pod | | |
| 84 | KubeletManagedEtcHosts — should test kubelet managed /etc/hosts file | | |
| 85 | Pod InPlace Resize — 6 containers various operations performed | | |
| 86 | Pods — should support remote command execution over websockets | | |

### sig-cli (10 failures)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 87 | Guestbook application — should create and stop a working application | | |
| 88 | Kubectl describe — should check if kubectl describe prints relevant information for rc and pods | | |
| 89 | Kubectl diff — should check if kubectl diff finds a difference for Deployments | | |
| 90 | Kubectl expose — should create services for rc | | |
| 91 | Kubectl label — should update the label on a resource | | |
| 92 | Kubectl patch — should add annotations for pods in rc | | |
| 93 | Kubectl replace — should update a single-container pod's image | | |
| 94 | Proxy server — should support proxy with --port 0 | | |
| 95 | Update Demo — should create and stop a replication controller | | |
| 96 | Update Demo — should scale a replication controller | | |

### sig-scheduling (5 failures)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 97 | validates basic preemption works | Fix #48 | Don't hard-delete evicted pods |
| 98 | validates lower priority pod preemption by critical pod | Fix #48 | Don't hard-delete evicted pods |
| 99 | validates pod disruption condition is added to the preempted pod | Fix #48 | Don't hard-delete evicted pods |
| 100 | runs ReplicaSets to verify preemption running path | Fix #48 | Don't hard-delete evicted pods |
| 101 | verify PriorityClass endpoints can be operated with different HTTP methods | Fix #44 | Resource version fix on patch revert |

### sig-storage (6 failures)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 102 | EmptyDir — should support (non-root,0666,default) | Fix #37,#38 | fsGroup + tmpfs for all emptyDir |
| 103 | EmptyDir — should support (non-root,0777,default) | Fix #37,#38 | fsGroup + tmpfs for all emptyDir |
| 104 | EmptyDir — should support (root,0666,default) | Fix #37,#38 | fsGroup + tmpfs for all emptyDir |
| 105 | EmptyDir — should support (root,0777,default) | Fix #37,#38 | fsGroup + tmpfs for all emptyDir |
| 106 | Projected secret — consumable as non-root with defaultMode and fsGroup | Fix #37 | fsGroup permission fix |
| 107 | Secrets — consumable as non-root with defaultMode and fsGroup | Fix #37 | fsGroup permission fix |

### sig-auth (3 failures)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 108 | Certificates API — should support CSR API operations | | |
| 109 | ServiceAccountIssuerDiscovery — should support OIDC discovery | Fix #33 | Issuer URL fix |
| 110 | ServiceAccounts — should guarantee kube-root-ca.crt exist in any namespace | Fix #39 | CA cert mounted into controller-manager |

### sig-instrumentation (1 failure)

| # | Test | Status | Notes |
|---|------|--------|-------|
| 111 | Events API — should ensure that an event can be fetched, patched, deleted, and listed | Fix #43 | Preserve metadata on update |

## Summary

| SIG | Failures | Addressed |
|-----|----------|-----------|
| api-machinery | 40 | ~23 (protobuf, namespace, quota) |
| apps | 22 | ~8 (deployment, statefulset) |
| network | 15 | 0 |
| node | 10 | 0 |
| cli | 10 | 0 |
| storage | 6 | ~6 (fsGroup, emptyDir) |
| scheduling | 5 | ~1 (priorityclass) |
| auth | 3 | ~2 (OIDC, kube-root-ca.crt) |
| instrumentation | 1 | ~1 (events) |
| **Total** | **112** | **~40** |

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

## Fixes Applied (Rounds 103–125)

30 fixes applied across rounds 103–125. See git log for details.
