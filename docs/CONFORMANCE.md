# Kubernetes v1.35 Conformance

Rusternetes is a from-scratch Rust reimplementation of Kubernetes. This document tracks conformance testing progress against the official Kubernetes v1.35 e2e conformance suite.

## Conformance Test Results

We run the official Kubernetes conformance test suite (441 tests) via Sonobuoy against a Rusternetes cluster running on Docker Desktop.

| Round | Date       | Pass | Fail | Pass Rate | Notes |
|-------|------------|------|------|-----------|-------|
| 103   | 2026-03-10 | 245  | 196  | 56%       | Initial baseline |
| 104   | 2026-03-14 | 405  | 36   | 92%       | Major fix batch |
| 105   | 2026-03-17 | ~410 | ~31  | 93%       | |
| 106   | 2026-03-20 | ~416 | ~25  | 94%       | |
| 107   | 2026-03-23 | ~422 | ~19  | 96%       | Best deployed result |
| 108   | 2026-03-27 | 263  | 178  | 60%       | Regression (interaction bugs) |
| 110   | 2026-03-29 | 283  | 158  | 64%       | Fixes committed, not yet deployed |
| 116   | 2026-03-31 | 128  | 94   | 58%       | Pre-deploy, watch cancel loops |
| 117   | 2026-03-31 | 89   | 44   | 67%       | Partial run, first deploy of session fixes |
| 118   | 2026-04-01 | 299  | 142  | 68%       | Full run, all major fixes deployed |
| 119   | 2026-04-01 | —    | —    | —         | Pre-fix baseline, 16 fixes pending |
| 120   | 2026-04-01 | —    | —    | —         | Round with 16 new fixes deployed |
| 125   | 2026-04-04 | 329  | 112  | 74.6%     | New high score — 30 fixes deployed |
| 127   | 2026-04-07 | 397  | 44   | 90.0%     | Pre-regression baseline |
| 132   | 2026-04-09 | 363  | 78   | 82.3%     | First round with major fixes deployed |
| 133   | 2026-04-10 | 370  | 71   | 83.9%     | 47 fixes deployed, 18 staged |
| 135   | 2026-04-11 | 373  | 68   | 84.6%     | New high score, 57 unique failure locations |

**Current best deployed**: Round 135 at 84.6% (373/441).

**Latest status (Round 135)**: 373/441 conformance tests passing with 68 failures across 57 unique locations. Key remaining failure areas: webhook dispatch (12), CRD OpenAPI schema publishing (9, fix staged), DNS pod startup (6), service networking (6), apps controller timing (8), preemption extended resources (4, fix staged), field validation (3, fix staged). Additional fixes staged for Round 136 targeting ~17 tests.

**Total commits**: 1,314+ across 30+ rounds of iterative testing and debugging.

## Failure Categories

Based on Round 135 analysis (68 failures, 57 unique locations):

- **Webhook dispatch (~12)**: Webhook services reachable but readiness check times out. Needs deep comparison against K8s dispatch flow.
- **CRD OpenAPI (~9)**: OpenAPI handler uses typed deserialization losing nested `items` schemas. Fix staged (0188c3c).
- **Apps controllers (~8)**: Deployment, ReplicaSet, ReplicationController, StatefulSet timing/watch issues.
- **DNS (~6)**: Rate limiter timeout, pods not starting.
- **Service networking (~6)**: ClusterIP service unreachable from exec pods.
- **EmptyDir (~4)**: Stale webhook configuration blocks pod creation (webhook cascade).
- **Preemption (~4)**: Extended resources not checked in preemption. Fix staged (e1f4bd0).
- **Node lifecycle (~4)**: Lifecycle hooks, runtime, init containers, pod resize.
- **Field validation (~3)**: Unknown top-level CR fields not rejected; YAML dup format. Fix staged (a18febe).
- **Auth (~2)**: ServiceAccount mount token, OIDC discovery.
- **kubectl (~2)**: Proxy, describe.
- **Other (~8)**: DaemonSet, Job, Init container, Discovery, Namespace, ResourceQuota, HostPort, Aggregator, EndpointSlice mirroring.

Detailed tracking in `docs/CONFORMANCE_FAILURES.md`.

## API Resources Implemented

Rusternetes implements 60+ resource types across 14 API groups. All resources support full CRUD operations, watch, list with field/label selectors, and status subresources where applicable.

### Core (api/v1)

