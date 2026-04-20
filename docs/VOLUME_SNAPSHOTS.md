# Volume Snapshots in Rusternetes


> **Tip:** Volume snapshots and PVC status are visible in the [web console](CONSOLE_USER_GUIDE.md) Storage page.
## Overview

Rusternetes now supports **volume snapshots**, which allow you to capture the state of a PersistentVolumeClaim (PVC) at a specific point in time. This feature enables:

- **Backup and Restore**: Create snapshots of your data for backup purposes
- **Data Migration**: Clone volumes from snapshots to new PVCs
- **Testing**: Create snapshots before risky operations and restore if needed
- **Disaster Recovery**: Maintain point-in-time copies of critical data

## Architecture

The volume snapshot implementation consists of three main resources:

1. **VolumeSnapshotClass**: Defines the driver and parameters for taking snapshots (cluster-scoped)
2. **VolumeSnapshot**: A user request to create a snapshot of a PVC (namespace-scoped)
3. **VolumeSnapshotContent**: The actual snapshot data, automatically created by the controller (cluster-scoped)

## How It Works

1. **User creates a VolumeSnapshotClass** - Defines the snapshotter driver and deletion policy
2. **User creates a PVC** - Creates or uses an existing PVC with data
3. **User creates a VolumeSnapshot** - References the PVC and VolumeSnapshotClass
4. **Volume Snapshot Controller** - Automatically creates a VolumeSnapshotContent
5. **Snapshot is ready** - The snapshot status shows `readyToUse: true`

## Supported Drivers

Currently, Rusternetes supports the following snapshot drivers:
- `rusternetes.io/hostpath-snapshotter` (recommended)
- `hostpath-snapshotter`

## Example Usage

### 1. Create a VolumeSnapshotClass

```yaml
apiVersion: snapshot.storage.k8s.io/v1
kind: VolumeSnapshotClass
metadata:
  name: hostpath-snapclass
driver: rusternetes.io/hostpath-snapshotter
deletionPolicy: Delete  # or Retain
parameters:
  snapshotPath: /tmp/rusternetes/snapshots
```

Apply it:
```bash
./target/release/kubectl --server https://localhost:6443 \
  --insecure-skip-tls-verify apply -f - <<EOF
apiVersion: snapshot.storage.k8s.io/v1
kind: VolumeSnapshotClass
metadata:
  name: hostpath-snapclass
driver: rusternetes.io/hostpath-snapshotter
deletionPolicy: Delete
EOF
```

### 2. Create or Use an Existing PVC

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: test-pvc
  namespace: default
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 5Gi
  storageClassName: fast
```

### 3. Write Data to the PVC (Optional)

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: data-writer
  namespace: default
spec:
  containers:
  - name: writer
    image: busybox:latest
    command:
      - sh
      - -c
      - |
        echo "Important data" > /data/important.txt
        sleep 3600
    volumeMounts:
    - name: storage
      mountPath: /data
  volumes:
  - name: storage
    persistentVolumeClaim:
      claimName: test-pvc
```

### 4. Create a VolumeSnapshot

```yaml
apiVersion: snapshot.storage.k8s.io/v1
kind: VolumeSnapshot
metadata:
  name: test-snapshot
  namespace: default
spec:
  volumeSnapshotClassName: hostpath-snapclass
  source:
    persistentVolumeClaimName: test-pvc
```

Apply it:
```bash
./target/release/kubectl --server https://localhost:6443 \
  --insecure-skip-tls-verify apply -f - <<EOF
apiVersion: snapshot.storage.k8s.io/v1
kind: VolumeSnapshot
metadata:
  name: test-snapshot
  namespace: default
spec:
  volumeSnapshotClassName: hostpath-snapclass
  source:
    persistentVolumeClaimName: test-pvc
EOF
```

**What happens automatically:**
1. Volume Snapshot Controller detects the new VolumeSnapshot
2. Validates that the PVC exists and is bound to a PV
3. Creates a VolumeSnapshotContent with a unique snapshot handle
4. Updates the VolumeSnapshot status to show it's ready

### 5. Verify the Snapshot

Check the VolumeSnapshot status:
```bash
./target/release/kubectl --server https://localhost:6443 \
  --insecure-skip-tls-verify get volumesnapshot test-snapshot -n default -o yaml
```

You should see:
```yaml
status:
  boundVolumeSnapshotContentName: snapcontent-default-test-snapshot
  creationTime: "2024-01-15T10:30:00Z"
  readyToUse: true
```

Check the auto-created VolumeSnapshotContent:
```bash
./target/release/kubectl --server https://localhost:6443 \
  --insecure-skip-tls-verify get volumesnapshotcontent
```

