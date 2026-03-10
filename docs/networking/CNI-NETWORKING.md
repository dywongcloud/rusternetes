# CNI Networking in Rusternetes

## Overview

Rusternetes includes full Container Network Interface (CNI) support for production Kubernetes environments. The CNI implementation follows the standard Kubernetes networking model with proper network namespace isolation, IP address management, and DNS configuration.

## Architecture

### CNI Runtime Components

Located in `crates/kubelet/src/cni/`:

- **CniRuntime**: Main CNI orchestrator that manages network setup/teardown
- **CniPluginManager**: Discovers and manages CNI plugins
- **CniConfig**: Parses and validates CNI network configurations
- **Network Namespace Management**: Creates isolated network namespaces per pod

### Integration Points

The kubelet integrates CNI at these key points:

1. **Initialization** (`runtime.rs:75-88`): Detects CNI plugins and configuration
2. **Pod Start** (`runtime.rs:304-308`): Creates network namespace and configures networking
3. **Container Attach** (`runtime.rs:957-961`): Attaches containers to pod network namespace
4. **Pod Stop** (`runtime.rs:1047-1053`): Tears down CNI networking and cleans up
5. **IP Retrieval** (`runtime.rs:1348-1356`): Gets pod IPs from CNI runtime

### CNI Configuration

Default configuration at `/etc/cni/net.d/10-rusternetes.conflist`:

```json
{
  "cniVersion": "1.0.0",
  "name": "rusternetes",
  "plugins": [
    {
      "type": "bridge",
      "bridge": "cni0",
      "isGateway": true,
      "ipMasq": true,
      "hairpinMode": true,
      "ipam": {
        "type": "host-local",
        "ranges": [[{"subnet": "10.244.0.0/16"}]],
        "routes": [{"dst": "0.0.0.0/0"}]
      },
      "dns": {
        "nameservers": ["10.96.0.10"],
        "domain": "cluster.local",
        "search": ["svc.cluster.local", "cluster.local"],
        "options": ["ndots:5"]
      }
    },
    {
      "type": "portmap",
      "capabilities": {"portMappings": true},
      "snat": true
    },
    {
      "type": "firewall"
    }
  ]
}
```

## Production Deployment

### Requirements

For CNI to work in production:

1. **CNI Plugins**: Install at `/opt/cni/bin/`
   ```bash
   curl -L https://github.com/containernetworking/plugins/releases/download/v1.4.0/cni-plugins-linux-amd64-v1.4.0.tgz | \
     tar -C /opt/cni/bin -xz
   ```

2. **Network Configuration**: Place config at `/etc/cni/net.d/`

3. **Kernel Support**: Linux kernel with network namespace support

4. **iptables**: Required for bridge plugin

5. **Capabilities**: Kubelet needs `NET_ADMIN` capability

### Deployment Environments

✅ **Supported**:
- Bare metal Linux servers
- Linux VMs with full kernel support
- Standard Kubernetes distributions (kubeadm, etc.)
- Container runtimes: containerd, CRI-O, Docker

✅ **Rusternetes CNI Works**:
- When deployed on Linux with proper kernel support
- With any standard CNI plugins (Calico, Flannel, Weave, etc.)
- In production Kubernetes clusters

## Podman Machine (macOS) Limitations

### Why CNI Doesn't Work in Podman Machine

Podman Machine on macOS runs containers inside a Fedora VM. This creates a networking isolation problem:

1. **Kubelet Container**: Runs inside the VM, creates network namespaces at `/var/run/netns/cni-pod-name`
2. **Pod Containers**: Also run inside the VM, but in separate container contexts
3. **Namespace Isolation**: Podman cannot share network namespaces between containers when they're created in different container contexts

**The Problem**:
```
kubelet container → creates /var/run/netns/cni-test-pod
pod container → tries to join ns:/var/run/netns/cni-test-pod
                ❌ Error: "cannot find specified network namespace path"
```

The network namespace exists in the kubelet container's filesystem, but pod containers can't access it because they have their own isolated filesystem view.

### Fallback: Podman Bridge Networking

When CNI is unavailable (detected at kubelet startup), Rusternetes automatically falls back to Podman's native bridge networking with custom DNS configuration:

**Features**:
- ✅ Each pod gets an IP address from Podman's bridge network
- ✅ Custom `/etc/resolv.conf` mounted with cluster DNS settings
- ✅ DNS points to CoreDNS (10.96.0.10)
- ✅ Service discovery works via DNS
- ✅ Kube-proxy routes traffic to services
- ✅ Passes Kubernetes conformance tests (networking features)

**Implementation** (`runtime.rs:885-912`):
```rust
// Create and mount custom resolv.conf for non-CoreDNS pods
if pod_name != "coredns" {
    let resolv_conf_path = format!("{}/{}/resolv.conf", self.volumes_base_path, pod_name);
    let resolv_conf_content = format!(
        "nameserver {}\nsearch {}.svc.{} svc.{} {}\noptions ndots:5\n",
        self.cluster_dns,
        namespace,
        self.cluster_domain,
        self.cluster_domain,
        self.cluster_domain
    );

    std::fs::write(&resolv_conf_path, resolv_conf_content)?;
    binds.push(format!("{}:/etc/resolv.conf:ro", resolv_conf_path));
}
```

## Detecting CNI Availability

The kubelet automatically detects CNI at startup:

```rust
// Initialize CNI if plugins are available
let (cni, use_cni) = match Self::initialize_cni() {
    Ok(cni_runtime) => {
        info!("CNI networking enabled");
        (Some(cni_runtime), true)
    }
    Err(e) => {
        warn!("CNI not available, falling back to Podman networking: {}", e);
        (None, false)
    }
};
```

**Log messages**:
- `CNI networking enabled` → Full CNI support active
- `CNI not available, falling back to Podman networking` → Using Podman bridge with custom DNS

## Testing

### Test CNI in Production Environment

```bash
# Deploy to Linux server or VM
kubectl apply -f test-cni-pod.yaml

# Verify CNI setup
kubectl exec test-pod -- ip addr show eth0
# Should show IP from 10.244.0.0/16 range

# Verify DNS
kubectl exec test-pod -- nslookup kubernetes.default
# Should resolve via CoreDNS
```

### Test Podman Machine (Development)

```bash
# Podman will fall back to bridge networking
kubectl apply -f test-pod.yaml

# Pod gets Podman bridge IP (10.89.x.x)
kubectl get pod test-pod -o wide

# DNS still works via custom resolv.conf
kubectl exec test-pod -- nslookup kubernetes.default
```

## Conformance Testing

Rusternetes passes Kubernetes conformance tests in both modes:

- **With CNI** (production): Full conformance with proper network isolation
- **With Podman** (development): Conformance for networking features (DNS, services, connectivity)

Network policies are not tested in basic conformance and would require CNI.

## Future Enhancements

Potential improvements for Podman Machine support:

1. **Shared Network Namespace Volume**: Mount `/var/run/netns` from host into all containers
2. **CNI in Host Mode**: Run kubelet with `--network=host` to share VM's network namespace
3. **Custom CNI Plugin**: Build a Podman-aware CNI plugin that uses Podman's network APIs

For now, the fallback approach provides full Kubernetes functionality for development and testing.
