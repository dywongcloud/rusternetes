# Runtime Abstraction Plan: Pluggable Container Runtimes

## Vision

Transform rusternetes from "Kubernetes in Rust that needs Docker" to "Kubernetes in Rust that runs anywhere." A single binary that can orchestrate Docker containers, Podman containers, native host processes, or Wasm modules — selected with a flag.

```bash
rusternetes --runtime docker     # Default. Uses bollard/Docker API.
rusternetes --runtime podman     # Podman socket. Drop-in Docker replacement.
rusternetes --runtime process    # Native host processes. No container runtime needed.
rusternetes --runtime wasm       # WebAssembly modules via wasmtime.
```

## Why This Matters

### The OpenShell Use Case

Nvidia OpenShell needs to run secure sandboxes for AI agents (like Claude) on developer machines. Today that requires k3s + Docker Desktop + container images. With a process-based runtime, a developer could:

1. `cargo install rusternetes` (or download a single binary)
2. `rusternetes --runtime process --data-dir ./cluster.db`
3. `kubectl apply -f agent-sandbox.yaml`
4. An isolated process starts on the host with memory limits, network constraints, and a restricted filesystem

No Docker. No images. No VM. Just a Rust binary managing real workloads with OS-native isolation.

### Broader Impact

- **Edge/IoT**: Devices too small for Docker can run rusternetes with the process runtime
- **CI/CD**: Spin up a K8s API in a GitHub Action without Docker-in-Docker
- **Air-gapped environments**: No container registry needed with process runtime
- **Development**: Test K8s controllers/operators against a real API without any container infrastructure
- **Wasm**: Cloud-native Wasm workloads with Kubernetes semantics

## Current State

### How Docker is Used Today

Docker (via bollard) is concentrated in three places:

| File | Lines | Usage |
|------|-------|-------|
| `crates/kubelet/src/runtime.rs` | ~10,500 | 90% of Docker API calls. `ContainerRuntime` struct wraps `bollard::Docker`. Handles pod lifecycle, container creation, image pulling, probes, volumes, networking. |
| `crates/kubelet/src/main.rs` | ~300 | `handle_exec()` endpoint creates its own Docker connection for exec/attach. |
| `crates/kubelet/src/eviction.rs` | ~600 | `get_pod_stats_async()` creates its own Docker connection for container stats. |

**14 unique Docker API methods used:**
- Container lifecycle: `create_container`, `start_container` (implicit), `stop_container`, `remove_container`
- Discovery: `list_containers`, `inspect_container`
- Images: `create_image` (pull), `inspect_image`
- Exec: `create_exec`, `start_exec`, `inspect_exec`
- Volumes: `list_volumes`, `remove_volume`
- Files: `download_from_container`, `logs`

**Key observation:** `ContainerRuntime` is a concrete struct, not a trait. But it already encapsulates most Docker calls — the abstraction boundary exists, it just isn't formalized.

### What Works Without Docker Today

Running `rusternetes --disable-kubelet --disable-proxy` gives you the full control plane:
- API server (all K8s REST endpoints, watches, CRDs, webhooks, RBAC)
- Scheduler (affinity, taints, preemption — schedules pods but nothing starts them)
- Controller manager (31 controllers reconciling state)
- Web console (topology, metrics, resource management)
- Storage (embedded SQLite)

Pods get scheduled to nodes and stay in `Pending`. This is already useful for API development and testing.

## Implementation Plan

### Phase 1: Define the Trait Interface

**Goal:** Extract a `ContainerRuntime` trait that captures the 14 Docker API methods in runtime-agnostic types.

```rust
// crates/kubelet/src/runtime_trait.rs

#[async_trait]
pub trait ContainerRuntime: Send + Sync + 'static {
    // Container lifecycle
    async fn create_container(&self, config: CreateContainerConfig) -> Result<String>;
    async fn stop_container(&self, id: &str, timeout: Option<Duration>) -> Result<()>;
    async fn remove_container(&self, id: &str, force: bool) -> Result<()>;
    async fn list_containers(&self, filters: ListContainerFilters) -> Result<Vec<ContainerSummary>>;
    async fn inspect_container(&self, id: &str) -> Result<ContainerInspectInfo>;

    // Images
    async fn pull_image(&self, image: &str) -> Result<()>;
    async fn inspect_image(&self, image: &str) -> Result<ImageInfo>;

    // Exec
    async fn exec_create(&self, container_id: &str, cmd: Vec<String>, opts: ExecConfig) -> Result<String>;
    async fn exec_start(&self, exec_id: &str) -> Result<ExecOutput>;
    async fn exec_inspect(&self, exec_id: &str) -> Result<ExecInspectInfo>;

    // Volumes
    async fn list_volumes(&self) -> Result<Vec<VolumeInfo>>;
    async fn remove_volume(&self, name: &str) -> Result<()>;

    // Logs
    async fn container_logs(&self, id: &str, opts: LogOptions) -> Result<Vec<u8>>;

    // Stats
    async fn container_stats(&self, id: &str) -> Result<ContainerStats>;

    // Files
    async fn download_from_container(&self, id: &str, path: &str) -> Result<Vec<u8>>;
}
```

