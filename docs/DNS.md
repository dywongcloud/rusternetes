# Rusternetes DNS Server

The Rusternetes DNS Server provides Kubernetes-style DNS-based service discovery using Hickory DNS (formerly trust-dns). It enables pods to discover services and other pods using DNS names.

## Features

### Service Discovery
- **ClusterIP Services**: Resolve to the service's ClusterIP
- **Headless Services**: Resolve to all pod IPs backing the service
- **SRV Records**: Port and protocol discovery for headless services
- **Namespace Support**: Services are scoped to namespaces

### Pod Discovery
- **Name-based Resolution**: `<pod-name>.<namespace>.pod.cluster.local`
- **IP-based Resolution**: `<ip-with-dashes>.<namespace>.pod.cluster.local`
- **Running Pods Only**: Only running pods are registered

### DNS Record Types
- **A Records**: IPv4 addresses
- **AAAA Records**: IPv6 addresses (when available)
- **SRV Records**: Service discovery with port information

## DNS Naming Convention

Rusternetes follows the Kubernetes DNS naming convention:

### Services

```
<service>.<namespace>.svc.<cluster-domain>
```

Examples:
- `nginx.default.svc.cluster.local` → ClusterIP of nginx service in default namespace
- `redis.production.svc.cluster.local` → ClusterIP of redis service in production namespace
- `database.svc.cluster.local` → Short form (assumes default namespace)

### Headless Services

For services without a ClusterIP (headless services), DNS returns all pod IPs:

```bash
# Query headless service
$ dig nginx-headless.default.svc.cluster.local

# Returns multiple A records:
nginx-headless.default.svc.cluster.local. 10 IN A 10.244.1.5
nginx-headless.default.svc.cluster.local. 10 IN A 10.244.1.6
nginx-headless.default.svc.cluster.local. 10 IN A 10.244.1.7
```

### SRV Records for Headless Services

```
_<port-name>._<protocol>.<service>.<namespace>.svc.<cluster-domain>
```

Example:
```bash
$ dig SRV _http._tcp.nginx-headless.default.svc.cluster.local

# Returns:
_http._tcp.nginx-headless.default.svc.cluster.local. 10 IN SRV 0 100 80 nginx-pod-1.default.pod.cluster.local.
_http._tcp.nginx-headless.default.svc.cluster.local. 10 IN SRV 0 100 80 nginx-pod-2.default.pod.cluster.local.
```

### Pods

```
<pod-name>.<namespace>.pod.<cluster-domain>
```

Or IP-based:
```
<ip-with-dashes>.<namespace>.pod.<cluster-domain>
```

Examples:
- `nginx-pod-abc123.default.pod.cluster.local` → Pod's IP address
- `10-244-1-5.default.pod.cluster.local` → 10.244.1.5

## Configuration

The DNS server accepts the following command-line arguments:

| Flag | Default | Description |
|------|---------|-------------|
| `--etcd-endpoint` | `http://localhost:2379` | etcd server endpoint |
| `--listen-addr` | `0.0.0.0:53` | DNS server listen address |
| `--cluster-domain` | `cluster.local` | Cluster DNS domain |
| `--ttl` | `10` | DNS record TTL in seconds |
| `--sync-interval-secs` | `30` | Resource sync interval |

### Deployment Notes

**Port Configuration for Development:**
- **Standard Port 53**: Requires privileged access (NET_BIND_SERVICE capability)
- **Recommended Port 8053**: Works without privileges in Podman/Docker
- Port 5353 may conflict with macOS mDNS/Bonjour service
- The docker-compose.yml uses port 8053 for development compatibility

### Example Configuration

**Production (with privileges):**
```bash
rusternetes-dns-server \
  --etcd-endpoint http://etcd:2379 \
  --listen-addr 0.0.0.0:53 \
  --cluster-domain cluster.local \
  --ttl 10 \
  --sync-interval-secs 30
```

**Development (unprivileged):**
```bash
rusternetes-dns-server \
  --etcd-endpoint http://etcd:2379 \
  --listen-addr 0.0.0.0:8053 \
  --cluster-domain cluster.local \
  --ttl 10 \
  --sync-interval-secs 30
```

## Architecture

### Components

1. **DNS Server** (`server.rs`)
   - Listens on UDP port 53
   - Handles DNS queries using Hickory DNS protocol
   - Responds with A, AAAA, and SRV records

2. **Kubernetes Resolver** (`resolver.rs`)
   - In-memory DNS record cache
   - Maps Kubernetes resources to DNS records
   - Handles service and pod name resolution

3. **Resource Watcher** (`watcher.rs`)
   - Monitors etcd for Service, Endpoint, and Pod changes
   - Updates DNS resolver cache every 30 seconds
   - Ensures DNS records stay in sync with cluster state

### Data Flow

