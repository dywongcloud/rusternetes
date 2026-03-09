# Rusternetes Architecture

This document describes the architecture and design of Rusternetes, a Kubernetes reimplementation in Rust.

## Overview

Rusternetes follows the standard Kubernetes architecture with a control plane and node components, all communicating through a shared etcd storage backend.

## Project Structure

```
rusternetes/
├── crates/
│   ├── common/              # Shared types and resource definitions
│   ├── storage/             # etcd storage abstraction layer
│   ├── api-server/          # Kubernetes API server
│   ├── scheduler/           # Pod scheduler
│   ├── controller-manager/  # Resource controllers
│   ├── kubelet/            # Node agent
│   ├── kube-proxy/         # Network proxy (stub)
│   └── kubectl/            # CLI tool
├── examples/               # Example YAML manifests
├── README.md
├── GETTING_STARTED.md
└── Cargo.toml             # Workspace configuration
```

## Components

### 1. Common Library (`rusternetes-common`)

The common library provides shared types and data structures:

**Resource Types:**
- `Pod` - Smallest deployable unit
- `Service` - Service abstraction for load balancing
- `Deployment` - Declarative pod management
- `Node` - Worker node representation
- `Namespace` - Resource isolation

**Core Types:**
- `ObjectMeta` - Metadata for all resources
- `TypeMeta` - API versioning information
- `Phase` - Resource lifecycle states
- `LabelSelector` - Label-based selection
- `ResourceRequirements` - CPU/memory requirements

**Error Handling:**
- Unified `Error` enum for all components
- `Result<T>` type alias for error propagation

### 2. Storage Layer (`rusternetes-storage`)

The storage layer provides an abstraction over etcd:

**Traits:**
- `Storage` - Async CRUD operations for resources

**Implementation:**
- `EtcdStorage` - etcd-backed storage implementation
- Watch support for real-time updates
- Automatic JSON serialization/deserialization

**Key Structure:**
```
/registry/{resource_type}/{namespace}/{name}
```

### 3. API Server (`rusternetes-api-server`)

The API server exposes RESTful APIs for all resources:

**Endpoints:**
```
# Core v1 API
GET/POST    /api/v1/namespaces
GET/PUT/DELETE /api/v1/namespaces/{name}
GET/POST    /api/v1/namespaces/{ns}/pods
GET/PUT/DELETE /api/v1/namespaces/{ns}/pods/{name}
GET/POST    /api/v1/namespaces/{ns}/services
GET/PUT/DELETE /api/v1/namespaces/{ns}/services/{name}
GET/POST    /api/v1/nodes
GET/PUT/DELETE /api/v1/nodes/{name}

# Apps v1 API
GET/POST    /apis/apps/v1/namespaces/{ns}/deployments
GET/PUT/DELETE /apis/apps/v1/namespaces/{ns}/deployments/{name}
```

**Technology:**
- Built with Axum web framework
- Tower middleware for tracing
- JSON request/response serialization

### 4. Scheduler (`rusternetes-scheduler`)

The scheduler assigns pods to nodes:

**Algorithm:**
1. Watch for unscheduled pods (pods without `spec.nodeName`)
2. Filter schedulable nodes (not marked unschedulable)
3. Check node selectors if specified
4. Select first available node (simple round-robin)
5. Bind pod to node by updating `spec.nodeName`

**Features:**
- Node selector support
- Periodic scheduling loop (default: 5 seconds)
- Handles pending pods automatically

### 5. Controller Manager (`rusternetes-controller-manager`)

Runs controllers that maintain desired state:

**Deployment Controller:**
1. Watches all Deployment resources
2. Compares current vs desired replica count
3. Creates or deletes pods to match desired state
4. Uses label selectors to identify deployment pods

**Reconciliation Loop:**
- Periodic sync (default: 10 seconds)
- Creates pods with unique names (UUID-based)
- Maintains pod-to-deployment ownership

### 6. Kubelet (`rusternetes-kubelet`)

Node agent that manages containers:

**Responsibilities:**
1. Register node with the API server
2. Watch for pods assigned to this node
3. Manage container lifecycle via Docker API
4. Report node and pod status

**Container Runtime:**
- Uses Docker via Bollard library
- Container naming: `{pod_name}_{container_name}`
- Automatic container start/stop based on pod phase