**Runtime-agnostic types** (not bollard types):

```rust
pub struct CreateContainerConfig {
    pub name: String,
    pub image: String,
    pub command: Option<Vec<String>>,
    pub args: Option<Vec<String>>,
    pub env: Vec<(String, String)>,
    pub labels: HashMap<String, String>,
    pub ports: Vec<PortMapping>,
    pub volumes: Vec<VolumeMount>,
    pub network_mode: Option<String>,
    pub memory_limit: Option<u64>,
    pub cpu_quota: Option<i64>,
    pub cpu_period: Option<i64>,
    pub security_opts: Vec<String>,
    pub privileged: bool,
    pub user: Option<String>,
    pub working_dir: Option<String>,
    pub hostname: Option<String>,
    pub dns: Vec<String>,
    pub sysctls: HashMap<String, String>,
    pub ipc_mode: Option<String>,
    pub pid_mode: Option<String>,
}

pub struct ContainerSummary {
    pub id: String,
    pub names: Vec<String>,
    pub image: String,
    pub state: ContainerState,
    pub labels: HashMap<String, String>,
    pub created: i64,
}

pub enum ContainerState {
    Created,
    Running,
    Paused,
    Restarting,
    Removing,
    Exited(i64), // exit code
    Dead,
}

pub struct ContainerStats {
    pub cpu_usage_nanocores: u64,
    pub memory_usage_bytes: u64,
    pub memory_working_set_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}
```

**Files to create:**
- `crates/kubelet/src/runtime_trait.rs` — trait + types
- `crates/kubelet/src/runtime_docker.rs` — `DockerRuntime` impl wrapping bollard

**Files to modify:**
- `crates/kubelet/src/runtime.rs` — change `ContainerRuntime` to use trait, `self.docker` becomes `self.runtime`
- `crates/kubelet/src/kubelet.rs` — make generic over `R: ContainerRuntime`
- `crates/kubelet/src/main.rs` — fix exec handler to use trait
- `crates/kubelet/src/eviction.rs` — fix stats to use trait
- `crates/kubelet/src/lib.rs` — expose runtime selection

**Estimated effort:** 2-3 days. No behavior change — pure refactor.

### Phase 2: DockerRuntime Implementation

**Goal:** Wrap all existing bollard calls in `DockerRuntime` struct that implements `ContainerRuntime`.

This is mostly moving code from `runtime.rs` into `runtime_docker.rs` and converting between bollard types and the new runtime-agnostic types.

The `bollard` dependency becomes optional behind a `docker` feature flag:

```toml
[features]
default = ["docker"]
docker = ["bollard"]
```

**Estimated effort:** 1-2 days. Should be mechanical.

### Phase 3: PodmanRuntime Implementation

**Goal:** Add Podman support via its Docker-compatible API.

Podman exposes a Docker-compatible REST API on a Unix socket. The implementation is nearly identical to `DockerRuntime` but connects to a different socket:

```rust
pub struct PodmanRuntime {
    docker: bollard::Docker, // Podman's API is Docker-compatible
}

impl PodmanRuntime {
    pub fn new() -> Result<Self> {
        let socket = std::env::var("PODMAN_SOCKET")
            .unwrap_or_else(|_| format!("/run/user/{}/podman/podman.sock", unsafe { libc::getuid() }));
        let docker = bollard::Docker::connect_with_socket(&socket, 120, bollard::API_DEFAULT_VERSION)?;
        Ok(Self { docker })
    }
}
```

Most methods delegate directly to the same bollard calls as DockerRuntime. The differences are minor (rootless paths, some API quirks).

**Estimated effort:** 1 day.

### Phase 4: ProcessRuntime — The Game Changer

**Goal:** Run pods as isolated host processes without any container runtime.

This is the key differentiator. Instead of creating Docker containers, the kubelet:

1. **Forks a child process** for each container in the pod
2. **Applies OS-level isolation:**
   - Linux: unshare namespaces (PID, NET, MNT, UTS, IPC), cgroups v2 for resource limits, seccomp filters, chroot/pivot_root
   - macOS: sandbox-exec profiles, process resource limits via `setrlimit`
3. **Sets up networking:** veth pairs + network namespace (Linux) or loopback-only (macOS)
4. **Manages the process lifecycle:** start, stop (SIGTERM → SIGKILL), restart, health checks

```rust
pub struct ProcessRuntime {
    processes: Arc<RwLock<HashMap<String, ProcessContainer>>>,
    work_dir: PathBuf, // base directory for container filesystems
}

struct ProcessContainer {
    id: String,
    pid: u32,
    state: ContainerState,
    config: CreateContainerConfig,
    started_at: Option<Instant>,
}
```

**What a "container" looks like in process mode:**

```
{work_dir}/{container_id}/
├── rootfs/          # chroot root (or just the working directory)
├── proc/            # proc mount (Linux)
├── env              # environment variables
├── stdout.log       # captured stdout
├── stderr.log       # captured stderr
└── pid              # PID file
```

**Container creation flow:**

```rust
async fn create_container(&self, config: CreateContainerConfig) -> Result<String> {
    let id = generate_id();
    let work_dir = self.work_dir.join(&id);
    fs::create_dir_all(&work_dir)?;

    // Write environment
    let env_file = work_dir.join("env");
    write_env_file(&env_file, &config.env)?;

    // The "image" in process mode is a binary path or a command
    // e.g., image: "/usr/bin/python3" or image: "node"
    let binary = resolve_binary(&config.image)?;

    let container = ProcessContainer {
        id: id.clone(),
        pid: 0,
        state: ContainerState::Created,
        config,
        started_at: None,
    };

    self.processes.write().await.insert(id.clone(), container);
    Ok(id)
}
```

**Starting a container (Linux):**

```rust
async fn start_container(&self, id: &str) -> Result<()> {
    let mut processes = self.processes.write().await;
    let container = processes.get_mut(id).ok_or(Error::NotFound)?;

    let child = Command::new(&container.config.image)
        .args(&container.config.args.unwrap_or_default())
        .envs(container.config.env.iter().map(|(k, v)| (k, v)))
        .current_dir(&self.work_dir.join(id))
        .stdout(File::create(self.work_dir.join(id).join("stdout.log"))?)
        .stderr(File::create(self.work_dir.join(id).join("stderr.log"))?)
        // Linux: apply namespace isolation
        .pre_exec(|| {
            // New PID namespace
            unshare(CloneFlags::CLONE_NEWPID)?;
            // Resource limits via cgroups
            apply_cgroup_limits(memory_limit, cpu_quota)?;
            Ok(())
        })
        .spawn()?;

    container.pid = child.id();
    container.state = ContainerState::Running;
    container.started_at = Some(Instant::now());

    // Spawn background task to wait for process exit
    tokio::spawn(watch_process_exit(id.to_string(), child, self.processes.clone()));

    Ok(())
}
```

**Key design decisions:**

| Aspect | Docker | Process |
|--------|--------|---------|
| Image format | OCI image layers | Binary path on host |
| Isolation | namespaces + cgroups + overlay fs | namespaces + cgroups (Linux) or sandbox-exec (macOS) |
| Networking | Docker bridge / CNI | veth pairs (Linux) or host networking |
| Storage | overlay2 filesystem | directory on host |
| Pull | Registry download | No-op (binary already on host) or `curl` |
| Logs | Docker log driver | stdout/stderr files |
| Health checks | Docker healthcheck | Same probe logic, just different exec path |

**Estimated effort:** 3-5 days for Linux, +2 days for macOS support.

### Phase 5: WasmRuntime (Future)

**Goal:** Run WebAssembly modules as pods.

```rust
pub struct WasmRuntime {
    engine: wasmtime::Engine,
    instances: Arc<RwLock<HashMap<String, WasmInstance>>>,
}
```

The "image" is a `.wasm` file. The runtime compiles and instantiates it with resource limits (memory pages, fuel metering for CPU). WASI provides filesystem and network access.

This is the most portable option — works on any OS, any architecture, with maximum sandboxing.

**Estimated effort:** 5-7 days.

### Phase 6: Runtime Selection in All-in-One Binary

