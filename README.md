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

This is an educational project to understand Kubernetes internals while leveraging Rust's safety guarantees.

## License

Apache-2.0
