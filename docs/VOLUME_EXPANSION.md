# Volume Expansion in Rusternetes


> **Tip:** PVC status and capacity are visible in the [web console](CONSOLE_USER_GUIDE.md) Storage page.
## Overview

Volume expansion allows you to increase the size of a PersistentVolumeClaim (PVC) after it has been created and bound to a PersistentVolume (PV). This feature is useful when your application needs more storage without having to create a new volume and migrate data.

## Features

- **Dynamic expansion**: Automatically resize volumes when PVC storage request is increased
- **StorageClass control**: Enable/disable expansion per StorageClass
- **Status tracking**: Monitor expansion progress with resize status fields
- **Safe operations**: Only expand bound PVCs, prevent shrinking
- **Kubernetes-compatible**: Follows Kubernetes CSI volume expansion conventions

## Prerequisites

- StorageClass must have `allowVolumeExpansion: true`
- PVC must be bound to a PV
- New size must be greater than current size
- Volume Expansion controller must be running

## How It Works

1. **User updates PVC**: Increase `spec.resources.requests.storage` in PVC
2. **Controller detects change**: Volume Expansion controller notices requested size > current capacity
3. **Validation**: Controller checks if expansion is allowed by StorageClass
4. **Status update**: PVC status shows `resizeStatus: ControllerResizeInProgress`
5. **PV expansion**: Controller resizes the underlying PersistentVolume
6. **Completion**: PVC capacity updated, resize status cleared

## Usage

### 1. Create StorageClass with Expansion Enabled

```yaml
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: expandable-storage
provisioner: rusternetes.io/hostpath
allowVolumeExpansion: true  # Enable expansion
reclaimPolicy: Delete
```

### 2. Create PersistentVolumeClaim

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: my-pvc
  namespace: default
spec:
  accessModes:
    - ReadWriteOnce
  storageClassName: expandable-storage
  resources:
    requests:
      storage: 5Gi  # Initial size
```

### 3. Wait for PVC to Bind

```bash
kubectl get pvc my-pvc -n default

# Output:
# NAME     STATUS   VOLUME                CAPACITY   ACCESS MODES   STORAGECLASS
# my-pvc   Bound    pvc-default-my-pvc    5Gi        RWO            expandable-storage
```

### 4. Expand the Volume

Update the PVC to request more storage:

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: my-pvc
  namespace: default
spec:
  accessModes:
    - ReadWriteOnce
  storageClassName: expandable-storage
  resources:
    requests:
      storage: 10Gi  # Increased from 5Gi
```

Apply the update:

```bash
kubectl apply -f my-pvc.yaml
```

### 5. Monitor Expansion Progress

Check PVC status during expansion:

```bash
kubectl get pvc my-pvc -n default -o yaml
```

**During expansion:**
```yaml
status:
  phase: Bound
  capacity:
    storage: 5Gi  # Original size
  allocatedResources:
    storage: 10Gi  # Target size
  resizeStatus: ControllerResizeInProgress
```

**After expansion:**
```yaml
status:
  phase: Bound
  capacity:
    storage: 10Gi  # Updated size
  allocatedResources:
    storage: 10Gi
  resizeStatus: null  # Cleared when complete
```

## API Fields

### StorageClass

| Field | Type | Description |
|-------|------|-------------|
| `allowVolumeExpansion` | `bool` | Enable/disable volume expansion for this storage class |

### PersistentVolumeClaimStatus

| Field | Type | Description |
|-------|------|-------------|
| `capacity` | `map[string]string` | Current allocated storage capacity |
| `allocatedResources` | `map[string]string` | Target capacity during expansion |
| `resizeStatus` | `enum` | Current state of resize operation |

### ResizeStatus Values

| Status | Description |
|--------|-------------|
| `null` or `""` | No resize operation in progress |
| `ControllerResizeInProgress` | Controller is resizing the volume |
| `ControllerResizeFailed` | Controller resize failed |
| `NodeResizeRequired` | Node-level resize needed (filesystem) |
| `NodeResizeInProgress` | Kubelet is resizing the filesystem |
| `NodeResizeFailed` | Node resize failed |

## Examples

### Complete Workflow Example

See [examples/volume-expansion-example.yaml](examples/volume-expansion-example.yaml) for a complete example including:
- StorageClass with expansion enabled
- PVC with initial size
- Pod using the PVC

