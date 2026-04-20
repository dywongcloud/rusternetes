# CNI Networking Guide

This guide covers how container networking works in rusternetes and how to use third-party CNI plugins like Calico, Cilium, or Flannel.

## How Pod Networking Works

When a pod starts, the kubelet:

1. Creates a network namespace for the pod
2. Calls the CNI plugin to configure networking inside that namespace
3. The plugin assigns an IP address, sets up routes, and configures DNS
4. All containers in the pod share this network namespace (via the pause container)

## Default Configuration

Rusternetes ships with a bridge CNI configuration that provides basic pod networking:

```
Pod CIDR:     10.244.0.0/16
Service CIDR: 10.96.0.0/12
Cluster DNS:  10.96.0.10 (kube-dns service)
```

The default CNI config is at `cni-config/10-rusternetes.conflist` and uses:
- **bridge** plugin — creates a Linux bridge for pod-to-pod communication on the same node
- **host-local** IPAM — assigns IPs from the pod CIDR range
- **portmap** — supports `hostPort` in pod specs
- **firewall** — enforces network policies

## Using Third-Party CNI Plugins

Rusternetes implements the standard CNI specification (v0.4.0+). Any CNI plugin that follows the spec will work.

### Requirements

- Linux host with network namespace support (`ip netns`)
- CNI plugin binaries in `/opt/cni/bin/`
- CNI configuration in `/etc/cni/net.d/`
- The kubelet must be able to execute the plugin binaries

### Calico

[Calico](https://www.projectcalico.org/) provides BGP-based networking with full network policy support.

```bash
# Install Calico CNI binaries
curl -L https://github.com/projectcalico/cni-plugin/releases/latest/download/calico-amd64 -o /opt/cni/bin/calico
curl -L https://github.com/projectcalico/cni-plugin/releases/latest/download/calico-ipam-amd64 -o /opt/cni/bin/calico-ipam
chmod +x /opt/cni/bin/calico /opt/cni/bin/calico-ipam

# Create Calico CNI config
cat > /etc/cni/net.d/10-calico.conflist <<EOF
{
  "name": "k8s-pod-network",
  "cniVersion": "0.3.1",
  "plugins": [
    {
      "type": "calico",
      "log_level": "info",
      "datastore_type": "kubernetes",
      "ipam": { "type": "calico-ipam" },
      "policy": { "type": "k8s" }
    },
    { "type": "portmap", "capabilities": { "portMappings": true } }
  ]
}
EOF
```

### Cilium

[Cilium](https://cilium.io/) uses eBPF for high-performance networking.

```bash
# Install Cilium CNI
cilium install --config cluster-pool-ipv4-cidr=10.244.0.0/16

# Cilium automatically installs its CNI config and binaries
```

### Flannel

[Flannel](https://github.com/flannel-io/flannel) provides a simple overlay network.

```bash
# Install Flannel CNI binary
curl -L https://github.com/flannel-io/cni-plugin/releases/latest/download/flannel-amd64 -o /opt/cni/bin/flannel
chmod +x /opt/cni/bin/flannel

# Create Flannel CNI config
cat > /etc/cni/net.d/10-flannel.conflist <<EOF
{
  "name": "cbr0",
  "cniVersion": "0.3.1",
  "plugins": [
    {
      "type": "flannel",
      "delegate": { "hairpinMode": true, "isDefaultGateway": true }
    },
    { "type": "portmap", "capabilities": { "portMappings": true } }
  ]
}
EOF
```

## Docker Compose Deployment

In the Docker Compose setup, CNI plugins are mounted into the kubelet containers:

```yaml
kubelet:
  volumes:
    - /opt/cni/bin:/opt/cni/bin:ro
    - ./cni-config:/etc/cni/net.d:ro
```

To use a custom CNI plugin:
1. Place the plugin binaries in `/opt/cni/bin/` on the Docker host
2. Replace `cni-config/10-rusternetes.conflist` with your plugin's config
3. Restart the kubelet containers

## Limitations

### macOS (Docker Desktop / Podman Machine)

CNI plugins require Linux network namespaces. On macOS:
- Docker Desktop runs containers in a Linux VM, but the VM's network namespace isolation is limited
- The kubelet detects this and **automatically falls back to Docker's bridge networking**
- Pods still get IPs and can communicate, but CNI plugins are not invoked

### Containerized Kubelets

When kubelets run inside containers (Docker Compose deployment):
- Network namespace creation may be restricted depending on the container runtime's security settings
- The kubelet tests namespace support on startup and falls back if it fails
- For full CNI support, run kubelets directly on the host or in privileged containers

## Verifying CNI

Check if CNI is active:

```bash
# Check kubelet logs for CNI initialization
docker logs rusternetes-kubelet 2>&1 | grep -i cni

# Check pod IPs (CNI-assigned IPs are from the pod CIDR range)
kubectl get pods -o wide

# Test pod-to-pod connectivity
kubectl exec pod-a -- ping <pod-b-ip>
```

## Network Policies

Rusternetes supports Kubernetes NetworkPolicy resources. When using the default bridge CNI with the firewall plugin, network policies are enforced via iptables rules.

Third-party CNI plugins like Calico and Cilium bring their own network policy implementations, which may offer additional features like:
- Layer 7 (HTTP) policies (Cilium)
- Global network policies (Calico)
- DNS-based policies (Cilium)

See [Network Policies](networking/network-policies.md) for detailed usage.

## Troubleshooting

**Pods stuck in ContainerCreating:**
- Check kubelet logs for CNI errors: `docker logs rusternetes-kubelet 2>&1 | grep -i "cni\|network"`
- Verify CNI binaries exist: `ls /opt/cni/bin/`
- Verify CNI config exists: `ls /etc/cni/net.d/`

**Pods can't reach each other:**
- Check if pods have IPs: `kubectl get pods -o wide`
- Verify the CNI plugin created the bridge: `ip link show cni0` (on the node)
- Check iptables rules: `iptables -L -t nat | grep KUBE`

**CNI not initializing:**
- The kubelet tests network namespace support on startup
- If it fails, it logs a warning and falls back to Docker networking
- Check: `docker logs rusternetes-kubelet 2>&1 | grep "CNI\|fallback\|namespace"`