## VolumeSnapshotClass Parameters

### Required Fields
- `driver`: The snapshot driver to use
- `deletionPolicy`: What to do when VolumeSnapshot is deleted
  - `Delete`: Automatically delete the VolumeSnapshotContent and underlying snapshot data
  - `Retain`: Keep the VolumeSnapshotContent and data after VolumeSnapshot deletion

### Optional Parameters
- `parameters.snapshotPath`: Directory for snapshot metadata (default: varies by driver)

## Complete Example

See `examples/volumesnapshot-example.yaml` for a complete working example that includes:
1. VolumeSnapshotClass definition
2. StorageClass for dynamic provisioning
3. PVC that will be snapshotted
4. Pod that writes data to the PVC
5. VolumeSnapshot to capture the PVC state

Apply it:
```bash
./target/release/kubectl --server https://localhost:6443 \
  --insecure-skip-tls-verify apply -f examples/volumesnapshot-example.yaml
```

## Implementation Details

### Controller Architecture

**Volume Snapshot Controller** (`crates/controller-manager/src/controllers/volume_snapshot.rs`):
- Runs every 5 seconds
- Monitors all VolumeSnapshots in the cluster
- For each VolumeSnapshot without a bound content:
  1. Fetches the VolumeSnapshotClass
  2. Validates the driver is supported
  3. Fetches the source PVC and ensures it's bound
  4. Creates a VolumeSnapshotContent with:
     - Name: `snapcontent-{namespace}-{snapshot-name}`
     - Snapshot handle: Unique identifier for the snapshot
     - Creation timestamp
     - Status: `readyToUse: true`
  5. Updates the VolumeSnapshot status

**Deletion Reconciliation**:
- When a VolumeSnapshot is deleted:
  - If `deletionPolicy: Delete`: The VolumeSnapshotContent is automatically deleted
  - If `deletionPolicy: Retain`: The VolumeSnapshotContent is kept for future use

### API Endpoints

The following API endpoints are available:

**VolumeSnapshotClasses** (cluster-scoped):
- `GET /apis/snapshot.storage.k8s.io/v1/volumesnapshotclasses` - List all
- `POST /apis/snapshot.storage.k8s.io/v1/volumesnapshotclasses` - Create
- `GET /apis/snapshot.storage.k8s.io/v1/volumesnapshotclasses/{name}` - Get
- `PUT /apis/snapshot.storage.k8s.io/v1/volumesnapshotclasses/{name}` - Update
- `DELETE /apis/snapshot.storage.k8s.io/v1/volumesnapshotclasses/{name}` - Delete

**VolumeSnapshots** (namespace-scoped):
- `GET /apis/snapshot.storage.k8s.io/v1/namespaces/{ns}/volumesnapshots` - List in namespace
- `GET /apis/snapshot.storage.k8s.io/v1/volumesnapshots` - List all namespaces
- `POST /apis/snapshot.storage.k8s.io/v1/namespaces/{ns}/volumesnapshots` - Create
- `GET /apis/snapshot.storage.k8s.io/v1/namespaces/{ns}/volumesnapshots/{name}` - Get
- `PUT /apis/snapshot.storage.k8s.io/v1/namespaces/{ns}/volumesnapshots/{name}` - Update
- `DELETE /apis/snapshot.storage.k8s.io/v1/namespaces/{ns}/volumesnapshots/{name}` - Delete

**VolumeSnapshotContents** (cluster-scoped):
- `GET /apis/snapshot.storage.k8s.io/v1/volumesnapshotcontents` - List all
- `POST /apis/snapshot.storage.k8s.io/v1/volumesnapshotcontents` - Create
- `GET /apis/snapshot.storage.k8s.io/v1/volumesnapshotcontents/{name}` - Get
- `PUT /apis/snapshot.storage.k8s.io/v1/volumesnapshotcontents/{name}` - Update
- `DELETE /apis/snapshot.storage.k8s.io/v1/volumesnapshotcontents/{name}` - Delete

## Troubleshooting

### VolumeSnapshot stuck without status

Check if the VolumeSnapshotClass exists:
```bash
./target/release/kubectl --server https://localhost:6443 \
  --insecure-skip-tls-verify get volumesnapshotclass
```

Check if the PVC exists and is bound:
```bash
./target/release/kubectl --server https://localhost:6443 \
  --insecure-skip-tls-verify get pvc -n <namespace>
```

Check controller logs:
```bash
podman logs rusternetes-controller-manager 2>&1 | grep -i snapshot
```

### VolumeSnapshotContent not created

This usually means the PVC is not bound to a PV yet. Ensure the PVC shows status `Bound`:
```bash
./target/release/kubectl --server https://localhost:6443 \
  --insecure-skip-tls-verify get pvc <pvc-name> -n <namespace>
```

