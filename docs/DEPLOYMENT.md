# Rusternetes Deployment Guide

**Last Updated:** March 10, 2026

This guide covers deploying the complete Rusternetes cluster with all 7 components.

## Quick Start

### 1. Build All Components

```bash
# Build release binaries
cargo build --release

# Build container images
podman-compose build
```

**Build Time:** ~6 minutes (first build), ~30 seconds (incremental)

### 2. Start the Cluster

```bash
# Start all services
podman-compose up -d

# View status
podman-compose ps
```

### 3. Verify Deployment

```bash
# Check all containers are running
podman ps --format "table {{.Names}}\t{{.Status}}"

# Check etcd health
podman exec rusternetes-etcd /usr/local/bin/etcdctl \
  --endpoints=http://localhost:2379 endpoint health

# Test API server
curl -k https://localhost:6443/healthz

# Check DNS server logs
podman logs rusternetes-dns-server
```

## Component Overview

### Running Services

| Component | Container | Port | Status Check |
|-----------|-----------|------|--------------|
| etcd | rusternetes-etcd | 2379, 2380 | `podman exec rusternetes-etcd etcdctl endpoint health` |
| API Server | rusternetes-api-server | 6443 | `curl -k https://localhost:6443/healthz` |
| DNS Server | rusternetes-dns-server | 8053 | `podman logs rusternetes-dns-server` |
| Scheduler | rusternetes-scheduler | - | `podman logs rusternetes-scheduler` |
| Controller Manager | rusternetes-controller-manager | - | `podman logs rusternetes-controller-manager` |
| Kubelet | rusternetes-kubelet | 8082 | `podman logs rusternetes-kubelet` |
| Kube-proxy | rusternetes-kube-proxy | - | `podman logs rusternetes-kube-proxy` |

## Using kubectl

### Basic Commands

```bash
# Set alias for convenience (optional)
alias kubectl='./target/release/kubectl --insecure-skip-tls-verify'

# Get cluster resources
kubectl get nodes
kubectl get namespaces
kubectl get services --namespace default
kubectl get pods --namespace default

# Create resources
kubectl create -f examples/workloads/test-deployment.yaml

# Delete resources
kubectl delete deployment test-deployment --namespace default
```

### Authentication

The cluster runs with `--skip-auth` by default for development. For production:

```bash
# Use token authentication
kubectl --token <jwt-token> get pods
```

## DNS Configuration

### Port Notes

The DNS server runs on **port 8053** instead of the standard port 53:

- **Port 53**: Requires NET_BIND_SERVICE capability (privileged)
- **Port 5353**: Conflicts with macOS mDNS/Bonjour
- **Port 8053**: Development-friendly, works without privileges

### Testing DNS

```bash
# Test DNS resolution (from host)
dig @localhost -p 8053 test-service.default.svc.cluster.local

# Inside a pod (configure pod to use DNS server at ClusterIP)
kubectl exec -it test-pod -- nslookup test-service
```

### DNS Naming Conventions

- **Services**: `<service>.<namespace>.svc.cluster.local`
- **Pods (name-based)**: `<pod-name>.<namespace>.pod.cluster.local`
- **Pods (IP-based)**: `<ip-with-dashes>.<namespace>.pod.cluster.local`
- **SRV Records**: `_<port-name>._<protocol>.<service>.<namespace>.svc.cluster.local`

## Common Operations

### Viewing Logs

```bash
# Follow logs for a component
podman logs -f rusternetes-api-server
podman logs -f rusternetes-kubelet
podman logs -f rusternetes-dns-server

# View last N lines
podman logs --tail 50 rusternetes-controller-manager
```

### Restarting Components

```bash
# Restart a single component
podman-compose restart api-server

# Restart all components
podman-compose restart

# Force recreate a component
podman-compose up -d --force-recreate api-server
```

### Rebuilding After Code Changes

```bash
# Rebuild a specific component
podman-compose build api-server
podman-compose up -d --force-recreate api-server

# Rebuild all components
podman-compose build
podman-compose up -d --force-recreate
```

### Stopping the Cluster

```bash
# Stop all components
podman-compose down

# Stop and remove volumes (WARNING: deletes all data)
podman-compose down -v
```

## Network Configuration

### Network Details

- **Network Name**: `rusternetes-network`
- **Driver**: bridge
- **CIDR**: Auto-assigned by Podman
- **ClusterIP Range**: 10.96.0.0/12 (1,048,576 IPs)

### Port Mappings

| Service | Host Port | Container Port | Protocol |
|---------|-----------|----------------|----------|
| etcd | 2379 | 2379 | TCP |
| etcd (peer) | 2380 | 2380 | TCP |
| API Server | 6443 | 6443 | TCP (HTTPS) |
| DNS Server | 8053 | 8053 | UDP, TCP |
| Kubelet | 8082 | 8082 | TCP |

## Storage Configuration

### Volumes

- **etcd-data**: Persistent etcd data (`rusternetes-etcd-data` volume)
- **Kubelet socket**: Podman socket bind mount for container management

### Volume Cleanup

```bash
# List volumes
podman volume ls

# Remove etcd data (WARNING: deletes cluster state)
podman volume rm rusternetes-etcd-data
```

## Troubleshooting

