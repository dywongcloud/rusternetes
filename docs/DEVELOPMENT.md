# Rusternetes Local Development Guide

This guide explains how to set up and use the local development environment for Rusternetes using Podman (or Docker).

## Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Development Workflows](#development-workflows)
- [Using Podman Compose](#using-podman-compose)
- [Using the Makefile](#using-the-makefile)
- [Manual Development](#manual-development)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### Required

1. **Rust** (1.75 or later)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Podman** (recommended) or Docker

   **macOS:**
   ```bash
   brew install podman podman-compose
   podman machine init
   podman machine start
   ```

   **Linux (Fedora/RHEL/CentOS):**
   ```bash
   sudo dnf install podman podman-compose
   ```

   **Linux (Ubuntu/Debian):**
   ```bash
   sudo apt-get install podman podman-compose
   ```

3. **podman-compose** or docker-compose
   ```bash
   pip3 install podman-compose
   ```

### Optional

- **make** - For using the Makefile shortcuts
- **etcd** - If you want to run components outside containers

## Quick Start

### Option 1: Interactive Setup Script (Recommended for First-Time Setup)

```bash
./dev-setup.sh
```

This interactive script will:
- Check all prerequisites
- Guide you through building images
- Start the development cluster
- Show you next steps

### Option 2: Using Make (Recommended for Daily Development)

```bash
# Build images and start the cluster
make dev-full

# Or step by step:
make build-images    # Build all container images
make dev-up          # Start the cluster

# View logs
make dev-logs

# Run kubectl commands
make kubectl-get-pods

# Stop the cluster
make dev-down
```

### Option 3: Using Podman Compose Directly

```bash
# Build images
podman-compose build

# Start the cluster
podman-compose up -d

# View logs
podman-compose logs -f

# Stop the cluster
podman-compose down
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
   cargo run --bin kubectl -- --server http://localhost:6443 create -f examples/pod.yaml
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

## Using Podman Compose

The `docker-compose.yml` file defines all services. Here are common operations:

### Start/Stop Services

```bash
# Start all services in background
podman-compose up -d

# Start and follow logs
podman-compose up

# Stop all services
podman-compose down

# Stop and remove volumes (clean slate)
podman-compose down -v
```

### View Logs

```bash
# All services
podman-compose logs -f

# Specific service
podman-compose logs -f api-server
podman-compose logs -f scheduler
podman-compose logs -f kubelet
```

### Check Service Status

```bash
# List running containers
podman-compose ps

# Check specific service health
podman exec rusternetes-api-server /app/api-server --version
```

### Rebuild Services

```bash
# Rebuild all images
podman-compose build

# Rebuild specific service
podman-compose build api-server

# Rebuild and restart
podman-compose up -d --build api-server
```

### Execute Commands in Containers

```bash
# Open shell in container
podman-compose exec api-server /bin/sh

# Run specific command
podman-compose exec api-server ps aux
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

### Podman Machine Not Running (macOS)

```bash
podman machine start
```

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
# Clean and rebuild
make clean
podman system prune -a
make build-images
```

### Kubelet Cannot Access Podman Socket

The kubelet needs access to the container runtime. On macOS with Podman:

```bash
# Make sure Podman machine is running
podman machine start

# Check socket location
podman machine inspect | grep -i socket

# Update docker-compose.yml volume mount if needed
```

For Docker users, change the volume mount in `docker-compose.yml`:
```yaml
volumes:
  - /var/run/docker.sock:/var/run/docker.sock:ro
```

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
