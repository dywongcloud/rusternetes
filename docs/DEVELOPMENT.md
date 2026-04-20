# Rūsternetes Development Guide

How to build, test, and run Rusternetes locally.

## Prerequisites

- **Rust** (latest stable, via [rustup](https://rustup.rs))
- **Container runtime** -- see [Container Runtime Setup](#container-runtime-setup) below

## Project Structure

Rusternetes is a Cargo workspace with 10 crates (216,000+ lines of Rust, 3,100+ tests):

| Crate | Purpose |
|-------|---------|
| `crates/common` | Shared resource types (Pod, Service, Deployment, etc.), errors, utilities |
| `crates/api-server` | Axum-based REST API with 75+ handler files and router.rs |
| `crates/storage` | Pluggable storage: etcd, SQLite (rhino), and in-memory backends |
| `crates/controller-manager` | 31 reconciliation controllers |
| `crates/kubelet` | Node agent, Docker container runtime via bollard |
| `crates/kube-proxy` | iptables-based service routing (host network mode) |
| `crates/scheduler` | Pod scheduling with affinity, taints, priority/preemption plugins |
| `crates/kubectl` | CLI tool |
| `crates/cloud-providers` | AWS/GCP/Azure integrations |
| `crates/rusternetes` | All-in-one binary (all components as concurrent tokio tasks) |

## Build and Test

```bash
# Build
cargo build                    # Debug build (fast iteration)
cargo build --release          # Release build

# Test
cargo test                     # All workspace tests
cargo test -p rusternetes_api_server  # Single crate (use underscores in package name)
cargo test test_name           # Single test by name
cargo test test_name -- --nocapture  # With stdout

# Lint and format
cargo fmt --all                # Format all code
cargo clippy --all-targets --all-features -- -D warnings  # Lint

# Pre-commit (format + clippy + test)
make pre-commit
```

## Container Runtime Setup

The cluster runs 7 services: etcd, api-server (port 6443 with TLS), scheduler,
controller-manager, two kubelets (node-1, node-2), and kube-proxy. kube-proxy
requires `CAP_NET_ADMIN` for iptables, which means rootful container execution.

### macOS -- Docker Desktop

Docker Desktop is the recommended runtime on macOS. Podman Machine has known
issues with macOS Sequoia 15.7+ where the Apple Virtualization Framework
prevents VMs from starting.

```bash
# Install Docker Desktop
brew install --cask docker
# Then start Docker Desktop from Applications

# Verify
docker info
docker compose version
```

### Linux -- Docker Engine

```bash
# Install Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
# Log out and back in for group change to take effect

# Verify
docker info
docker compose version
```

### Linux -- Podman (rootful mode required)

Podman works on Linux but must run in rootful mode for kube-proxy iptables access.
All `docker compose` commands below become `sudo podman-compose` commands.

```bash
# Fedora/RHEL/CentOS
sudo dnf install podman podman-compose

# Ubuntu/Debian
sudo apt-get install podman podman-compose

# All commands must run as root
sudo KUBELET_VOLUMES_PATH=$(pwd)/.rusternetes/volumes podman-compose build
sudo KUBELET_VOLUMES_PATH=$(pwd)/.rusternetes/volumes podman-compose up -d
```

The rest of this guide uses `docker compose` syntax. If you are using Podman,
substitute `sudo podman-compose` wherever you see `docker compose`.

## Running the Cluster

### Start

```bash
export KUBELET_VOLUMES_PATH=$(pwd)/.rusternetes/volumes

docker compose build           # Build images (~10-15 min first time)
docker compose up -d           # Start all services
bash scripts/bootstrap-cluster.sh  # Create CoreDNS, services, SA tokens
```

### KUBECONFIG

```bash
export KUBECONFIG=~/.kube/rusternetes-config
kubectl get nodes
kubectl get pods -A
```

### Logs and Status

```bash
docker compose ps              # List running containers
docker compose logs -f         # All service logs
docker compose logs -f api-server  # Single service
```

### Rebuild a Single Service

```bash
docker compose build api-server
docker compose up -d api-server
```

### Stop

```bash
docker compose down            # Stop services
docker compose down -v         # Stop and remove volumes (clean slate)
```

## Development Workflow

1. Make code changes
2. `cargo build` to verify compilation
3. `cargo test` to run tests
4. `docker compose build <service>` to rebuild the image for the changed crate
5. `docker compose up -d <service>` to restart it
6. `docker compose logs -f <service>` to verify

## Conformance Testing

```bash
bash scripts/run-conformance.sh       # Full conformance lifecycle
bash scripts/conformance-progress.sh  # Monitor pass/fail progress
```

E2e output is at `/tmp/sonobuoy/results/e2e.log` inside the e2e container. Save logs before cleanup:

```bash
docker cp "$E2E_CONTAINER:/tmp/sonobuoy/results/e2e.log" /tmp/e2e-roundXXX.log
```

## Adding a New Resource

1. Define the struct in `crates/common/src/resources/{type}.rs`
2. Add handlers in `crates/api-server/src/handlers/{type}.rs`
3. Register routes in `crates/api-server/src/router.rs`
4. Add a controller in `crates/controller-manager/src/controllers/` if needed
5. Add tests

## Key Conventions

### Serialization (critical for Kubernetes API compatibility)

- All resource structs use `#[serde(rename_all = "camelCase")]`
- Optional fields use `#[serde(skip_serializing_if = "Option::is_none")]`
- TypeMeta is flattened: `#[serde(flatten)] pub type_meta: TypeMeta`
- camelCase abbreviations follow Kubernetes style: `podIP`, `hostIP`, `containerID`

### Controller Pattern

```rust
pub struct FooController<S: Storage> {
    storage: Arc<S>,
    interval: Duration,
}

impl<S: Storage> FooController<S> {
    pub async fn run(&self) -> Result<()> {
        loop {
            self.reconcile_all().await?;
            tokio::time::sleep(self.interval).await;
        }
    }
}
```

### Testing

- Async tests use `#[tokio::test]`
- Use `MemoryStorage` (not etcd) for unit tests
- Use `#[serial_test::serial]` when tests share mutable state

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):
`feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `chore:`

## Cluster Architecture

```
Docker Network (rusternetes-network)

  etcd          api-server (6443, TLS)    scheduler
  controller-manager    node-1 (kubelet)    node-2 (kubelet)

Host Network:
  kube-proxy (CAP_NET_ADMIN for iptables)
```

- TLS certs are in `.rusternetes/certs/`, generated by `scripts/generate-certs.sh`
- Cert SANs include Docker bridge IPs (172.18.0.2-5)
- CoreDNS ClusterIP is pinned to 10.96.0.10
- Pods use Docker bridge networking; containers use `container:pause` network mode
- kube-proxy requires `CAP_NET_ADMIN` for iptables rules

## Troubleshooting

### Port Already in Use

```bash
lsof -i :6443
lsof -i :2379
```

Stop conflicting services or change ports in `docker-compose.yml`.

### Container Build Fails

```bash
# Docker
docker system prune -a
docker compose build

# Podman
sudo podman system prune -a
sudo podman-compose build
```

### Podman: kube-proxy Permission Denied

If kube-proxy logs show `Permission denied (you must be root)`, you are not
running in rootful mode. Use `sudo podman-compose` for all commands.

### Podman Machine Fails on macOS

If you see `VZErrorDomain Code=1` or `vfkit exited unexpectedly`, this is a
known issue with macOS Sequoia 15.7+. Use Docker Desktop instead.

### etcd Issues

```bash
docker compose logs etcd
docker compose restart etcd

# If persistent, clean and restart
docker compose down -v
docker compose up -d
```

### API Server Unreachable

```bash
docker compose ps api-server
docker compose logs api-server
curl -k https://localhost:6443/healthz
```

### Code Changes Not Reflected

Container images cache built binaries. Rebuild after changes:

```bash
docker compose build api-server
docker compose up -d --force-recreate api-server
```

### Clean Slate

```bash
docker compose down -v
docker system prune -a
docker compose build
docker compose up -d
bash scripts/bootstrap-cluster.sh
```