```
┌─────────────────────────────────────────────────────────────┐
│                         etcd                                │
│         (Services, Endpoints, Pods)                         │
└─────────────────────────────────────────────────────────────┘
                           │
                           │ watch/sync (30s)
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                   Resource Watcher                          │
│   - List Services & Endpoints                               │
│   - List Pods (Running only)                                │
│   - Update resolver cache                                   │
└─────────────────────────────────────────────────────────────┘
                           │
                           │ update records
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                 Kubernetes Resolver                         │
│   - In-memory DNS record cache                              │
│   - A/AAAA/SRV record mapping                               │
└─────────────────────────────────────────────────────────────┘
                           │
                           │ lookup
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                      DNS Server                             │
│   - UDP socket on port 53                                   │
│   - Hickory DNS protocol handler                            │
│   - Query parsing & response generation                     │
└─────────────────────────────────────────────────────────────┘
                           │
                           │ DNS query/response
                           ▼
                        Clients
```

## Usage Examples

### Testing DNS Resolution

Using `dig` from inside a pod:

```bash
# Install dig in a pod
kubectl exec -it test-pod -- apk add bind-tools

# Query a service
kubectl exec -it test-pod -- dig nginx.default.svc.cluster.local

# Query a headless service
kubectl exec -it test-pod -- dig nginx-headless.default.svc.cluster.local

# Query SRV record
kubectl exec -it test-pod -- dig SRV _http._tcp.nginx-headless.default.svc.cluster.local

# Query a pod
kubectl exec -it test-pod -- dig nginx-pod-abc123.default.pod.cluster.local
```

Using `nslookup`:

```bash
# Query service
nslookup nginx.default.svc.cluster.local 10.96.0.10

# Query pod
nslookup 10-244-1-5.default.pod.cluster.local 10.96.0.10
```

### Configuring Pod DNS

Pods automatically use the DNS server when configured properly:

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: dns-test
  namespace: default
spec:
  containers:
  - name: test
    image: alpine:latest
    command: ["sleep", "3600"]
  dnsPolicy: ClusterFirst
  dnsConfig:
    nameservers:
      - 10.96.0.10  # DNS server ClusterIP
    searches:
      - default.svc.cluster.local
      - svc.cluster.local
      - cluster.local
```

### Creating a DNS Service

To make the DNS server accessible via ClusterIP:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: kube-dns
  namespace: kube-system
  labels:
    k8s-app: kube-dns
spec:
  clusterIP: 10.96.0.10
  selector:
    k8s-app: dns-server
  ports:
  - name: dns
    port: 53
    protocol: UDP
    targetPort: 53
  - name: dns-tcp
    port: 53
    protocol: TCP
    targetPort: 53
```

## Service Types and DNS Behavior

### ClusterIP Service

```yaml
apiVersion: v1
kind: Service
metadata:
  name: nginx
  namespace: default
spec:
  type: ClusterIP
  clusterIP: 10.96.1.100
  selector:
    app: nginx
  ports:
  - port: 80
    targetPort: 8080
```

DNS Resolution:
```bash
$ dig nginx.default.svc.cluster.local
nginx.default.svc.cluster.local. 10 IN A 10.96.1.100
```

### Headless Service (No ClusterIP)

```yaml
apiVersion: v1
kind: Service
metadata:
  name: nginx-headless
  namespace: default
spec:
  type: ClusterIP
  clusterIP: None  # Headless
  selector:
    app: nginx
  ports:
  - name: http
    port: 80
    targetPort: 8080
```

DNS Resolution (returns all pod IPs):
```bash
$ dig nginx-headless.default.svc.cluster.local
nginx-headless.default.svc.cluster.local. 10 IN A 10.244.1.5
nginx-headless.default.svc.cluster.local. 10 IN A 10.244.1.6
nginx-headless.default.svc.cluster.local. 10 IN A 10.244.1.7
```

SRV Records:
```bash
$ dig SRV _http._tcp.nginx-headless.default.svc.cluster.local
_http._tcp.nginx-headless.default.svc.cluster.local. 10 IN SRV 0 100 80 pod-1.default.pod.cluster.local.
_http._tcp.nginx-headless.default.svc.cluster.local. 10 IN SRV 0 100 80 pod-2.default.pod.cluster.local.
```

### NodePort Service

NodePort services behave like ClusterIP services for DNS:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: nginx-nodeport
  namespace: default
spec:
  type: NodePort
  clusterIP: 10.96.1.200
  selector:
    app: nginx
  ports:
  - port: 80
    targetPort: 8080
    nodePort: 30080
```

DNS Resolution:
```bash
$ dig nginx-nodeport.default.svc.cluster.local
nginx-nodeport.default.svc.cluster.local. 10 IN A 10.96.1.200
```

### LoadBalancer Service

LoadBalancer services behave like ClusterIP services for DNS:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: nginx-lb
  namespace: default
spec:
  type: LoadBalancer
  clusterIP: 10.96.1.250
  selector:
    app: nginx
  ports:
  - port: 80
    targetPort: 8080
```

