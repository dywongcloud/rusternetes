# Rusternetes Architecture

Kubernetes reimplemented in Rust. 9 crates, 161,000+ lines of Rust, 929 tests.

## System Architecture

```
                              +-----------+
                              |  kubectl  |
                              +-----+-----+
                                    |
                                    v
+-------------------------------------------------------------------+
|                         Control Plane                              |
|                                                                    |
|  +-------------------------+        +---------------------------+  |
|  |    API Server (HTTPS)   |<-------| Controller Manager        |  |
|  |                         |        |   31 reconciliation loops  |  |
|  |  - Axum + TLS/mTLS     |        +---------------------------+  |
|  |  - RBAC + Webhooks     |                                       |
|  |  - Watch API (SSE)     |        +---------------------------+  |
|  |  - Server-Side Apply   |<-------| Scheduler                 |  |
|  |  - 75 handler files    |        |   Filter/Score plugins     |  |
|  |  - OpenAPI discovery   |        +---------------------------+  |
|  +-----------+-------------+                                      |
|              |                                                     |
|  +-----------v-------------+                                      |
|  |     etcd Storage        |                                      |
|  |  /registry/{type}/...   |                                      |
|  +-------------------------+                                      |
+-------------------------------------------------------------------+
        |                              |
        | Watch pods                   | Watch services
        v                              v
+-------------------+          +-------------------+
| Kubelet (node-1)  |          | Kube-proxy        |
| Kubelet (node-2)  |          |   iptables rules  |
|                   |          |   host network     |
| - bollard/Docker  |          +-------------------+
| - pause container |
| - probes          |          +-------------------+
| - volumes         |          | CoreDNS           |
| - CAS status      |          |   10.96.0.10      |
+-------------------+          +-------------------+
```

All components communicate exclusively through the API server. Only the API server
accesses etcd directly.

## Project Structure

```
rusternetes/
  crates/
    common/                # Shared types, 36 resource files in src/resources/
    storage/               # Storage trait, etcd + memory backends
    api-server/            # Axum HTTPS API, 75 handler files
    controller-manager/    # 31 controllers in src/controllers/
    scheduler/             # Filter/Score plugin architecture
    kubelet/               # Node agent, Docker via bollard
    kube-proxy/            # iptables service routing
    kubectl/               # CLI tool
    cloud-providers/       # AWS/GCP/Azure integrations
```

## Crates

### 1. common

Shared types and resource definitions used by every other crate.

**36 resource type files** in `src/resources/`, including:
Pod, Deployment, ReplicaSet, StatefulSet, DaemonSet, Job, CronJob, Service,
Endpoints, EndpointSlice, ConfigMap, Secret, ServiceAccount, Node, Namespace,
PersistentVolume, PersistentVolumeClaim, StorageClass, Ingress, NetworkPolicy,
Role, RoleBinding, ClusterRole, ClusterRoleBinding, CustomResourceDefinition,
HorizontalPodAutoscaler, VerticalPodAutoscaler, PodDisruptionBudget,
LimitRange, ResourceQuota, RuntimeClass, PriorityClass, Lease,
ValidatingAdmissionPolicy, FlowControl, Certificates, CSI, DRA, IPAddress,
ServiceCIDR, ControllerRevision, and more.

**Core types:** TypeMeta, ObjectMeta, LabelSelector, ResourceRequirements,
Affinity, Toleration, Taint.

**Error handling:** Unified `Error` enum mapping to Kubernetes StatusReason.
Each variant maps to the correct HTTP status code (404, 409, 422, etc.) via
Axum's `IntoResponse`.

**Serialization conventions:**
- `#[serde(rename_all = "camelCase")]` on all structs
- `#[serde(skip_serializing_if = "Option::is_none")]` on optional fields
- `#[serde(flatten)]` for TypeMeta
- K8s-style abbreviations: `podIP`, `hostIP`, `containerID`

### 2. api-server

Axum-based HTTPS REST API server. Router defined in `src/router.rs` (~2,135 lines).
75 handler files in `src/handlers/`, one per resource type.

**API surface:**
- core/v1: pods, services, configmaps, secrets, serviceaccounts, nodes, namespaces,
  endpoints, events, persistentvolumes, persistentvolumeclaims, limitranges,
  resourcequotas, replicationcontrollers
