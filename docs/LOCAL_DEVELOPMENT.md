# Local Development Guide

This guide covers local development setup and tools specific to running Rusternetes on macOS with Podman Machine.

## Overview

Rusternetes is designed to run in a production-like environment where:
- Services communicate within the cluster network
- DNS is only accessible from pods inside the cluster
- External access is via API Server, LoadBalancer services, or Ingress

For **local development and testing**, we provide additional tools to make debugging easier.

## Local Development Tools

### DNS Proxy (macOS/Podman Machine Only)

#### Why is this needed?

In production Kubernetes, DNS is **internal-only** and accessible only to pods. However, when developing locally on macOS with Podman Machine, you might want to test DNS resolution from your host machine for debugging purposes.

**Problem**: Podman Machine on macOS doesn't support UDP port forwarding from host → VM → container.

**Solution**: A development-only DNS proxy container that runs inside the cluster network.

#### Usage

```bash
# Start the DNS proxy (development only)
./scripts/dns-proxy.sh start

# Check proxy status
./scripts/dns-proxy.sh status

# Test DNS from your macOS terminal
dig @localhost -p 15353 myservice.default.svc.cluster.local

# Stop the proxy
./scripts/dns-proxy.sh stop
```

#### Important Notes

⚠️ **This is NOT part of the production cluster setup**
- The proxy is only for local debugging from your macOS terminal
- Pods inside the cluster can already resolve DNS without this proxy
- In production, DNS is internal-only (this is correct behavior)
- Do not use this in any production or CI/CD environment

#### How it works

```
macOS Host (dig)
    ↓ port 15353
Podman VM
    ↓
DNS Proxy Container (socat)
    ↓ port 8053
DNS Server Container
```

The proxy runs **inside the cluster network** where it can reach the DNS server, then exposes it to your macOS host.

## NodePort Services (Not Supported on macOS)

### Why NodePort doesn't work locally

NodePort services require `kube-proxy` to set up iptables rules, which needs:
1. Root privileges
2. Host networking mode
3. Direct access to the host's network stack

On macOS with Podman Machine:
- Containers run in a Linux VM, not directly on macOS
- Root access to the macOS network stack is not available
- iptables rules in the VM don't affect macOS host networking

### Alternative: Use LoadBalancer Services

For local development, use **LoadBalancer** type services with MetalLB instead:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: my-app
spec:
  type: LoadBalancer  # Use this instead of NodePort
  selector:
    app: my-app
  ports:
    - port: 80
      targetPort: 8080
```

MetalLB will assign an IP from the configured range that's accessible from your macOS host.

## Accessing Services Locally

### Method 1: API Server (Always Available)

```bash
# Access the API server from your macOS terminal
kubectl --server https://localhost:6443 --insecure-skip-tls-verify get pods
```

### Method 2: LoadBalancer Services (Recommended)

Use MetalLB LoadBalancer services - they work seamlessly on macOS.

### Method 3: Port Forwarding (kubectl)

```bash
# Forward a pod's port to your local machine
kubectl --server https://localhost:6443 port-forward pod/my-pod 8080:80

# Access via localhost
curl http://localhost:8080
```

### Method 4: DNS Proxy (For Testing DNS)

```bash
# Start the development DNS proxy
./scripts/dns-proxy.sh start

# Query services by name
dig @localhost -p 15353 my-service.default.svc.cluster.local
```

## Platform Differences

| Feature | Linux (Podman) | macOS (Podman Machine) | Production K8s |
|---------|---------------|----------------------|----------------|
| DNS (internal) | ✅ | ✅ | ✅ |
| DNS (from host) | ✅ | ⚠️ Needs proxy | ❌ Not accessible |
| NodePort | ✅ | ❌ | ✅ |
| LoadBalancer | ✅ | ✅ | ✅ |
| ClusterIP | ✅ | ✅ | ✅ |

## Best Practices for Local Development

1. **Use LoadBalancer services** instead of NodePort
2. **Only use the DNS proxy** when you need to debug DNS from your terminal
3. **Test production scenarios** by accessing services the way pods would (via service names)
4. **Remember**: If it works from inside a pod, it will work in production

## Production vs Development

### Production Behavior
- DNS is only accessible from pods (correct)
- Services are accessed via Ingress, LoadBalancer, or ClusterIP
- NodePort is available on cluster nodes

### Development Shortcuts (macOS only)
- DNS proxy for debugging DNS from host
- Use MetalLB LoadBalancer instead of NodePort
- Direct API server access at localhost:6443

---

**Remember**: The goal of local development is to simulate production as closely as possible. Use development shortcuts sparingly and only for debugging.
