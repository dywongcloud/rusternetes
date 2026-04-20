# Docker Desktop Migration Guide

**Date**: March 12, 2026

## Summary

Rusternetes now uses **Docker Desktop** on macOS instead of Podman Machine due to compatibility issues with macOS Sequoia 15.7+. This change resolves critical blockers for networking and service routing.

## Why the Migration?

### Podman Machine Issue (macOS Sequoia 15.7+)

Podman Machine on macOS Sequoia 15.7+ encounters a critical bug in the Apple Virtualization Framework:

```
Error: vfkit exited unexpectedly with exit code 1
Error Domain=VZErrorDomain Code=1
Description="Internal Virtualization error. The virtual machine failed to start."
```

This prevents Podman Machine VMs from starting entirely, making local development impossible on affected macOS versions.

### Rootful Mode Requirement

Kube-proxy requires rootful container execution to access iptables for service routing. Without rootful mode, kube-proxy fails with:

```
Permission denied (you must be root)
```

Docker Desktop automatically provides rootful execution, while Podman Machine on macOS requires manual configuration that wasn't possible due to the VM startup issue.

## Platform Support

### macOS
- **Recommended**: Docker Desktop (required on Sequoia 15.7+)
- **Not Recommended**: Podman Machine (known issues on Sequoia 15.7+)

### Linux
- **Option 1**: Docker (standard installation)
- **Option 2**: Podman in rootful mode (`sudo podman-compose -f compose.yml up -d`)

### Windows
- **Recommended**: Docker Desktop
- **Alternative**: WSL2 with Docker or Podman

## Changes Made

### docker-compose.yml

**Socket Path:**
```yaml
# Before (Podman)
- /run/user/501/podman/podman.sock:/var/run/docker.sock:rw

# After (Docker)
- /var/run/docker.sock:/var/run/docker.sock:rw
```

**Kube-proxy Configuration:**
```yaml
# Removed Podman-specific option
# userns_mode: host  # Not supported in Docker Compose
```

### Documentation Updates

Updated files:
- `README.md` - Quick start now shows Docker Desktop setup
- `docs/DEVELOPMENT.md` - Comprehensive Docker Desktop instructions
- `docs/PODMAN_TIPS.md` - Added macOS compatibility warning

## Verified Working Features

All core features now working with Docker Desktop:

✅ **Kube-proxy with iptables**
- Iptables chains initialized successfully
- ClusterIP service routing working
- No permission errors

✅ **Pod IP Reporting**
- Pods receive IPs from Docker bridge network
- IPs correctly reported in pod status
- Endpoints controller populates service endpoints

✅ **DNS Resolution**
- CoreDNS pod running at 172.18.0.7
- kube-dns service at 10.96.0.10
- Service IP routing via kube-proxy working
- Test pod successfully resolved `kubernetes.default.svc.cluster.local`

✅ **CNI Fallback**
- CNI framework detects Docker environment
- Gracefully falls back to Docker bridge networking
- Custom resolv.conf bypasses Docker's default DNS

## Code Changes (All Platform-Agnostic)

### crates/kubelet/src/runtime.rs (lines 1409-1429)

**Fixed pod IP retrieval** to check specific network:

```rust
// First try to get IP from the specific network we're using
if let Some(networks) = network_settings.networks {
    if let Some(network_info) = networks.get(&self.network) {
        if let Some(ip) = &network_info.ip_address {
            if !ip.is_empty() && ip != "0.0.0.0" {
                return Ok(Some(ip.clone()));
            }
        }
    }
}
```

**Why needed**: Docker creates multiple networks, and the default network may not have the IP. We need to check the named `rusternetes-network`.

### crates/kube-proxy/src/iptables.rs

**Use iptables-legacy with full path:**

```rust
const IPTABLES_CMD: &str = "/usr/sbin/iptables-legacy";
```

**Why needed**: Ensures compatibility across different container environments. Some systems have both `iptables-nft` and `iptables-legacy`.

### crates/kubelet/src/runtime.rs (CNI integration)

**CNI with automatic fallback:**

```rust
let (cni, use_cni) = match Self::initialize_cni() {
    Ok(cni_runtime) => {
        info!("CNI networking enabled");
        (Some(cni_runtime), true)
    }
    Err(e) => {
        warn!("CNI not available, falling back to Docker networking: {}", e);
        (None, false)
    }
};
```

