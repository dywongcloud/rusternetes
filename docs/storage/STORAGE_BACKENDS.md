# Storage Backends

Rusternetes supports three deployment modes with different storage backends.
The same component binaries work in all modes — the only difference is
configuration flags and which compose file you use.

| Mode | Storage | Compose file | External deps | Use case |
|------|---------|-------------|---------------|----------|
| Normal | etcd | `docker-compose.yml` | etcd cluster | Production, HA, multi-node |
| SQLite (normal) | rhino (gRPC) | `docker-compose.sqlite.yml` | rhino container | Multi-container without etcd |
| SQLite (embedded) | rhino (in-process) | — | None | All-in-one single binary |

---

## Deployment Modes

### 1. Normal mode with etcd (default)

The standard deployment. Each component runs in its own container. etcd
provides distributed consensus-based storage.

```bash
docker compose build
docker compose up -d
bash scripts/bootstrap-cluster.sh
```

### 2. Normal mode with SQLite via rhino

Same multi-container architecture, but rhino replaces etcd. Rhino is an
etcd-compatible gRPC server backed by SQLite — it speaks the exact same
etcd v3 API, so components use their existing `--etcd-servers` flag pointed
at `http://rhino:2379`. **No recompilation or feature flags needed.**

```bash
docker compose -f docker-compose.sqlite.yml build
docker compose -f docker-compose.sqlite.yml up -d
bash scripts/bootstrap-cluster.sh
```

