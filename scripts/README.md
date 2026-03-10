# Rusternetes Development Scripts

This directory contains development-only helper scripts for local testing on macOS with Podman Machine.

## ⚠️ IMPORTANT: Local Development Only

**These scripts are NOT part of the production cluster setup.** They are workarounds for platform-specific limitations when running on macOS with Podman Machine.

## Available Scripts

### dns-proxy.sh - DNS UDP Proxy (macOS/Podman Machine Only)

**Purpose:** Enable DNS queries from your macOS terminal for debugging.

**Why is this needed?**
- In production Kubernetes, DNS is internal-only (correct behavior)
- Pods inside the cluster can already resolve DNS without this tool
- Podman Machine on macOS doesn't support UDP port forwarding
- This proxy allows testing DNS from your macOS host terminal

**Usage:**
```bash
# Start the DNS proxy
./scripts/dns-proxy.sh start

# Check if it's running
./scripts/dns-proxy.sh status

# Test DNS from your macOS terminal
dig @localhost -p 15353 myservice.default.svc.cluster.local

# Stop the proxy
./scripts/dns-proxy.sh stop
```

**How it works:**
```
macOS Host (dig @localhost:15353)
    ↓
Podman VM (port 15353 mapped)
    ↓
DNS Proxy Container (socat, inside rusternetes-network)
    ↓
DNS Server Container (port 8053)
```

The proxy runs **inside the cluster network** where it can reach the DNS server, then exposes it to your macOS host on port 15353.

**When to use:**
- Debugging DNS resolution from your macOS terminal
- Testing service discovery before deploying pods
- Verifying DNS records are correctly created

**When NOT to use:**
- In production deployments
- In CI/CD pipelines
- When testing from inside pods (they already have DNS access)

## Platform Differences

| Tool | Linux/Podman | macOS/Podman Machine | Production K8s |
|------|-------------|----------------------|----------------|
| DNS from host | Not needed (direct access) | Needs proxy | Not applicable |
| DNS from pods | ✅ Works | ✅ Works | ✅ Works |

## Alternative: Test from Inside the Cluster

Instead of using the proxy, you can test DNS from inside a container:

```bash
# Run a test container with dig
podman run --rm --network rusternetes-network alpine sh -c \
  "apk add --no-cache bind-tools && dig @rusternetes-dns-server -p 8053 myservice.default.svc.cluster.local"
```

This is the **recommended approach** as it tests DNS the way production pods would use it.

## See Also

- [LOCAL_DEVELOPMENT.md](../docs/LOCAL_DEVELOPMENT.md) - Complete local development guide
- [STATUS.md](../docs/STATUS.md) - Project status and architecture

## Contributing

When adding new development scripts:
1. Clearly mark them as development-only
2. Document why they're needed (what platform limitation they work around)
3. Explain when to use them vs testing in a production-like way
4. Add them to this README
