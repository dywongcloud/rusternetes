# LoadBalancer Services in Rusternetes

Rusternetes supports LoadBalancer-type Services through two approaches:

1. **MetalLB** - For local development, bare-metal, and on-premises deployments (recommended for non-cloud environments)
2. **Cloud Provider Integration** - For production cloud deployments (AWS, GCP, Azure)

## Quick Start

### For Local Development and Testing

Use **MetalLB** for the simplest setup without cloud credentials:

```bash
# Install MetalLB
kubectl apply -f https://raw.githubusercontent.com/metallb/metallb/v0.14.3/config/manifests/metallb-native.yaml

# Configure IP pool (adjust for your network)
kubectl apply -f examples/metallb/metallb-config-podman.yaml

# Create a LoadBalancer service
kubectl apply -f examples/networking/test-loadbalancer-service.yaml

# Get the external IP
kubectl get svc
```

See the [MetalLB Integration Guide](docs/METALLB_INTEGRATION.md) for complete instructions.

### For Cloud Deployments

Use the built-in cloud provider integration for AWS, GCP, or Azure. See [Cloud Provider Configuration](#configuration) below.

## Overview

LoadBalancer Services provide external access to your applications. Depending on your environment:

- **MetalLB** allocates IPs from a local pool and announces them via ARP/BGP
- **Cloud Providers** create managed load balancers (AWS NLB, GCP LB, Azure LB)

Both approaches work with the same Kubernetes Service API - just create a Service with `type: LoadBalancer`.

## MetalLB for Local and On-Premises Deployments

**MetalLB** is the recommended solution for:
- Local development and testing
- Bare-metal clusters
- On-premises data centers
- Edge computing scenarios
- Environments without cloud provider access

### Why MetalLB?

- ✅ **No cloud credentials required**
- ✅ **Free and open source**
- ✅ **Production-ready** (used by thousands of clusters)
- ✅ **Simple setup** (5 minutes to get started)
- ✅ **Works anywhere** (local, bare-metal, edge)

### Quick MetalLB Setup

1. **Install MetalLB:**
   ```bash
   kubectl apply -f https://raw.githubusercontent.com/metallb/metallb/v0.14.3/config/manifests/metallb-native.yaml
   ```

2. **Configure IP pool:**
   ```bash
   cat <<EOF | kubectl apply -f -
   apiVersion: metallb.io/v1beta1
   kind: IPAddressPool
   metadata:
     name: default-pool
     namespace: metallb-system
   spec:
     addresses:
     - 192.168.1.240-192.168.1.250  # Adjust for your network
   ---
   apiVersion: metallb.io/v1beta1
   kind: L2Advertisement
   metadata:
     name: default-l2
     namespace: metallb-system
   EOF
   ```

3. **Create a LoadBalancer service** - MetalLB will automatically assign an IP!

For complete instructions, configuration examples, and troubleshooting, see:
**[MetalLB Integration Guide](docs/METALLB_INTEGRATION.md)**

## Cloud Provider Integration

For production cloud deployments, Rusternetes includes built-in cloud provider support:

### Supported Cloud Providers

#### AWS (Amazon Web Services) ✅ IMPLEMENTED
- **Load Balancer Type**: Network Load Balancer (NLB)
- **Features**:
  - Automatic NLB creation with target groups
  - NodePort-based load balancing
  - Internet-facing or internal load balancers
  - Automatic tagging for cluster management
  - DNS-based access (hostname in status.loadBalancer.ingress)

### GCP (Google Cloud Platform) ⚠️ STUB
- **Status**: Stub implementation, not yet functional
- **Planned**: Google Cloud Load Balancing integration

#### Azure (Microsoft Azure) ⚠️ STUB
- **Status**: Stub implementation, not yet functional
- **Planned**: Azure Load Balancer integration

## Architecture

### Components

1. **Cloud Provider Trait** (`rusternetes-common/src/cloud_provider.rs`)
   - Defines the interface all cloud providers must implement
   - Methods: `ensure_load_balancer()`, `delete_load_balancer()`, `get_load_balancer_status()`

2. **Cloud Provider Implementations** (`rusternetes-cloud-providers/src/`)
   - AWS provider: Full NLB implementation
   - GCP provider: Stub implementation
   - Azure provider: Stub implementation

3. **LoadBalancer Controller** (`controller-manager/src/controllers/loadbalancer.rs`)
   - Watches Services with `type: LoadBalancer`
   - Reconciles cloud provider load balancers with service specifications
   - Updates Service status with external IPs/hostnames
   - 30-second reconciliation loop (configurable)

## Configuration

### Enable Cloud Provider Support

Build the controller-manager with cloud provider support:

```bash
# AWS only
cargo build --release --bin controller-manager --features aws

# All cloud providers
cargo build --release --bin controller-manager --features all-cloud-providers

# Specific combination
cargo build --release --bin controller-manager --features "aws,gcp"
```

### Controller Manager Configuration

Start the controller-manager with cloud provider configuration:

```bash
controller-manager \
  --cloud-provider aws \
  --cluster-name my-cluster \
  --cloud-region us-west-2
```

**Command Line Arguments:**
- `--cloud-provider`: Cloud provider name (aws, gcp, azure, or none)
- `--cluster-name`: Cluster name used for tagging cloud resources (default: "rusternetes")
- `--cloud-region`: Cloud provider region (optional, auto-detected on AWS)

**Environment Variables:**
The controller manager also supports configuration via environment variables:
- `CLOUD_PROVIDER`: Same as `--cloud-provider`
- `CLUSTER_NAME`: Same as `--cluster-name`
- `CLOUD_REGION`: Same as `--cloud-region`

### AWS-Specific Configuration

The AWS provider requires:

1. **VPC Configuration**: Set via environment variables
   ```bash
   export AWS_VPC_ID=vpc-xxxxx
   export AWS_SUBNET_IDS=subnet-xxxxx,subnet-yyyyy
   ```

2. **AWS Credentials**: Configured via standard AWS credential chain
   - IAM instance role (recommended for EC2 instances)
   - AWS credentials file (`~/.aws/credentials`)
   - Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)

