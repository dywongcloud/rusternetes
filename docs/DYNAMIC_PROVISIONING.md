# Dynamic Volume Provisioning in Rusternetes

## Overview

Rusternetes now supports **dynamic volume provisioning**, which automatically creates PersistentVolumes (PVs) when PersistentVolumeClaims (PVCs) are created with a StorageClass specified. This eliminates the need to manually pre-create PVs.

## How It Works

1. **User creates a StorageClass** - Defines the provisioner and parameters
2. **User creates a PVC** - References the StorageClass
3. **Dynamic Provisioner Controller** - Automatically creates a matching PV
4. **PV Binder Controller** - Automatically binds the PVC to the newly created PV
5. **Kubelet** - Mounts the volume when pods using the PVC are scheduled

## Supported Provisioners

Currently, Rusternetes supports the following provisioners:
- `rusternetes.io/hostpath` (recommended)
- `kubernetes.io/hostpath`
- `hostpath`

## Example Usage

### 1. Create a StorageClass

```yaml
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: fast
provisioner: rusternetes.io/hostpath
parameters:
  path: /tmp/rusternetes/dynamic-pvs  # Base path for volumes
reclaimPolicy: Delete  # or Retain
volumeBindingMode: Immediate
```

Apply it:
```bash
./target/release/kubectl --server https://localhost:6443 \
  --insecure-skip-tls-verify apply -f - <<EOF
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: fast
provisioner: rusternetes.io/hostpath
parameters:
  path: /tmp/rusternetes/dynamic-pvs
reclaimPolicy: Delete
volumeBindingMode: Immediate
EOF
```

### 2. Create a PVC

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: my-pvc
  namespace: default
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 5Gi
  storageClassName: fast  # References the StorageClass
```

Apply it:
```bash
./target/release/kubectl --server https://localhost:6443 \
  --insecure-skip-tls-verify apply -f - <<EOF
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: my-pvc
  namespace: default
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 5Gi
  storageClassName: fast
EOF
```

**What happens automatically:**
1. Dynamic Provisioner detects the PVC with `storageClassName: fast`
2. Creates a PV named `pvc-default-my-pvc` with:
   - Path: `/tmp/rusternetes/dynamic-pvs/pvc-default-my-pvc`
   - Capacity: `5Gi`
   - Access modes: `ReadWriteOnce`
   - Reclaim policy: `Delete` (from StorageClass)
3. PV Binder binds the PVC to the new PV

### 3. Use the PVC in a Pod

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: app-pod
  namespace: default
spec:
  containers:
  - name: app
    image: nginx:1.25-alpine
    volumeMounts:
    - name: storage
      mountPath: /data
  volumes:
  - name: storage
    persistentVolumeClaim:
      claimName: my-pvc
```

## Verifying Dynamic Provisioning

### Check the PVC status

```bash
./target/release/kubectl --server https://localhost:6443 \
  --insecure-skip-tls-verify get pvc my-pvc -n default -o yaml
```

You should see:
```yaml
status:
  phase: Bound
  accessModes:
    - ReadWriteOnce
  capacity:
    storage: 5Gi
```

### Check the auto-created PV

```bash
./target/release/kubectl --server https://localhost:6443 \
  --insecure-skip-tls-verify get pv
```

You should see a PV named `pvc-default-my-pvc` with labels:
- `pvc-name: my-pvc`
- `pvc-namespace: default`
- `provisioner: rusternetes.io/hostpath`
- `storage-class: fast`

### Check controller logs

```bash
# Dynamic Provisioner logs
podman logs rusternetes-controller-manager 2>&1 | grep -i "dynamic"

# PV Binder logs
podman logs rusternetes-controller-manager 2>&1 | grep -i "binding"
```

## StorageClass Parameters

### Required Fields
- `provisioner`: Must be one of the supported provisioners
- `metadata.name`: Name to reference in PVCs

### Optional Parameters
- `parameters.path`: Base directory for volumes (default: `/tmp/rusternetes/dynamic-pvs`)
- `reclaimPolicy`: What to do when PVC is deleted
  - `Delete`: Automatically delete the PV and underlying storage
  - `Retain`: Keep the PV and data after PVC deletion
- `volumeBindingMode`:
  - `Immediate`: Bind PVC to PV as soon as PV is created (default)
  - `WaitForFirstConsumer`: Wait until a pod using the PVC is scheduled

## Complete Example

See `examples/test-dynamic-pvc.yaml` for a complete working example that includes:
1. StorageClass definition
2. PVC that will be dynamically provisioned
3. Pod that uses the dynamically provisioned volume

Apply it:
```bash
./target/release/kubectl --server https://localhost:6443 \
  --insecure-skip-tls-verify apply -f examples/test-dynamic-pvc.yaml
```

## Implementation Details

### Controller Architecture

**Dynamic Provisioner Controller** (`crates/controller-manager/src/controllers/dynamic_provisioner.rs`):
- Runs every 5 seconds
- Monitors all PVCs in the cluster
- For each unbound PVC with a `storageClassName`:
  1. Fetches the StorageClass
  2. Validates the provisioner is supported
  3. Creates a PV with:
     - Name: `pvc-{namespace}-{pvc-name}`
     - Capacity from PVC requests
     - Access modes from PVC
     - Reclaim policy from StorageClass
     - Path: `{base-path}/pvc-{namespace}-{pvc-name}`
  4. Adds labels and annotations for tracking
  5. Sets PV status to `Available`

**PV Binder Controller** (`crates/controller-manager/src/controllers/pv_binder.rs`):
- Runs every 5 seconds
- Monitors all unbound PVCs
- Matches PVCs to available PVs based on:
  - Storage class name
  - Capacity (PV must have >= PVC requested)
  - Access modes (PV must support all PVC modes)
- Binds PVC to PV by:
  1. Setting `pv.spec.claimRef` to reference the PVC
  2. Setting `pvc.spec.volumeName` to the PV name
  3. Updating both statuses to `Bound`

### Integration Flow

```
PVC Created
    ↓
Dynamic Provisioner Controller
    ↓
PV Created (status: Available)
    ↓
PV Binder Controller
    ↓
PVC ←→ PV Bound
    ↓
Pod Scheduled
    ↓
Kubelet Mounts Volume
```

## Troubleshooting

### PVC stuck in Pending

Check if the StorageClass exists:
```bash
./target/release/kubectl --server https://localhost:6443 \
  --insecure-skip-tls-verify get storageclass
```

Check controller logs:
```bash
podman logs rusternetes-controller-manager 2>&1 | grep -i "provision\|binding"
```

### PV created but not bound

This usually means the PV Binder is working on it. Wait a few seconds (up to 5 seconds for the next reconciliation loop).

### Volume not mounted in pod

Check kubelet logs:
```bash
podman logs rusternetes-kubelet 2>&1 | grep -i volume
```

Ensure the PVC is `Bound` before the pod starts:
```bash
./target/release/kubectl --server https://localhost:6443 \
  --insecure-skip-tls-verify get pvc -n <namespace>
```

## Limitations

1. **Provisioner Support**: Only hostpath volumes are currently supported
2. **Volume Expansion**: Cannot resize volumes after creation
3. **Snapshots**: Volume snapshots are not yet implemented
4. **Cloud Providers**: AWS EBS, Azure Disk, GCP PD provisioners not implemented

## Future Enhancements

- Support for additional provisioners (NFS, iSCSI, cloud providers)
- Volume expansion (resizing existing volumes)
- Volume snapshots and cloning
- WaitForFirstConsumer binding mode (topology-aware provisioning)
- Volume health monitoring