- apps/v1: deployments, replicasets, statefulsets, daemonsets, controllerrevisions
- batch/v1: jobs, cronjobs
- rbac.authorization.k8s.io/v1: roles, rolebindings, clusterroles, clusterrolebindings
- networking.k8s.io/v1: ingresses, networkpolicies
- storage.k8s.io/v1: storageclasses, csidrivers, csinodes, csistoragecapacities
- policy/v1: poddisruptionbudgets
- autoscaling/v1-v2: horizontalpodautoscalers
- apiextensions.k8s.io/v1: customresourcedefinitions
- certificates.k8s.io/v1: certificatesigningrequests
- coordination.k8s.io/v1: leases
- discovery.k8s.io/v1: endpointslices
- scheduling.k8s.io/v1: priorityclasses
- flowcontrol.apiserver.k8s.io/v1: flowschemas, prioritylevelconfigurations
- resource.k8s.io/v1alpha3: resourceclaims
- admissionregistration.k8s.io/v1: mutatingwebhookconfigurations, validatingwebhookconfigurations

**Key features:**
- Watch API via Server-Sent Events (SSE). `rv=0` sends initial ADDED events;
  `rv>0` watches from that etcd revision.
- RBAC authorization with JWT-based service account authentication.
- Admission webhooks (mutating and validating).
- Server-Side Apply with field manager tracking and conflict detection.
- PATCH support: JSON Patch, JSON Merge Patch, Strategic Merge Patch.
- OpenAPI v2/v3 discovery endpoints.
- Aggregated API discovery.
- Table format responses for kubectl compatibility.
- Subresource endpoints: `/status`, `/scale`, `/log`, `/exec`, `/attach`,
  `/portforward`, `/eviction`, `/token`, `/finalize`, `/binding`.

**State** (`src/state.rs`): Holds storage backend, auth configuration, ClusterIP
allocator, webhook manager, and watch cache.

### 3. storage

Defines the `Storage` trait in `src/lib.rs` with two backends:

- **EtcdStorage** (`src/etcd.rs`): Production backend. Resource versions map to
  etcd `mod_revision`. Watch support via etcd watch API.
- **MemoryStorage** (`src/memory.rs`): In-memory backend for unit tests.

**Key pattern:** `/registry/{resource_type}/{namespace}/{name}` for namespaced
resources; `/registry/{resource_type}/{name}` for cluster-scoped resources.

**Optimistic concurrency:** All updates use compare-and-swap (CAS) based on
etcd `mod_revision`. Clients must send the current `resourceVersion` to update;
conflicts return HTTP 409.

### 4. controller-manager

31 controllers, each running as a concurrent tokio task with an independent
reconciliation loop.

**Controller pattern:**
```rust
pub struct FooController<S: Storage> {
    storage: Arc<S>,
    interval: Duration,
}
// run() loops forever: reconcile_all() then sleep(interval)
```

**All 31 controllers:**

| Controller | Description |
|---|---|
| Deployment | Manages ReplicaSets for rolling updates |
| ReplicaSet | Maintains desired pod replica count |
| StatefulSet | Ordered pod deployment with stable identities |
| DaemonSet | Ensures one pod per eligible node |
| Job | Batch execution with completions and parallelism |
| CronJob | Time-based job scheduling with cron syntax |
| ReplicationController | Legacy replica management |
| Endpoints | Populates Endpoints from pod IPs matching Services |
| EndpointSlice | Populates EndpointSlices (modern endpoint API) |
| Service | Manages Service lifecycle and defaults |
| ServiceAccount | Creates default ServiceAccount per namespace |
| Namespace | Handles namespace lifecycle and finalizers |
| Node | Manages node status and conditions |
| PV Binder | Binds PersistentVolumeClaims to PersistentVolumes |
| Dynamic Provisioner | Creates PVs from StorageClass for unbound PVCs |
| Volume Snapshot | Manages VolumeSnapshot lifecycle |
| Volume Expansion | Handles PVC resize operations |
| Garbage Collector | Owner-reference-based cascade deletion |
| TTL Controller | Cleans up finished Jobs after TTL expiry |
| Taint Eviction | Evicts pods from tainted nodes (NoExecute) |
| HPA | Horizontal Pod Autoscaler (metrics-based scaling) |
| VPA | Vertical Pod Autoscaler (resource right-sizing) |
| PDB | Enforces PodDisruptionBudget constraints |
| Events | Manages Event lifecycle and cleanup |
| ResourceQuota | Enforces namespace resource quotas |
| ResourceClaim | Manages DRA ResourceClaim lifecycle |
| LoadBalancer | Assigns external IPs to LoadBalancer Services |
| Ingress | Manages Ingress status |
| Network Policy | Manages NetworkPolicy status |
| CSR | Processes CertificateSigningRequests |
| CRD | Manages CustomResourceDefinition status and conditions |

