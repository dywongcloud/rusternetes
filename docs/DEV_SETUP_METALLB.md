# dev-setup.sh MetalLB Integration

## What Was Added

The `dev-setup.sh` script now includes **automatic MetalLB installation** as option 9.

## How to Use

### Step 1: Start Your Cluster

```bash
./dev-setup.sh
# Choose option 8 (Full setup - build + start)
```

### Step 2: Install MetalLB

```bash
./dev-setup.sh
# Choose option 9 (Install MetalLB)
```

The script will:
1. Check if kubectl is available
2. Verify the cluster is running
3. Install MetalLB v0.14.3
4. Wait for MetalLB pods to be ready
5. Auto-detect your Podman network range
6. Configure IP address pool for LoadBalancer services
7. Set up Layer 2 advertisement

## What It Detects Automatically

### For Podman Users:
- Detects Podman network subnet (typically `10.88.0.0/16`)
- Configures IP pool: `10.88.100.1-10.88.100.50`
- These IPs work from your host machine

### For Other Environments:
- Falls back to default range: `192.168.1.240-192.168.1.250`
- You may need to adjust this for your network

## Testing

After installation, test with:

```bash
# Apply a test LoadBalancer service
kubectl apply -f examples/networking/test-loadbalancer-service.yaml

# Watch for external IP
kubectl get svc --watch

# Or run automated test
./examples/metallb/test-metallb.sh
```

## Complete Workflow Example

```bash
# 1. Build and start cluster
./dev-setup.sh
# Choose: 8

# 2. Install MetalLB
./dev-setup.sh
# Choose: 9

# 3. Create a test service
kubectl apply -f examples/networking/test-loadbalancer-service.yaml

# 4. Get the external IP
kubectl get svc test-loadbalancer-service

# 5. Test it (if using Podman)
EXTERNAL_IP=$(kubectl get svc test-loadbalancer-service -o jsonpath='{.status.loadBalancer.ingress[0].ip}')
curl http://$EXTERNAL_IP
```

## Menu Options Updated

The dev-setup.sh menu now has:

```
What would you like to do?
  1) Build all container images
  2) Start the development cluster
  3) Stop the development cluster
  4) Clean up (remove all containers and volumes)
  5) View logs
  6) Build Rust binaries locally
  7) Run tests
  8) Full setup (build + start)
  9) Install MetalLB (local LoadBalancer support)    ← NEW!
 10) Exit
```

## Features

### Smart Detection
- Automatically finds kubectl (system or local binary)
- Detects container runtime (Podman or Docker)
- Auto-discovers Podman network range
- Validates cluster is running before installation

### Error Handling
- Checks if kubectl is available
- Verifies cluster accessibility
- Provides helpful error messages
- Suggests next steps on success

### Integration
- Uses same color-coded output as rest of script
- Follows same UX patterns
- Provides documentation links

## Troubleshooting

### "kubectl is not available"
```bash
# Build kubectl first
cargo build --release --bin kubectl
# Then run dev-setup.sh option 9 again
```

### "Kubernetes cluster is not running"
```bash
# Start the cluster first
./dev-setup.sh
# Choose option 2 (or 8 for full setup)
```

### Wrong IP Range
If you need a different IP range, edit the configuration manually:
```bash
kubectl edit ipaddresspool -n metallb-system default-pool
```

Or apply a custom configuration:
```bash
kubectl apply -f examples/metallb/metallb-config-local.yaml
```

## Benefits

✅ **One-click MetalLB installation** - No manual steps
✅ **Automatic configuration** - Detects your environment
✅ **Works out of the box** - For Podman and Docker
✅ **Integrated workflow** - Part of dev setup process
✅ **Smart validation** - Checks prerequisites before running

## Documentation

For more information:
- [Complete MetalLB Guide](docs/METALLB_INTEGRATION.md)
- [Quick Start Guide](examples/metallb/QUICKSTART.md)
- [LoadBalancer Overview](LOADBALANCER.md)

## Summary

The `dev-setup.sh` script now provides a complete development workflow:

1. Build containers (option 1 or 8)
2. Start cluster (option 2 or 8)
3. **Install MetalLB (option 9)** ← NEW!
4. Deploy services with LoadBalancer type
5. Access them via external IPs

All with automatic configuration and environment detection!