See [examples/volume-expansion-resize.yaml](examples/volume-expansion-resize.yaml) for expansion steps.

### Quick Example

```bash
# 1. Create resources
kubectl apply -f examples/volume-expansion-example.yaml

# 2. Verify PVC is bound
kubectl get pvc expandable-pvc -n default

# 3. Expand the volume
kubectl apply -f examples/volume-expansion-resize.yaml

# 4. Check expansion progress
kubectl get pvc expandable-pvc -n default -o jsonpath='{.status}'
```

## Supported Volume Types

Currently, volume expansion is supported for:

- **HostPath volumes**: Immediate expansion (metadata only)
- **Local volumes**: Immediate expansion (metadata only)

For production CSI drivers, the expansion would invoke the CSI `ControllerExpandVolume` RPC.

## Limitations

### Current Limitations

1. **No shrinking**: Cannot decrease PVC size (Kubernetes behavior)
2. **Offline expansion only**: For some volume types, filesystem resize requires pod restart
3. **Manual filesystem resize**: Some volumes may need manual `resize2fs` or similar
4. **Metadata only**: Current implementation updates capacity without actual disk operations

### Kubernetes Compatibility Notes

In Kubernetes with CSI drivers:
- **Controller expansion**: Resizes the underlying storage (disk)
- **Node expansion**: Resizes the filesystem (`resize2fs`, `xfs_growfs`, etc.)

Rusternetes currently implements controller expansion. Node expansion (filesystem resize) is handled by the Kubelet for volumes that require it.

## Error Handling

### Common Errors

**"Volume expansion not allowed for StorageClass"**
- **Cause**: `allowVolumeExpansion` is false or not set
- **Fix**: Update StorageClass with `allowVolumeExpansion: true`

**"PVC has no status"**
- **Cause**: PVC is not yet bound
- **Fix**: Wait for PVC to bind before expanding

**"VolumeSnapshot {name} not found"**
- **Cause**: PVC references non-existent snapshot in dataSource
- **Fix**: Ensure snapshot exists before restoring

**Resize status stuck at "ControllerResizeInProgress"**
- **Cause**: Controller encountered an error
- **Fix**: Check controller logs for errors

### Checking Controller Logs

```bash
# View expansion controller logs
podman logs rusternetes-controller-manager | grep "Volume Expansion"

# Or if running locally
kubectl logs -f deployment/controller-manager -n kube-system | grep expansion
```

## Advanced Usage

### Combining Expansion with Snapshots

You can expand a volume that was restored from a snapshot:

```yaml
# 1. Create PVC from snapshot
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: restored-pvc
spec:
  dataSource:
    name: my-snapshot
    kind: VolumeSnapshot
    apiGroup: snapshot.storage.k8s.io
  storageClassName: expandable-storage
  resources:
    requests:
      storage: 5Gi  # Same as original

# 2. Later, expand the restored volume
# Update to:
  resources:
    requests:
      storage: 10Gi  # Larger than snapshot
```

### Automated Expansion with Monitoring

You can monitor PVC usage and automatically expand volumes:

```bash
# Check PVC usage (requires metrics)
kubectl top pvc my-pvc -n default

# If usage > 80%, expand by 50%
# (Implementation would be a controller or CronJob)
```

## Testing

### Unit Tests

Run unit tests for expansion logic:

```bash
cargo test --package rusternetes-controller-manager --lib volume_expansion
```

### Integration Tests

Run integration tests (requires etcd):

```bash
# Start etcd
podman-compose up -d etcd

# Run tests
cargo test --test volume_expansion_test -- --ignored
```

Test scenarios covered:
- ✅ Expansion with `allowVolumeExpansion: true`
- ✅ Blocked expansion with `allowVolumeExpansion: false`
- ✅ Only expand bound PVCs
- ✅ No expansion when sizes are equal

### Manual Testing

```bash
# 1. Start the cluster
podman-compose up -d

# 2. Create test resources
kubectl apply -f examples/volume-expansion-example.yaml

# 3. Verify initial state
kubectl get pvc expandable-pvc -n default -o yaml | grep -A 3 status

# 4. Expand the volume
kubectl apply -f examples/volume-expansion-resize.yaml

# 5. Watch expansion progress
kubectl get pvc expandable-pvc -n default -o yaml -w
```