| Resource | Status | Notes |
|----------|--------|-------|
| Namespaces | Implemented | Isolation, finalizers, cascading delete |
| Pods | Implemented | Full lifecycle, init containers, probes, exec/attach/port-forward |
| Services | Implemented | ClusterIP, NodePort, LoadBalancer |
| Endpoints | Implemented | Auto-managed by endpoints controller |
| ConfigMaps | Implemented | |
| Secrets | Implemented | |
| Nodes | Implemented | Registration, status reporting, conditions |
| ServiceAccounts | Implemented | Token generation, automount |
| Events | Implemented | |
| PersistentVolumes | Implemented | Binding, reclaim policies |
| PersistentVolumeClaims | Implemented | Dynamic provisioning |
| ResourceQuotas | Implemented | |
| LimitRanges | Implemented | Default injection, constraint validation |
| ReplicationControllers | Implemented | |
| PodTemplates | Implemented | |
| ComponentStatus | Implemented | |
| Bindings | Implemented | Used by scheduler |

### Apps (apps/v1)

| Resource | Status | Notes |
|----------|--------|-------|
| Deployments | Implemented | Rolling updates, rollbacks, scale subresource |
| ReplicaSets | Implemented | Scale subresource, owner references |
| StatefulSets | Implemented | Ordered pod management, scale subresource, rolling updates |
| DaemonSets | Implemented | Node-targeted scheduling |
| ControllerRevisions | Implemented | History tracking for StatefulSets and DaemonSets |

### Batch (batch/v1)

| Resource | Status | Notes |
|----------|--------|-------|
| Jobs | Implemented | Completions, parallelism, backoff limits, indexed mode, FailIndex |
| CronJobs | Implemented | Schedule-based job creation |

### Networking (networking.k8s.io/v1)

| Resource | Status | Notes |
|----------|--------|-------|
| Ingress | Implemented | HTTP/HTTPS routing |
| IngressClass | Implemented | |
| NetworkPolicies | Implemented | Ingress/egress rules, pod/namespace selectors |

### Networking (networking.k8s.io/v1alpha1)

| Resource | Status | Notes |
|----------|--------|-------|
| IPAddresses | Implemented | |
| ServiceCIDRs | Implemented | |

### RBAC (rbac.authorization.k8s.io/v1)

| Resource | Status | Notes |
|----------|--------|-------|
| Roles | Implemented | |
| RoleBindings | Implemented | |
| ClusterRoles | Implemented | Aggregation rules |
| ClusterRoleBindings | Implemented | |

### Storage (storage.k8s.io/v1)

| Resource | Status | Notes |
|----------|--------|-------|
| StorageClasses | Implemented | |
| CSIDrivers | Implemented | |
| CSINodes | Implemented | |
| CSIStorageCapacity | Implemented | |
| VolumeAttachments | Implemented | |

### Scheduling (scheduling.k8s.io/v1)

| Resource | Status | Notes |
|----------|--------|-------|
| PriorityClasses | Implemented | Preemption support |

### Coordination (coordination.k8s.io/v1)

| Resource | Status | Notes |
|----------|--------|-------|
| Leases | Implemented | Leader election, node heartbeats |

### API Extensions (apiextensions.k8s.io/v1)

| Resource | Status | Notes |
|----------|--------|-------|
| CustomResourceDefinitions | Implemented | Validation, status/scale subresources, categories |
| Custom Resource Instances | Implemented | Full CRUD for any registered CRD |

### Admission Registration (admissionregistration.k8s.io/v1)

| Resource | Status | Notes |
|----------|--------|-------|
| ValidatingWebhookConfigurations | Implemented | |
| MutatingWebhookConfigurations | Implemented | |
| ValidatingAdmissionPolicies | Implemented | CEL expression evaluation |

### Flow Control (flowcontrol.apiserver.k8s.io/v1)

| Resource | Status | Notes |
|----------|--------|-------|
| PriorityLevelConfigurations | Implemented | |
| FlowSchemas | Implemented | |

### Certificates (certificates.k8s.io/v1)

| Resource | Status | Notes |
|----------|--------|-------|
| CertificateSigningRequests | Implemented | Approval subresource |

### Policy (policy/v1)

| Resource | Status | Notes |
|----------|--------|-------|
| PodDisruptionBudgets | Implemented | |

### Autoscaling (autoscaling/v2)

| Resource | Status | Notes |
|----------|--------|-------|
| HorizontalPodAutoscalers | Implemented | |