3. **IAM Permissions**: The controller manager needs permissions to:
   - Create/delete Network Load Balancers
   - Create/delete target groups
   - Register/deregister targets
   - Create/delete listeners
   - Tag load balancers

Example IAM policy:
```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "elasticloadbalancing:CreateLoadBalancer",
        "elasticloadbalancing:DeleteLoadBalancer",
        "elasticloadbalancing:DescribeLoadBalancers",
        "elasticloadbalancing:CreateTargetGroup",
        "elasticloadbalancing:DeleteTargetGroup",
        "elasticloadbalancing:DescribeTargetGroups",
        "elasticloadbalancing:RegisterTargets",
        "elasticloadbalancing:DeregisterTargets",
        "elasticloadbalancing:CreateListener",
        "elasticloadbalancing:DeleteListener",
        "elasticloadbalancing:DescribeListeners",
        "elasticloadbalancing:AddTags"
      ],
      "Resource": "*"
    }
  ]
}
```

## Usage Examples

### Basic LoadBalancer Service

```yaml
apiVersion: v1
kind: Service
metadata:
  name: my-app
  namespace: default
spec:
  type: LoadBalancer
  selector:
    app: my-app
  ports:
    - name: http
      protocol: TCP
      port: 80
      targetPort: 8080
      nodePort: 30080  # Must be allocated for LoadBalancer
```

Apply the service:
```bash
kubectl apply -f service.yaml
```

Check the external IP/hostname:
```bash
kubectl get service my-app

# Output:
# NAME     TYPE           CLUSTER-IP      EXTERNAL-IP                                      PORT(S)
# my-app   LoadBalancer   10.96.100.123   a1b2c3-us-west-2.elb.amazonaws.com             80:30080/TCP
```

### Internal LoadBalancer (AWS)

Use annotations to create an internal load balancer:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: internal-app
  namespace: default
  annotations:
    service.beta.kubernetes.io/aws-load-balancer-internal: "true"
spec:
  type: LoadBalancer
  selector:
    app: internal-app
  ports:
    - port: 443
      targetPort: 8443
      nodePort: 30443
```

### Multi-Port LoadBalancer

```yaml
apiVersion: v1
kind: Service
metadata:
  name: multi-port-app
  namespace: default
spec:
  type: LoadBalancer
  selector:
    app: multi-port-app
  ports:
    - name: http
      protocol: TCP
      port: 80
      targetPort: 8080
      nodePort: 30080
    - name: https
      protocol: TCP
      port: 443
      targetPort: 8443
      nodePort: 30443
```

## Status and Monitoring

### Service Status

Once the load balancer is provisioned, check the status:

```bash
kubectl get service my-app -o yaml
```

Look for the `status.loadBalancer.ingress` field:

```yaml
status:
  loadBalancer:
    ingress:
      - hostname: a1b2c3-us-west-2.elb.amazonaws.com
```

### Controller Logs

Monitor the LoadBalancer controller logs:

```bash
# If running in Podman
podman logs -f rusternetes-controller-manager | grep LoadBalancer