## Architecture

### Controller Logic

The Volume Expansion controller runs every 5 seconds and:

1. Lists all PVCs
2. For each bound PVC:
   - Compares `spec.resources.requests.storage` with `status.capacity.storage`
   - If requested > current, initiates expansion
3. Validates `allowVolumeExpansion` on StorageClass
4. Updates PVC status with resize progress
5. Resizes the PV capacity
6. Clears resize status when complete

### Flow Diagram

```
User updates PVC.spec.resources.requests.storage
                    ↓
         Volume Expansion Controller
                    ↓
   Compare requested vs current capacity
                    ↓
           Requested > Current?
                    ↓
         Check allowVolumeExpansion
                    ↓
              Allowed = true?
                    ↓
    Update PVC status: ControllerResizeInProgress
                    ↓
         Resize PersistentVolume
                    ↓
      Update PVC capacity to new size
                    ↓
         Clear resizeStatus → Complete
```

## Performance Considerations

- **Controller interval**: Runs every 5 seconds (configurable)
- **Concurrent expansions**: Controller processes PVCs sequentially
- **Etcd updates**: Each expansion involves 2-3 etcd writes (PVC status, PV spec, PVC final)
- **Scalability**: Can handle hundreds of PVCs with minimal overhead

## Comparison with Kubernetes

| Feature | Kubernetes CSI | Rusternetes |
|---------|---------------|-------------|
| Controller expansion | ✅ Via CSI driver | ✅ Built-in |
| Node expansion | ✅ Via CSI driver | ⏳ Planned |
| Resize status tracking | ✅ Full | ✅ Full |
| Online expansion | ✅ Some drivers | ⏳ Planned |
| Offline expansion | ✅ All drivers | ✅ All types |
| allowVolumeExpansion | ✅ | ✅ |

## Future Enhancements

1. **Online expansion**: Expand volumes while pods are running
2. **Filesystem resize**: Implement node-level expansion (resize2fs)
3. **Metrics**: Expose expansion metrics (count, duration, failures)
4. **Events**: Emit Kubernetes events for expansion lifecycle
5. **Quotas**: Respect namespace resource quotas during expansion
6. **CSI integration**: Support real CSI drivers for cloud volumes

## Related Documentation

- [VOLUME_SNAPSHOTS.md](VOLUME_SNAPSHOTS.md) - Volume snapshot documentation
- [DEPLOYMENT.md](DEPLOYMENT.md) - Cluster deployment guide
- [examples/volume-expansion-example.yaml](examples/volume-expansion-example.yaml) - Example manifests

## Troubleshooting Guide

### PVC Not Expanding

1. Check if StorageClass allows expansion:
   ```bash
   kubectl get sc expandable-storage -o yaml | grep allowVolumeExpansion
   ```

2. Verify PVC is bound:
   ```bash
   kubectl get pvc my-pvc -n default | grep Bound
   ```

3. Check requested size is greater:
   ```bash
   kubectl get pvc my-pvc -n default -o yaml | grep -A 2 "requests:\|capacity:"
   ```

4. Review controller logs:
   ```bash
   podman logs rusternetes-controller-manager | grep -i expansion
   ```

### Expansion Stuck in Progress

1. Check for controller errors in logs
2. Verify PV exists and is accessible
3. Restart controller if needed:
   ```bash
   podman-compose restart controller-manager
   ```

### Filesystem Not Resized

Some volume types require filesystem resize after controller expansion:

```bash
# Inside the pod
df -h /data  # Check current size

# Resize filesystem (ext4)
resize2fs /dev/sdX

# Resize filesystem (xfs)
xfs_growfs /data
```

## Contributing

When adding new volume types or provisioners:

1. Update `VolumeExpansionController::resize_pv()` with type-specific logic
2. Add integration tests in `tests/volume_expansion_test.rs`
3. Update this documentation with supported types
4. Consider adding CSI driver support for production use

## Conclusion

Volume expansion provides a seamless way to increase storage capacity without downtime or data migration. The implementation follows Kubernetes conventions and integrates with the existing storage infrastructure in Rusternetes.

For questions or issues, check the controller logs and refer to the integration tests for expected behavior.
