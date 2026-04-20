# Rusternetes Console

The rusternetes console is a web-based cluster management dashboard served directly from the API server. It provides real-time topology visualization, live metrics, pod log streaming, and full resource management — with zero external dependencies.

## Accessing the Console

Open your browser to `https://localhost:6443/console/`. You'll need to accept the self-signed certificate warning on first visit.

The console auto-deploys with the cluster — no separate setup needed. It's baked into the API server Docker image and served at `/console/`.

## Overview Dashboard

[![Overview Dashboard](screenshots/thumbs/console-overview.png)](screenshots/console-overview.png)

The Overview is the landing page showing real-time cluster health at a glance.

**Health Rings** — Three animated circular gauges at the top showing:
- **Pods Running** (green) — running pods vs total pods
- **Nodes Ready** (blue) — ready nodes vs total nodes
- **Deploys Available** (orange) — available deployments vs total

**Metrics Cards** — Four cards below the rings with live sparkline charts:
- **Total Pods** — count with trend over time. Click to browse all pods.
- **Nodes** — count with ready count. Click to go to Nodes view.
- **Restarts** — total container restart count. Color changes to red when high.
- **Resource Types** — how many API resource types were discovered. Click to open the Resource Explorer.

The sparklines collect data every 30 seconds and show the last 30 minutes of history.

**Deployments** — Shows deployment rollout progress with color-coded bars:
- Green = fully available
- Yellow = partially ready
- Red = no ready replicas

Click "View all" to see all deployments.

**Recent Events** — Live event feed showing the latest cluster activity. Warning events are highlighted in yellow. Click "View all" to go to the full Events view.

## Cluster Topology

[![Cluster Topology](screenshots/thumbs/console-topology.png)](screenshots/console-topology.png)

The Topology view is an animated visual map of your entire cluster.

**Nodes** — Shown as containers with:
- Node name and pod count
- CPU and Memory utilization bars with percentages (from real Docker stats)
- Green dot = Ready, pulsing red dot = NotReady

**Pods** — Colored squares inside their node:
- Green = Running
- Yellow = Pending
- Red = Failed
- Brightness indicates CPU usage — brighter pods are using more CPU

**Services** — Orange boxes across the top showing:
- Service name and ClusterIP
- Service type (ClusterIP, NodePort, LoadBalancer)
- Protocol/port badges (TCP, UDP, etc.)
- Endpoint count

**Network Connections** — Animated dashed lines connecting services to their target pods. Tiny particles flow along the lines showing traffic direction. Lines fan out from the service bottom edge.

**Controls:**
- **Namespaces** toggle — shows/hides namespace color zones around pod groups
- **Protocols** toggle — shows/hides port/protocol badges on services
- **Zoom** — in/out/reset controls

**Click a Pod** to:
- See its detail panel (phase, CPU, memory, restarts, IP, ports)
- Automatically open live logs in a bottom overlay
- Highlight its service connections

**Click a Service** to see its detail panel (type, ClusterIP, endpoints, port mappings).

### Live Logs

[![Topology with Live Logs](screenshots/thumbs/console-topology-logs.png)](screenshots/console-topology-logs.png)

When you click a pod in the topology, a **Live Logs** panel slides up from the bottom of the screen showing the pod's recent log output.

- Timestamps are dimmed for readability
- Log levels are color-coded: ERROR (red), WARN (yellow), INFO (blue)
- Green pulsing dot indicates logs are streaming (refreshes every 5 seconds)
- Click "Close" or click the pod again to dismiss

### Activity Timeline

At the bottom of the topology view, the **Cluster Activity** timeline shows:
- A bar chart of pod count history over the last 30 minutes
- Green bars = pods added, red bars = pods removed
- Live indicator (pulsing green dot) when viewing current state
- Click any bar to see the cluster state at that point in time
- Pod count, node count, and service count for the selected time

## Resource Explorer

Navigate to **Explore All** in the sidebar to see every resource type in the cluster.

The explorer auto-discovers all resource types from the API server, including CRDs. Resources are grouped into categories:
- Workloads, Networking, Storage, Access Control, Configuration, Cluster, Extensions, etc.

**Search** — Type in the search bar to filter by kind, plural name, or short name (e.g., "po", "deploy", "svc").

**Resource Cards** — Each resource type shows:
- Kind name and API group/version
- Short names (e.g., "po" for pods)
- "ns" badge if namespaced
- Live resource count