# Look for messages like:
# [INFO] Initializing aws cloud provider
# [INFO] Ensuring AWS NLB for service default/my-app
# [INFO] Creating NLB: rusternetes-default-my-app
# [INFO] Successfully reconciled LoadBalancer service default/my-app
```

## Lifecycle

### Creation Flow

1. User creates Service with `type: LoadBalancer`
2. API server allocates ClusterIP (if not specified)
3. LoadBalancer controller detects new LoadBalancer service
4. Controller calls cloud provider's `ensure_load_balancer()`
5. Cloud provider creates load balancer with target groups
6. Cloud provider returns external IP/hostname
7. Controller updates Service status with external IP/hostname

### Update Flow

1. User updates Service (e.g., adds a port)
2. LoadBalancer controller detects change
3. Controller reconciles load balancer configuration
4. Cloud provider updates listeners and target groups
5. Service status remains updated

### Deletion Flow

1. User deletes Service
2. LoadBalancer controller detects deletion
3. Controller calls cloud provider's `delete_load_balancer()`
4. Cloud provider deletes load balancer and associated resources
5. ClusterIP is released back to the pool

## Troubleshooting

### Load Balancer Not Created

Check controller logs:
```bash
podman logs rusternetes-controller-manager | grep -i error
```

Common issues:
- **No cloud provider configured**: Ensure `--cloud-provider` is set
- **AWS credentials missing**: Check IAM role or credentials file
- **VPC/subnet not configured**: Set `AWS_VPC_ID` and `AWS_SUBNET_IDS`
- **Missing NodePorts**: LoadBalancer services require NodePorts

### Service Status Shows No Ingress

1. Check if cloud provider feature is enabled:
   ```bash
   controller-manager --help | grep cloud-provider
   ```

2. Verify controller is running:
   ```bash
   podman ps | grep controller-manager
   ```

3. Check for reconciliation errors in logs

### AWS-Specific Issues

**NLB not accessible:**
- Verify security groups allow traffic on NodePort ranges (30000-32767)
- Ensure node instances are healthy in target groups
- Check VPC routing and internet gateway configuration

**Target registration failed:**
- Verify node IP addresses are correct
- Check if nodes are in the same VPC as load balancer
- Ensure subnets have proper route tables

## Implementation Details

### AWS Provider

- **Load Balancer Type**: Network Load Balancer (Layer 4)
- **Target Type**: IP-based targets
- **Protocol**: TCP (UDP support planned)
- **Naming**: `{cluster-name}-{namespace}-{service-name}` (max 32 chars)
- **Tags**: Automatically tagged with cluster name and "managed-by: rusternetes"

### Controller Behavior

- **Reconciliation Interval**: 30 seconds (matches service sync interval)
- **Concurrent Operations**: One reconciliation loop handles all LoadBalancer services
- **Idempotency**: Re-running reconciliation is safe; existing load balancers are reused
- **Error Handling**: Errors logged but don't stop other services from reconciling

## Limitations

1. **No health check configuration**: Uses default cloud provider health checks
2. **Limited annotations**: Only `service.beta.kubernetes.io/aws-load-balancer-internal` supported
3. **No cross-zone load balancing**: Depends on cloud provider defaults
4. **NodePort required**: LoadBalancer services must have NodePorts allocated
5. **GCP/Azure not implemented**: Only stub implementations currently

## Future Enhancements

- [ ] GCP Cloud Load Balancing implementation
- [ ] Azure Load Balancer implementation
- [ ] Health check configuration via annotations
- [ ] Session affinity support
- [ ] Cross-zone load balancing control
- [ ] Application Load Balancer (ALB) support for AWS
- [ ] SSL/TLS termination configuration
- [ ] Access logging configuration

## Choosing Between MetalLB and Cloud Providers

| Use Case | Recommended Solution | Why |
|----------|---------------------|-----|
| Local development | **MetalLB** | No cloud costs, simple setup |
| Testing in Podman/Docker | **MetalLB** | Works with container networks |
| CI/CD pipelines | **MetalLB** | Fast, no external dependencies |
| Bare-metal production | **MetalLB** | No cloud provider available |
| On-premises data center | **MetalLB** | Full control over networking |
| Edge computing | **MetalLB** | Works in disconnected environments |
| AWS production | **Cloud Provider** | Managed NLB with health checks |
| GCP production | **Cloud Provider** (planned) | Managed Cloud Load Balancing |
| Azure production | **Cloud Provider** (planned) | Managed Azure Load Balancer |
| Multi-cloud | **MetalLB or Cloud** | Depends on requirements |

## Related Documentation

- **[MetalLB Integration Guide](docs/METALLB_INTEGRATION.md)** - Complete guide for local/bare-metal deployments
- [Service Networking](STATUS.md#networking--service-discovery-features)
- [Kube-Proxy](STATUS.md#networking--service-discovery-features)
- [Cloud Provider Architecture](crates/common/src/cloud_provider.rs)
- [AWS Provider Implementation](crates/cloud-providers/src/aws.rs)
- [LoadBalancer Controller](crates/controller-manager/src/controllers/loadbalancer.rs)
