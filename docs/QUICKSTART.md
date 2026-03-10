# Rusternetes Quick Start

Get up and running with Rusternetes in under 5 minutes!

## Prerequisites

Install these tools first:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Podman (macOS)
brew install podman podman-compose
podman machine init
podman machine start

# Or install Docker
brew install docker docker-compose
```

## 3-Step Setup

### 1. Clone and Navigate
```bash
git clone https://github.com/rusternetes/rusternetes.git
cd rusternetes
```

### 2. Start the Cluster
```bash
# Option A: Interactive setup
./dev-setup.sh

# Option B: One-command setup
make dev-full
```

This will:
- Build all container images (~5-10 minutes first time)
- Start etcd, API server, scheduler, controller manager, kubelet, and kube-proxy
- Create a local development cluster

### 3. Verify It Works
```bash
# Check cluster status
make dev-ps

# List pods
make kubectl-get-pods

# Create an example pod
make kubectl-create-example-pod

# View logs
make dev-logs
```

## What Just Happened?

You now have a complete Kubernetes-like cluster running locally with:

- **etcd** (http://localhost:2379) - Cluster state storage
- **API Server** (https://localhost:6443) - Main API endpoint with TLS/HTTPS
- **Scheduler** - Assigns pods to nodes
- **Controller Manager** - Manages cluster state (Deployment, StatefulSet, DaemonSet, Job, CronJob)
- **Kubelet** - Runs containers on nodes
- **Kube-proxy** - Network proxy (stub implementation)

## Common Operations

### View Cluster Status
```bash
# All services
make dev-ps

# Specific service logs
make dev-logs-api-server
make dev-logs-scheduler
make dev-logs-kubelet
```

### Create Resources
```bash
# Create a namespace
cargo run --bin kubectl -- --server https://localhost:6443 --insecure-skip-tls-verify create -f examples/tests/test-namespace.yaml

# Create a pod
cargo run --bin kubectl -- --server https://localhost:6443 --insecure-skip-tls-verify create -f examples/workloads/test-pod.yaml

# Create a deployment
cargo run --bin kubectl -- --server https://localhost:6443 --insecure-skip-tls-verify create -f examples/workloads/test-deployment.yaml
```

### List Resources
```bash
make kubectl-get-pods
make kubectl-get-deployments
make kubectl-get-services
make kubectl-get-namespaces
```

### Make Code Changes
```bash
# 1. Edit code
vim crates/api-server/src/main.rs

# 2. Rebuild the component
podman-compose build api-server

# 3. Restart it
podman-compose up -d --force-recreate api-server

# 4. Check logs
make dev-logs-api-server
```

### Stop the Cluster
```bash
make dev-down
```

### Restart the Cluster
```bash
make dev-up
```

### Clean Slate (remove everything)
```bash
make dev-clean
```

## Troubleshooting

### "Port already in use"
```bash
# Find what's using port 6443 or 2379
lsof -i :6443
lsof -i :2379

# Kill it or use different ports
```

### "Podman machine not running"
```bash
podman machine start
```

### "Cannot connect to API server"
```bash
# Check if services are running
make dev-ps

# Restart everything
make dev-down
make dev-up
```

### "Build is taking too long"
```bash
# First build takes 5-10 minutes
# Subsequent builds are much faster due to caching

# For faster iteration during development:
make build-dev  # Debug builds are faster
```

## Next Steps

- **Learn more**: Read [DEVELOPMENT.md](DEVELOPMENT.md) for detailed workflows
- **Explore examples**: Check out `examples/` directory
- **Understand architecture**: Read [ARCHITECTURE.md](ARCHITECTURE.md)
- **Contribute**: See [CONTRIBUTING.md](CONTRIBUTING.md)

## Quick Reference

| Task | Command |
|------|---------|
| Start cluster | `make dev-up` |
| Stop cluster | `make dev-down` |
| View logs | `make dev-logs` |
| List pods | `make kubectl-get-pods` |
| Build images | `make build-images` |
| Run tests | `make test` |
| Format code | `make fmt` |
| Clean up | `make dev-clean` |
| See all commands | `make help` |

## What's Different from Kubernetes?

Rusternetes is a **learning implementation** of Kubernetes in Rust. It:

- ✅ Implements core Kubernetes concepts (Pods, Deployments, Services, etc.)
- ✅ Uses the same architecture (API server, scheduler, kubelet, etc.)
- ✅ Stores state in etcd like Kubernetes
- ✅ Provides a kubectl-like CLI
- ❌ Is not production-ready (use for learning!)
- ❌ Doesn't implement all Kubernetes features
- ❌ Is not compatible with real Kubernetes clusters

**Use it to:**
- Learn how Kubernetes works internally
- Experiment with Rust for systems programming
- Understand distributed system concepts
- Build custom controllers and extensions

**Don't use it for:**
- Production workloads
- Critical infrastructure
- Running real applications (use real Kubernetes!)

## Resources

- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [Rust Book](https://doc.rust-lang.org/book/)
- [Podman Documentation](https://docs.podman.io/)

Happy learning! 🚀