### Common Issues

#### 1. "Port already in use"

```bash
# Check what's using the port
lsof -i :6443
lsof -i :8053

# Stop conflicting service or change port in docker-compose.yml
```

#### 2. "Container already exists"

```bash
# Clean up and restart
podman-compose down
podman-compose up -d
```

#### 3. etcd Not Healthy

```bash
# Check etcd logs
podman logs rusternetes-etcd

# Wait for etcd to initialize (can take 10-30 seconds)
sleep 30 && podman-compose ps
```

#### 4. API Server Certificate Errors

The API server uses self-signed certificates. Use `--insecure-skip-tls-verify`:

```bash
./target/release/kubectl --insecure-skip-tls-verify get nodes
```

#### 5. DNS Server Connection Timeout

- Ensure DNS server is running: `podman ps | grep dns-server`
- Check logs: `podman logs rusternetes-dns-server`
- Verify etcd is healthy (DNS server requires etcd)
- Wait for 30-second sync interval

#### 6. Kubelet Can't Pull Images

```bash
# Check Podman socket is accessible
podman ps

# Verify socket mount in docker-compose.yml
# Should be: /run/user/501/podman/podman.sock:/var/run/docker.sock:rw
```

### Debug Mode

Enable debug logging for components:

```yaml
# In docker-compose.yml, change:
environment:
  - RUST_LOG=info

# To:
environment:
  - RUST_LOG=debug
```

Then restart: `podman-compose up -d --force-recreate <component>`

## Health Checks

### Automated Health Check Script

```bash
#!/bin/bash
# cluster-health.sh

echo "=== Rusternetes Cluster Health Check ==="

# Check etcd
echo -n "etcd: "
podman exec rusternetes-etcd /usr/local/bin/etcdctl \
  --endpoints=http://localhost:2379 endpoint health 2>&1 | grep -q "healthy" && \
  echo "✅ HEALTHY" || echo "❌ UNHEALTHY"

# Check API Server
echo -n "API Server: "
curl -k -s https://localhost:6443/healthz 2>&1 | grep -q "ok" && \
  echo "✅ HEALTHY" || echo "❌ UNHEALTHY"

# Check DNS Server
echo -n "DNS Server: "
podman logs --tail 5 rusternetes-dns-server 2>&1 | grep -q "ready to handle queries" && \
  echo "✅ RUNNING" || echo "❌ NOT READY"

# Check Kubelet
echo -n "Kubelet: "
podman logs --tail 5 rusternetes-kubelet 2>&1 | grep -q "Starting kubelet" && \
  echo "✅ RUNNING" || echo "❌ NOT RUNNING"

# Check Controller Manager
echo -n "Controller Manager: "
podman logs --tail 5 rusternetes-controller-manager 2>&1 | grep -q "controller" && \
  echo "✅ RUNNING" || echo "❌ NOT RUNNING"

# Check Scheduler
echo -n "Scheduler: "
podman logs --tail 5 rusternetes-scheduler 2>&1 | grep -q "scheduler" && \
  echo "✅ RUNNING" || echo "❌ NOT RUNNING"

# Check Kube-proxy
echo -n "Kube-proxy: "
podman logs --tail 5 rusternetes-kube-proxy 2>&1 | grep -q "kube-proxy" && \
  echo "✅ RUNNING" || echo "❌ NOT RUNNING"

echo ""
echo "=== Container Status ==="
podman ps --format "table {{.Names}}\t{{.Status}}" | grep rusternetes
```

Make it executable: `chmod +x cluster-health.sh`

## Production Considerations

### Security

1. **TLS Certificates**: Replace self-signed certs with CA-signed certificates
2. **Authentication**: Remove `--skip-auth` flag and configure RBAC properly
3. **Secrets**: Encrypt secrets at rest in etcd
4. **Network Policies**: Implement network isolation between namespaces

### High Availability

1. **etcd Cluster**: Deploy 3 or 5 etcd nodes with quorum
2. **Multi-Master**: Run multiple API server instances with load balancer
3. **Leader Election**: Enable leader election for controller-manager and scheduler
4. **Health Monitoring**: Implement automated health checks and failover

### Performance

1. **Resource Limits**: Set appropriate CPU/memory limits for components
2. **etcd Tuning**: Optimize etcd settings for your workload
3. **Caching**: Implement caching layers for frequently accessed resources
4. **Metrics**: Enable Prometheus metrics for all components

## Next Steps

1. **Create Test Resources**: Apply examples from `examples/` directory
2. **Deploy Applications**: Create deployments, services, and pods
3. **Configure Storage**: Set up PersistentVolumes and StorageClasses
4. **Test DNS**: Verify service discovery between pods
5. **Monitor Cluster**: Check logs and metrics regularly

## Additional Resources

- [STATUS.md](STATUS.md) - Current implementation status
- [DNS.md](DNS.md) - Complete DNS documentation
- [QUICKSTART.md](QUICKSTART.md) - Quick start guide
- [DEVELOPMENT.md](DEVELOPMENT.md) - Development guide
- [examples/](examples/) - Example YAML files

## Support

For issues, check:
1. Component logs: `podman logs rusternetes-<component>`
2. STATUS.md for known limitations
3. GitHub issues for similar problems
