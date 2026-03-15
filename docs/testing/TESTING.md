# Rusternetes Testing Guide

This document describes how to test Rusternetes functionality with manual testing procedures.

> 📊 **For comprehensive test status and coverage**: See [TEST_STATUS.md](./TEST_STATUS.md)

## Quick Test

Run the basic cluster health test:

```bash
./scripts/test-cluster.sh
```

## Automated Tests

### Run All Tests

```bash
cargo test --no-default-features
```

### Run Specific Test Suites

```bash
# Controller integration tests (42 tests)
cargo test --test hpa_controller_test --no-default-features
cargo test --test vpa_controller_test --no-default-features
cargo test --test replicaset_controller_test --no-default-features
cargo test --test endpoints_controller_test --no-default-features
cargo test --test endpointslice_controller_test --no-default-features

# Scheduler tests (19 tests)
cargo test -p rusternetes-scheduler --test scheduler_test --no-default-features

# E2E workflow tests (4 tests)
cargo test --test e2e_workflow_test --no-default-features

# CNI and init container tests (16 tests)
cargo test --test cni_integration_test --no-default-features
cargo test --test init_containers_test --no-default-features
```

**Current Status**: 469+ passing tests with ~75% coverage

See [TEST_STATUS.md](./TEST_STATUS.md) for complete test breakdown and coverage details.

## Manual Testing

### Test 1: Health Checks

```bash
# API server health
curl -k https://localhost:6443/healthz
# Should return HTTP 200

# Readiness
curl -k https://localhost:6443/readyz
# Should return HTTP 200

# Metrics
curl -k https://localhost:6443/metrics
# Should return Prometheus metrics
```

### Test 2: List Resources

```bash
./target/release/kubectl --server https://localhost:6443 --insecure-skip-tls-verify get namespaces
./target/release/kubectl --server https://localhost:6443 --insecure-skip-tls-verify get nodes
./target/release/kubectl --server https://localhost:6443 --insecure-skip-tls-verify get pods --all-namespaces
```

Should return successfully with `--skip-auth` enabled (development mode).

### Test 3: Apply Resources

```bash
./target/release/kubectl --server https://localhost:6443 --insecure-skip-tls-verify apply -f examples/tests/test-namespace.yaml
./target/release/kubectl --server https://localhost:6443 --insecure-skip-tls-verify apply -f examples/workloads/test-deployment.yaml
./target/release/kubectl --server https://localhost:6443 --insecure-skip-tls-verify apply -f examples/networking/test-service.yaml
```

### Test 4: Verify Pod Scheduling and Execution

```bash
# Create a test namespace
./target/release/kubectl --insecure-skip-tls-verify create -f examples/tests/test-namespace.yaml

# Create a pod
./target/release/kubectl --insecure-skip-tls-verify create -f examples/workloads/test-pod.yaml

# Wait a few seconds for scheduler and kubelet
sleep 15

# Check pod status
./target/release/kubectl --insecure-skip-tls-verify get pod nginx-pod -n test-namespace

# Expected output shows:
# - "node_name": "node-1" (assigned by scheduler)
# - "phase": "Running" (kubelet started container)

# Verify container is running
podman ps | grep nginx-pod

# Check container logs
podman logs nginx-pod_nginx
```

### Test 5: Verify Controller Behavior

After applying deployments, check that:

1. ✅ Deployment controller creates ReplicaSets
2. ✅ ReplicaSet controller creates Pods
3. ✅ Scheduler assigns Pods to nodes
4. ✅ Kubelet manages container lifecycle
5. ✅ Controllers maintain desired replica counts
6. ✅ Self-healing when pods are deleted

## Component Testing

### etcd

```bash
podman exec rusternetes-etcd /usr/local/bin/etcdctl \
  --endpoints=http://localhost:2379 endpoint health
```

### API Server

```bash
podman logs rusternetes-api-server --tail 50
```

### Scheduler

```bash
podman logs rusternetes-scheduler --tail 50
```

### Controller Manager

```bash
podman logs rusternetes-controller-manager --tail 50
```

### Kubelet

```bash
podman logs rusternetes-kubelet --tail 50
```

### Kube-proxy

```bash
podman logs rusternetes-kube-proxy --tail 50
```

## Testing with Authentication

The development cluster runs with `--skip-auth` flag enabled for easier testing. For production-like testing with authentication:

### Option 1: Use Token Authentication

The API server supports JWT token authentication:

```bash
# Generate a token (requires creating a ServiceAccount first)
./target/release/kubectl --server https://localhost:6443 \
  --insecure-skip-tls-verify \
  --token YOUR_JWT_TOKEN \
  get pods
```

### Option 2: Create Service Account

1. Create a ServiceAccount resource
2. Create a RoleBinding or ClusterRoleBinding for permissions
3. Extract the service account token from the API
4. Use the token with `--token` flag

### Development Mode (Current)

The cluster runs with authentication disabled (`--skip-auth`) for convenience:
- No token required for API calls
- All users have full cluster-admin privileges
- Not recommended for production use

## Test Resources

Example resource files are organized in `examples/`:

- `examples/tests/test-namespace.yaml` - Test namespace
- `examples/workloads/test-deployment.yaml` - Nginx deployment with 3 replicas
- `examples/networking/test-service.yaml` - Service exposing the deployment
- `examples/workloads/test-job.yaml` - Batch job calculating pi
- `examples/workloads/test-cronjob.yaml` - CronJob running every 5 minutes

See [examples/README.md](../../examples/README.md) for complete list of examples.

## Known Issues

1. **Service Networking Not Implemented** - Kube-proxy is a stub
   - Impact: Services can be created but traffic routing doesn't work
   - Solution: See STATUS.md Priority 1 - Networking implementation

2. **YAML Field Naming** - Resource definitions use snake_case (api_version) instead of camelCase (apiVersion)
   - Impact: Different from standard Kubernetes YAML
   - Note: This is by design for Rust serde compatibility

3. **Self-Signed Certificates** - Development uses self-signed TLS certs
   - Impact: Must use `--insecure-skip-tls-verify` flag
   - Solution: For production, use proper CA-signed certificates

## Feature Status

### Completed ✅

1. **TLS/HTTPS Support**
   - API server running with TLS 1.3
   - Self-signed certificates for development
   - kubectl supports `--insecure-skip-tls-verify` flag

2. **kubectl Enhancements**
   - Added TLS certificate skip verification support
   - Fixed ObjectMeta deserialization (uid field now optional)
   - Added Job and CronJob resource support
   - Default server changed to `https://localhost:6443`

3. **Cluster Health**
   - API server responds to /healthz endpoint (HTTP 200)
   - All 6 components running successfully
   - etcd healthy and accessible

4. **Pod Scheduling and Execution**
   - ✅ Scheduler successfully assigns pods to nodes
   - ✅ Kubelet pulls images automatically (supports ImagePullPolicy)
   - ✅ Kubelet creates and starts containers
   - ✅ Containers run successfully with proper configuration
   - ✅ Pod status updates reflect actual container state
   - ✅ Container logs accessible via `podman logs`

5. **Kubelet Features**
   - ✅ Automatic image pulling (Always, IfNotPresent, Never policies)
   - ✅ Health probes (HTTP GET, TCP Socket, Exec)
   - ✅ Liveness and readiness probe support
   - ✅ Container restart policies (Always, OnFailure, Never)
   - ✅ Container status reporting (states, exit codes, restart counts)
   - ✅ Pod status updates to etcd
   - ✅ Node heartbeat and condition reporting

6. **Controller Behavior**
   - ✅ Deployment creates ReplicaSets (fixed architecture)
   - ✅ ReplicaSet creates Pods
   - ✅ HPA scales deployments based on metrics
   - ✅ VPA generates resource recommendations
   - ✅ Endpoints controller creates endpoints from services
   - ✅ EndpointSlice controller syncs endpoints

### In Progress 🚧

1. **Service Networking**
   - Kube-proxy is a stub implementation
   - Services can be created but traffic routing not implemented
   - **Next Step**: Implement kube-proxy with iptables/ipvs

## Success Metrics

- [x] All components healthy
- [x] Resources can be created/read/updated/deleted
- [x] Controllers reconcile state correctly
- [x] Scheduler places pods appropriately
- [x] Kubelet manages container lifecycle
- [x] Kubelet pulls images automatically
- [x] Container health probes working
- [x] Volume support (EmptyDir, HostPath, PV/PVC APIs)
- [x] ConfigMap and Secret volume support
- [ ] Services route traffic correctly (kube-proxy stub)
- [x] RBAC enforces permissions (when auth enabled)
- [x] 469+ automated tests passing
- [x] ~75% test coverage

## Testing Checklist

- [x] Cluster starts successfully
- [x] All 6 components running
- [x] TLS enabled on API server
- [x] Health endpoints responding
- [x] kubectl can connect to API server
- [x] kubectl can list resources
- [x] kubectl can create resources
- [x] Deployments create ReplicaSets
- [x] ReplicaSets create Pods
- [x] Scheduler assigns pods to nodes
- [x] Kubelet pulls container images
- [x] Kubelet creates containers
- [x] Containers run successfully
- [x] Pod status updates work
- [x] Container health probes function
- [x] Volumes work (EmptyDir, HostPath, ConfigMap, Secret, PVC)
- [x] Jobs complete successfully
- [x] CronJobs trigger on schedule
- [x] StatefulSet ordered deployment
- [x] DaemonSet node deployment
- [ ] Services route traffic (kube-proxy stub)
- [x] RBAC permissions enforced (when enabled)

## Next Steps

### Priority 1: Service Networking
- Add kube-proxy with iptables/ipvs support
- Implement service endpoint controller
- Add ClusterIP networking
- Integrate DNS service (CoreDNS)

### Priority 2: Test Coverage Expansion
- Leader Election integration tests (currently 1 test)
- Admission Webhook E2E tests (21 unit tests exist)
- API handler tests for all resources
- HA cluster tests

See [TEST_STATUS.md](./TEST_STATUS.md) for detailed test roadmap and coverage goals.

---

**Related Documentation**:
- [TEST_STATUS.md](./TEST_STATUS.md) - Comprehensive test status and coverage report
- [examples/README.md](../../examples/README.md) - Example resource files