Click any resource type to open its list view.

## Resource List View

When you click a resource type in the Explorer, you see a table of all instances:
- Name (clickable to view details), Namespace, Status, Age
- **Search** bar to filter by name or namespace
- **Create** button opens the create form pre-populated for this resource type
- **Refresh** button to re-fetch data
- **Delete** button (trash icon) on each row with confirmation
- **View** button (eye icon) to open the detail view

The list updates in real-time via K8s watch streams — resources appear and disappear live.

## Resource Detail View

Click any resource name to see its full details:

**Overview Tab:**
- Metadata: UID, generation, deletion timestamp, finalizers
- Labels displayed as colored chips
- Conditions table with status badges, reasons, messages, and age
- Owner references with controller badges
- Annotations

**YAML/JSON Tab:**
- Full resource JSON in an editor
- Edit the JSON and click **Save** to apply changes
- **Reset** to revert to the server version
- Changes are applied via PUT to the K8s API

**Events Tab:**
- Events filtered to this specific resource
- Auto-refreshes every 15 seconds

**Actions (in header):**
- **Scale** (Deployments/StatefulSets) — +/- buttons to adjust replica count
- **Restart** (Deployments/StatefulSets/DaemonSets) — rolling restart via annotation
- **Copy** — copy the full JSON to clipboard
- **Delete** — delete the resource with confirmation

## Workloads

[![Workloads](screenshots/thumbs/console-workloads.png)](screenshots/console-workloads.png)

Navigate to **Workloads** in the sidebar.

**Pod Phase Chart** — Donut chart showing the breakdown of pod phases (Running, Pending, Failed, Succeeded).

**Deployment Cards** — Each deployment shows:
- Name and namespace
- Rollout progress bar (green/yellow/red based on readiness)
- Ready/desired replica count
- Scale controls (+/- buttons)
- Restart button for rolling restart
- View and delete actions

**Restart Heatmap** — Shows pods with the most container restarts. Bar length indicates restart count relative to the worst pod.

**Pod Table** — Full table of all pods with:
- Name (clickable), namespace, status badge, ready count, restart count, node, age
- View and delete actions per row
- Real-time updates via watch

**Zero State** — When no workloads exist, shows a "Quick Deploy" button to create your first deployment.

## Networking

[![Networking](screenshots/thumbs/console-networking.png)](screenshots/console-networking.png)

**Cluster Network Configuration** — Four cards showing:
- **Service CIDR** — the IP range for ClusterIP services (e.g., 10.96.0.0/12)
- **Pod CIDRs** — per-node pod CIDR allocations
- **Cluster DNS** — kube-dns ClusterIP, ports (53/UDP, 53/TCP, 9153/TCP)
- **Kube-Proxy** — mode (iptables), supported service types

This panel shows even when there are no services — it's always useful context.

**Service Type Summary** — Colored chips showing how many services of each type exist (ClusterIP, NodePort, LoadBalancer).

**Service Routing** — Visual diagrams showing service-to-pod connections. Each service box connects with an arrow to its target pods, showing pod name, status, and IP.

**Service Cards** — Cards for each service showing type badge, ClusterIP, port mappings (with arrows showing port→targetPort/protocol), and target pod count.

**Ingresses** and **Network Policies** — Tables and cards for these resources when they exist.

## Storage

[![Storage](screenshots/thumbs/console-storage.png)](screenshots/console-storage.png)

**Overview Panel** — Five stat cards: Claims, Volumes, Classes, CSI Drivers, Total Capacity.

**Storage Capabilities** — What the cluster supports:
- Supported volume types (emptyDir, hostPath, configMap, secret, projected, etc.)
- Access modes (RWO, ROX, RWX) with descriptions
- Reclaim policies (Delete, Retain, Recycle)
- Dynamic provisioning status

**Create StorageClass** — Click the "StorageClass" button to open an inline form:
- Name, provisioner (defaults to rusternetes.io/hostpath), reclaim policy, binding mode
- Helper text explains what each provisioner does

**Create PVC** — Click "Create PVC" to open an inline form:
- Name, namespace, Storage Class dropdown (populated from existing classes), size, access mode
- Shows "Create a StorageClass first" hint when none exist

**StorageClass Cards** — Show provisioner, reclaim policy, binding mode, and expandable badge.

