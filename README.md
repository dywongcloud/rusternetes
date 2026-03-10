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

### Quick Start with Podman

For local development, we provide a complete container-based development environment:

```bash
# Interactive setup
./dev-setup.sh

# Choose from options:
# - Option 8: Full setup (build + start cluster)
# - Option 9: Install MetalLB (for LoadBalancer services)

# Or using Make
make dev-full        # Build and start everything
make dev-logs        # View logs
make kubectl-get-pods # Try it out!
```

**Enable LoadBalancer Services:**

After starting your cluster, you can install MetalLB for local LoadBalancer support:

```bash
./dev-setup.sh  # Choose option 9
# Or run the automated test:
./examples/metallb/test-metallb.sh
```

This gives you working LoadBalancer services without cloud provider credentials! See [docs/METALLB_INTEGRATION.md](docs/METALLB_INTEGRATION.md) for details.

See [DEVELOPMENT.md](DEVELOPMENT.md) for detailed development workflows, troubleshooting, and advanced usage.

### Traditional Development

If you prefer to run components locally without containers, see [GETTING_STARTED.md](GETTING_STARTED.md).

This is an educational project to understand Kubernetes internals while leveraging Rust's safety guarantees.

## License

Apache-2.0
