# CSI (Container Storage Interface) Integration Guide

**Last Updated**: 2026-03-13
**Status**: Production Pattern Documentation
**CSI Specification**: v1.9.0

This document explains how Container Storage Interface (CSI) drivers work with Rusternetes, including setup, configuration, and integration patterns.

---

## Table of Contents

1. [Overview](#overview)
2. [CSI Architecture](#csi-architecture)
3. [Current Implementation Status](#current-implementation-status)
4. [Using External CSI Drivers](#using-external-csi-drivers)
5. [Supported CSI Drivers](#supported-csi-drivers)
6. [Installation Examples](#installation-examples)
7. [Volume Provisioning](#volume-provisioning)
8. [Volume Expansion](#volume-expansion)
9. [Volume Snapshots](#volume-snapshots)
10. [Troubleshooting](#troubleshooting)
11. [Development Roadmap](#development-roadmap)

---

## Overview

The Container Storage Interface (CSI) is a standard for exposing arbitrary block and file storage systems to containerized workloads on Container Orchestration Systems (COs) like Kubernetes and Rusternetes.

**Key Benefits**:
- Vendor-neutral storage integration
- Standardized storage operations (provision, attach, mount, snapshot, etc.)
- Pluggable architecture - storage vendors provide their own drivers
- Separation of concerns - drivers run as separate pods/processes

**CSI Operations**:
- Dynamic volume provisioning
- Volume expansion (resize)
- Volume snapshots and clones
- Volume topology (zone/region awareness)
- Raw block volumes
- Ephemeral inline volumes

---

## CSI Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Rusternetes Control Plane                    │
│                                                                  │
│  ┌──────────────────┐      ┌───────────────────────────┐       │
│  │  API Server      │      │  Controller Manager       │       │
│  │                  │      │                            │       │
│  │  - StorageClass  │      │  - PV Binder              │       │
│  │  - PVC           │      │  - Volume Expansion       │       │
│  │  - PV            │      │  - Dynamic Provisioner    │       │
│  │  - CSIDriver     │      │                            │       │
│  └──────────────────┘      └───────────────────────────┘       │
└──────────────────────────────────────┬───────────────────────────┘
                                       │
                    ┌──────────────────┴──────────────────┐
                    │                                     │
         ┌──────────▼─────────┐              ┌───────────▼──────────┐
         │  CSI Controller     │              │   Kubelet (Node)     │
         │  Plugin (External)  │              │                      │
         │                     │              │  - Volume Manager    │
         │  gRPC Server:       │              │  - Pod Manager       │
         │  /csi/controller    │              │                      │
         │                     │              └──────────┬───────────┘
         │  RPCs:              │                         │
         │  - CreateVolume     │              ┌──────────▼───────────┐
         │  - DeleteVolume     │              │  CSI Node Plugin     │
         │  - ControllerPublish│              │  (External)          │
         │  - ControllerExpand │              │                      │
         │  - CreateSnapshot   │              │  gRPC Server:        │
         └─────────────────────┘              │  /csi/node           │
                                              │                      │
                                              │  RPCs:               │
                                              │  - NodeStageVolume   │
                                              │  - NodePublishVolume │
                                              │  - NodeExpandVolume  │
                                              │  - NodeGetVolumeStats│
                                              └──────────────────────┘
```

### Component Responsibilities

**CSI Controller Plugin** (runs on control plane):
- **CreateVolume**: Provisions new volumes on storage backend
- **DeleteVolume**: Deletes volumes from storage backend
- **ControllerPublishVolume**: Attaches volume to a node (e.g., attach EBS to EC2)
- **ControllerUnpublishVolume**: Detaches volume from a node
- **ControllerExpandVolume**: Expands volume size on storage backend
- **CreateSnapshot**: Creates volume snapshots
- **ListVolumes**: Lists available volumes

**CSI Node Plugin** (runs on each node):
- **NodeStageVolume**: Mounts volume to a global staging path (e.g., format, mount block device)
- **NodeUnstageVolume**: Unmounts from staging path
- **NodePublishVolume**: Bind-mounts staged volume into pod's directory
- **NodeUnpublishVolume**: Unmounts from pod directory
- **NodeExpandVolume**: Expands filesystem (after controller expansion)
- **NodeGetVolumeStats**: Reports volume usage statistics

**Rusternetes Components**:
- **API Server**: Stores StorageClass, PVC, PV resources
- **Controller Manager**: Orchestrates provisioning, binding, expansion
- **Kubelet**: Calls CSI Node Plugin for mount/unmount operations

---

## Current Implementation Status

### ✅ Implemented

1. **API Resources**:
   - ✅ StorageClass CRUD operations
   - ✅ PersistentVolume CRUD operations
   - ✅ PersistentVolumeClaim CRUD operations
   - ✅ CSIDriver resource support
   - ✅ CSINode resource support
   - ✅ VolumeAttachment resource support
   - ✅ CSIStorageCapacity resource support
   - ✅ VolumeAttributesClass resource support

2. **Controllers**:
   - ✅ PV/PVC Binder (binds claims to volumes)
   - ✅ Dynamic Provisioner (creates PVs for PVCs)
   - ✅ Volume Expansion Controller (detects resize requests)

3. **Kubelet**:
   - ✅ Volume directory creation for CSI volumes
   - ✅ CSI ephemeral inline volume support (directory-based)

### ⏳ Delegated to External CSI Drivers

The following operations are **delegated to external CSI driver deployments** (standard Kubernetes pattern):

1. **Volume Provisioning**:
   - CSI Controller Plugin calls `CreateVolume` RPC
   - Rusternetes Dynamic Provisioner creates PV from result

2. **Volume Mounting**:
   - Kubelet detects CSI volume in pod spec
   - External CSI Node Plugin handles `NodeStageVolume` and `NodePublishVolume`
   - Volumes appear at expected mount paths

3. **Volume Expansion**:
   - Rusternetes Volume Expansion Controller detects size increase
   - External CSI Controller Plugin handles `ControllerExpandVolume`
   - External CSI Node Plugin handles `NodeExpandVolume` (filesystem resize)

4. **Volume Snapshots**:
   - External CSI Controller Plugin handles `CreateSnapshot`, `DeleteSnapshot`
   - Snapshot Controller (if deployed) manages VolumeSnapshot resources

### 🔧 Integration Pattern

Rusternetes follows the **standard Kubernetes CSI integration pattern**:

```
User creates PVC with StorageClass
        ↓
Rusternetes Dynamic Provisioner detects PVC
        ↓
Calls CSI Driver CreateVolume via gRPC
        ↓
CSI Driver provisions storage on backend
        ↓
Returns volume handle
        ↓
Rusternetes creates PV with CSI volume source
        ↓
PV/PVC Binder binds PV to PVC
        ↓
Kubelet schedules pod with PVC
        ↓
Kubelet creates volume directory
        ↓
CSI Node Plugin mounts volume (external)
        ↓
Pod starts with mounted volume
```

**Key Point**: CSI drivers are **external components** that operators install separately. Rusternetes provides the orchestration layer, while CSI drivers provide the storage backend integration.

---

## Using External CSI Drivers

### Prerequisites

1. **CSI Driver Deployed**: Install your chosen CSI driver as a DaemonSet (node plugin) and Deployment (controller plugin)
2. **Unix Socket Access**: CSI drivers expose gRPC servers on Unix sockets (e.g., `/var/lib/kubelet/plugins/csi-driver/csi.sock`)
3. **StorageClass Created**: Define a StorageClass that references the CSI driver

### Directory Structure

CSI drivers expect this directory structure:

```
/var/lib/kubelet/
├── plugins/
│   └── <driver-name>/
│       └── csi.sock              # Node plugin socket
├── plugins_registry/
│   └── <driver-name>-reg.sock    # Registration socket
└── pods/
    └── <pod-uid>/
        └── volumes/
            └── kubernetes.io~csi/
                └── <volume-name>/
                    ├── mount/     # Staged volume
                    └── vol_data.json
```

### CSI Driver Registration

CSI drivers register with kubelet via the Node Driver Registrar pattern:

```yaml
# Part of CSI driver DaemonSet
containers:
- name: node-driver-registrar
  image: registry.k8s.io/sig-storage/csi-node-driver-registrar:v2.9.0
  args:
    - "--csi-address=/csi/csi.sock"
    - "--kubelet-registration-path=/var/lib/kubelet/plugins/csi-driver/csi.sock"
  volumeMounts:
    - name: plugin-dir
      mountPath: /csi
    - name: registration-dir
      mountPath: /registration
```

---

## Supported CSI Drivers

Rusternetes is compatible with any CSI driver that follows the CSI v1.x specification. Below are tested/recommended drivers:

### Cloud Provider Drivers

| Driver | Provider | Features | Status |
|--------|----------|----------|--------|
| **aws-ebs-csi-driver** | AWS | EBS volumes, snapshots, expansion | ✅ Compatible |
| **gcp-compute-persistent-disk-csi-driver** | GCP | Persistent disks, snapshots | ✅ Compatible |
| **azuredisk-csi-driver** | Azure | Azure Disks, snapshots | ✅ Compatible |

### Open Source Drivers

| Driver | Purpose | Features | Status |
|--------|---------|----------|--------|
| **hostpath-csi** | Development | Local directory volumes | ✅ Recommended for dev |
| **nfs-csi-driver** | NFS Storage | Network file storage | ✅ Compatible |
| **ceph-csi** | Ceph RBD/CephFS | Distributed storage | ✅ Compatible |
| **longhorn** | Distributed block storage | Replicated volumes | ✅ Compatible |
| **rook-ceph** | Ceph Operator | Automated Ceph management | ✅ Compatible |

### Enterprise Drivers

| Driver | Vendor | Features | Status |
|--------|--------|----------|--------|
| **Dell EMC PowerStore CSI** | Dell | Enterprise SAN/NAS | ⚠️ Not tested |
| **NetApp Trident** | NetApp | ONTAP, Element | ⚠️ Not tested |
| **Pure Storage** | Pure Storage | FlashArray, FlashBlade | ⚠️ Not tested |

---

## Installation Examples

### Example 1: HostPath CSI Driver (Development)

The HostPath CSI driver is perfect for development and testing:

```bash
# Clone the hostpath CSI driver
git clone https://github.com/kubernetes-csi/csi-driver-host-path.git
cd csi-driver-host-path

# Deploy the driver
kubectl apply -f deploy/kubernetes-latest/

# Verify deployment
kubectl get pods -n kube-system | grep hostpath
# Expected:
# csi-hostpath-attacher-0       1/1     Running
# csi-hostpath-plugin-xxxxx     3/3     Running (DaemonSet)
# csi-hostpath-provisioner-0    1/1     Running
# csi-hostpath-resizer-0        1/1     Running
# csi-hostpath-snapshotter-0    1/1     Running

# Check CSIDriver registration
kubectl get csidriver
# NAME                   ATTACHREQUIRED   PODINFOONMOUNT   STORAGECAPACITY
# hostpath.csi.k8s.io    true             true             false
```

Create a StorageClass:

```yaml
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: csi-hostpath
provisioner: hostpath.csi.k8s.io
allowVolumeExpansion: true
volumeBindingMode: Immediate
parameters:
  # Driver-specific parameters
  type: "local"
```

Test with a PVC:

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: test-pvc
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 1Gi
  storageClassName: csi-hostpath
```

Use in a Pod:

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: test-pod
spec:
  containers:
  - name: app
    image: nginx
    volumeMounts:
    - name: data
      mountPath: /usr/share/nginx/html
  volumes:
  - name: data
    persistentVolumeClaim:
      claimName: test-pvc
```

### Example 2: AWS EBS CSI Driver (Production)

For production AWS deployments:

```bash
# Install AWS EBS CSI driver using Helm
helm repo add aws-ebs-csi-driver https://kubernetes-sigs.github.io/aws-ebs-csi-driver
helm repo update

helm upgrade --install aws-ebs-csi-driver \
  aws-ebs-csi-driver/aws-ebs-csi-driver \
  --namespace kube-system \
  --set enableVolumeScheduling=true \
  --set enableVolumeResizing=true \
  --set enableVolumeSnapshot=true \
  --set controller.region=us-west-2

# Verify installation
kubectl get pods -n kube-system | grep ebs-csi
kubectl get csidriver ebs.csi.aws.com
```

Create StorageClasses:

```yaml
# General purpose SSD (gp3)
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: gp3
provisioner: ebs.csi.aws.com
allowVolumeExpansion: true
volumeBindingMode: WaitForFirstConsumer
parameters:
  type: gp3
  iops: "3000"
  throughput: "125"
  encrypted: "true"
---
# Provisioned IOPS SSD (io2)
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: io2-high-perf
provisioner: ebs.csi.aws.com
allowVolumeExpansion: true
volumeBindingMode: WaitForFirstConsumer
parameters:
  type: io2
  iops: "10000"
  encrypted: "true"
```

### Example 3: NFS CSI Driver (Network Storage)

For shared network storage:

```bash
# Install NFS CSI driver
kubectl apply -f https://raw.githubusercontent.com/kubernetes-csi/csi-driver-nfs/master/deploy/install-driver.sh

# Create NFS server (example - you may have an existing one)
kubectl apply -f - <<EOF
apiVersion: v1
kind: PersistentVolume
metadata:
  name: nfs-server-pv
spec:
  capacity:
    storage: 100Gi
  accessModes:
    - ReadWriteMany
  nfs:
    server: nfs-server.example.com
    path: "/exports"
EOF
```

StorageClass for NFS:

```yaml
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: nfs-csi
provisioner: nfs.csi.k8s.io
parameters:
  server: nfs-server.example.com
  share: /exports
mountOptions:
  - hard
  - nfsvers=4.1
```

---

## Volume Provisioning

### Static Provisioning

Manually create PV, then bind PVC to it:

```yaml
# Create PersistentVolume
apiVersion: v1
kind: PersistentVolume
metadata:
  name: static-pv
spec:
  capacity:
    storage: 10Gi
  accessModes:
    - ReadWriteOnce
  persistentVolumeReclaimPolicy: Retain
  csi:
    driver: hostpath.csi.k8s.io
    volumeHandle: "static-volume-id-12345"
    fsType: ext4
---
# Create PersistentVolumeClaim
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: static-pvc
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi
  volumeName: static-pv  # Binds to specific PV
```

### Dynamic Provisioning

Let Rusternetes automatically provision volumes:

```yaml
# Just create the PVC
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: dynamic-pvc
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 5Gi
  storageClassName: csi-hostpath  # References StorageClass
```

**What happens**:
1. Rusternetes Dynamic Provisioner detects new PVC
2. Calls CSI driver's `CreateVolume` RPC
3. CSI driver provisions storage on backend
4. Returns volume handle
5. Rusternetes creates PV automatically
6. PV/PVC Binder binds them together

### Volume Topology

For zone-aware provisioning (multi-AZ):

```yaml
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: topology-aware
provisioner: ebs.csi.aws.com
volumeBindingMode: WaitForFirstConsumer  # Important for topology
allowedTopologies:
- matchLabelExpressions:
  - key: topology.kubernetes.io/zone
    values:
    - us-west-2a
    - us-west-2b
```

---

## Volume Expansion

### Enable Expansion

StorageClass must allow expansion:

```yaml
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: expandable-storage
provisioner: hostpath.csi.k8s.io
allowVolumeExpansion: true  # Required for expansion
```

### Resize a Volume

Edit the PVC to increase storage:

```bash
# Original PVC
kubectl get pvc my-pvc
# NAME     STATUS   VOLUME    CAPACITY   ACCESS MODES   STORAGECLASS
# my-pvc   Bound    pvc-123   5Gi        RWO            expandable-storage

# Edit PVC to increase size
kubectl edit pvc my-pvc

# Change:
#   resources:
#     requests:
#       storage: 5Gi
# To:
#   resources:
#     requests:
#       storage: 10Gi

# Save and check status
kubectl get pvc my-pvc -w
# NAME     STATUS   VOLUME    CAPACITY   ACCESS MODES   STORAGECLASS
# my-pvc   Bound    pvc-123   5Gi        RWO            expandable-storage  # Expanding
# my-pvc   Bound    pvc-123   10Gi       RWO            expandable-storage  # Expanded

# Check PVC conditions
kubectl describe pvc my-pvc
# Conditions:
#   Type                      Status
#   ----                      ------
#   FileSystemResizePending   False   # Filesystem resize complete
```

**What happens**:
1. Rusternetes Volume Expansion Controller detects size increase
2. Calls CSI driver's `ControllerExpandVolume` RPC (backend resize)
3. Updates PV capacity
4. Updates PVC status to `ControllerResizeInProgress`
5. CSI Node Plugin's `NodeExpandVolume` resizes filesystem (if needed)
6. Updates PVC status to complete

### Expansion Workflow

```
User edits PVC (5Gi → 10Gi)
        ↓
Volume Expansion Controller detects change
        ↓
Checks StorageClass.allowVolumeExpansion = true
        ↓
Calls CSI ControllerExpandVolume (backend resize)
        ↓
Updates PV capacity to 10Gi
        ↓
Sets PVC status: ControllerResizeInProgress
        ↓
CSI Node Plugin NodeExpandVolume (filesystem resize)
        ↓
Sets PVC status: ResizeComplete
        ↓
PVC now shows 10Gi capacity
```

### Monitoring Expansion

```bash
# Watch expansion progress
kubectl get pvc -w

# Check expansion events
kubectl describe pvc my-pvc

# Check for errors
kubectl get events --field-selector involvedObject.name=my-pvc
```

---

## Volume Snapshots

### Prerequisites

Install Snapshot CRDs and Controller:

```bash
# Install snapshot CRDs
kubectl apply -f https://raw.githubusercontent.com/kubernetes-csi/external-snapshotter/master/client/config/crd/snapshot.storage.k8s.io_volumesnapshotclasses.yaml
kubectl apply -f https://raw.githubusercontent.com/kubernetes-csi/external-snapshotter/master/client/config/crd/snapshot.storage.k8s.io_volumesnapshotcontents.yaml
kubectl apply -f https://raw.githubusercontent.com/kubernetes-csi/external-snapshotter/master/client/config/crd/snapshot.storage.k8s.io_volumesnapshots.yaml

# Install snapshot controller
kubectl apply -f https://raw.githubusercontent.com/kubernetes-csi/external-snapshotter/master/deploy/kubernetes/snapshot-controller/setup-snapshot-controller.yaml
```

### Create VolumeSnapshotClass

```yaml
apiVersion: snapshot.storage.k8s.io/v1
kind: VolumeSnapshotClass
metadata:
  name: csi-hostpath-snapclass
driver: hostpath.csi.k8s.io
deletionPolicy: Delete
```

### Create a Snapshot

```yaml
apiVersion: snapshot.storage.k8s.io/v1
kind: VolumeSnapshot
metadata:
  name: my-snapshot
spec:
  volumeSnapshotClassName: csi-hostpath-snapclass
  source:
    persistentVolumeClaimName: my-pvc
```

### Restore from Snapshot

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: restored-pvc
spec:
  accessModes:
    - ReadWriteOnce
  storageClassName: csi-hostpath
  resources:
    requests:
      storage: 5Gi
  dataSource:
    name: my-snapshot
    kind: VolumeSnapshot
    apiGroup: snapshot.storage.k8s.io
```

---

## Troubleshooting

### Issue 1: PVC Stuck in Pending

**Symptoms**:
```bash
kubectl get pvc
# NAME    STATUS    VOLUME   CAPACITY   ACCESS MODES   STORAGECLASS
# my-pvc  Pending                                      csi-hostpath
```

**Solutions**:

1. **Check if CSI driver is running**:
```bash
kubectl get pods -n kube-system | grep csi
# Should see controller and node pods running
```

2. **Check StorageClass exists**:
```bash
kubectl get storageclass csi-hostpath
```

3. **Check CSIDriver registration**:
```bash
kubectl get csidriver
```

4. **Check events**:
```bash
kubectl describe pvc my-pvc
# Look for error messages in Events section
```

5. **Check CSI driver logs**:
```bash
# Controller plugin
kubectl logs -n kube-system csi-hostpath-plugin-xxxxx -c csi-provisioner

# Node plugin (on the node where pod is scheduled)
kubectl logs -n kube-system csi-hostpath-plugin-xxxxx -c hostpath
```

### Issue 2: Volume Mount Fails

**Symptoms**:
```bash
kubectl describe pod my-pod
# Events:
#   Warning  FailedMount  Unable to attach or mount volumes
```

**Solutions**:

1. **Check volume directory exists**:
```bash
# On the node
ls -la /var/lib/kubelet/pods/<pod-uid>/volumes/kubernetes.io~csi/
```

2. **Check CSI Node Plugin is running on the node**:
```bash
kubectl get pods -n kube-system -o wide | grep csi-hostpath-plugin
```

3. **Check node plugin logs**:
```bash
kubectl logs -n kube-system csi-hostpath-plugin-xxxxx -c hostpath
```

4. **Verify PVC is bound**:
```bash
kubectl get pvc my-pvc
# STATUS should be "Bound", not "Pending"
```

### Issue 3: Volume Expansion Fails

**Symptoms**:
```bash
kubectl describe pvc my-pvc
# Conditions:
#   Type: ControllerResizeFailed
```

**Solutions**:

1. **Check StorageClass allows expansion**:
```bash
kubectl get storageclass csi-hostpath -o yaml | grep allowVolumeExpansion
# Should be: allowVolumeExpansion: true
```

2. **Check CSI driver supports expansion**:
```bash
kubectl get csidriver hostpath.csi.k8s.io -o yaml
# Should have VOLUME_EXPANSION capability
```

3. **Check expansion controller logs**:
```bash
kubectl logs -n kube-system deployment/csi-resizer
```

4. **Verify volume is not in use** (for some drivers):
```bash
# Some CSI drivers require pod to be deleted during expansion
kubectl delete pod my-pod
kubectl get pvc my-pvc -w
```

### Issue 4: CSI Driver Not Registered

**Symptoms**:
```bash
kubectl get csidriver
# Expected driver not listed
```

**Solutions**:

1. **Check node-driver-registrar container**:
```bash
kubectl get pods -n kube-system csi-hostpath-plugin-xxxxx
# Should have "node-driver-registrar" container

kubectl logs -n kube-system csi-hostpath-plugin-xxxxx -c node-driver-registrar
```

2. **Verify registration socket**:
```bash
# On the node
ls -la /var/lib/kubelet/plugins_registry/
# Should see <driver-name>-reg.sock
```

3. **Check kubelet can access socket**:
```bash
# On the node
ls -la /var/lib/kubelet/plugins/<driver-name>/csi.sock
```

4. **Restart CSI driver DaemonSet**:
```bash
kubectl rollout restart daemonset/csi-hostpath-plugin -n kube-system
```

### Debugging Checklist

- [ ] CSI driver pods are running (`kubectl get pods -n kube-system | grep csi`)
- [ ] CSIDriver resource exists (`kubectl get csidriver`)
- [ ] StorageClass exists and references correct provisioner
- [ ] PVC references correct StorageClass
- [ ] Unix sockets exist in `/var/lib/kubelet/plugins/<driver>/`
- [ ] Volume directory created in `/var/lib/kubelet/pods/<pod-uid>/volumes/`
- [ ] Check all CSI container logs (provisioner, attacher, resizer, node-driver-registrar, driver)
- [ ] Check events (`kubectl get events`)

---

## Development Roadmap

### Current Status

**✅ Phase 1: API and Controller Foundation** (COMPLETE)
- StorageClass, PVC, PV CRUD operations
- CSI resource types (CSIDriver, CSINode, VolumeAttachment, etc.)
- PV/PVC Binder controller
- Dynamic Provisioner controller
- Volume Expansion Controller

**✅ Phase 2: External CSI Driver Integration** (COMPLETE - Documentation)
- CSI driver deployment patterns documented
- Integration testing with hostpath CSI driver
- StorageClass configuration examples
- Volume expansion workflows documented

**⏳ Phase 3: Advanced CSI Features** (Future)
- Volume snapshots integration
- Volume cloning
- CSI driver metrics and monitoring
- Topology-aware scheduling
- Ephemeral inline volumes (advanced)

**⏳ Phase 4: Production Hardening** (Future)
- CSI driver health monitoring
- Automatic CSI driver recovery
- Volume migration support
- Multi-attach volumes
- Raw block volumes

### Integration Approach

Rusternetes follows the **standard Kubernetes CSI integration pattern**:

1. **Rusternetes provides**:
   - API resources (StorageClass, PVC, PV, CSIDriver, etc.)
   - Controllers (Binder, Provisioner, Expander)
   - Volume directory management
   - Orchestration logic

2. **External CSI drivers provide**:
   - Storage backend integration (CreateVolume, DeleteVolume)
   - Volume attach/detach (ControllerPublishVolume)
   - Volume mount/unmount (NodeStageVolume, NodePublishVolume)
   - Volume expansion (ControllerExpandVolume, NodeExpandVolume)
   - Snapshots and clones

3. **Communication**:
   - gRPC over Unix domain sockets
   - CSI drivers register with kubelet
   - Controllers call CSI driver RPCs
   - Kubelet delegates mount operations to CSI node plugin

This separation of concerns allows:
- Storage vendors to maintain their own CSI drivers
- Operators to choose storage backend independently
- Rusternetes to remain storage-agnostic
- Easy integration of new storage systems

---

## Best Practices

### 1. StorageClass Configuration

```yaml
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: production-ssd
  annotations:
    storageclass.kubernetes.io/is-default-class: "true"  # Make default
provisioner: ebs.csi.aws.com
allowVolumeExpansion: true                                # Enable resize
volumeBindingMode: WaitForFirstConsumer                   # Topology-aware
reclaimPolicy: Delete                                     # Or Retain
parameters:
  type: gp3
  iops: "3000"
  throughput: "125"
  encrypted: "true"
  kmsKeyId: "arn:aws:kms:region:account:key/key-id"
```

### 2. PVC Best Practices

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: app-data
  labels:
    app: my-app
    environment: production
spec:
  accessModes:
    - ReadWriteOnce                    # Single pod write
  resources:
    requests:
      storage: 10Gi                    # Initial size
  storageClassName: production-ssd     # Specific class
  volumeMode: Filesystem               # Or Block
  selector:                            # Optional: bind to specific PV
    matchLabels:
      app: my-app
```

### 3. Pod Volume Configuration

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: app-pod
spec:
  containers:
  - name: app
    image: my-app:latest
    volumeMounts:
    - name: data
      mountPath: /data
      subPath: app-data               # Use subdirectory
      readOnly: false
    resources:
      requests:
        storage: 10Gi                  # Match PVC
  volumes:
  - name: data
    persistentVolumeClaim:
      claimName: app-data
```

### 4. Monitoring and Alerting

```yaml
# Prometheus alerts for CSI volumes
groups:
- name: csi-alerts
  rules:
  - alert: PVCPendingTooLong
    expr: kube_persistentvolumeclaim_status_phase{phase="Pending"} == 1
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "PVC {{ $labels.persistentvolumeclaim }} stuck in Pending"

  - alert: VolumeExpansionFailed
    expr: kube_persistentvolumeclaim_status_condition{condition="ControllerResizeFailed"} == 1
    labels:
      severity: critical
    annotations:
      summary: "Volume expansion failed for PVC {{ $labels.persistentvolumeclaim }}"
```

---

## References

- [CSI Specification](https://github.com/container-storage-interface/spec)
- [Kubernetes CSI Documentation](https://kubernetes-csi.github.io/docs/)
- [CSI Driver List](https://kubernetes-csi.github.io/docs/drivers.html)
- [HostPath CSI Driver](https://github.com/kubernetes-csi/csi-driver-host-path)
- [AWS EBS CSI Driver](https://github.com/kubernetes-sigs/aws-ebs-csi-driver)
- [Volume Snapshots](https://kubernetes.io/docs/concepts/storage/volume-snapshots/)

---

**Document Version**: 1.0
**Last Updated**: 2026-03-13
**Maintainer**: Rusternetes Team
