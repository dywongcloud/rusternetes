# Rusternetes

A Kubernetes reimplementation in Rust, focusing on memory safety, performance, and educational value.

## Architecture

Rusternetes follows the standard Kubernetes architecture with the following components:

### Control Plane Components

- **API Server** (`api-server`): Central management component that exposes the Kubernetes API
- **Scheduler** (`scheduler`): Assigns pods to nodes based on resource requirements and constraints
- **Controller Manager** (`controller-manager`): Runs controllers that regulate the state of the cluster

### Node Components

- **Kubelet** (`kubelet`): Agent that runs on each node and manages containers
- **Kube-proxy** (`kube-proxy`): Network proxy that maintains network rules

### Additional Components

- **CoreDNS**: Standard Kubernetes DNS for service discovery (deployed via bootstrap-cluster.yaml)

### CLI Tools

- **kubectl** (`kubectl`): Command-line interface for interacting with the cluster

### Shared Libraries

- **Common** (`common`): Shared types, utilities, and resource definitions
- **Storage** (`storage`): Abstraction layer for etcd and persistent storage

## Building

```bash
cargo build --release
```

## Running

### Prerequisites

- etcd cluster running (for state storage)
- Container runtime (Docker or containerd)

### Start Control Plane

```bash
# Start API server
cargo run --bin api-server

# Start scheduler
cargo run --bin scheduler

# Start controller manager
cargo run --bin controller-manager
```

### Start Node Components

```bash
# Start kubelet
cargo run --bin kubelet

# Start kube-proxy
cargo run --bin kube-proxy
```

## Development

### Quick Setup Scripts

We provide automated setup scripts for easy development:

**macOS (Docker Desktop)**:
```bash
./scripts/dev-setup-macos.sh
```

**Fedora/RHEL/CentOS (Podman or Docker)**:
```bash
sudo ./scripts/dev-setup-fedora.sh
```

These scripts install all dependencies, build the project, create helper scripts, and set up your development environment.

### Quick Start with Docker

For local development, we provide a complete container-based development environment:

**Important Prerequisites:**
- **Docker Desktop** is required on macOS (Podman Machine has compatibility issues on macOS Sequoia 15.7+)
- **Docker with rootful mode** is required on Linux for kube-proxy iptables access
- Set the `KUBELET_VOLUMES_PATH` environment variable before starting

```bash
# Set the volumes path (required)
export KUBELET_VOLUMES_PATH=$(pwd)/.rusternetes/volumes

# Build and start the cluster
docker-compose build
docker-compose up -d

# Apply bootstrap resources (CoreDNS, services)
cat bootstrap-cluster.yaml | envsubst > /tmp/bootstrap-expanded.yaml
KUBECONFIG=/dev/null ./target/release/kubectl --insecure-skip-tls-verify apply -f /tmp/bootstrap-expanded.yaml

# Verify the cluster
KUBECONFIG=/dev/null ./target/release/kubectl --insecure-skip-tls-verify get pods -A
```

**Why Docker Desktop on macOS?**

Podman Machine on macOS Sequoia 15.7+ has a known issue with the Apple Virtualization Framework that prevents VMs from starting. Additionally, kube-proxy requires rootful container execution for iptables access, which Docker Desktop provides automatically.

**Note:** See [DEVELOPMENT.md](docs/DEVELOPMENT.md) for detailed development workflows, troubleshooting, Podman alternatives on Linux, and advanced usage.

### Linux Users

On Linux, you can use either Docker or Podman. For Podman, you must run in **rootful mode** for kube-proxy to access iptables:

```bash
# Create a rootful Podman machine (if using Podman Desktop)
podman machine init --rootful --cpus 4 --memory 8192

# Or run Podman containers as root
sudo podman-compose up -d
```

This is an educational project to understand Kubernetes internals while leveraging Rust's safety guarantees.

## Documentation

**📚 Complete Documentation Index:** See [docs/DOCUMENTATION_INDEX.md](docs/DOCUMENTATION_INDEX.md) for a comprehensive, organized guide to all documentation.

### Getting Started
- [QUICKSTART.md](docs/QUICKSTART.md) - Quick start guide for trying Rusternetes
- [GETTING_STARTED.md](docs/GETTING_STARTED.md) - Traditional development setup
- [DEVELOPMENT.md](docs/DEVELOPMENT.md) - Comprehensive development guide
- [DEPLOYMENT.md](docs/DEPLOYMENT.md) - Production deployment guide

### Features & Implementation
- [STATUS.md](docs/STATUS.md) - Current implementation status and roadmap
- [ARCHITECTURE.md](docs/ARCHITECTURE.md) - System architecture and design
- [HIGH_AVAILABILITY.md](docs/HIGH_AVAILABILITY.md) - High Availability setup with etcd clustering, load balancing, and leader election ⭐ NEW
- [API_FEATURES_COMPLETE.md](docs/API_FEATURES_COMPLETE.md) - API features implementation (PATCH, Field Selectors, Server-Side Apply)
- [PATCH_IMPLEMENTATION.md](docs/PATCH_IMPLEMENTATION.md) - Detailed PATCH operations guide