**PVC Cards** — Show status (Bound/Pending), requested size, actual capacity, access modes, storage class, and PV binding.

**PV Table** — Status, capacity, reclaim policy, and claim reference.

## Nodes

[![Nodes](screenshots/thumbs/console-nodes.png)](screenshots/console-nodes.png)

**Node Cards** — Each node shows:
- Name (clickable to detail view) and Ready/NotReady status badge
- Role badges (e.g., "control-plane")
- "cordoned" badge if unschedulable
- Version, OS/architecture, pod count, age
- **CPU gauge** — real utilization bar with percentage from Docker stats
- **Memory gauge** — real utilization bar with percentage
- Taint badges showing key=value:effect
- **View** button and **Cordon/Uncordon** toggle

## Configuration

[![Configuration](screenshots/thumbs/console-config.png)](screenshots/console-config.png)

**Summary Chips** — ConfigMaps count, Secrets count, Service Accounts count.

**ConfigMap Cards** — Name, namespace, data key badges (colored by key name). View and delete actions.

**Secret Cards** — Name, namespace, type badge (e.g., kubernetes.io/service-account-token), key count. Eye-off icon indicates secret content. View and delete actions.

**Service Account List** — Compact cards showing name and namespace. Click to view details.

## RBAC (Access Control)

[![RBAC](screenshots/thumbs/console-rbac.png)](screenshots/console-rbac.png)

**Subjects** — Shows who has access. Each card displays:
- Subject identity (ServiceAccount, Group, or User) with colored icon
- Roles bound to this subject

**ClusterRoleBindings** — Visualizes subject → role connections:
- Subject badges colored by type (teal=ServiceAccount, yellow=Group, blue=User)
- Arrow pointing to the role reference

**ClusterRoles** — Cards showing:
- Role name and rule count
- Rule badges showing verbs (get, list, create, etc.) and resources
- "full access (*)" indicator for admin roles

Click "View all" links to see the complete list via the Resource Explorer.

## Events

[![Events](screenshots/thumbs/console-events.png)](screenshots/console-events.png)

**Event Frequency Histogram** — Stacked bar chart showing event count over the last hour in 5-minute buckets. Blue = Normal, yellow = Warning.

**Filter Controls:**
- **Type filter** — All, Warning, Normal. Warning count shown as badge.
- **Text search** — Filter by reason, message, or involved object name.
- **Quick reason filters** — One-click buttons for common reasons (Created, Pulled, Scheduled, Started).

**Event List** — Each event shows:
- Severity icon (blue=Normal, yellow=Warning, red=Error)
- Reason and involved object (clickable to navigate to that resource)
- Message text
- Time ago and occurrence count
- Warning events have amber background

Auto-refreshes every 10 seconds. Shows up to 100 events.

## Create Resource

[![Create Resource](screenshots/thumbs/console-create.png)](screenshots/console-create.png)

Four creation modes:

**Quick Deploy** — Form-based deployment:
- App name, container image, replicas, port, namespace
- Optional "Create Service" checkbox to expose the deployment
- Click Deploy to create the Deployment (and optionally Service)

**From JSON** — Paste or edit a JSON resource definition:
- Pre-populated with a Deployment template
- When opened from a resource list's Create button, pre-populated with the correct resource type template

**Service** — JSON template pre-populated for creating a Service.

**ConfigMap** — JSON template pre-populated for creating a ConfigMap.

## Multi-Cluster (Fleet Mode)

Click the **Fleet** button in the header bar to enable multi-cluster mode.

- Click **+** to register a remote cluster (name + API server URL)
- Click a cluster name to switch active context
- All views automatically route API calls to the selected cluster
- Cluster registrations persist in browser localStorage

## Namespace Filtering

Use the **namespace dropdown** in the header bar to filter all views to a specific namespace. Select "All namespaces" to see everything.

## Header Bar

The header bar shows:
- **Namespace selector** — filter all views by namespace
- **Fleet cluster switcher** — switch between clusters (when enabled)
- **Resource type count** — how many API resource types were discovered
- **Connection status** — green dot with "Connected" indicator

---

## Managing Workloads

### Deploy an Application

The fastest way to deploy is the **Quick Deploy** form (sidebar > Create):

1. Enter an app name (e.g., `nginx`)
2. Enter a container image (e.g., `nginx:latest`)
3. Set replicas and port
4. Check "Create Service" to expose it
5. Click **Deploy**

