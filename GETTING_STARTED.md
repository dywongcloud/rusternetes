# Getting Started with Rusternetes

This guide will help you get Rusternetes up and running on your local machine.

## Prerequisites

1. **Rust** - Install from [rustup.rs](https://rustup.rs/)
2. **etcd** - Distributed key-value store for cluster state
3. **Docker** - Container runtime (for kubelet)

### Installing etcd

Using Docker:
```bash
docker run -d \
  --name etcd \
  -p 2379:2379 \
  -p 2380:2380 \
  -e ALLOW_NONE_AUTHENTICATION=yes \
  bitnami/etcd:latest
```

Or using Homebrew (macOS):
```bash
brew install etcd
etcd
```

## Building Rusternetes

Build all components:
```bash
cargo build --release
```

The binaries will be available in `target/release/`:
- `api-server`
- `scheduler`
- `controller-manager`
- `kubelet`
- `kube-proxy`
- `kubectl`

## Running the Control Plane

### 1. Start the API Server

```bash
cargo run --bin api-server -- \
  --bind-address 0.0.0.0:6443 \
  --etcd-servers http://localhost:2379
```

### 2. Start the Scheduler

```bash
cargo run --bin scheduler -- \
  --etcd-servers http://localhost:2379
```

### 3. Start the Controller Manager

```bash
cargo run --bin controller-manager -- \
  --etcd-servers http://localhost:2379
```

## Running Node Components

### 1. Start the Kubelet

```bash
cargo run --bin kubelet -- \
  --node-name node-1 \
  --etcd-servers http://localhost:2379
```

### 2. Start Kube-proxy (optional)

```bash
cargo run --bin kube-proxy -- \
  --node-name node-1
```

## Using kubectl

### Create a namespace

```bash
cargo run --bin kubectl -- \
  --server http://localhost:6443 \
  create -f examples/namespace.yaml
```

### Create a pod

```bash
cargo run --bin kubectl -- \
  --server http://localhost:6443 \
  create -f examples/pod.yaml
```

### List pods

```bash
cargo run --bin kubectl -- \
  --server http://localhost:6443 \
  get pods
```

### Create a deployment

```bash
cargo run --bin kubectl -- \
  --server http://localhost:6443 \
  create -f examples/deployment.yaml
```

### List deployments

```bash
cargo run --bin kubectl -- \
  --server http://localhost:6443 \
  get deployments
```

### Delete a pod

```bash
cargo run --bin kubectl -- \
  --server http://localhost:6443 \
  delete pod nginx-pod
```

## Architecture Overview

```
┌─────────────────────────────────────────┐
│           Control Plane                  │
│  ┌──────────────┐  ┌──────────────┐    │
│  │  API Server  │  │  Scheduler   │    │
│  └──────────────┘  └──────────────┘    │
│  ┌──────────────┐                       │
│  │ Controller   │                       │
│  │  Manager     │                       │
│  └──────────────┘                       │
└─────────────────────────────────────────┘
              │
              │ (communicates via etcd)
              ▼
      ┌──────────────┐
      │     etcd     │
      └──────────────┘
              │
              ▼
┌─────────────────────────────────────────┐
│             Node                         │
│  ┌──────────────┐  ┌──────────────┐    │
│  │   Kubelet    │  │ Kube-proxy   │    │
│  └──────────────┘  └──────────────┘    │
│         │                                │
│         ▼                                │
│  ┌──────────────┐                       │
│  │    Docker    │                       │
│  └──────────────┘                       │
└─────────────────────────────────────────┘
```

## Next Steps

- Explore the example YAML files in the `examples/` directory
- Read the main README.md for architecture details
- Check out the source code in `crates/`

## Troubleshooting

### etcd connection errors
Make sure etcd is running and accessible at `localhost:2379`

### Docker connection errors
Ensure Docker daemon is running for the kubelet to start containers

### Port already in use
The API server uses port 6443 by default. Change with `--bind-address`

## Development

Run tests:
```bash
cargo test
```

Run with debug logging:
```bash
cargo run --bin api-server -- --log-level debug
```