### Storage & Volumes
- [DYNAMIC_PROVISIONING.md](docs/DYNAMIC_PROVISIONING.md) - Dynamic volume provisioning
- [VOLUME_SNAPSHOTS.md](docs/VOLUME_SNAPSHOTS.md) - Volume snapshot feature
- [VOLUME_EXPANSION.md](docs/VOLUME_EXPANSION.md) - Volume expansion feature

### Networking
- [DNS.md](docs/DNS.md) - DNS server and service discovery
- [LOADBALANCER.md](docs/LOADBALANCER.md) - LoadBalancer service type with MetalLB
- [Network Policies](docs/networking/network-policies.md) - NetworkPolicy validation and CNI plugin integration ⭐ NEW

### Security
- [SECURITY.md](docs/SECURITY.md) - Security features (Admission Controllers, Pod Security Standards, Encryption, Audit)
- [ServiceAccount Token Signing](docs/security/service-account-tokens.md) - JWT token signing with RS256 for production deployments ⭐ NEW
- [WEBHOOK_INTEGRATION.md](docs/WEBHOOK_INTEGRATION.md) - Admission webhook integration guide
- [WEBHOOK_TESTING.md](docs/WEBHOOK_TESTING.md) - Comprehensive webhook testing guide
- [TLS_GUIDE.md](docs/TLS_GUIDE.md) - TLS configuration

### Testing & Observability
- [TESTING_IMPLEMENTATION_GUIDE.md](docs/TESTING_IMPLEMENTATION_GUIDE.md) - Comprehensive testing guide
- [TESTING.md](docs/TESTING.md) - Testing procedures
- [TRACING.md](docs/TRACING.md) - Distributed tracing with OpenTelemetry

### Development & Utilities
- [CONTRIBUTING.md](docs/CONTRIBUTING.md) - Contribution guidelines
- [DEV_SETUP_METALLB.md](docs/DEV_SETUP_METALLB.md) - MetalLB setup for LoadBalancer services
- [PODMAN_TIPS.md](docs/PODMAN_TIPS.md) - Podman troubleshooting and tips
- [SETUP_NOTES.md](docs/SETUP_NOTES.md) - Setup and configuration notes

## Current Status

Rusternetes implements core Kubernetes features including:

- ✅ **API Server** - Full CRUD operations with TLS, RBAC, authentication
- ✅ **Scheduler** - Advanced scheduling with affinity/anti-affinity, taints/tolerations, priority/preemption
- ✅ **Controllers** - Deployment, StatefulSet, Job, DaemonSet, CronJob, Endpoints, PV/PVC Binder, Dynamic Provisioner, Volume Snapshot, LoadBalancer
- ✅ **Storage** - PV/PVC, Dynamic Provisioning, Volume Snapshots, Volume Expansion
- ✅ **Networking** - ClusterIP, NodePort, LoadBalancer services, CoreDNS, kube-proxy with iptables
- ✅ **Security** - RBAC, Admission Webhooks, Pod Security Standards, Secrets Encryption, Audit Logging
- ✅ **High Availability** - Multi-master API servers, etcd clustering (3-5 nodes), leader election, automatic failover ⭐ NEW
- ✅ **Advanced API** - PATCH (all resources), Field Selectors, Server-Side Apply, Watch API, CRDs with hot-reload
- ✅ **Observability** - Prometheus metrics, Events API, OpenTelemetry tracing

**Latest Addition (March 10, 2026):** Production-grade High Availability support with:
- **etcd Clustering**: 3-5 node clusters with quorum for fault tolerance
- **Multi-Master API Servers**: Active-active API servers behind HAProxy load balancer
- **Leader Election**: Controller-manager and scheduler use etcd-based leader election for active-standby HA
- **Automatic Failover**: ~15 second failover time for all components
- **Enhanced Health Checks**: Comprehensive liveness/readiness probes with storage connectivity checks

Run in HA mode: `docker-compose -f docker-compose.ha.yml up` or test with `./scripts/test-ha.sh`

**Test Coverage:** 1306+ tests passing including:
- 21 admission webhook tests
- 16 CNI framework tests
- 16 LoadBalancer tests
- 8 autoscaling/init container tests
- 42 controller unit tests
- 371 status subresource tests
- 324 garbage collector tests
- 402 TTL controller tests
- Integration tests for all workload controllers

See [STATUS.md](docs/STATUS.md) for detailed implementation status and [CONFORMANCE_PLAN.md](docs/planning/CONFORMANCE_PLAN.md) for Kubernetes conformance tracking.

## License

Apache-2.0