This mode uses `Dockerfile.rhino` to build the rhino-server container and
requires the [rhino](https://github.com/calfonso/rhino) repo adjacent to
this one:

```
dev/
  rusternetes/   (this repo)
  rhino/         (https://github.com/calfonso/rhino)
```

### 3. Embedded SQLite (all-in-one)

All components run as tokio tasks in a single process with rhino's
`SqliteBackend` embedded directly — no gRPC, no network, pure in-process
Rust calls. Requires building with `--features sqlite`.

```bash
cargo build --features sqlite
./target/debug/rusternetes --storage-backend sqlite --data-dir ./cluster.db
```

This mode is not yet implemented as a unified binary but the storage
plumbing is in place. Each individual binary can be run with
`--storage-backend sqlite` today.

---

## Architecture

All storage access flows through the `Storage` trait defined in `crates/storage/src/lib.rs`.

**Normal mode (etcd or rhino gRPC):**

```
    Component  --etcd-servers-->  EtcdStorage  --gRPC-->  etcd (or rhino)
```

Components use the `etcd-client` crate to talk to either real etcd or rhino
over gRPC. From the component's perspective, there is no difference.

**Embedded mode (in-process SQLite):**

```
    Component  --storage-backend=sqlite-->  RhinoStorage  --direct-->  SQLite file
```

The `StorageBackend` enum dispatches to the concrete implementation:

```
    +-----------------+          +------------------+
    | StorageBackend  |          | StorageBackend   |
    |   ::Etcd        |          |   ::Sqlite       |
    |                 |          |                  |
    | EtcdStorage     |          | RhinoStorage     |
    |   etcd-client   |          |   SqliteBackend  |
    |   gRPC          |          |   in-process     |
    +-----------------+          +------------------+
           |                             |
           v                             v
     etcd or rhino              SQLite file on disk
     (network, :2379)           (e.g. ./data/cluster.db)
```

---

## Feature Flag

SQLite support is behind the `sqlite` Cargo feature to keep the default build
lean. When disabled, the SQLite code and all sqlx/rhino dependencies are
excluded entirely.

```bash
# Build with SQLite support
cargo build --features sqlite

# Build without (default — etcd only)
cargo build
```

The feature propagates through the crate graph:

```
rusternetes-api-server/sqlite
  -> rusternetes-storage/sqlite
    -> dep:rhino (path = "../../../rhino")
```

Every binary crate (api-server, scheduler, controller-manager, kubelet,
kube-proxy) defines a `sqlite` feature that forwards to
`rusternetes-storage/sqlite`.

---

## Usage

### etcd (default)

No changes from the existing deployment model. etcd is the default backend
when `--storage-backend` is omitted.

```bash
# Docker Compose
docker compose up -d

# Or run binaries directly
api-server --etcd-servers http://etcd:2379
scheduler --etcd-servers http://etcd:2379
controller-manager --etcd-servers http://etcd:2379
kubelet --node-name node-1 --etcd-servers http://etcd:2379
kube-proxy --node-name node-1 --etcd-servers http://etcd:2379
```

### SQLite via rhino (normal multi-container)

Use `docker-compose.sqlite.yml` to swap etcd for rhino. Components point
their `--etcd-servers` flag at rhino instead of etcd. Same binaries, no
recompilation.

```bash
docker compose -f docker-compose.sqlite.yml build
docker compose -f docker-compose.sqlite.yml up -d
bash scripts/bootstrap-cluster.sh
```

Or run rhino and the binaries directly:

```bash
# Start rhino (from the rhino repo)
rhino-server --listen-address 0.0.0.0:2379 --endpoint ./cluster.db

# Point components at rhino
api-server --etcd-servers http://localhost:2379
scheduler --etcd-servers http://localhost:2379
# ... etc
```

### SQLite embedded (all-in-one, single process)

Build with the `sqlite` feature and use `--storage-backend sqlite`. Each
component embeds rhino's SQLite backend directly — no network, no gRPC.

```bash
cargo build --features sqlite

api-server --storage-backend sqlite --data-dir ./data/cluster.db
scheduler --storage-backend sqlite --data-dir ./data/cluster.db
controller-manager --storage-backend sqlite --data-dir ./data/cluster.db
kubelet --node-name node-1 --storage-backend sqlite --data-dir ./data/cluster.db
kube-proxy --node-name node-1 --storage-backend sqlite --data-dir ./data/cluster.db
```

The database file is created automatically if it does not exist. The parent
directory is also created.

---

## CLI Flags

Every binary accepts these additional flags:

| Flag | Default | Description |
|------|---------|-------------|
| `--storage-backend` | `etcd` | `etcd` or `sqlite` |
| `--data-dir` | `./data/rusternetes.db` | SQLite database file path (ignored when backend is etcd) |
| `--etcd-servers` | `http://localhost:2379` | etcd endpoints, comma-separated (ignored when backend is sqlite) |

---

## How It Works

### Key mapping

Both backends use the same key schema: `/registry/{resource_type}/{namespace}/{name}`.
Resource versions map to monotonically increasing revision numbers — etcd's
`mod_revision` for the etcd backend, SQLite row IDs for the sqlite backend.
This is important: resource versions are **backend-specific integers**, not
portable across backends.

### Watch support

Both backends support the full watch API including `watch_from_revision`.

- **etcd**: Uses etcd's native gRPC watch streams with `prev_kv` for delete
  events.
- **SQLite**: Rhino's poll loop detects new rows in the `kine` table and
  broadcasts events via `tokio::sync::broadcast` channels. Historical replay
  is supported by querying rows with `id > revision`.

### Optimistic concurrency

Both backends enforce the Kubernetes resource version contract. Updates with a
stale `resourceVersion` are rejected with a 409 Conflict. The mechanism
differs:

- **etcd**: Compare-and-swap transactions checking `mod_revision`.
- **SQLite**: The `kine` table's `(name, prev_revision)` unique index
  prevents concurrent updates to the same key at the same revision.

### Compaction

- **etcd**: Managed by etcd's built-in compaction.
- **SQLite**: Rhino runs a background compaction loop (default every 300s)
  that removes superseded revisions, keeping at least the most recent 1000.
  After compaction, a `PRAGMA wal_checkpoint(FULL)` reclaims disk space.

---

## Crate Structure

```
crates/storage/
  src/
    lib.rs          # Storage trait, StorageConfig, StorageBackend enum
    etcd.rs         # EtcdStorage — etcd-client gRPC implementation
    rhino.rs        # RhinoStorage — direct rhino::Backend implementation (behind sqlite feature)
    memory.rs       # MemoryStorage — in-memory for unit tests
    concurrency.rs  # resourceVersion <-> mod_revision conversion
  Cargo.toml        # rhino = { optional = true }, [features] sqlite = ["dep:rhino"]
```

### StorageConfig

```rust
pub enum StorageConfig {
    Etcd { endpoints: Vec<String> },
    #[cfg(feature = "sqlite")]
    Sqlite { path: String },
}
```

### StorageBackend

```rust
pub enum StorageBackend {
    Etcd(EtcdStorage),
    #[cfg(feature = "sqlite")]
    Sqlite(RhinoStorage),
}

impl Storage for StorageBackend { /* dispatches to inner */ }
impl AuthzStorage for StorageBackend { /* dispatches to inner */ }
```

Components that previously used `Arc<EtcdStorage>` now use
`Arc<StorageBackend>`. The `StorageBackend` implements both `Storage` and
`AuthzStorage`, so no other code changes were needed.

---

## Leader Election

Leader election (used by the scheduler and controller-manager for HA) uses etcd
leases directly and is **independent of the storage backend**. When running in
all-in-one mode with SQLite, leader election is typically disabled
(`--enable-leader-election` is not set) since there is only one instance.

If leader election is needed, the `--etcd-servers` flag is still parsed to
connect the `LeaderElector` to an etcd cluster, even when the storage backend
is SQLite.

---

## Rhino gRPC Mode (docker-compose.sqlite.yml)

The simplest way to use SQLite in a normal multi-container deployment.
Rhino replaces etcd as a drop-in: same gRPC API, backed by SQLite.

**Files:**
- `Dockerfile.rhino` — builds the rhino-server binary from the adjacent repo
- `docker-compose.sqlite.yml` — full cluster with rhino instead of etcd

**How it works:** Components use their existing `--etcd-servers` flag pointed
at `http://rhino:2379`. The `etcd-client` crate in `EtcdStorage` connects to
rhino's tonic gRPC server, which translates operations to SQLite queries.
Watch streams work via rhino's poll loop (1-second intervals).

**Advantages over the embedded approach:**
- No feature flags or recompilation needed — same binaries as etcd mode
- Watches work correctly across process boundaries via gRPC streaming
- Can inspect cluster state with `sqlite3 /data/db/state.db`

**Trade-off:** One extra container (rhino) vs. zero containers for embedded.

---

## Rhino Library Dependency (embedded mode)

Rhino is included as a git dependency in the storage crate. This assumes the following directory layout for local development:

```
dev/
  rusternetes/    # this repo
  rhino/          # https://github.com/calfonso/rhino
```

Rhino provides three database backends (SQLite, PostgreSQL, MySQL) behind its
`Backend` trait. Rusternetes currently uses `SqliteBackend` only. Adding
PostgreSQL or MySQL support would require adding new `StorageConfig` variants
and corresponding `StorageBackend` arms — the plumbing is identical.

### Key rhino details

- **Schema**: Log-structured `kine` table with monotonic row IDs as revisions
- **WAL mode**: SQLite runs in WAL journal mode for concurrent reads during writes
- **Busy timeout**: 30 seconds (handles brief lock contention gracefully)
- **Connection pool**: 5 connections via sqlx
- **Gap filling**: Revision gaps (from rolled-back transactions) are detected
  and filled with placeholder records to maintain sequential ordering

---

## Limitations

- **No cross-backend migration**: Data cannot be moved between etcd and SQLite
  without a manual export/import process. Resource versions are not portable.
- **Single-writer for SQLite**: SQLite supports concurrent reads but serializes
  writes. This is fine for single-node and small multi-container deployments
  but would bottleneck under heavy multi-node write load.
- **Leader election requires etcd**: Even with embedded SQLite storage, leader
  election still needs an etcd cluster. This is a non-issue for the primary
  use case (single-node, no HA). In rhino gRPC mode, leader election could
  use rhino's etcd-compatible lease API.
- **Embedded watch latency**: When multiple processes share a SQLite file
  directly (embedded mode across containers), watch notifications rely on
  rhino's 1-second poll interval rather than instant gRPC streaming. Use
  the rhino gRPC mode (`docker-compose.sqlite.yml`) for multi-container
  deployments to get proper streaming watches.

---

## Related

- [Rhino](https://github.com/calfonso/rhino) — the SQLite/SQL-backed etcd shim
- [kine](https://github.com/k3s-io/kine) — the Go project rhino is inspired by
- [CSI Integration](csi-integration.md) — volume plugin storage (separate concern)