This creates a Deployment and optionally a ClusterIP Service. View the result in **Workloads** — you'll see the deployment card with a rollout progress bar.

### Scale a Deployment

On the **Workloads** page, each deployment card has **+/-** buttons to adjust the replica count. Or click the deployment name to open the detail view, where the scale controls are in the header.

### Rolling Restart

Click the **restart** icon (circular arrow) on a deployment card. This adds a `restartedAt` annotation to the pod template, triggering a rolling restart of all pods.

### Delete Resources

Every resource has a trash icon. Click it and confirm. The resource is deleted via the K8s API and disappears from the view in real-time via the watch stream.

### Create Any Resource from JSON

Use **Create > From JSON** to create any Kubernetes resource. When you click "Create" from a resource list (e.g., from the Resource Explorer), the JSON editor is pre-populated with a template for that specific resource type.

## Managing Storage

### Create a StorageClass

Navigate to **Storage** and click **+ StorageClass**:
- **Name**: e.g., `fast-storage`
- **Provisioner**: `rusternetes.io/hostpath` (auto-provisions host directories) or `kubernetes.io/no-provisioner` (manual PV binding)
- **Reclaim Policy**: `Delete` (auto-cleanup) or `Retain` (keep data)
- **Volume Binding Mode**: `WaitForFirstConsumer` (bind when pod uses it) or `Immediate`

A default `standard` StorageClass with the hostpath provisioner is created automatically on cluster startup.

### Create a PVC

Click **+ Create PVC**:
- Select a **Storage Class** from the dropdown (populated from existing classes)
- Choose **Size** and **Access Mode** (RWO, ROX, RWX)
- Click **Create PVC**

If the StorageClass has a dynamic provisioner, a PV is automatically created and the PVC transitions from Pending to Bound.

### View Storage Status

The Storage page shows:
- **Overview panel**: PVC count, PV count, StorageClass count, CSI drivers, total capacity
- **Storage Capabilities**: supported volume types, access modes, reclaim policies
- **Binding visualization**: PVCs show which PV they're bound to

## Networking

### View Cluster Network Configuration

The **Networking** page always shows four configuration cards at the top:
- **Service CIDR** — the IP range for ClusterIP allocation (e.g., `10.96.0.0/12`)
- **Pod CIDRs** — per-node pod CIDR assignments
- **Cluster DNS** — the kube-dns service IP (`10.96.0.10`) and its ports
- **Kube-Proxy** — mode (`iptables`), network mode, supported service types

### Understand Service Routing

The **Service Routing** section shows visual diagrams of how each service connects to its target pods — service box → arrow → matched pods with their IPs and status.

### Service Types

