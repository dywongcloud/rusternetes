# Storage Backends

Rusternetes supports pluggable storage backends. Every component — API server,
scheduler, controller manager, kubelet, kube-proxy — accepts a `--storage-backend`
flag to select the backend at startup.

| Backend | Flag value | Persistence | External deps | Use case |
|---------|-----------|-------------|---------------|----------|
| etcd | `etcd` (default) | Yes | etcd cluster | Production, HA, multi-node |
| SQLite | `sqlite` | Yes | None | All-in-one, dev, edge, CI |

---

## Architecture

All storage access flows through the `Storage` trait defined in `crates/storage/src/lib.rs`.
The `StorageBackend` enum dispatches to the concrete implementation chosen at startup:

```
                          --storage-backend=etcd
                         /
    StorageBackend::new()
                         \
                          --storage-backend=sqlite

    +-----------------+          +------------------+
    | StorageBackend  |          | StorageBackend   |
    |   ::Etcd        |          |   ::Sqlite       |
    |                 |          |                  |
    | EtcdStorage     |          | RhinoStorage     |
    |   etcd-client   |          |   SqliteBackend  |
    |   gRPC to etcd  |          |   in-process     |
    +-----------------+          +------------------+
           |                             |
           v                             v
     external etcd              SQLite file on disk
     cluster (2379)             (e.g. ./data/cluster.db)
```

The SQLite backend uses [rhino](https://github.com/calfonso/rhino) — a Rust
reimplementation of the kine project — embedded as a library dependency. There
is **no gRPC hop** for the SQLite path: rusternetes calls rhino's `Backend`
trait directly in-process.

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
api-server --etcd-servers http://etcd:2379
scheduler --etcd-servers http://etcd:2379
controller-manager --etcd-servers http://etcd:2379
kubelet --node-name node-1 --etcd-servers http://etcd:2379
kube-proxy --node-name node-1 --etcd-servers http://etcd:2379
```

### SQLite (all-in-one)

Point all components at the same SQLite database file. No external etcd
needed.

```bash
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

## Rhino Dependency

Rhino is included as a path dependency at `../../../rhino` (relative to the
storage crate). This assumes the following directory layout:

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
  writes. This is fine for single-node deployments but would bottleneck under
  heavy multi-node write load.
- **Leader election requires etcd**: Even with SQLite storage, leader election
  still needs an etcd cluster. This is a non-issue for the primary use case
  (single-node, no HA).

---

## Related

- [Rhino](https://github.com/calfonso/rhino) — the SQLite/SQL-backed etcd shim
- [kine](https://github.com/k3s-io/kine) — the Go project rhino is inspired by
- [CSI Integration](csi-integration.md) — volume plugin storage (separate concern)