DNS Resolution:
```bash
$ dig nginx-lb.default.svc.cluster.local
nginx-lb.default.svc.cluster.local. 10 IN A 10.96.1.250
```

## Integration with Rusternetes

The DNS server integrates with the following components:

### Endpoints Controller
The Endpoints controller maintains endpoint lists for services. The DNS server reads these to:
- Determine if a service is headless (no ClusterIP)
- Get pod IPs for headless services
- Create SRV records with pod information

### Service API
Services are stored in etcd at `/registry/services/<namespace>/<name>`. The DNS server:
- Reads the ClusterIP for normal services
- Detects headless services (ClusterIP = "None")
- Creates DNS A/AAAA records

### Pod API
Pods are stored in etcd at `/registry/pods/<namespace>/<name>`. The DNS server:
- Only registers running pods
- Reads pod IPs from status
- Creates name-based and IP-based DNS records

## Troubleshooting

### DNS Not Resolving

1. **Check DNS server is running:**
   ```bash
   podman ps | grep dns-server
   ```

2. **Check DNS server logs:**
   ```bash
   podman logs rusternetes-dns-server
   ```

3. **Verify service exists in etcd:**
   ```bash
   ./target/release/kubectl get service nginx -n default
   ```

4. **Test DNS directly:**
   ```bash
   # Development port
   dig @localhost -p 8053 nginx.default.svc.cluster.local

   # Or standard port (if running with privileges)
   dig @localhost -p 53 nginx.default.svc.cluster.local
   ```

### No Records for Service

Check that:
- The service exists in the correct namespace
- The service has a ClusterIP or endpoints
- The DNS sync interval has elapsed (default 30s)

### SRV Records Not Working

Ensure:
- The service is headless (clusterIP: None)
- The service has endpoints with pod references
- Port names are defined in the service spec

## Performance Considerations

### Memory Usage
- Each DNS record requires ~100-200 bytes of memory
- 1000 services × 10 pods each = ~2 MB memory

### Cache Updates
- Default sync interval: 30 seconds
- Adjust `--sync-interval-secs` based on cluster size
- Larger intervals reduce etcd load but increase DNS staleness

### Query Performance
- In-memory cache provides sub-millisecond lookups
- No external dependencies during queries
- UDP protocol for low latency

## Comparison with CoreDNS

| Feature | Rusternetes DNS | CoreDNS |
|---------|----------------|---------|
| Service Discovery | ✅ | ✅ |
| Pod Discovery | ✅ | ✅ |
| SRV Records | ✅ | ✅ |
| Custom Domains | ⏹️ | ✅ |
| External DNS | ⏹️ | ✅ |
| Caching | ✅ In-memory | ✅ Configurable |
| Plugins | ⏹️ | ✅ Extensive |
| Performance | Fast (Rust) | Fast (Go) |

## Future Enhancements

### Planned Features
- ⏹️ **Watch API**: Real-time updates instead of polling
- ⏹️ **TCP Support**: Full TCP DNS support
- ⏹️ **Metrics**: Prometheus metrics for DNS queries
- ⏹️ **DNSSEC**: Security extensions
- ⏹️ **External DNS**: Forward to upstream resolvers
- ⏹️ **Custom Domains**: Support additional domains

### Performance Improvements
- ⏹️ **LRU Cache**: Limit memory usage for large clusters
- ⏹️ **Incremental Updates**: Watch API for delta updates
- ⏹️ **Connection Pooling**: Reuse etcd connections

## Development

### Building

```bash
cargo build --release --bin rusternetes-dns-server
```

### Testing

```bash
# Run unit tests
cargo test --package rusternetes-dns-server

# Start DNS server locally
./target/release/rusternetes-dns-server \
  --etcd-endpoint http://localhost:2379 \
  --listen-addr 127.0.0.1:5353
```

### Testing DNS Queries

**With standard port 53:**
```bash
dig @127.0.0.1 nginx.default.svc.cluster.local
nslookup nginx.default.svc.cluster.local 127.0.0.1
```

**With development port 8053:**
```bash
dig @127.0.0.1 -p 8053 nginx.default.svc.cluster.local
nslookup -port=8053 nginx.default.svc.cluster.local 127.0.0.1
```

## Security Considerations

### Network Access
- DNS server binds to 0.0.0.0:53 by default
- Use firewall rules to restrict access if needed
- Consider using `--listen-addr` to bind to specific interface

### DNS Spoofing
- DNS responses are not signed (no DNSSEC)
- Use network policies to restrict DNS access
- Consider implementing DNSSEC for production

### etcd Access
- DNS server requires read access to etcd
- Use etcd authentication if needed
- Limit DNS server to read-only operations

## License

Apache 2.0 - Same as Rusternetes project
