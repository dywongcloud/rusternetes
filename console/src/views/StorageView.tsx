import { useNavigate } from "react-router-dom";
import { useK8sList } from "../hooks/useK8sList";
import { useK8sWatch } from "../hooks/useK8sWatch";
import { useUIStore } from "../store/uiStore";
import { StatusBadge } from "../components/StatusBadge";
import type { K8sResource } from "../engine/types";
import { HardDrive, Eye, Database, ArrowRight } from "lucide-react";

interface PVC extends K8sResource {
  spec: { accessModes?: string[]; resources?: { requests?: Record<string, string> }; storageClassName?: string; volumeName?: string };
  status?: { phase?: string; capacity?: Record<string, string> };
}

interface PV extends K8sResource {
  spec: { capacity?: Record<string, string>; accessModes?: string[]; persistentVolumeReclaimPolicy?: string; storageClassName?: string; claimRef?: { name?: string; namespace?: string } };
  status?: { phase?: string };
}

interface SC extends K8sResource {
  provisioner?: string;
  reclaimPolicy?: string;
  volumeBindingMode?: string;
}

/** PVC card with capacity bar and binding info. */
function PVCCard({ pvc }: { pvc: PVC }) {
  const navigate = useNavigate();
  const requested = pvc.spec.resources?.requests?.["storage"] ?? "-";
  const actual = pvc.status?.capacity?.["storage"];
  const phase = pvc.status?.phase ?? "Unknown";

  return (
    <div className="rounded-lg border border-surface-3 bg-surface-1 p-4 transition-colors hover:border-accent/20">
      <div className="flex items-start justify-between">
        <button
          onClick={() =>
            navigate(`/resources/${encodeURIComponent("core/v1/persistentvolumeclaims")}/${pvc.metadata.namespace}/${pvc.metadata.name}`)
          }
          className="text-left"
        >
          <div className="font-mono text-sm text-[#e8ddd0] hover:text-rust-light">{pvc.metadata.name}</div>
          <div className="text-xs text-[#a89880]">{pvc.metadata.namespace}</div>
        </button>
        <StatusBadge status={phase} />
      </div>

      <div className="mt-3 space-y-1.5 text-[10px]">
        <div className="flex justify-between">
          <span className="text-[#a89880]">Requested</span>
          <span className="font-mono text-[#e8ddd0]">{requested}</span>
        </div>
        {actual && (
          <div className="flex justify-between">
            <span className="text-[#a89880]">Capacity</span>
            <span className="font-mono text-walle-yellow">{actual}</span>
          </div>
        )}
        <div className="flex justify-between">
          <span className="text-[#a89880]">Access</span>
          <span className="text-[#e8ddd0]">{pvc.spec.accessModes?.join(", ") ?? "-"}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-[#a89880]">StorageClass</span>
          <span className="text-[#e8ddd0]">{pvc.spec.storageClassName ?? "default"}</span>
        </div>
        {pvc.spec.volumeName && (
          <div className="flex items-center gap-1">
            <span className="text-[#a89880]">Bound to</span>
            <ArrowRight size={8} className="text-[#5a4a3a]" />
            <span className="font-mono text-container-teal">{pvc.spec.volumeName}</span>
          </div>
        )}
      </div>
    </div>
  );
}

export function StorageView() {
  const ns = useUIStore((s) => s.selectedNamespace);
  const navigate = useNavigate();

  const { data: pvcsData, isLoading: pvcsLoading } = useK8sList<PVC>("", "v1", "persistentvolumeclaims", ns || undefined);
  const { data: pvsData } = useK8sList<PV>("", "v1", "persistentvolumes");
  const { data: scsData } = useK8sList<SC>("storage.k8s.io", "v1", "storageclasses");

  useK8sWatch("", "v1", "persistentvolumeclaims", ns || undefined);

  const pvcs = pvcsData?.items ?? [];
  const pvs = pvsData?.items ?? [];
  const scs = scsData?.items ?? [];

  if (pvcsLoading) {
    return <div className="flex items-center justify-center py-16 text-[#a89880]">Loading storage...</div>;
  }

  // Zero state
  if (pvcs.length === 0 && pvs.length === 0 && scs.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-24">
        <HardDrive size={48} className="mb-4 text-[#5a4a3a]" />
        <h2 className="font-retro text-xl text-walle-yellow">No storage resources</h2>
        <p className="mt-2 text-sm text-[#a89880]">Create PersistentVolumeClaims to provision storage</p>
      </div>
    );
  }

  const boundPVCs = pvcs.filter((p) => p.status?.phase === "Bound").length;
  const availablePVs = pvs.filter((p) => p.status?.phase === "Available").length;

  return (
    <div className="space-y-6">
      <div>
        <h1 className="font-retro text-2xl text-walle-yellow">Storage</h1>
        <p className="text-sm text-[#a89880]">
          {pvcs.length} PVCs ({boundPVCs} bound) &middot; {pvs.length} PVs ({availablePVs} available) &middot; {scs.length} storage classes
        </p>
      </div>

      {/* Storage Classes */}
      {scs.length > 0 && (
        <div>
          <h3 className="mb-3 text-sm font-medium text-[#e8ddd0]">Storage Classes</h3>
          <div className="flex flex-wrap gap-2">
            {scs.map((sc) => (
              <button
                key={sc.metadata.uid}
                onClick={() =>
                  navigate(`/resources/${encodeURIComponent("storage.k8s.io/v1/storageclasses")}/${sc.metadata.name}`)
                }
                className="flex items-center gap-2 rounded-lg border border-surface-3 bg-surface-1 px-3 py-2 hover:border-accent/20"
              >
                <Database size={14} className="text-container-teal" />
                <div className="text-left">
                  <div className="font-mono text-xs text-[#e8ddd0]">{sc.metadata.name}</div>
                  <div className="text-[9px] text-[#a89880]">
                    {sc.provisioner ?? "unknown"} &middot; {sc.reclaimPolicy ?? "Delete"}
                  </div>
                </div>
              </button>
            ))}
          </div>
        </div>
      )}

      {/* PVCs */}
      {pvcs.length > 0 && (
        <div>
          <h3 className="mb-3 text-sm font-medium text-[#e8ddd0]">
            PersistentVolumeClaims ({pvcs.length})
          </h3>
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {pvcs.map((pvc) => (
              <PVCCard key={pvc.metadata.uid ?? pvc.metadata.name} pvc={pvc} />
            ))}
          </div>
        </div>
      )}

      {/* PVs */}
      {pvs.length > 0 && (
        <div>
          <h3 className="mb-3 text-sm font-medium text-[#e8ddd0]">
            PersistentVolumes ({pvs.length})
          </h3>
          <div className="overflow-x-auto rounded-lg border border-surface-3">
            <table className="w-full text-left text-sm">
              <thead>
                <tr className="border-b border-surface-3 bg-surface-2">
                  <th className="px-3 py-2 text-xs font-medium uppercase tracking-wider text-[#a89880]">Name</th>
                  <th className="px-3 py-2 text-xs font-medium uppercase tracking-wider text-[#a89880]">Status</th>
                  <th className="px-3 py-2 text-xs font-medium uppercase tracking-wider text-[#a89880]">Capacity</th>
                  <th className="px-3 py-2 text-xs font-medium uppercase tracking-wider text-[#a89880]">Reclaim</th>
                  <th className="px-3 py-2 text-xs font-medium uppercase tracking-wider text-[#a89880]">Claim</th>
                  <th className="px-3 py-2 text-xs font-medium uppercase tracking-wider text-[#a89880] text-right">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-surface-3">
                {pvs.map((pv) => (
                  <tr key={pv.metadata.uid} className="hover:bg-surface-2">
                    <td className="px-3 py-2">
                      <button
                        onClick={() =>
                          navigate(`/resources/${encodeURIComponent("core/v1/persistentvolumes")}/${pv.metadata.name}`)
                        }
                        className="font-mono text-[#e8ddd0] hover:text-rust-light"
                      >
                        {pv.metadata.name}
                      </button>
                    </td>
                    <td className="px-3 py-2"><StatusBadge status={pv.status?.phase ?? "Unknown"} /></td>
                    <td className="px-3 py-2 font-mono text-xs text-[#e8ddd0]">{pv.spec.capacity?.["storage"] ?? "-"}</td>
                    <td className="px-3 py-2 text-xs text-[#a89880]">{pv.spec.persistentVolumeReclaimPolicy ?? "-"}</td>
                    <td className="px-3 py-2 text-xs text-[#a89880]">
                      {pv.spec.claimRef ? `${pv.spec.claimRef.namespace}/${pv.spec.claimRef.name}` : "-"}
                    </td>
                    <td className="px-3 py-2 text-right">
                      <button
                        onClick={() =>
                          navigate(`/resources/${encodeURIComponent("core/v1/persistentvolumes")}/${pv.metadata.name}`)
                        }
                        className="rounded p-1 text-[#a89880] hover:bg-surface-3 hover:text-container-blue"
                      >
                        <Eye size={13} />
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
