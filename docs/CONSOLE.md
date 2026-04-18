# Rusternetes Console

The rusternetes console is a web-based dashboard embedded directly in the API server. It provides real-time visibility into cluster resources, workloads, and events — without any external dependencies, proxies, or separate deployments.

## Architecture

The console is a React single-page application served by the Axum API server at `/console/`. Because the SPA and API share the same origin, there is no CORS configuration, no nginx proxy, and no separate deployment. The console calls the standard Kubernetes REST API endpoints (`/api/v1/...`, `/apis/...`) directly.

```
Browser ─── https://localhost:6443/console/
                │
                ├── /console/              Static SPA (Axum ServeDir)
                ├── /api/v1/pods           K8s REST API (same server)
                ├── /api/v1/pods?watch=1   Watch stream (chunked HTTP)
                └── /apis/apps/v1/...      API group resources
```

### Tech Stack

| Layer | Technology |
|-------|-----------|
| Frontend | React 19, TypeScript 5.9 |
| Bundler | Vite |
| State | Zustand (UI state) + TanStack Query (server state) |
| Styling | Tailwind CSS + Radix UI |
| Serving | Axum `tower-http::ServeDir` |

### Project Structure

```
console/
├── package.json
├── vite.config.ts
├── tsconfig.json
├── tailwind.config.js
├── index.html
├── public/
│   └── favicon.svg
└── src/
    ├── main.tsx              Entry point
    ├── App.tsx               Router setup
    ├── engine/               K8s API client layer
    │   ├── types.ts          Resource type interfaces
    │   ├── query.ts          REST CRUD operations
    │   ├── watch.ts          Watch stream with reconnection
    │   ├── discovery.ts      Resource type discovery
    │   └── fleet.ts          Multi-cluster connection
    ├── store/                Zustand state stores
    │   ├── uiStore.ts        Namespace, sidebar state
    │   ├── clusterStore.ts   Discovery registry
    │   └── fleetStore.ts     Multi-cluster state
    ├── hooks/                React hooks
    │   ├── useK8sList.ts     TanStack Query wrapper
    │   └── useDiscovery.ts   Resource discovery hook
    ├── components/           Reusable UI components
    │   ├── Shell.tsx          Root layout
    │   ├── Sidebar.tsx        Navigation
    │   ├── Header.tsx         Namespace + cluster switcher
    │   ├── ClusterSwitcher.tsx Multi-cluster UI
    │   ├── NamespaceSelector.tsx
    │   ├── ResourceTable.tsx  Generic resource table
    │   └── StatusBadge.tsx    Status indicator
    └── views/                Page views
        ├── OverviewView.tsx   Cluster dashboard
        ├── WorkloadsView.tsx  Pods, Deployments, etc.
        ├── NetworkingView.tsx Services, Endpoints, etc.
        ├── NodesView.tsx      Node status & capacity
        ├── StorageView.tsx    PVCs, PVs, StorageClasses
        ├── ConfigView.tsx     ConfigMaps, Secrets, SAs
        ├── RBACView.tsx       Roles & Bindings
        └── EventsView.tsx     Live event stream
```

## Quick Start

### Enable the console

Build the SPA and pass the output directory to the API server:

```bash
# Build the console
cd console
npm install
npm run build

# Start rusternetes with the console
cd ..
./target/release/rusternetes --console-dir ./console/dist
```

Open your browser to `https://localhost:6443/console/`.

### Development mode

For hot-reloading during development, use the Vite dev server which proxies API calls to the rusternetes API server:

```bash
# Terminal 1: start the API server (without --console-dir)
./target/release/rusternetes

# Terminal 2: start the console dev server
cd console
npm run dev
```

Open `http://localhost:3000/console/` — the Vite dev server proxies `/api` and `/apis` requests to `localhost:6443`.

### Docker Compose

When running the Docker Compose cluster, pass `--console-dir` to the API server container:

```yaml
api-server:
  command: >
    /usr/local/bin/api-server
      --bind-address 0.0.0.0:6443
      --console-dir /console/dist
  volumes:
    - ./console/dist:/console/dist:ro
```

### All-in-one binary

```bash
rusternetes --console-dir ./console/dist --data-dir ./cluster.db
```

## Views

### Overview
Cluster dashboard showing pod counts, node status, namespace count, discovered resource types, and a deployment summary.

### Workloads
Tabbed view for Pods, Deployments, StatefulSets, DaemonSets, and Jobs. Each tab shows a table with resource-specific columns (status, ready count, restarts, node, age).

### Networking
Tabbed view for Services (type, ClusterIP, ports), Endpoints, Ingresses, and NetworkPolicies.

### Nodes
Table showing node name, Ready status, roles, kubelet version, OS/architecture, CPU/memory capacity, and age.

### Storage
Tabbed view for PersistentVolumeClaims (status, capacity, access modes, storage class), PersistentVolumes, and StorageClasses.

### Config
Tabbed view for ConfigMaps (data key count), Secrets (type, key count), and ServiceAccounts.

### RBAC
Tabbed view for ClusterRoles (rule count), ClusterRoleBindings (role ref, subjects), Roles, and RoleBindings. Cluster-scoped resources ignore namespace filter; namespaced resources respect it.

### Events
Live event stream with auto-refresh (10s interval). Events are sorted by timestamp, color-coded by type (Normal, Warning, Error), and show the involved object, reason, message, age, and count.

## K8s API Client Layer

The console communicates with the rusternetes API server using the standard Kubernetes REST API. No custom endpoints are needed.

### Resource Discovery

On mount, the console fetches `/api/v1` and `/apis` to discover all available resource types. Results are cached for 5 minutes. This enables the console to work with any resources the API server exposes, including CRDs.

### Watch Streams

The `WatchManager` class implements the standard K8s watch protocol:
- Chunked HTTP with newline-delimited JSON
- `resourceVersion` tracking and bookmark support
- Automatic reconnection with exponential backoff (1s to 30s)
- 410 Gone recovery (resets resourceVersion and re-lists)

### Authentication

The console reads a JWT token from `sessionStorage` and passes it as a `Bearer` token in the `Authorization` header. Since the SPA is served from the same origin as the API, no CORS headers are needed.

For `--skip-auth` mode (development), no token is needed.

## Multi-Cluster Support

The console includes a fleet mode for managing multiple rusternetes clusters from a single interface.

### How it works

1. Click the **Fleet** button in the header to enable multi-cluster mode
2. Click **+** to register a remote cluster (name + API server URL)
3. Click a cluster name to switch the active context
4. All views and API calls automatically route to the active cluster

### Architecture

```
Browser → rusternetes-hub (Axum)
              ├── /api/v1/...               Local cluster API
              ├── /console/                 SPA static files
              └── /clusters/{id}/api/...    Proxy to remote cluster
```

For local cluster access, API calls go same-origin (no prefix). For remote clusters, the console prefixes API paths with `/clusters/{cluster-id}`, and the hub API server proxies requests to the remote cluster's API endpoint.

Cluster registrations are persisted in the browser's `localStorage` so they survive page reloads.

## CLI Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--console-dir` | *(disabled)* | Path to the console SPA build directory. Enables the web console at `/console/`. |

When `--console-dir` is not set, the API server does not serve any console routes — it behaves exactly as before.

## Building from Source

```bash
cd console

# Install dependencies
npm install

# Type-check
npm run type-check

# Development server (hot reload)
npm run dev

# Production build
npm run build
# Output: console/dist/ (~313KB JS + ~13KB CSS, gzipped ~96KB)
```

The production build generates static files in `console/dist/` that can be served by any web server or embedded in the API server via `--console-dir`.