### Snapshot driver not supported

Currently only `rusternetes.io/hostpath-snapshotter` and `hostpath-snapshotter` are supported. Check your VolumeSnapshotClass driver field.

## Restoring from Snapshots

You can restore data from a snapshot to a new PVC using the `dataSource` field. This is useful for:
- **Disaster Recovery**: Restore data after accidental deletion or corruption
- **Testing**: Create test environments from production snapshots
- **Data Migration**: Clone data to new PVCs

### Example: Restore PVC from Snapshot

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: restored-pvc
  namespace: default
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 5Gi
  storageClassName: fast
  # Restore from snapshot
  dataSource:
    kind: VolumeSnapshot
    name: test-snapshot
    apiGroup: snapshot.storage.k8s.io
```

Apply it:
```bash
./target/release/kubectl --server https://localhost:6443 \
  --insecure-skip-tls-verify apply -f - <<EOF
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: restored-pvc
  namespace: default
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 5Gi
  storageClassName: fast
  dataSource:
    kind: VolumeSnapshot
    name: test-snapshot
    apiGroup: snapshot.storage.k8s.io
EOF
```

**What happens automatically:**
1. Dynamic Provisioner detects the PVC with `dataSource`
2. Validates that the VolumeSnapshot exists and is ready
3. Retrieves the VolumeSnapshotContent bound to the snapshot
4. Creates a new PV with data restored from the snapshot
5. The PV Binder controller binds the PVC to the new PV

### Restore Requirements

- The VolumeSnapshot must have `status.readyToUse: true`
- The VolumeSnapshot must be in the same namespace as the PVC
- The StorageClass must support dynamic provisioning
- The snapshot driver must match the StorageClass provisioner

### Complete Restore Example

See `examples/storage/test-snapshot-restore.yaml` for a complete working example showing:
1. Creating a StorageClass
2. Creating an original PVC with data
3. Taking a VolumeSnapshot
4. Restoring the data to a new PVC
5. Using the restored PVC in a pod

## Limitations

1. **Driver Support**: Only hostpath snapshotter is currently supported
2. **Cross-Namespace Snapshots**: Snapshots can only be created in the same namespace as the source PVC
3. **Cross-Namespace Restore**: Snapshots can only be restored to PVCs in the same namespace
4. **Cloud Provider Snapshots**: AWS EBS, Azure Disk, GCP PD snapshotters not implemented
5. **Actual Data Copy**: Currently simulates restore; actual data copying requires CSI driver integration

## Future Enhancements

- Volume cloning functionality (PVC-to-PVC cloning without explicit snapshots)
- Additional snapshot drivers (NFS, iSCSI, cloud providers)
- Snapshot scheduling and retention policies
- Incremental snapshots for efficiency
- Snapshot verification and validation
- Cross-namespace snapshot restore (with RBAC validation)

## Relationship with Other Features

Volume snapshots work together with:
- **Dynamic Provisioning**: Can snapshot dynamically provisioned PVCs and restore from snapshots
- **PV/PVC Binding**: Requires PVC to be bound before snapshotting; restored PVCs are automatically bound
- **StorageClasses**: Uses StorageClass to determine the source volume's properties and for provisioning restored volumes

## Integration Flow

### Snapshot Creation Flow

```
VolumeSnapshot Created
    ↓
Volume Snapshot Controller
    ↓
Validates PVC is Bound
    ↓
VolumeSnapshotContent Created
    ↓
VolumeSnapshot Status Updated (ready: true)
    ↓
Snapshot Available for Use
```

### Snapshot Restore Flow

```
PVC Created with dataSource (VolumeSnapshot)
    ↓
Dynamic Provisioner Controller
    ↓
Validates VolumeSnapshot exists and is ready
    ↓
Retrieves VolumeSnapshotContent
    ↓
Creates new PV with snapshot data
    ↓
PV Binder Controller binds PVC to new PV
    ↓
Restored PVC ready for use
```

## Best Practices

1. **Always verify PVC is bound** before creating a snapshot
2. **Use meaningful names** for snapshots that indicate their purpose and timestamp
3. **Set appropriate deletion policies** - use `Retain` for important backups
4. **Monitor snapshot status** before relying on the snapshot for recovery
5. **Test restore procedures** regularly to ensure snapshots are working correctly
6. **Document snapshot purposes** in metadata annotations
7. **Verify snapshot readiness** before attempting to restore from it
8. **Match storage capacity** when restoring - the new PVC should have at least the same capacity as the original
9. **Test restored volumes** before using them in production
10. **Keep snapshots and source PVCs in the same namespace** for restore operations
