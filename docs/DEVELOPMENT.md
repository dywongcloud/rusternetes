# Rusternetes Local Development Guide

This guide explains how to set up and use the local development environment for Rusternetes using Docker or Podman.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Development Workflows](#development-workflows)
- [Using Docker Compose](#using-docker-compose)
- [Using Podman Compose (Linux Only)](#using-podman-compose-linux-only)
- [Using the Makefile](#using-the-makefile)
- [Manual Development](#manual-development)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### Required

1. **Rust** (1.75 or later)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Container Runtime**

   **macOS:**
   ```bash
   # Docker Desktop (REQUIRED on macOS Sequoia 15.7+)
   brew install --cask docker
   # Then start Docker Desktop from Applications
   ```

   **Why Docker Desktop on macOS?** Podman Machine has a known issue with macOS Sequoia 15.7+ where the Apple Virtualization Framework prevents VMs from starting. Additionally, Rusternetes requires rootful container execution for kube-proxy iptables access, which Docker Desktop provides automatically.

   **Linux (with Docker):**
   ```bash
   # Install Docker
   curl -fsSL https://get.docker.com | sh
   sudo usermod -aG docker $USER
   ```

   **Linux (with Podman - rootful mode required):**
   ```bash
   # Fedora/RHEL/CentOS
   sudo dnf install podman podman-compose

   # Ubuntu/Debian
   sudo apt-get install podman podman-compose

   # IMPORTANT: Run in rootful mode for kube-proxy
   sudo podman-compose up -d
   ```

### Optional

- **make** - For using the Makefile shortcuts
- **etcd** - If you want to run components outside containers

## Quick Start

### Automated Setup Scripts

For the fastest setup experience, use our automated development setup scripts:

**macOS:**
```bash
./scripts/dev-setup-macos.sh
```

**Fedora/RHEL/CentOS:**
```bash
sudo ./scripts/dev-setup-fedora.sh
```

These scripts will:
- Install all dependencies (Rust, Docker/Podman, etc.)
- Build the project binaries and images
- Create convenient helper scripts in `.dev/` directory
- Set up shell functions and aliases for cluster management
- Optionally create systemd service for auto-start

After running the setup script, you'll have commands like:
- `cluster-start` - Start the cluster
- `cluster-stop` - Stop the cluster
- `cluster-logs` - View logs
- `cluster-status` - Check status
- `k` - kubectl alias (e.g., `k get pods -A`)

### Manual Setup - Docker Desktop (macOS and Windows)

```bash
# Set the volumes path (REQUIRED)
export KUBELET_VOLUMES_PATH=$(pwd)/.rusternetes/volumes

# Build images
docker-compose build

# Start the cluster
docker-compose up -d

# Apply bootstrap resources (CoreDNS, services)
cat bootstrap-cluster.yaml | envsubst > /tmp/bootstrap-expanded.yaml
KUBECONFIG=/dev/null ./target/release/kubectl --insecure-skip-tls-verify apply -f /tmp/bootstrap-expanded.yaml

# Verify the cluster
KUBECONFIG=/dev/null ./target/release/kubectl --insecure-skip-tls-verify get pods -A

# View logs
docker-compose logs -f

# Stop the cluster
docker-compose down
```

### Podman (Linux Only - Rootful Mode)

```bash
# Set the volumes path (REQUIRED)
export KUBELET_VOLUMES_PATH=$(pwd)/.rusternetes/volumes

# Build images (as root for rootful mode)
sudo KUBELET_VOLUMES_PATH=$KUBELET_VOLUMES_PATH podman-compose build

# Start the cluster (rootful mode required for kube-proxy)
sudo KUBELET_VOLUMES_PATH=$KUBELET_VOLUMES_PATH podman-compose up -d

# Apply bootstrap resources
cat bootstrap-cluster.yaml | envsubst > /tmp/bootstrap-expanded.yaml
KUBECONFIG=/dev/null ./target/release/kubectl --insecure-skip-tls-verify apply -f /tmp/bootstrap-expanded.yaml

# Verify
KUBECONFIG=/dev/null ./target/release/kubectl --insecure-skip-tls-verify get pods -A
```

## Development Workflows

### Daily Development Workflow

1. **Start your development session:**
   ```bash
   make dev-up
   ```

2. **Make code changes** in your editor

3. **Rebuild and restart a specific component:**
   ```bash
   # Example: Rebuild and restart the API server
   make build-image-api-server
   podman-compose up -d --force-recreate api-server
   ```

4. **View logs for debugging:**
   ```bash
   # All services
   make dev-logs

   # Specific service
   make dev-logs-api-server
   ```

5. **Test your changes:**
   ```bash
   # Run Rust tests
   make test

   # Try kubectl commands
   make kubectl-get-pods
   cargo run --bin kubectl -- --server http://localhost:6443 create -f examples/workloads/pod.yaml
   ```

6. **Stop the cluster when done:**
   ```bash
   make dev-down
   ```

### Testing Changes Without Containers

Sometimes you want to test changes quickly without rebuilding containers:

```bash
# Terminal 1: Start dependencies (etcd)
podman-compose up -d etcd

# Terminal 2: Run API server locally
make run-api-server
# Or: cargo run --bin api-server -- --bind-address 0.0.0.0:6443 --etcd-servers http://localhost:2379

# Terminal 3: Run scheduler locally
make run-scheduler

# Terminal 4: Run controller manager locally
make run-controller

# Terminal 5: Test with kubectl
cargo run --bin kubectl -- --server http://localhost:6443 get pods
```

### Code Quality Checks

Run these before committing:

```bash
# Format code
make fmt

# Run linter
make clippy

# Run tests
make test

# All pre-commit checks
make pre-commit
```

## Using Docker Compose

The `docker-compose.yml` file defines all services. Here are common operations:

### Start/Stop Services

```bash
# Set volumes path (REQUIRED)
export KUBELET_VOLUMES_PATH=$(pwd)/.rusternetes/volumes

# Start all services in background
docker-compose up -d

# Start and follow logs
docker-compose up

# Stop all services
docker-compose down

# Stop and remove volumes (clean slate)
docker-compose down -v
```

### View Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f api-server
docker-compose logs -f scheduler
docker-compose logs -f kubelet
docker-compose logs -f kube-proxy
```

### Check Service Status

```bash
# List running containers
docker-compose ps

# Check specific service health
docker exec rusternetes-api-server /app/api-server --version
```

### Rebuild Services

```bash
# Rebuild all images
KUBELET_VOLUMES_PATH=$(pwd)/.rusternetes/volumes docker-compose build

# Rebuild specific service
KUBELET_VOLUMES_PATH=$(pwd)/.rusternetes/volumes docker-compose build api-server

# Rebuild and restart
KUBELET_VOLUMES_PATH=$(pwd)/.rusternetes/volumes docker-compose up -d --build api-server
```

### Execute Commands in Containers

```bash
# Open shell in container
docker-compose exec api-server /bin/sh

# Run specific command
docker-compose exec api-server ps aux
```

## Using Podman Compose (Linux Only)

On Linux, you can use Podman instead of Docker. **Rootful mode is required for kube-proxy iptables access.**

All commands are the same as Docker Compose, but prefix with `sudo` and pass the environment variable:

```bash
# Example
sudo KUBELET_VOLUMES_PATH=$(pwd)/.rusternetes/volumes podman-compose up -d
sudo KUBELET_VOLUMES_PATH=$(pwd)/.rusternetes/volumes podman-compose logs -f
```

## Using the Makefile

The Makefile provides convenient shortcuts. View all available commands:

```bash
make help
```

### Common Make Targets

**Development Cluster:**
- `make dev-up` - Start the cluster
- `make dev-down` - Stop the cluster
- `make dev-restart` - Restart the cluster
- `make dev-logs` - View all logs
- `make dev-logs-api-server` - View specific service logs
- `make dev-ps` - Show running containers
- `make dev-clean` - Remove all containers and volumes
- `make dev-full` - Build images and start cluster

**Building:**
- `make build` - Build Rust binaries (release mode)
- `make build-dev` - Build Rust binaries (debug mode)
- `make build-images` - Build all container images
- `make build-image-api-server` - Build specific image

**Testing:**
- `make test` - Run all tests
- `make test-verbose` - Run tests with output
- `make check` - Run cargo check
- `make clippy` - Run linter
- `make fmt` - Format code
- `make pre-commit` - Run all pre-commit checks

**Running Locally:**
- `make run-api-server` - Run API server locally
- `make run-scheduler` - Run scheduler locally
- `make run-controller` - Run controller manager locally
- `make run-kubelet` - Run kubelet locally
- `make run-kube-proxy` - Run kube-proxy locally

**kubectl Commands:**
- `make kubectl-get-pods` - List pods
- `make kubectl-get-deployments` - List deployments
- `make kubectl-get-services` - List services
- `make kubectl-create-example-pod` - Create example pod

## Manual Development

### Building Locally

```bash
# Build all components in release mode
cargo build --release

# Build specific component
cargo build --release --bin api-server

# Build in debug mode (faster compilation)
cargo build
```

Binaries will be in `target/release/` or `target/debug/`.

### Running Locally (without containers)

1. **Start etcd:**
   ```bash
   podman run -d --name etcd \
     -p 2379:2379 -p 2380:2380 \
     -e ALLOW_NONE_AUTHENTICATION=yes \
     bitnami/etcd:latest
   ```

2. **Run components:**
   ```bash
   # API Server
   ./target/release/api-server --bind-address 0.0.0.0:6443 --etcd-servers http://localhost:2379

   # Scheduler
   ./target/release/scheduler --etcd-servers http://localhost:2379

   # Controller Manager
   ./target/release/controller-manager --etcd-servers http://localhost:2379

   # Kubelet (needs Docker/Podman socket)
   ./target/release/kubelet --node-name node-1 --etcd-servers http://localhost:2379
   ```

### Building Container Images Manually

```bash
# Build all images
podman build -f Dockerfile.api-server -t rusternetes/api-server:latest .
podman build -f Dockerfile.scheduler -t rusternetes/scheduler:latest .
podman build -f Dockerfile.controller-manager -t rusternetes/controller-manager:latest .
podman build -f Dockerfile.kubelet -t rusternetes/kubelet:latest .
podman build -f Dockerfile.kube-proxy -t rusternetes/kube-proxy:latest .
podman build -f Dockerfile.kubectl -t rusternetes/kubectl:latest .
```

## Architecture

The development environment consists of:

```
┌─────────────────────────────────────────────┐
│         Podman/Docker Network               │
│                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐ │
│  │   etcd   │  │API Server│  │Scheduler │ │
│  │  :2379   │  │  :6443   │  │          │ │
│  └──────────┘  └──────────┘  └──────────┘ │
│                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐ │
│  │Controller│  │ Kubelet  │  │   Kube   │ │
│  │ Manager  │  │          │  │  Proxy   │ │
│  └──────────┘  └──────────┘  └──────────┘ │
│                                             │
└─────────────────────────────────────────────┘
         │                    │
         │                    │
    Host :6443           Host :2379
    (API Server)         (etcd client)
```

## Troubleshooting

### Docker Desktop Not Running (macOS/Windows)

```bash
# Start Docker Desktop from Applications, or:
open -a Docker

# Verify Docker is running
docker info
```

### Podman Machine Virtualization Error (macOS)

If you see:
```
Error: vfkit exited unexpectedly with exit code 1
Error Domain=VZErrorDomain Code=1
```

This is a known issue with macOS Sequoia 15.7+ and Podman Machine. **Use Docker Desktop instead.** See [PODMAN_TIPS.md](PODMAN_TIPS.md) for details.

### Kube-proxy Permission Denied

If kube-proxy logs show:
```
Permission denied (you must be root)
```

This means you're not running in rootful mode:
- **Docker Desktop**: Should work automatically (already rootful)
- **Podman**: Must use `sudo podman-compose up -d` or create rootful machine
- See the [rootful mode section](#podman-linux-only---rootful-mode) above

### Port Already in Use

If port 6443 or 2379 is already in use:

```bash
# Find what's using the port
lsof -i :6443
lsof -i :2379

# Stop conflicting services or change ports in docker-compose.yml
```

### Container Build Fails

```bash
# Docker
docker system prune -a
KUBELET_VOLUMES_PATH=$(pwd)/.rusternetes/volumes docker-compose build

# Podman
sudo podman system prune -a
sudo KUBELET_VOLUMES_PATH=$(pwd)/.rusternetes/volumes podman-compose build
```

### Kubelet Cannot Access Container Runtime

The kubelet needs access to the container runtime socket.

**Docker Desktop:**
- Socket is at `/var/run/docker.sock` (already configured in `docker-compose.yml`)
- Should work automatically

**Podman:**
- Make sure you're running in rootful mode with `sudo`
- Socket path should be `/run/podman/podman.sock`

### etcd Health Check Fails

```bash
# Check etcd logs
podman-compose logs etcd

# Restart etcd
podman-compose restart etcd

# If persistent, clean and restart
podman-compose down -v
podman-compose up -d
```

### Cannot Connect to API Server

```bash
# Check if API server is running
podman-compose ps api-server

# Check logs
podman-compose logs api-server

# Verify etcd is healthy
podman exec rusternetes-etcd etcdctl endpoint health

# Try accessing directly
curl http://localhost:6443/healthz
```

### Code Changes Not Reflected

Container images cache the built binaries. After code changes:

```bash
# Rebuild the specific component
podman-compose build api-server

# Restart with the new image
podman-compose up -d --force-recreate api-server
```

### Clean Slate Restart

When things go wrong, start fresh:

```bash
# Stop everything and remove volumes
make dev-clean

# Or manually:
podman-compose down -v
podman system prune -a

# Rebuild and restart
make dev-full
```

## Environment Variables

You can customize the environment by creating a `.env` file:

```bash
# .env file
RUST_LOG=debug
ETCD_SERVERS=http://etcd:2379
API_SERVER_ADDRESS=0.0.0.0:6443
```

### Optional Security Configuration (Production Features)

For **local development**, these are **optional**. The system works without them:

**ServiceAccount Token Signing** (Production-only):
```bash
# Generate signing keys (only needed for production)
./scripts/generate-sa-signing-key.sh

# Configure controller-manager (optional for dev)
export SA_SIGNING_KEY_PATH=~/.rusternetes/keys/sa-signing-key.pem
```

> **Note**: Without a signing key, the controller-manager will log a warning but work fine with unsigned tokens. This is acceptable for development. For production deployments, see [docs/security/service-account-tokens.md](security/service-account-tokens.md).

## Performance Tips

1. **Use Debug Builds for Iteration:**
   ```bash
   cargo build  # Much faster than --release
   ```

2. **Run Components Locally During Development:**
   - Keep etcd in a container
   - Run the component you're working on locally
   - Run other components in containers

3. **Use Cargo Watch for Auto-Rebuild:**
   ```bash
   cargo install cargo-watch
   cargo watch -x 'run --bin api-server'
   ```

4. **Layer Caching:**
   - Dockerfiles are optimized for layer caching
   - Separate dependency builds from code changes

## Next Steps

- Read [GETTING_STARTED.md](GETTING_STARTED.md) for basic usage
- Check [ARCHITECTURE.md](ARCHITECTURE.md) for system design
- Explore example manifests in `examples/`
- Try creating pods, deployments, and services

## Contributing

Before submitting PRs:

```bash
# Run all checks
make pre-commit

# Or manually:
make fmt
make clippy
make test
```

## Additional Resources

- [Podman Documentation](https://docs.podman.io/)
- [Podman Compose](https://github.com/containers/podman-compose)
- [Rust Book](https://doc.rust-lang.org/book/)
- [Kubernetes Architecture](https://kubernetes.io/docs/concepts/architecture/)
