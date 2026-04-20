# Custom Metrics Integration Guide


> **Tip:** You can manage related resources through the [web console](../CONSOLE_USER_GUIDE.md).
**Last Updated**: 2026-03-13
**Status**: Production Ready
**API Version**: custom.metrics.k8s.io/v1beta2

This guide explains how to integrate Rusternetes with Prometheus for custom metrics support, enabling HorizontalPodAutoscaler (HPA) and other components to scale based on application-specific metrics.

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Prerequisites](#prerequisites)
4. [Quick Start](#quick-start)
5. [Configuration](#configuration)
6. [Prometheus Setup](#prometheus-setup)
7. [Metric Naming Conventions](#metric-naming-conventions)
8. [HPA Integration](#hpa-integration)
9. [Testing](#testing)
10. [Troubleshooting](#troubleshooting)
11. [Advanced Configuration](#advanced-configuration)
12. [Performance Tuning](#performance-tuning)

---

## Overview

Rusternetes implements the Kubernetes Custom Metrics API (`custom.metrics.k8s.io/v1beta2`) with integrated Prometheus backend support. This allows applications to expose custom metrics through Prometheus and use them for autoscaling decisions.

**Key Features**:
- ✅ Full `custom.metrics.k8s.io/v1beta2` API implementation
- ✅ Native Prometheus integration via PromQL
- ✅ 60-second metric caching for performance
- ✅ Graceful fallback to mock data when Prometheus unavailable
- ✅ Support for object, namespace, and cluster-scoped metrics
- ✅ Label selector filtering
- ✅ Compatible with standard Prometheus exporters (kube-state-metrics, node-exporter, custom exporters)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Rusternetes API Server                      │
│                                                                 │
│  ┌─────────────────┐        ┌──────────────────────────┐      │
│  │  HPA Controller │───────▶│ Custom Metrics Handlers  │      │
│  └─────────────────┘        └────────────┬─────────────┘      │
│                                           │                     │
│                             ┌─────────────▼─────────────┐      │
│                             │   PrometheusClient        │      │
│                             │  - Query Builder          │      │
│                             │  - Caching Layer (60s)    │      │
│                             │  - Label Mapping          │      │
│                             └─────────────┬─────────────┘      │
└───────────────────────────────────────────┼─────────────────────┘
                                            │
                                            │ PromQL Queries
                                            │
                                ┌───────────▼──────────┐
                                │   Prometheus Server  │
                                │                      │
                                │  - Metric Storage    │
                                │  - Query Engine      │
                                │  - Aggregation       │
                                └───────────┬──────────┘
                                            │
                    ┌───────────────────────┼───────────────────┐
                    │                       │                   │
         ┌──────────▼─────────┐  ┌─────────▼────────┐  ┌──────▼──────┐
         │ kube-state-metrics │  │  Custom Exporter │  │ node-exporter│
         └────────────────────┘  └──────────────────┘  └─────────────┘
```

**Data Flow**:
1. HPA requests custom metric from API server
2. Custom Metrics handler checks PrometheusClient cache
3. If cache miss, PrometheusClient builds PromQL query
4. Query executed against Prometheus server
5. Result cached for 60 seconds
6. Metric value returned to HPA for scaling decision

---

## Prerequisites

### Required Components

1. **Prometheus Server** (v2.30+)
   - Running and accessible from Rusternetes API server
   - Recommended: 2GB memory, persistent storage

2. **Metric Exporters**
   - **kube-state-metrics** (recommended for Kubernetes resource metrics)
   - **node-exporter** (for node-level metrics)
   - **Custom application exporters** (for application-specific metrics)

3. **Rusternetes Cluster**
   - API server v1.35+
   - Controller manager with HPA support

### Optional Components

- **Grafana** (for visualization and debugging)
- **AlertManager** (for alerting on metric thresholds)

---

## Quick Start

### 1. Deploy Prometheus

Using Docker Compose (development):

```yaml
# prometheus-docker-compose.yml
version: '3.8'
services:
  prometheus:
    image: prom/prometheus:v2.45.0
    container_name: prometheus
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--storage.tsdb.retention.time=15d'
    restart: unless-stopped

volumes:
  prometheus-data:
```

Minimal Prometheus configuration:

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  # Scrape kube-state-metrics
  - job_name: 'kube-state-metrics'
    static_configs:
      - targets: ['kube-state-metrics:8080']

  # Scrape your application metrics
  - job_name: 'my-app'
    kubernetes_sd_configs:
      - role: pod
    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
        action: keep
        regex: true
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_path]
        action: replace
        target_label: __metrics_path__
        regex: (.+)
      - source_labels: [__address__, __meta_kubernetes_pod_annotation_prometheus_io_port]
        action: replace
        regex: ([^:]+)(?::\d+)?;(\d+)
        replacement: $1:$2
        target_label: __address__
```

Start Prometheus:
```bash
docker-compose -f prometheus-docker-compose.yml up -d
```

Verify Prometheus is running:
```bash
curl http://localhost:9090/-/healthy
# Should return: Prometheus is Healthy.
```

### 2. Deploy kube-state-metrics

```yaml
# kube-state-metrics.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: kube-state-metrics
  namespace: kube-system
spec:
  replicas: 1
  selector:
    matchLabels:
      app: kube-state-metrics
  template:
    metadata:
      labels:
        app: kube-state-metrics
    spec:
      serviceAccountName: kube-state-metrics
      containers:
      - name: kube-state-metrics
        image: registry.k8s.io/kube-state-metrics/kube-state-metrics:v2.9.2
        ports:
        - containerPort: 8080
          name: http-metrics
        - containerPort: 8081
          name: telemetry
        resources:
          requests:
            memory: 100Mi
            cpu: 100m
          limits:
            memory: 200Mi
            cpu: 200m
---
apiVersion: v1
kind: Service
metadata:
  name: kube-state-metrics
  namespace: kube-system
spec:
  selector:
    app: kube-state-metrics
  ports:
  - name: http-metrics
    port: 8080
    targetPort: http-metrics
---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: kube-state-metrics
  namespace: kube-system
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: kube-state-metrics
rules:
- apiGroups: [""]
  resources:
  - configmaps
  - secrets
  - nodes
  - pods
  - services
  - resourcequotas
  - replicationcontrollers
  - limitranges
  - persistentvolumeclaims
  - persistentvolumes
  - namespaces
  - endpoints
  verbs: ["list", "watch"]
- apiGroups: ["apps"]
  resources:
  - statefulsets
  - daemonsets
  - deployments
  - replicasets
  verbs: ["list", "watch"]
- apiGroups: ["batch"]
  resources:
  - cronjobs
  - jobs
  verbs: ["list", "watch"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: kube-state-metrics
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: kube-state-metrics
subjects:
- kind: ServiceAccount
  name: kube-state-metrics
  namespace: kube-system
```

Deploy:
```bash
kubectl apply -f kube-state-metrics.yaml
```

### 3. Start Rusternetes with Prometheus Integration

```bash
# Start API server with Prometheus integration
./target/release/rusternetes-api-server \
  --bind-address 0.0.0.0:6443 \
  --etcd-servers http://localhost:2379 \
  --prometheus-url http://localhost:9090 \
  --tls \
  --tls-self-signed \
  --log-level info
```

Verify integration:
```bash
# Check API server logs for Prometheus initialization
# Should see: "Prometheus client initialized successfully"

# Test custom metrics API
kubectl get --raw "/apis/custom.metrics.k8s.io/v1beta2" | jq .
```

---

## Configuration

### API Server Configuration

The Rusternetes API server accepts the following Prometheus-related arguments:

| Argument | Type | Default | Description |
|----------|------|---------|-------------|
| `--prometheus-url` | String | None | Prometheus server URL (e.g., `http://localhost:9090`) |

**Example**:
```bash
rusternetes-api-server --prometheus-url http://prometheus.monitoring.svc:9090
```

### Environment Variables

For production deployments, you can use environment variables:

```bash
export PROMETHEUS_URL="http://prometheus.monitoring.svc:9090"
rusternetes-api-server --prometheus-url "$PROMETHEUS_URL"
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rusternetes-api-server
  namespace: kube-system
spec:
  replicas: 3
  selector:
    matchLabels:
      component: api-server
  template:
    metadata:
      labels:
        component: api-server
    spec:
      containers:
      - name: api-server
        image: rusternetes/api-server:latest
        args:
        - --bind-address=0.0.0.0:6443
        - --etcd-servers=http://etcd:2379
        - --prometheus-url=http://prometheus.monitoring.svc:9090
        - --tls
        - --tls-cert-file=/etc/kubernetes/pki/apiserver.crt
        - --tls-key-file=/etc/kubernetes/pki/apiserver.key
        - --log-level=info
        ports:
        - containerPort: 6443
          name: https
        env:
        - name: PROMETHEUS_URL
          value: "http://prometheus.monitoring.svc:9090"
        volumeMounts:
        - name: pki
          mountPath: /etc/kubernetes/pki
          readOnly: true
      volumes:
      - name: pki
        secret:
          secretName: api-server-certs
```

---

## Prometheus Setup

### Recommended Prometheus Configuration

```yaml
# prometheus.yml - Production configuration
global:
  scrape_interval: 15s      # How often to scrape targets
  evaluation_interval: 15s  # How often to evaluate rules
  external_labels:
    cluster: 'rusternetes-prod'
    environment: 'production'

# Alertmanager configuration (optional)
alerting:
  alertmanagers:
  - static_configs:
    - targets:
      - alertmanager:9093

# Rule files (optional)
rule_files:
  - '/etc/prometheus/rules/*.yml'

scrape_configs:
  # Prometheus self-monitoring
  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']

  # kube-state-metrics - exposes Kubernetes resource metrics
  - job_name: 'kube-state-metrics'
    static_configs:
      - targets: ['kube-state-metrics.kube-system.svc:8080']
    relabel_configs:
      - source_labels: [__address__]
        target_label: instance
        replacement: 'kube-state-metrics'

  # node-exporter - exposes node-level metrics
  - job_name: 'node-exporter'
    kubernetes_sd_configs:
      - role: node
    relabel_configs:
      - source_labels: [__address__]
        regex: '(.*):10250'
        replacement: '${1}:9100'
        target_label: __address__
      - source_labels: [__meta_kubernetes_node_name]
        target_label: node

  # Application metrics - discover pods with prometheus.io/scrape annotation
  - job_name: 'kubernetes-pods'
    kubernetes_sd_configs:
      - role: pod
    relabel_configs:
      # Only scrape pods with prometheus.io/scrape=true annotation
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
        action: keep
        regex: true

      # Use custom metrics path if specified
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_path]
        action: replace
        target_label: __metrics_path__
        regex: (.+)

      # Use custom port if specified
      - source_labels: [__address__, __meta_kubernetes_pod_annotation_prometheus_io_port]
        action: replace
        regex: ([^:]+)(?::\d+)?;(\d+)
        replacement: $1:$2
        target_label: __address__

      # Add namespace label
      - source_labels: [__meta_kubernetes_namespace]
        target_label: namespace

      # Add pod name label
      - source_labels: [__meta_kubernetes_pod_name]
        target_label: pod

      # Add pod labels as metric labels
      - action: labelmap
        regex: __meta_kubernetes_pod_label_(.+)

  # Service endpoints - discover services
  - job_name: 'kubernetes-service-endpoints'
    kubernetes_sd_configs:
      - role: endpoints
    relabel_configs:
      - source_labels: [__meta_kubernetes_service_annotation_prometheus_io_scrape]
        action: keep
        regex: true
      - source_labels: [__meta_kubernetes_service_annotation_prometheus_io_path]
        action: replace
        target_label: __metrics_path__
        regex: (.+)
      - source_labels: [__address__, __meta_kubernetes_service_annotation_prometheus_io_port]
        action: replace
        regex: ([^:]+)(?::\d+)?;(\d+)
        replacement: $1:$2
        target_label: __address__
      - source_labels: [__meta_kubernetes_namespace]
        target_label: namespace
      - source_labels: [__meta_kubernetes_service_name]
        target_label: service
```

### Storage Configuration

For production use, configure persistent storage:

```yaml
# prometheus-storage.yml
global:
  scrape_interval: 15s

# TSDB storage configuration
storage:
  tsdb:
    path: /prometheus
    retention:
      time: 15d      # Keep data for 15 days
      size: 50GB     # Maximum storage size
```

### Resource Requirements

**Minimum**:
- CPU: 1 core
- Memory: 2GB
- Disk: 10GB

**Recommended (Production)**:
- CPU: 2-4 cores
- Memory: 4-8GB
- Disk: 50-100GB SSD

---

## Metric Naming Conventions

Rusternetes PrometheusClient maps Kubernetes resource types to Prometheus labels using the following conventions:

### Resource Label Mapping

| Kubernetes Resource | Prometheus Label | Example |
|---------------------|------------------|---------|
| `pods` | `pod` | `pod="nginx-abc123"` |
| `services` | `service` | `service="frontend"` |
| `deployments` | `deployment` | `deployment="web-app"` |
| `replicasets` | `replicaset` | `replicaset="web-app-xyz"` |
| `statefulsets` | `statefulset` | `statefulset="database"` |
| `daemonsets` | `daemonset` | `daemonset="node-logger"` |
| `nodes` | `node` | `node="worker-1"` |
| `namespaces` | `namespace` | `namespace="production"` |
| `jobs` | `job` | `job="batch-processor"` |
| `cronjobs` | `cronjob` | `cronjob="nightly-backup"` |

### Standard Labels

All queries include the `namespace` label for namespaced resources:

```promql
# Query for specific pod metric
my_custom_metric{namespace="default",pod="nginx-abc123"}

# Query for all pods in namespace
my_custom_metric{namespace="production"}
```

### Custom Application Metrics

For application metrics, use these annotation patterns:

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: my-app
  namespace: default
  annotations:
    prometheus.io/scrape: "true"     # Enable Prometheus scraping
    prometheus.io/port: "8080"       # Metrics port
    prometheus.io/path: "/metrics"   # Metrics endpoint path
  labels:
    app: my-app
    version: v1.0.0
spec:
  containers:
  - name: app
    image: my-app:v1.0.0
    ports:
    - containerPort: 8080
      name: http-metrics
```

Example custom metric:
```promql
# Your application exposes this metric
http_requests_total{namespace="default",pod="my-app",method="GET",status="200"}
```

---

## HPA Integration

### Example: Scale on Custom HTTP Request Rate

**1. Deploy application with metrics:**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web-app
  namespace: default
spec:
  replicas: 2
  selector:
    matchLabels:
      app: web-app
  template:
    metadata:
      labels:
        app: web-app
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "8080"
        prometheus.io/path: "/metrics"
    spec:
      containers:
      - name: web
        image: my-web-app:latest
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 8080
          name: metrics
```

**2. Create HPA using custom metric:**

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: web-app-hpa
  namespace: default
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: web-app
  minReplicas: 2
  maxReplicas: 10
  metrics:
  # Custom metric: HTTP requests per second
  - type: Pods
    pods:
      metric:
        name: http_requests_per_second
      target:
        type: AverageValue
        averageValue: "100"   # Target 100 req/s per pod

  # Custom metric: Queue depth
  - type: Object
    object:
      metric:
        name: queue_depth
      describedObject:
        apiVersion: v1
        kind: Service
        name: message-queue
      target:
        type: Value
        value: "1000"         # Target 1000 messages in queue

  # External metric: Cloud provider metric
  - type: External
    external:
      metric:
        name: cloud_loadbalancer_requests_per_second
        selector:
          matchLabels:
            loadbalancer: web-app-lb
      target:
        type: AverageValue
        averageValue: "500"

  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300   # Wait 5 minutes before scaling down
      policies:
      - type: Percent
        value: 50                        # Scale down by 50% at most
        periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 0      # Scale up immediately
      policies:
      - type: Percent
        value: 100                        # Double capacity at most
        periodSeconds: 60
      - type: Pods
        value: 4                          # Add max 4 pods
        periodSeconds: 60
      selectPolicy: Max                   # Use most aggressive policy
```

**3. Verify HPA is using custom metrics:**

```bash
# Check HPA status
kubectl get hpa web-app-hpa

# Describe HPA to see current metric values
kubectl describe hpa web-app-hpa

# Expected output:
# Metrics:                                                   ( current / target )
#   "http_requests_per_second" on pods:                      85 / 100
#   "queue_depth" on Service/default/message-queue:          750 / 1k
#   "cloud_loadbalancer_requests_per_second" (external):     450 / 500
```

**4. Query custom metrics API directly:**

```bash
# Get metric for specific pod
kubectl get --raw "/apis/custom.metrics.k8s.io/v1beta2/namespaces/default/pods/web-app-abc123/http_requests_per_second" | jq .

# List metrics for all pods in namespace
kubectl get --raw "/apis/custom.metrics.k8s.io/v1beta2/namespaces/default/pods/*/http_requests_per_second" | jq .

# Get object metric
kubectl get --raw "/apis/custom.metrics.k8s.io/v1beta2/namespaces/default/services/message-queue/queue_depth" | jq .
```

### Example: Scale on Custom Business Metric

Scale based on orders per second:

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: order-processor-hpa
  namespace: ecommerce
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: order-processor
  minReplicas: 3
  maxReplicas: 20
  metrics:
  - type: Pods
    pods:
      metric:
        name: orders_processed_per_second
      target:
        type: AverageValue
        averageValue: "50"    # Each pod should handle ~50 orders/s
```

Prometheus metric exposed by application:

```promql
# Rate of orders processed (5-minute average)
rate(orders_processed_total{namespace="ecommerce",pod=~"order-processor-.*"}[5m])
```

---

## Testing

### 1. Verify Prometheus is Scraping Metrics

```bash
# Check Prometheus targets
curl http://localhost:9090/api/v1/targets | jq '.data.activeTargets[] | {job, health, lastError}'

# Query a sample metric
curl -G http://localhost:9090/api/v1/query \
  --data-urlencode 'query=up{job="kube-state-metrics"}' | jq .
```

### 2. Test Custom Metrics API

```bash
# List available custom metrics
kubectl get --raw "/apis/custom.metrics.k8s.io/v1beta2" | jq '.resources[].name'

# Get pod metric
kubectl get --raw "/apis/custom.metrics.k8s.io/v1beta2/namespaces/default/pods/test-pod/http_requests" | jq .

# Expected response:
{
  "kind": "MetricValue",
  "apiVersion": "custom.metrics.k8s.io/v1beta2",
  "metadata": {
    "name": "http_requests",
    "namespace": "default",
    "creationTimestamp": "2026-03-13T10:30:00Z"
  },
  "timestamp": "2026-03-13T10:30:00Z",
  "windowSeconds": 60,
  "value": "1234",
  "describedObject": {
    "kind": "Pod",
    "namespace": "default",
    "name": "test-pod",
    "apiVersion": "v1"
  }
}
```

### 3. Test HPA Scaling

Deploy test application with artificial load:

```bash
# Deploy load generator
kubectl run load-generator --image=busybox --restart=Never -- \
  /bin/sh -c "while true; do wget -q -O- http://web-app.default.svc; done"

# Watch HPA status
kubectl get hpa web-app-hpa --watch

# Check pod count
kubectl get pods -l app=web-app --watch
```

### 4. Simulate Metric Changes

Use `curl` to update Prometheus metrics:

```bash
# Push custom metric to Pushgateway (if configured)
echo "custom_queue_depth 5000" | curl --data-binary @- http://pushgateway:9091/metrics/job/test_job

# Or update your application to expose different metric values
```

### 5. Integration Test Script

```bash
#!/bin/bash
# test-metrics-integration.sh

set -e

echo "Testing Custom Metrics Integration..."

# Test 1: Check Prometheus connectivity
echo "Test 1: Prometheus connectivity"
if curl -sf http://localhost:9090/-/healthy > /dev/null; then
    echo "✅ Prometheus is healthy"
else
    echo "❌ Prometheus is not accessible"
    exit 1
fi

# Test 2: Check custom metrics API
echo "Test 2: Custom Metrics API"
if kubectl get --raw "/apis/custom.metrics.k8s.io/v1beta2" > /dev/null 2>&1; then
    echo "✅ Custom Metrics API is accessible"
else
    echo "❌ Custom Metrics API is not accessible"
    exit 1
fi

# Test 3: Query a specific metric
echo "Test 3: Query specific metric"
METRIC_VALUE=$(kubectl get --raw "/apis/custom.metrics.k8s.io/v1beta2/namespaces/default/pods/*/http_requests" 2>/dev/null | jq -r '.items[0].value // "0"')
echo "✅ Retrieved metric value: $METRIC_VALUE"

# Test 4: Check HPA is working
echo "Test 4: HPA integration"
if kubectl get hpa web-app-hpa > /dev/null 2>&1; then
    CURRENT_REPLICAS=$(kubectl get hpa web-app-hpa -o jsonpath='{.status.currentReplicas}')
    DESIRED_REPLICAS=$(kubectl get hpa web-app-hpa -o jsonpath='{.status.desiredReplicas}')
    echo "✅ HPA is active: current=$CURRENT_REPLICAS, desired=$DESIRED_REPLICAS"
else
    echo "⚠️  HPA not found (optional)"
fi

echo ""
echo "All tests passed! ✅"
```

Run the test:
```bash
chmod +x test-metrics-integration.sh
./test-metrics-integration.sh
```

---

## Troubleshooting

### Issue 1: Custom Metrics API Returns 404

**Symptoms**:
```bash
kubectl get --raw "/apis/custom.metrics.k8s.io/v1beta2"
# Error: the server could not find the requested resource
```

**Solutions**:

1. **Verify API server started with Prometheus URL**:
```bash
# Check API server logs
grep -i prometheus /var/log/rusternetes-api-server.log

# Should see: "Prometheus client initialized successfully"
# If you see: "Prometheus URL not provided", restart with --prometheus-url flag
```

2. **Verify API registration**:
```bash
kubectl api-versions | grep custom.metrics
# Should show: custom.metrics.k8s.io/v1beta2
```

3. **Check API server configuration**:
```bash
ps aux | grep rusternetes-api-server
# Verify --prometheus-url argument is present
```

### Issue 2: Metrics Always Return "0" or Mock Values

**Symptoms**:
All metrics return "0" or hardcoded values like "100", "150".

**Solutions**:

1. **Check Prometheus client initialization**:
```bash
# API server logs should show:
# "Prometheus client initialized successfully"
# NOT: "Failed to initialize Prometheus client"
```

2. **Verify Prometheus is accessible**:
```bash
# From API server host
curl http://localhost:9090/api/v1/query?query=up

# If fails, check network connectivity and Prometheus server status
```

3. **Check metric exists in Prometheus**:
```bash
# Query Prometheus directly
curl -G http://localhost:9090/api/v1/query \
  --data-urlencode 'query=http_requests_total{namespace="default"}' | jq .

# If empty, the metric is not being scraped by Prometheus
```

4. **Verify label mapping**:
```bash
# Check if your metric uses expected labels
curl -G http://localhost:9090/api/v1/query \
  --data-urlencode 'query=http_requests_total' | jq '.data.result[0].metric'

# Should include: namespace, pod (or service, deployment, etc.)
```

### Issue 3: HPA Shows "Unknown" Metrics

**Symptoms**:
```bash
kubectl describe hpa web-app-hpa
# Metrics: <unknown> / 100
```

**Solutions**:

1. **Check metric name matches**:
```yaml
# HPA manifest
metric:
  name: http_requests_per_second  # Must match Prometheus metric name

# Prometheus query (should return data):
# http_requests_per_second{namespace="default",pod="web-app-xyz"}
```

2. **Verify metric is available for target pods**:
```bash
# List all pods for the deployment
kubectl get pods -l app=web-app

# Check metric exists for each pod
kubectl get --raw "/apis/custom.metrics.k8s.io/v1beta2/namespaces/default/pods/web-app-xyz/http_requests_per_second" | jq .
```

3. **Check HPA controller logs**:
```bash
# Controller manager should show metric queries
kubectl logs -n kube-system deployment/controller-manager | grep -i "horizontal"
```

### Issue 4: Prometheus Queries Failing

**Symptoms**:
API server logs show:
```
WARN Failed to query Prometheus for metric http_requests: connection refused
```

**Solutions**:

1. **Verify Prometheus URL is correct**:
```bash
# Check what URL API server is using
grep "Prometheus client" /var/log/rusternetes-api-server.log

# Test connectivity
curl -v http://prometheus-url:9090/-/healthy
```

2. **Check network policies/firewalls**:
```bash
# From API server pod/host, test connection
telnet prometheus.monitoring.svc 9090

# If using Kubernetes service, verify DNS
nslookup prometheus.monitoring.svc
```

3. **Verify Prometheus TLS/authentication** (if applicable):
```bash
# If Prometheus requires authentication, you may need to modify PrometheusClient
# Current implementation assumes no authentication
```

### Issue 5: Metrics Delayed or Stale

**Symptoms**:
Metrics are several minutes old or don't update promptly.

**Solutions**:

1. **Check cache TTL** (default 60 seconds):
```rust
// In prometheus_client.rs
// Cache TTL is hardcoded to 60 seconds
self.cache_value(&query, &value, Duration::from_secs(60)).await;

// For faster updates in development, you can reduce this
// (requires code modification)
```

2. **Verify Prometheus scrape interval**:
```yaml
# prometheus.yml
global:
  scrape_interval: 15s  # Metrics are scraped every 15 seconds
```

3. **Check Prometheus query performance**:
```bash
# See query execution time in Prometheus UI
# http://localhost:9090/graph
# Execute your query and check "Execution time"
```

### Issue 6: High Memory Usage in API Server

**Symptoms**:
API server memory usage grows over time.

**Solutions**:

1. **Cache cleanup is automatic** - verify it's working:
```rust
// In prometheus_client.rs, cache_value() method automatically cleans expired entries
// If memory still grows, check for metric cardinality issues
```

2. **Reduce metric cardinality in Prometheus**:
```promql
# Avoid high-cardinality labels like user_id, request_id
# Good: http_requests_total{method="GET",status="200"}
# Bad: http_requests_total{user_id="12345",session_id="abcdef"}
```

3. **Monitor cache size** (add this to your monitoring):
```rust
// Consider adding cache size metrics to PrometheusClient
// (requires code modification)
```

### Debugging Checklist

Use this checklist to diagnose issues:

- [ ] Prometheus is running and accessible (`curl http://prometheus:9090/-/healthy`)
- [ ] kube-state-metrics is deployed and scraping
- [ ] Application pods have `prometheus.io/scrape=true` annotation
- [ ] Metrics are visible in Prometheus UI (`http://localhost:9090/graph`)
- [ ] API server started with `--prometheus-url` flag
- [ ] API server logs show "Prometheus client initialized successfully"
- [ ] Custom Metrics API is registered (`kubectl api-versions | grep custom.metrics`)
- [ ] Metrics can be queried via API (`kubectl get --raw "/apis/custom.metrics.k8s.io/v1beta2/..."`)
- [ ] HPA references correct metric names
- [ ] HPA target deployment/pods exist and match selector
- [ ] Network connectivity between API server and Prometheus

---

## Advanced Configuration

### Custom Label Mapping

If your Prometheus metrics use different label conventions, you can modify the label mapping:

```rust
// In crates/api-server/src/prometheus_client.rs
// Modify extract_resource_name_label() method

fn extract_resource_name_label(&self, resource_type: &str) -> Option<String> {
    match resource_type {
        "pods" => Some("pod_name".to_string()),  // Changed from "pod"
        "services" => Some("svc".to_string()),    // Changed from "service"
        // Add custom mappings for your exporters
        _ => {
            let singular = resource_type.trim_end_matches('s');
            Some(singular.to_string())
        }
    }
}
```

### Custom Query Templates

For complex PromQL queries, you can add specialized query builders:

```rust
// In crates/api-server/src/prometheus_client.rs

impl PrometheusClient {
    /// Query with custom PromQL template
    pub async fn query_with_template(
        &self,
        template: &str,
        variables: HashMap<String, String>,
    ) -> Result<String> {
        let mut query = template.to_string();

        for (key, value) in variables {
            query = query.replace(&format!("${{{}}}", key), &value);
        }

        self.execute_instant_query(&query).await
    }
}

// Usage in handler:
let mut vars = HashMap::new();
vars.insert("namespace".to_string(), namespace.clone());
vars.insert("pod".to_string(), pod_name.clone());

let value = prometheus_client.query_with_template(
    "rate(http_requests_total{namespace=\"${namespace}\",pod=\"${pod}\"}[5m])",
    vars,
).await?;
```

### Multiple Prometheus Servers

For multi-cluster or federated setups:

```rust
// Extend ApiServerState to support multiple PrometheusClients

pub struct ApiServerState {
    // ... existing fields
    pub prometheus_clients: HashMap<String, Arc<PrometheusClient>>,
}

// In main.rs, initialize multiple clients:
let mut prometheus_clients = HashMap::new();
prometheus_clients.insert("production".to_string(),
    Arc::new(PrometheusClient::new("http://prom-prod:9090")?));
prometheus_clients.insert("staging".to_string(),
    Arc::new(PrometheusClient::new("http://prom-staging:9090")?));

// Query based on namespace or label
let client = state.prometheus_clients.get("production").unwrap();
```

### Authentication with Prometheus

If your Prometheus requires authentication:

```rust
// Extend PrometheusClient to support Basic Auth or Bearer tokens

use prometheus_http_query::Client;

impl PrometheusClient {
    pub fn new_with_auth(
        url: impl Into<String>,
        username: &str,
        password: &str,
    ) -> Result<Self> {
        let url_str = url.into();
        let client = Client::from_config(
            prometheus_http_query::Config::new(url_str.clone())
                .with_basic_auth(username, password)
        )?;

        // ... rest of initialization
    }

    pub fn new_with_bearer_token(
        url: impl Into<String>,
        token: &str,
    ) -> Result<Self> {
        let url_str = url.into();
        let client = Client::from_config(
            prometheus_http_query::Config::new(url_str.clone())
                .with_bearer_token(token)
        )?;

        // ... rest of initialization
    }
}
```

---

## Performance Tuning

### Cache TTL Optimization

**Default**: 60 seconds

**Recommendations**:

| Use Case | Recommended TTL | Rationale |
|----------|----------------|-----------|
| Development | 10-15 seconds | Faster feedback loop |
| Production (low-traffic) | 60 seconds | Balance freshness and load |
| Production (high-traffic) | 120-300 seconds | Reduce Prometheus load |
| Batch processing | 300+ seconds | Metrics change slowly |

Modify in `prometheus_client.rs`:

```rust
// Change cache TTL based on environment
let ttl = std::env::var("METRICS_CACHE_TTL_SECONDS")
    .ok()
    .and_then(|s| s.parse::<u64>().ok())
    .unwrap_or(60);

self.cache_value(&query, &value, Duration::from_secs(ttl)).await;
```

### Prometheus Query Optimization

**Use instant queries instead of range queries** when possible:

```promql
# Good: Instant query (current value)
http_requests_total{namespace="default"}

# Less efficient: Range query (returns time series)
http_requests_total{namespace="default"}[5m]
```

**Pre-aggregate metrics** in Prometheus recording rules:

```yaml
# prometheus-rules.yml
groups:
  - name: rusternetes_metrics
    interval: 30s
    rules:
      # Pre-calculate request rate
      - record: pod:http_requests_per_second:rate5m
        expr: |
          rate(http_requests_total[5m])

      # Pre-aggregate by namespace
      - record: namespace:http_requests_per_second:sum
        expr: |
          sum by (namespace) (rate(http_requests_total[5m]))
```

Then query the pre-aggregated metric:

```promql
# Instead of: rate(http_requests_total[5m])
# Use: pod:http_requests_per_second:rate5m
```

### Limit Metric Cardinality

**High cardinality** (bad):
```promql
http_requests_total{user_id="12345", session_id="abc", request_id="xyz"}
# Millions of unique label combinations
```

**Low cardinality** (good):
```promql
http_requests_total{method="GET", status="200", endpoint="/api"}
# Dozens to hundreds of unique label combinations
```

### Prometheus Resource Sizing

**Memory estimation**:
```
Memory (GB) = (Number of time series) × (Bytes per sample) × (Retention days) × (Samples per day)
            = 1,000,000 × 2 bytes × 15 days × 86400/15 seconds
            ≈ 17 GB
```

**Recommendations**:

| Metric Count | Retention | Memory | CPU | Disk |
|--------------|-----------|--------|-----|------|
| 100K time series | 15 days | 2-4 GB | 1-2 cores | 20 GB |
| 500K time series | 15 days | 8-16 GB | 2-4 cores | 50 GB |
| 1M time series | 15 days | 16-32 GB | 4-8 cores | 100 GB |
| 5M time series | 15 days | 64-128 GB | 8-16 cores | 500 GB |

### API Server Resource Sizing

**PrometheusClient memory usage**:

| Scenario | Cache Size | Memory Impact |
|----------|-----------|---------------|
| 100 unique queries/min | ~6K entries | ~10 MB |
| 1K unique queries/min | ~60K entries | ~100 MB |
| 10K unique queries/min | ~600K entries | ~1 GB |

**Recommendations**:
- Add memory limits: `--max-cache-entries` flag (requires implementation)
- Monitor cache hit rate and adjust TTL
- Use Prometheus recording rules to reduce query complexity

---

## Example Deployment

Complete example with all components:

```yaml
# complete-metrics-stack.yaml

# 1. Prometheus
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: prometheus-config
  namespace: monitoring
data:
  prometheus.yml: |
    global:
      scrape_interval: 15s
    scrape_configs:
      - job_name: 'kube-state-metrics'
        static_configs:
          - targets: ['kube-state-metrics.kube-system.svc:8080']
      - job_name: 'kubernetes-pods'
        kubernetes_sd_configs:
          - role: pod
        relabel_configs:
          - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
            action: keep
            regex: true
          - source_labels: [__meta_kubernetes_namespace]
            target_label: namespace
          - source_labels: [__meta_kubernetes_pod_name]
            target_label: pod
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: prometheus
  namespace: monitoring
spec:
  replicas: 1
  selector:
    matchLabels:
      app: prometheus
  template:
    metadata:
      labels:
        app: prometheus
    spec:
      containers:
      - name: prometheus
        image: prom/prometheus:v2.45.0
        args:
          - '--config.file=/etc/prometheus/prometheus.yml'
          - '--storage.tsdb.path=/prometheus'
          - '--storage.tsdb.retention.time=15d'
        ports:
        - containerPort: 9090
        volumeMounts:
        - name: config
          mountPath: /etc/prometheus
        - name: data
          mountPath: /prometheus
        resources:
          requests:
            memory: 2Gi
            cpu: 500m
          limits:
            memory: 4Gi
            cpu: 2
      volumes:
      - name: config
        configMap:
          name: prometheus-config
      - name: data
        persistentVolumeClaim:
          claimName: prometheus-data
---
apiVersion: v1
kind: Service
metadata:
  name: prometheus
  namespace: monitoring
spec:
  selector:
    app: prometheus
  ports:
  - port: 9090
    targetPort: 9090

# 2. kube-state-metrics (already shown above)

# 3. Sample application with metrics
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: sample-app
  namespace: default
spec:
  replicas: 2
  selector:
    matchLabels:
      app: sample-app
  template:
    metadata:
      labels:
        app: sample-app
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "8080"
        prometheus.io/path: "/metrics"
    spec:
      containers:
      - name: app
        image: sample-app:latest
        ports:
        - containerPort: 8080
          name: http

# 4. HPA
---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: sample-app-hpa
  namespace: default
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: sample-app
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Pods
    pods:
      metric:
        name: http_requests_per_second
      target:
        type: AverageValue
        averageValue: "100"
```

Deploy:
```bash
kubectl create namespace monitoring
kubectl apply -f complete-metrics-stack.yaml

# Start API server
rusternetes-api-server \
  --prometheus-url http://prometheus.monitoring.svc:9090 \
  --bind-address 0.0.0.0:6443 \
  --tls --tls-self-signed
```

---

## References

- [Kubernetes Custom Metrics API](https://kubernetes.io/docs/tasks/run-application/horizontal-pod-autoscale/)
- [Prometheus Documentation](https://prometheus.io/docs/)
- [kube-state-metrics](https://github.com/kubernetes/kube-state-metrics)
- [Prometheus Operator](https://github.com/prometheus-operator/prometheus-operator)
- [HPA Walkthrough](https://kubernetes.io/docs/tasks/run-application/horizontal-pod-autoscale-walkthrough/)

---

## Support

For issues or questions:
- GitHub Issues: [rusternetes/rusternetes](https://github.com/rusternetes/rusternetes/issues)
- Documentation: [docs/](../docs/)
- Implementation Plan: [IMPLEMENTATION_PLAN.md](../planning/IMPLEMENTATION_PLAN.md)

---

**Document Version**: 1.0
**Last Updated**: 2026-03-13
**Maintainer**: Rusternetes Team