**Why needed**: Allows CNI to work in production Linux environments while gracefully falling back to Docker/Podman bridge networking in development.

## Quick Start Commands

### Docker Desktop (macOS/Windows)

```bash
# Set volumes path
export KUBELET_VOLUMES_PATH=$(pwd)/.rusternetes/volumes

# Build and start
docker-compose build
docker-compose up -d

# Bootstrap
cat bootstrap-cluster.yaml | envsubst > /tmp/bootstrap-expanded.yaml
KUBECONFIG=/dev/null ./target/release/kubectl --insecure-skip-tls-verify apply -f /tmp/bootstrap-expanded.yaml

# Verify
KUBECONFIG=/dev/null ./target/release/kubectl --insecure-skip-tls-verify get pods -A
```

### Podman (Linux Only - Rootful)

```bash
# Set volumes path
export KUBELET_VOLUMES_PATH=$(pwd)/.rusternetes/volumes

# Build and start (rootful mode)
sudo KUBELET_VOLUMES_PATH=$KUBELET_VOLUMES_PATH podman-compose -f compose.yml build
sudo KUBELET_VOLUMES_PATH=$KUBELET_VOLUMES_PATH podman-compose -f compose.yml up -d

# Bootstrap
cat bootstrap-cluster.yaml | envsubst > /tmp/bootstrap-expanded.yaml
KUBECONFIG=/dev/null ./target/release/kubectl --insecure-skip-tls-verify apply -f /tmp/bootstrap-expanded.yaml

# Verify
KUBECONFIG=/dev/null ./target/release/kubectl --insecure-skip-tls-verify get pods -A
```

## Troubleshooting

### Docker Desktop Not Starting

```bash
# macOS - start from Applications or:
open -a Docker

# Verify
docker info
```

### Kube-proxy Permission Denied

This indicates not running in rootful mode:
- **Docker Desktop**: Should never happen (automatically rootful)
- **Podman**: Use `sudo podman-compose -f compose.yml up -d`

### Podman vfkit Error on macOS

If you see `vfkit exited unexpectedly`, you're hitting the macOS Sequoia bug. Install Docker Desktop:

```bash
brew install --cask docker
```

## Benefits of This Change

1. **Reliability**: Docker Desktop doesn't have the macOS Virtualization Framework bug
2. **Automatic Rootful**: No manual configuration needed for kube-proxy
3. **Wide Adoption**: More developers already have Docker Desktop installed
4. **Consistent Behavior**: Same experience across macOS and Windows
5. **Better Testing**: Matches production Kubernetes environments more closely

## Backward Compatibility

- **Linux users** can still use Podman (rootful mode recommended)
- **Existing code** works on both Docker and Podman
- **No Podman-specific workarounds** remain in the code
- **CNI fallback** works identically on both platforms

## Future Considerations

- Monitor Podman Machine fixes for macOS Sequoia
- Consider re-adding Podman Machine support when Apple fixes the Virtualization Framework
- Current code is platform-agnostic and ready for either runtime

## Related Documentation

- [README.md](../README.md) - Quick start guide
- [docs/DEVELOPMENT.md](DEVELOPMENT.md) - Detailed development setup
- [docs/PODMAN_TIPS.md](PODMAN_TIPS.md) - Podman tips (with macOS warning)
- [CNI Guide](CNI_GUIDE.md) - CNI implementation details

## Verification Checklist

✅ Docker Desktop installed and running
✅ Cluster starts successfully
✅ All pods running (etcd, api-server, scheduler, controller-manager, kubelet, kube-proxy)
✅ CoreDNS deployed and running
✅ Kube-proxy iptables initialized without errors
✅ Pod IPs reported in status
✅ Service endpoints populated
✅ DNS resolution working via service IP
✅ Documentation updated

## Support

For issues:
1. Check [DEVELOPMENT.md](DEVELOPMENT.md) troubleshooting section
2. Verify Docker Desktop is running: `docker info`
3. Check logs: `docker-compose logs -f`
4. For macOS Sequoia + Podman issues, see [PODMAN_TIPS.md](PODMAN_TIPS.md)
