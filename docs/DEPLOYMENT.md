# Deployment Guide

Rusternetes supports three deployment modes from the same codebase. Choose based on your needs.

| Mode | Storage | Best For | Console |
|---|---|---|---|
| Docker Compose + etcd | etcd cluster | Production, multi-node, HA | Included |
| Docker Compose + SQLite | SQLite via Rhino | Development, simpler ops | Included |
| All-in-one binary | Embedded SQLite | Edge, CI/CD, single-node, learning | Included |

The web console (`https://localhost:6443/console/`) deploys automatically in all modes.

## Mode 1: Docker Compose + etcd

The standard deployment. Separate containers for each component with etcd for state storage.

```bash
git clone https://github.com/calfonso/rusternetes.git
cd rusternetes

export KUBELET_VOLUMES_PATH=$(pwd)/.rusternetes/volumes
docker compose build
docker compose up -d
bash scripts/bootstrap-cluster.sh

export KUBECONFIG=~/.kube/rusternetes-config
kubectl get nodes
```

**Components started:**
- etcd (port 2379)
- API server (port 6443, HTTPS, with web console)
- Scheduler
- Controller manager (31 controllers)
- 2 kubelets (node-1, node-2)
- Kube-proxy (host network, iptables)
- CoreDNS (10.96.0.10)
- Default StorageClass (`standard` with hostpath provisioner)

**Open the console:** `https://localhost:6443/console/`

## Mode 2: Docker Compose + SQLite

Same cluster architecture, but [Rhino](https://github.com/calfonso/rhino) replaces etcd with SQLite. No etcd infrastructure to manage.

```bash
docker compose -f docker-compose.sqlite.yml build
docker compose -f docker-compose.sqlite.yml up -d
bash scripts/bootstrap-cluster.sh
```

Same components as Mode 1, but storage goes through Rhino's etcd-compatible gRPC API backed by SQLite.

## Mode 3: All-in-One Binary

All five Kubernetes components in a single Rust process with embedded SQLite. No containers for infrastructure — just one binary.

```bash
cargo build -p rusternetes --release

# Build the console (optional)
cd console && npm install && npm run build && cd ..

# Start
./target/release/rusternetes \
  --data-dir ./cluster.db \
  --console-dir ./console/dist
```

This starts the API server, scheduler, controller manager, kubelet, and kube-proxy as concurrent tokio tasks.

**Requirements:** Docker must be running on the host for the kubelet to create containers.

**Open the console:** `https://localhost:6443/console/`

### All-in-one flags

| Flag | Default | Description |
|---|---|---|
| `--data-dir` | `./data/rusternetes.db` | SQLite database path |
| `--bind-address` | `0.0.0.0:6443` | API server listen address |
| `--node-name` | `node-1` | Kubelet node name |
| `--tls` | off | Enable TLS with self-signed cert |
| `--skip-auth` | `true` | Skip authentication (dev mode) |
| `--console-dir` | *(disabled)* | Path to console SPA build |
| `--client-ca-file` | *(disabled)* | Client CA for mTLS auth |
| `--disable-proxy` | off | Disable kube-proxy (no iptables) |

## High Availability

For HA deployments with multiple API servers and etcd nodes, see [HIGH_AVAILABILITY.md](HIGH_AVAILABILITY.md).

```bash
docker compose -f docker-compose.ha.yml build
docker compose -f docker-compose.ha.yml up -d
```

This starts 3 etcd nodes, 3 API servers behind HAProxy, 2 schedulers, and 2 controller managers with leader election.

## TLS Certificates

TLS certificates are auto-generated during the Docker build in `.rusternetes/certs/`. For custom certificates:

```bash
bash scripts/generate-certs.sh
```

See [TLS_GUIDE.md](TLS_GUIDE.md) for custom cert configuration.

## Authentication

By default, `--skip-auth` is enabled (all requests are admin). To secure the cluster:

1. Generate RSA signing keys
2. Create an admin ServiceAccount
3. Remove `--skip-auth`

See [AUTHENTICATION.md](AUTHENTICATION.md) for the complete guide.

## Networking

Default network configuration:
- Service CIDR: `10.96.0.0/12`
- Pod CIDR: `10.244.0.0/16` (when using CNI)
- Cluster DNS: `10.96.0.10`
- Kube-proxy mode: iptables (host network)

Third-party CNI plugins (Calico, Cilium, Flannel) work on Linux. See [CNI_GUIDE.md](CNI_GUIDE.md).

## Storage

A default `standard` StorageClass with `rusternetes.io/hostpath` provisioner is created on API server startup. PVCs referencing this class are automatically provisioned.

See [Storage Backends](storage/STORAGE_BACKENDS.md) for etcd vs SQLite details.

## Stopping and Cleaning Up

```bash
# Stop the cluster (preserves state)
docker compose down

# Stop and wipe all state
docker compose down -v

# Clean up dangling containers from conformance tests
docker ps -a --filter "status=exited" -q | xargs docker rm 2>/dev/null
```

## Next Steps

- **[Console User Guide](CONSOLE_USER_GUIDE.md)** — explore the web console
- **[AUTHENTICATION.md](AUTHENTICATION.md)** — secure the cluster
- **[AWS_DEPLOYMENT.md](AWS_DEPLOYMENT.md)** — deploy on AWS
- **[FEDORA_SETUP.md](FEDORA_SETUP.md)** — Fedora Linux setup