### 5. kubelet

Node agent managing pod lifecycle via Docker (bollard crate).

**Container model:** Each pod starts a pause container first. Application
containers attach to the pause container's network namespace via
`container:pause` network mode. The pod IP comes from the pause container.

**Probes:** Liveness, readiness, and startup probes with HTTP, TCP, and exec
checks. Readiness status is written back to the API server.

**Volumes:** emptyDir (tmpfs and on-disk via bind mounts), hostPath, projected
volumes, configMap volumes, secret volumes.

**Status reporting:** Pod status updates use CAS (compare-and-swap) against the
API server. The kubelet re-reads the pod from storage before each update to get
a fresh `resourceVersion`, avoiding write conflicts.

**Node registration:** On startup, the kubelet registers itself as a Node
resource with capacity and allocatable fields.

### 6. kube-proxy

iptables-based service routing. Runs in host network mode with `CAP_NET_ADMIN`.

- ClusterIP services: DNAT rules in RUSTERNETES-SERVICES chain.
- NodePort services: Rules in RUSTERNETES-NODEPORTS chain (ports 30000-32767).
- LoadBalancer services: External IP routing.
- Probabilistic load balancing across endpoints.
- Reads both Endpoints and EndpointSlices.
- Automatic rule sync on a 30-second interval with cleanup on shutdown.

### 7. scheduler

Filter/Score plugin architecture for pod-to-node assignment.

**Filter phase** (eliminates ineligible nodes):
- Unschedulable nodes
- Taint/toleration checks (NoSchedule, PreferNoSchedule, NoExecute)
- Node selector matching
- Node affinity hard requirements
- Resource capacity (CPU/memory)

**Score phase** (ranks remaining nodes):
- Resource availability
- Node affinity preferences
- Pod affinity/anti-affinity
- Priority class weight

**Additional features:** Priority-based scheduling, preemption of lower-priority
pods, topology spread constraints.

### 8. kubectl

CLI tool with the following commands:

`get`, `create`, `apply`, `delete`, `describe`, `logs`, `exec`, `port-forward`,
`scale`, `edit`, `patch`, `cp`

Supports YAML/JSON input, tabular/JSON/YAML output, `--namespace` and
`--all-namespaces` flags, label selectors, field selectors, watch mode, and
multi-document YAML files. Uses kubeconfig for cluster connection.

### 9. cloud-providers

AWS, GCP, and Azure integration modules for cloud-specific functionality
(load balancer provisioning, storage backends, node management).

## Key Architecture Patterns

### Optimistic Concurrency

Every resource carries a `resourceVersion` derived from etcd's `mod_revision`.
Updates must include the current version. If the stored version has changed
since the client last read it, the API server returns HTTP 409 Conflict. This
prevents lost updates without pessimistic locking.

### Watch API

The Watch API uses Server-Sent Events. Behavior depends on the
`resourceVersion` query parameter:

- `rv=0`, `rv=1`, or omitted: The server sends an initial burst of ADDED events
  for all existing resources, then streams subsequent changes.
- `rv>1`: The server starts an etcd watch from that revision, streaming only
  new changes.

Controllers and the kubelet use watches to react to state changes in near
real-time rather than polling.

### Controller Reconciliation

Every controller follows the same pattern:

1. List all resources of its type from storage.
2. For each resource, compare desired state (spec) against actual state (status).
3. Take corrective action (create/delete pods, update status, etc.).
4. Sleep for a configured interval (typically 5-10 seconds).
5. Repeat.

Errors in one resource do not stop reconciliation of others.

### Pause Container Networking

The kubelet creates a pause container for each pod before starting application
containers. All application containers join the pause container's network
namespace (`container:pause` mode), sharing a single IP address. This matches
real Kubernetes networking semantics.