### Resource (resource.k8s.io/v1beta1)

| Resource | Status | Notes |
|----------|--------|-------|
| ResourceClaims | Implemented | |
| ResourceSlices | Implemented | |
| DeviceClasses | Implemented | |

### Snapshot Storage (snapshot.storage.k8s.io/v1)

| Resource | Status | Notes |
|----------|--------|-------|
| VolumeSnapshots | Implemented | |
| VolumeSnapshotClasses | Implemented | |
| VolumeSnapshotContents | Implemented | |

## Key Conformance Features

Beyond basic CRUD, these API-level features are implemented:

- **Server-Side Apply** — Field managers, conflict detection, field ownership tracking
- **Watch API** — Streaming watches with bookmarks, keep-alive, resource version semantics, WatchCache multiplexer
- **Patch formats** — Strategic Merge Patch, JSON Patch (RFC 6902), JSON Merge Patch (RFC 7386)
- **Subresources** — Status and scale subresources for all applicable resource types
- **API discovery** — Aggregated discovery, per-group resource listings, OpenAPI v2 and v3
- **Table format** — Responses formatted for kubectl's tabular output
- **Admission control** — Mutating and validating webhooks, ValidatingAdmissionPolicy with CEL
- **CRD features** — Schema validation, status subresource, scale subresource, categories
- **Pod operations** — Exec, attach, and port-forward via WebSocket
- **Dry-run** — Server-side dry-run for create, update, and patch
- **Selectors** — Field selectors and label selectors on list and watch
- **Pagination** — Limit/continue token support for large collections
- **Garbage collection** — Cascade delete via owner references, foreground and background modes
- **Finalizers** — Pre-deletion hooks with finalizer semantics
- **Authentication** — TLS/mTLS, token-based auth, service account tokens
- **Authorization** — Full RBAC evaluation
- **Pod security** — PodSecurity admission (enforce level from namespace labels)
- **In-place resize** — KEP-1287 pod resource resize without restart
- **RuntimeClass** — Overhead injection via podFixed

## Controllers

The controller manager runs 31 controllers:

- Deployment, ReplicaSet, StatefulSet, DaemonSet
- Job, CronJob
- Endpoints, EndpointSlice
- PV/PVC binding, dynamic volume provisioner
- Volume snapshot, volume expansion
- Garbage collector, TTL controller
- HPA, VPA, PDB
- Namespace lifecycle, taint eviction
- Service account token controller
- Service (ClusterIP/NodePort allocation), LoadBalancer
- Node lifecycle
- NetworkPolicy, Ingress
- CRD, CSR
- ResourceClaim

## Performance Optimizations

The following optimizations have been applied to improve throughput and reduce latency:

- **Lock-free etcd access** — etcd client uses gRPC/HTTP2 multiplexing (no mutex)
- **Watch-driven kubelet** — Reacts to pod changes via etcd watch instead of pure polling
- **Reduced etcd round-trips** — Create/update use transactions with inline GET for mod_revision
- **Single-pass selector filtering** — Field and label selectors applied in one JSON serialization pass
- **Bounded watch channels** — Prevents unbounded memory growth with slow clients
- **Release binary optimization** — LTO, single codegen unit, symbol stripping

See `docs/PERFORMANCE_PLAN.md` for the full optimization roadmap.

## Running Conformance Tests

```bash
# Build and start the cluster
export KUBELET_VOLUMES_PATH=$(pwd)/.rusternetes/volumes
docker compose build
docker compose up -d
bash scripts/bootstrap-cluster.sh

# Run the conformance suite
bash scripts/run-conformance.sh

# Monitor progress
bash scripts/conformance-progress.sh
```

E2e output is written to `/tmp/sonobuoy/results/e2e.log` inside the e2e container. To save logs:

```bash
E2E_CONTAINER=$(docker ps --filter name=e2e -q)
docker cp "$E2E_CONTAINER:/tmp/sonobuoy/results/e2e.log" /tmp/e2e-results.log
```

KUBECONFIG: `~/.kube/rusternetes-config`

## References

- [Kubernetes Conformance Requirements](https://github.com/cncf/k8s-conformance)
- [Kubernetes API Conventions](https://github.com/kubernetes/community/blob/master/contributors/devel/sig-architecture/api-conventions.md)
- [Kubernetes Conformance Testing](https://github.com/cncf/k8s-conformance/blob/master/instructions.md)