Services are color-coded by type:
- **Blue** — ClusterIP (cluster-internal only)
- **Green** — NodePort (accessible on every node's IP)
- **Orange** — LoadBalancer (external access via cloud provider or MetalLB)

### Using Third-Party CNI Plugins

Rusternetes supports standard CNI plugins on Linux. See the [CNI Guide](CNI_GUIDE.md) for setup instructions for Calico, Cilium, and Flannel.

### Load Balancers

For LoadBalancer services outside cloud providers, use MetalLB. See [LOADBALANCER.md](LOADBALANCER.md) and [METALLB_INTEGRATION.md](METALLB_INTEGRATION.md).

## Security & Authentication

### Current Auth Mode

By default, `--skip-auth` is enabled — all requests are treated as admin. This is fine for development but **must be disabled** before exposing the cluster to any network.

### Securing the Cluster

See the [Authentication Guide](AUTHENTICATION.md) for the full walkthrough:
1. Generate RSA signing keys for JWT tokens
2. Create an admin ServiceAccount and ClusterRoleBinding
3. Configure kubectl with the token
4. Restart without `--skip-auth`

### RBAC in the Console

The **RBAC** page visualizes:
- **Subjects** — who has access (ServiceAccounts, Users, Groups)
- **Bindings** — which subjects are bound to which roles
- **Roles** — what permissions each role grants (verbs × resources)
- **"full access (*)"** badge on admin roles

### TLS/mTLS

The API server supports TLS with self-signed or custom certificates, and mTLS via `--client-ca-file` for client certificate authentication. See [TLS_GUIDE.md](TLS_GUIDE.md).

## Monitoring & Metrics

### Real-Time Metrics in the Console

The console collects metrics from two sources:
- **K8s metrics API** (`/apis/metrics.k8s.io/v1beta1/nodes`) — real CPU/memory usage from Docker container stats, refreshed every 5 seconds
- **In-browser collection** — pod counts, restart rates, event frequency collected every 30 seconds with 30-minute history

### Overview Dashboard

Health rings show pod/node/deployment readiness. Sparkline charts track pod count, node count, restart trends, and event rates over time.

### Node Metrics

The **Nodes** page shows CPU and memory utilization gauges per node with actual percentages from Docker container stats. Values update every 5 seconds.

### Topology Metrics

The **Topology** view shows:
- Node CPU/MEM bars with percentages
- Pod brightness based on CPU usage (brighter = more CPU)
- Traffic particles along service connections

### Activity Timeline

At the bottom of Topology, the cluster activity timeline records snapshots every 15 seconds. Click any bar to see the cluster state at that point — pod count, node count, service count, and deltas.

## Custom Resource Definitions (CRDs)

### Discovering CRDs

The **Explore All** page auto-discovers all resource types from the API, including CRDs. CRDs appear under custom categories (grouped by API group).

### Managing CRDs

Click any CRD-backed resource type to see instances, create new ones (via JSON), view details, and delete. The full CRD lifecycle works through the console just like built-in resources.

See [CRD_IMPLEMENTATION.md](CRD_IMPLEMENTATION.md) for how to create CRDs via kubectl.

## Resource Explorer

The **Explore All** page discovers every resource type in the cluster (100+, including CRDs):
- Grouped by category (Workloads, Networking, Storage, Access Control, etc.)
- Search by kind, plural name, or short name
- Live resource count per type
- Click any type to browse instances

This is the universal entry point for any resource the API server knows about.

## Pod Log Streaming

### From Topology

Click any pod in the topology SVG to immediately open a **Live Logs** panel at the bottom of the screen. Logs refresh every 5 seconds with color-coded severity.

### From Resource Detail

Navigate to any pod's detail view and click the **Events** tab to see events for that specific pod.

## Troubleshooting

### Console shows "Loading..." forever

- Check that the API server is running: `curl -k https://localhost:6443/healthz`
- Check browser console for errors (F12 > Console tab)
- If using auth, ensure a valid token is in `sessionStorage`

### Console shows stale data

- The namespace filter may be set — check the dropdown in the header
- Click the refresh icon on any resource list
- Hard refresh the page (Cmd+Shift+R)

### Resource creates succeed but don't appear

- The resource may be in a different namespace — set namespace filter to "All namespaces"
- Watch updates may not be connected — check the "Connected" indicator in the header

### Metrics show 0%

- CPU/memory metrics come from Docker container stats
- Very low utilization (< 1%) shows as "0.1%" with a tiny bar
- If metrics are stuck at exactly 0, check that the API server can reach the Docker socket

---

## Architecture

The console is a React single-page application served by the Axum API server at `/console/`. Because the SPA and API share the same origin, there is no CORS configuration, no nginx proxy, and no separate deployment.

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
| Charts | Recharts |
| Serving | Axum `tower-http::ServeDir` |

### K8s API Client

The console communicates with the API server using the standard Kubernetes REST protocol. No custom endpoints are needed.

- **Resource Discovery** — fetches `/api/v1` and `/apis` to discover all resource types (including CRDs), cached for 5 minutes
- **Watch Streams** — chunked HTTP with newline-delimited JSON, `resourceVersion` tracking, bookmark support, exponential backoff reconnection (1s to 30s), 410 Gone recovery
- **Authentication** — reads JWT from `sessionStorage`, passes as `Authorization: Bearer`. No token needed in `--skip-auth` mode.

### CLI Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--console-dir` | *(disabled)* | Path to the console SPA build directory. Enables the web console at `/console/`. |

The console auto-deploys in Docker Compose — the Dockerfile builds the SPA and bakes it into the API server image at `/app/console`.

### Development

```bash
# Terminal 1: start the API server
./target/release/rusternetes

# Terminal 2: start the console dev server (hot reload)
cd console
npm install
npm run dev
# Open http://localhost:3000/console/
# Vite proxies /api and /apis to localhost:6443
```

### Building

```bash
cd console
npm install
npm run build
# Output: console/dist/ (~800KB JS + ~18KB CSS, gzipped ~230KB)
```