**Sync Loop:**
- Periodic reconciliation (default: 10 seconds)
- Starts containers for Running pods
- Stops containers for terminated pods

### 7. Kube-proxy (`rusternetes-kube-proxy`)

Network proxy component (stub implementation):

**Note:** This is a placeholder. A full implementation would:
- Watch Service and Endpoints resources
- Program iptables/ipvs rules
- Handle NodePort and LoadBalancer types

### 8. kubectl (`rusternetes-kubectl`)

Command-line interface for cluster management:

**Commands:**
- `get` - Retrieve resources
- `create` - Create from YAML
- `delete` - Remove resources
- `apply` - Update from YAML

**Features:**
- YAML file parsing
- Tabular output for lists
- JSON output for individual resources
- Namespace support

## Data Flow

### Creating a Deployment

1. User runs: `kubectl create -f deployment.yaml`
2. kubectl parses YAML and POSTs to API server
3. API server validates and stores in etcd
4. Controller manager watches deployments
5. Controller creates pods based on replicas
6. Scheduler assigns pods to nodes
7. Kubelet on node sees assigned pod
8. Kubelet starts containers via Docker
9. Pod status updated to Running

### Pod Lifecycle

```
Pending -> Scheduled -> Running -> Succeeded/Failed
   ↓          ↓           ↓
API Server  Scheduler  Kubelet
```

## Storage Schema

Resources are stored in etcd with the following key structure:

```
/registry/pods/{namespace}/{pod-name}
/registry/services/{namespace}/{service-name}
/registry/deployments/{namespace}/{deployment-name}
/registry/nodes/{node-name}
/registry/namespaces/{namespace-name}
```

## Concurrency Model

All components use Tokio for async I/O:

- **API Server**: Multiple concurrent HTTP requests
- **Scheduler**: Periodic async loop with storage operations
- **Controller Manager**: Separate controllers can run concurrently
- **Kubelet**: Async sync loop with Docker API calls

## Error Handling

Unified error handling via `rusternetes_common::Error`:

```rust
pub enum Error {
    NotFound(String),
    AlreadyExists(String),
    InvalidResource(String),
    Storage(String),
    Network(String),
    Authentication(String),
    Authorization(String),
    Serialization(SerdeError),
    Internal(String),
}
```

HTTP status codes mapped from errors in API server.

## Future Enhancements

1. **Authentication & Authorization**
   - Service accounts
   - RBAC (Role-Based Access Control)
   - TLS/mTLS

2. **Advanced Scheduling**
   - Resource-based scheduling (CPU/memory)
   - Affinity/anti-affinity rules
   - Taints and tolerations

3. **Networking**
   - CNI plugin support
   - Service mesh integration
   - Network policies

4. **Storage**
   - Persistent volumes
   - Volume plugins
   - StorageClasses

5. **High Availability**
   - Multi-master API servers
   - Leader election for controllers
   - etcd clustering

6. **Observability**
   - Metrics export (Prometheus)
   - Distributed tracing
   - Structured logging

7. **Additional Resources**
   - ConfigMaps and Secrets
   - StatefulSets
   - DaemonSets
   - Jobs and CronJobs
   - Ingress

## Performance Considerations

- **Storage**: All operations go through etcd - consider caching
- **Watch**: Use etcd watch API for efficient event streaming
- **Serialization**: JSON for API, could optimize with Protocol Buffers
- **Concurrency**: Tokio enables efficient async I/O without thread overhead

## Testing Strategy

1. **Unit Tests**: Test individual components and functions
2. **Integration Tests**: Test component interactions
3. **End-to-End Tests**: Full workflow testing with etcd
4. **Load Tests**: Test scalability and performance

## Contributing

When adding new features:

1. Define resource types in `common/resources/`
2. Add storage operations if needed
3. Implement API handlers in `api-server/handlers/`
4. Add controller logic in `controller-manager/controllers/`
5. Update kubectl commands as needed
6. Add example YAML files
7. Update documentation

## References

- [Kubernetes Architecture](https://kubernetes.io/docs/concepts/architecture/)
- [Kubernetes API Conventions](https://github.com/kubernetes/community/blob/master/contributors/devel/sig-architecture/api-conventions.md)
- [etcd Documentation](https://etcd.io/docs/)
- [Tokio Documentation](https://tokio.rs/)