## Docker Compose Cluster

The development cluster runs via Docker Compose with the following services:

| Service | Description |
|---|---|
| etcd | Key-value store for all cluster state |
| api-server | HTTPS on port 6443, TLS certs in `.rusternetes/certs/` |
| scheduler | Assigns pods to nodes |
| controller-manager | Runs all 31 controllers |
| kubelet (node-1) | First worker node |
| kubelet (node-2) | Second worker node |
| kube-proxy | Host network mode for iptables access |
| CoreDNS | Cluster DNS at 10.96.0.10 |

TLS certificates are generated by `scripts/generate-certs.sh`. SANs must
include Docker bridge IPs (172.18.0.2-5). The cluster is bootstrapped with
`scripts/bootstrap-cluster.sh`, which creates CoreDNS, default services, and
service account tokens.

KUBECONFIG: `~/.kube/rusternetes-config`

## Storage Schema

```
/registry/pods/{namespace}/{name}
/registry/services/{namespace}/{name}
/registry/deployments/{namespace}/{name}
/registry/statefulsets/{namespace}/{name}
/registry/daemonsets/{namespace}/{name}
/registry/jobs/{namespace}/{name}
/registry/cronjobs/{namespace}/{name}
/registry/configmaps/{namespace}/{name}
/registry/secrets/{namespace}/{name}
/registry/serviceaccounts/{namespace}/{name}
/registry/nodes/{name}
/registry/namespaces/{name}
/registry/persistentvolumes/{name}
/registry/persistentvolumeclaims/{namespace}/{name}
/registry/storageclasses/{name}
/registry/clusterroles/{name}
/registry/clusterrolebindings/{name}
/registry/roles/{namespace}/{name}
/registry/rolebindings/{namespace}/{name}
/registry/ingresses/{namespace}/{name}
/registry/customresourcedefinitions/{name}
...
```

## Data Flow: Creating a Deployment

1. `kubectl apply -f deployment.yaml` sends a POST/PATCH to the API server.
2. The API server authenticates, authorizes (RBAC), runs admission webhooks,
   validates the resource, and writes it to etcd.
3. The Deployment controller detects the new Deployment and creates a ReplicaSet.
4. The ReplicaSet controller detects the new ReplicaSet and creates Pods.
5. The scheduler watches for unscheduled Pods, scores nodes, and binds each Pod
   to a node by setting `spec.nodeName`.
6. The kubelet on the assigned node watches for Pods bound to it, creates a
   pause container, then starts application containers via Docker.
7. The kubelet reports Pod status back to the API server (phase, conditions,
   container statuses, pod IP).
8. The Endpoints and EndpointSlice controllers update service endpoints to
   include the new Pod IPs.
9. kube-proxy programs iptables rules so cluster traffic reaches the new Pods.

## Concurrency Model

All components use Tokio for async I/O:

- **API server**: Handles concurrent HTTPS requests. Each request is a Tokio task.
- **Controller manager**: Each of the 31 controllers runs as an independent
  `tokio::spawn` task.
- **Scheduler**: Periodic async loop with storage reads and writes.
- **Kubelet**: Async sync loop interleaving Docker API calls and status updates.
- **Storage**: All etcd operations are async. CAS prevents concurrent write
  conflicts.

## Testing

929 test functions across the workspace. All async tests use `#[tokio::test]`.
Unit tests use `MemoryStorage` instead of etcd. Tests requiring shared mutable
state use `#[serial_test::serial]`.

```bash
cargo test                                     # All tests
cargo test -p rusternetes_api_server           # Single crate
cargo test test_name -- --nocapture            # Single test with output
```

Conformance testing runs via Sonobuoy against the Docker Compose cluster.

## Dependencies

| Crate | Purpose |
|---|---|
| tokio | Async runtime |
| axum | HTTP framework for the API server |
| serde / serde_json | Serialization for API and storage |
| etcd-client | etcd storage backend |
| bollard | Docker API client for the kubelet |
| rustls / tokio-rustls | TLS implementation |
| rcgen | Self-signed certificate generation |
| jsonwebtoken | JWT for service account tokens |
| tracing / tracing-subscriber | Structured logging |
| chrono | Date/time handling |
| uuid | Resource UID generation |
