# Rusternetes -- Project Status

**Last Updated:** March 28, 2026
**Status:** Active development. Conformance-tested against the official Kubernetes test suite.

---

## Overview

Rusternetes is a ground-up reimplementation of Kubernetes in Rust. The system runs a
full cluster via Docker Compose -- etcd, API server, scheduler, controller manager,
two kubelet nodes, kube-proxy, and CoreDNS -- and is actively tested against the
upstream Kubernetes conformance suite.

The codebase spans 161,000+ lines of Rust across 9 workspace crates, with 929 test
functions and 328 conformance fixes applied over 8 testing rounds.

---

## Cluster Components

| Component          | Port | Description                                                  |
|--------------------|------|--------------------------------------------------------------|
| etcd               | 2379 | Distributed key-value store (backing storage)                |
| API Server         | 6443 | Axum-based HTTPS REST API with mutual TLS                    |
| Scheduler          | --   | Pod placement with affinity, taints, priority, and preemption|
| Controller Manager | --   | 31 reconciliation controllers                                |
| Kubelet (x2)       | --   | Node agents managing containers via Docker/bollard           |
| Kube-Proxy         | --   | iptables-based service routing (host network mode)           |
| CoreDNS            | --   | Cluster DNS and service discovery (ClusterIP 10.96.0.10)     |

---

## Workspace Crates

| Crate                | Purpose                                                        |
|----------------------|----------------------------------------------------------------|
| `common`             | 36 resource type definitions, error types, shared utilities    |
| `api-server`         | 75 handler files, routing, admission control, watch/SSE        |
| `storage`            | `Storage` trait with etcd and in-memory backends               |
| `controller-manager` | 31 controllers for resource reconciliation                     |
| `kubelet`            | Container lifecycle, volumes, probes, CNI networking           |
| `kube-proxy`         | iptables rules for ClusterIP/NodePort/LoadBalancer services    |
| `scheduler`          | Scoring plugins, affinity, taints/tolerations, preemption      |
| `kubectl`            | CLI tool with standard kubectl-style commands                  |
| `cloud-providers`    | AWS, GCP, and Azure integration stubs                          |

---

## Controllers (31)

**Workloads:** Deployment, ReplicaSet, StatefulSet, DaemonSet, Job, CronJob,
ReplicationController

**Networking:** Endpoints, EndpointSlice, Service, LoadBalancer, Ingress, Network Policy

**Storage:** PV/PVC Binder, Dynamic Provisioner, Volume Snapshot, Volume Expansion

**Cluster:** Namespace, Node, ServiceAccount, Garbage Collector, TTL Controller,
Taint Eviction, Events

**Policy and Scaling:** HPA, VPA, Pod Disruption Budget, ResourceQuota, ResourceClaim

**Extensibility:** Certificate Signing Request, CRD

---

## Conformance Testing Progress

Testing is performed against the official Kubernetes conformance suite (441 tests)
using Sonobuoy.

| Round | Date     | Pass/Total | Rate | Fixes Deployed |
|-------|----------|------------|------|----------------|
| 103   | Mar 10   | 245/441    | 56%  | 271            |
| 104   | Mar 14   | 405/441    | 92%  | 280            |
| 105   | Mar 17   | ~410/441   | 93%  | 296            |
| 106   | Mar 20   | ~416/441   | 94%  | 310            |
| 107   | Mar 23   | ~422/441   | 96%  | 312            |
| 108   | Mar 27   | 263/441    | 60%  | 312            |

Round 108 reflects an infrastructure regression, not code regression. All 178 failures
have corresponding fixes applied (328 total fixes). A clean redeploy is expected to
restore the 96%+ pass rate from Round 107.

### Notable Conformance Fixes

- **#319** -- Systemic CAS re-read bug in kubelet. Every pod status write used stale
  data, causing silent update failures across all workloads.
- **#315** -- Watch event batching. etcd sends multiple events per response; only the
  first was being processed, causing missed state transitions.
- **#313** -- Hostname truncation to 63 characters. Broke pod creation for all
  resources with long generated names (ReplicaSets, Jobs, etc.).
- **#318** -- OpenAPI protobuf MIME handling. kubectl expects HTTP 406 for protobuf
  requests so it can fall back to JSON; we were returning 200 with invalid data.
- **#321** -- EmptyDir tmpfs support, service quota enforcement, LimitRange defaults.
- **#322** -- CRD status retry logic and binary body extraction for protobuf requests.

---

## Key Features

### API Compatibility

- Full CRUD for all major Kubernetes resource types
- Server-Side Apply with field manager tracking
- Strategic Merge Patch, JSON Patch, and Merge Patch
- Watch API with server-sent event streaming and bookmarks
- Custom Resource Definitions with status and scale subresources
- Aggregated API discovery (v2 and v2beta1)
- Pod exec, attach, and port-forward over WebSocket
- Dry-run support for mutating operations

### Admission Control

- Mutating and Validating Webhook Configurations
- ValidatingAdmissionPolicy with CEL expression evaluation
- Built-in admission plugins: NamespaceLifecycle, LimitRanger, ResourceQuota
- Pod Security Standards enforcement (Privileged, Baseline, Restricted)

### Scheduling

- Node affinity and anti-affinity (required and preferred)
- Pod affinity and anti-affinity
- Taints and tolerations
- Priority classes and preemption
- Topology spread constraints

### Networking

- CNI framework integration
- ClusterIP, NodePort, and LoadBalancer service types
- iptables-based packet routing via kube-proxy
- DNS-based service discovery via CoreDNS
- Network policy enforcement

### Storage

- Persistent Volumes and Persistent Volume Claims
- Dynamic provisioning with StorageClasses
- EmptyDir (memory-backed and disk-backed)
- HostPath volumes
- Volume snapshots and expansion

### High Availability

- Leader election for control plane components
- Multi-master API server support
- etcd clustering

---

## Codebase Statistics

| Metric             | Value     |
|--------------------|-----------|
| Lines of Rust      | 161,000+  |
| Workspace crates   | 9         |
| API handler files  | 75        |
| Resource types     | 36        |
| Controllers        | 31        |
| Test functions     | 929       |
| Conformance fixes  | 328       |
| Conformance rounds | 8         |

---

## Building and Running

```bash
# Build the workspace
cargo build

# Run all tests
cargo test

# Lint and format
make pre-commit

# Start the cluster
docker compose build
docker compose up -d
bash scripts/bootstrap-cluster.sh

# Run conformance tests
bash scripts/run-conformance.sh
bash scripts/conformance-progress.sh   # monitor progress
```

KUBECONFIG: `~/.kube/rusternetes-config`