```rust
// crates/rusternetes/src/main.rs

#[arg(long, default_value = "docker")]
runtime: String,

// ...

let runtime: Arc<dyn ContainerRuntime> = match args.runtime.as_str() {
    "docker" => Arc::new(DockerRuntime::new()?),
    "podman" => Arc::new(PodmanRuntime::new()?),
    "process" => Arc::new(ProcessRuntime::new(args.work_dir)?),
    "wasm" => Arc::new(WasmRuntime::new()?),
    "none" => {
        info!("No container runtime — pods will not start");
        // disable kubelet entirely
    }
    other => bail!("Unknown runtime: {}", other),
};
```

## Migration Path

### Step 1: Non-breaking trait extraction (Phase 1-2)
- Define trait and types
- Wrap existing Docker code as `DockerRuntime`
- All tests pass, behavior unchanged
- Merge to main

### Step 2: Process runtime MVP (Phase 4, Linux only)
- Fork + exec with basic namespace isolation
- Logs via stdout/stderr files
- Resource limits via cgroups v2
- Health checks work (exec probes call into the process)
- Merge behind `--runtime process` flag

### Step 3: Podman (Phase 3)
- Nearly free since it's Docker-compatible
- Merge behind `--runtime podman` flag

### Step 4: macOS process support
- sandbox-exec for isolation
- setrlimit for resource limits
- No network namespace (host networking only)

### Step 5: Wasm (Phase 5)
- wasmtime integration
- WASI for filesystem/network
- Merge behind `--runtime wasm` flag

## What This Enables for OpenShell

```bash
# Developer installs rusternetes (single binary, no dependencies)
cargo install rusternetes

# Starts a local cluster with process-based runtime
rusternetes --runtime process --data-dir ./cluster.db

# Deploys an AI agent sandbox
cat <<EOF | kubectl apply -f -
apiVersion: apps/v1
kind: Deployment
metadata:
  name: claude-sandbox
spec:
  replicas: 1
  selector:
    matchLabels:
      app: claude-sandbox
  template:
    metadata:
      labels:
        app: claude-sandbox
    spec:
      containers:
      - name: agent
        image: /usr/local/bin/agent-runner
        args: ["--model", "claude-sonnet", "--sandbox"]
        resources:
          limits:
            memory: "2Gi"
            cpu: "2"
        securityContext:
          readOnlyRootFilesystem: true
          allowPrivilegeEscalation: false
EOF

# The agent runs as an isolated host process with:
# - 2GB memory limit (cgroups)
# - 2 CPU cores (cgroups)
# - Read-only filesystem
# - No privilege escalation
# - Network policy enforcement
# - Full K8s lifecycle management (health checks, restarts, scaling)
```

The developer gets Kubernetes semantics (declarative, self-healing, scalable) without Kubernetes infrastructure (Docker, containerd, images, registries). The console shows topology, metrics, logs — everything works because the kubelet speaks the same internal API regardless of whether the "container" is a Docker container or a host process.

## Open Questions

1. **Image format for process runtime**: Should `image` be a binary path, a URL to download, or something else? Could support both: `/path/to/binary` for local, `https://...` for remote.

2. **Networking for process runtime**: Full network namespace isolation (Linux only) or simplified host networking with port allocation?

3. **Pause container equivalent**: Docker uses a pause container to hold the network namespace. Process runtime could use a lightweight process or just manage namespaces directly.

4. **Volume semantics**: Docker volumes are overlay mounts. Process runtime volumes could be bind mounts or symlinks.

5. **Init containers**: In process mode, these are just sequential process executions before the main process starts.

6. **Probes**: Exec probes work naturally (run a command). HTTP/TCP probes work if the process is listening. No change needed.

## Files

Key files that will be modified or created:

```
crates/kubelet/
├── src/
│   ├── runtime_trait.rs       NEW — ContainerRuntime trait + types
│   ├── runtime_docker.rs      NEW — DockerRuntime (existing bollard code)
│   ├── runtime_podman.rs      NEW — PodmanRuntime
│   ├── runtime_process.rs     NEW — ProcessRuntime
│   ├── runtime_wasm.rs        NEW — WasmRuntime (future)
│   ├── runtime.rs             MODIFY — becomes generic, uses trait
│   ├── kubelet.rs             MODIFY — generic over R: ContainerRuntime
│   ├── main.rs                MODIFY — runtime selection, fix exec handler
│   ├── eviction.rs            MODIFY — use trait for stats
│   └── lib.rs                 MODIFY — expose runtime selection
└── Cargo.toml                 MODIFY — feature flags for each runtime

crates/rusternetes/
└── src/main.rs                MODIFY — --runtime flag
```
