import { useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { useK8sList } from "../hooks/useK8sList";
import { useK8sWatch } from "../hooks/useK8sWatch";
import { useUIStore } from "../store/uiStore";
import { StatusBadge } from "../components/StatusBadge";
import type { Service, Pod, K8sResource } from "../engine/types";
import { k8sDelete, buildApiPath } from "../engine/query";
import { useQueryClient } from "@tanstack/react-query";
import { Network, Globe, Eye, Trash2, ArrowRight, Plus } from "lucide-react";

const TYPE_COLORS: Record<string, string> = {
  ClusterIP: "#4a90b8",
  NodePort: "#7ec850",
  LoadBalancer: "#e8722a",
  ExternalName: "#4aaaa0",
};

/** Service card showing ports, type, and connected pods. */
function ServiceCard({ svc, matchingPods }: { svc: Service; matchingPods: number }) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const svcType = svc.spec.type ?? "ClusterIP";
  const typeColor = TYPE_COLORS[svcType] ?? "#a89880";

  const handleDelete = async () => {
    if (!confirm(`Delete service "${svc.metadata.name}"?`)) return;
    const path = buildApiPath("", "v1", "services", svc.metadata.namespace, svc.metadata.name);
    await k8sDelete(path);
    queryClient.invalidateQueries({ queryKey: ["k8s", "list", "", "v1", "services"] });
  };

  return (
    <div className="rounded-lg border border-surface-3 bg-surface-1 p-4 transition-colors hover:border-accent/20">
      {/* Header */}
      <div className="flex items-start justify-between">
        <button
          onClick={() =>
            navigate(`/resources/${encodeURIComponent("core/v1/services")}/${svc.metadata.namespace}/${svc.metadata.name}`)
          }
          className="text-left"
        >
          <div className="font-mono text-sm text-[#e8ddd0] hover:text-rust-light">
            {svc.metadata.name}
          </div>
          <div className="text-xs text-[#a89880]">{svc.metadata.namespace}</div>
        </button>
        <span
          className="rounded-full px-2 py-0.5 text-[10px] font-medium"
          style={{ backgroundColor: `${typeColor}15`, color: typeColor }}
        >
          {svcType}
        </span>
      </div>

      {/* ClusterIP */}
      <div className="mt-3 flex items-center gap-2">
        <Globe size={12} className="text-[#a89880]" />
        <span className="font-mono text-xs text-[#e8ddd0]">
          {svc.spec.clusterIP ?? "None"}
        </span>
      </div>

      {/* Ports */}
      {svc.spec.ports && svc.spec.ports.length > 0 && (
        <div className="mt-2 flex flex-wrap gap-1.5">
          {svc.spec.ports.map((p, i) => (
            <span
              key={i}
              className="flex items-center gap-1 rounded bg-surface-2 px-2 py-0.5 text-[10px]"
            >
              <span className="text-[#e8ddd0]">{p.port}</span>
              <ArrowRight size={8} className="text-[#5a4a3a]" />
              <span className="text-[#a89880]">{p.targetPort ?? p.port}</span>
              <span className="text-[#5a4a3a]">/{p.protocol ?? "TCP"}</span>
              {p.nodePort && (
                <span className="text-walle-yellow">:{p.nodePort}</span>
              )}
            </span>
          ))}
        </div>
      )}

      {/* Selector -> Pod count */}
      <div className="mt-2 flex items-center justify-between">
        <span className="text-[10px] text-[#a89880]">
          {matchingPods} target pod{matchingPods !== 1 ? "s" : ""}
        </span>
        <div className="flex items-center gap-1">
          <button
            onClick={() =>
              navigate(`/resources/${encodeURIComponent("core/v1/services")}/${svc.metadata.namespace}/${svc.metadata.name}`)
            }
            className="rounded p-1 text-[#a89880] hover:bg-surface-3 hover:text-container-blue"
          >
            <Eye size={13} />
          </button>
          <button
            onClick={handleDelete}
            className="rounded p-1 text-[#a89880] hover:bg-surface-3 hover:text-container-red"
          >
            <Trash2 size={13} />
          </button>
        </div>
      </div>
    </div>
  );
}

/** Visual service-to-pod connection diagram for a single service. */
function ServiceTopology({
  svc,
  pods,
}: {
  svc: Service;
  pods: Pod[];
}) {
  const selector = svc.spec.selector ?? {};
  const matchingPods = pods.filter((p) => {
    const labels = p.metadata.labels ?? {};
    return Object.entries(selector).every(([k, v]) => labels[k] === v);
  });

  if (Object.keys(selector).length === 0 || matchingPods.length === 0) return null;

  const svcType = svc.spec.type ?? "ClusterIP";
  const typeColor = TYPE_COLORS[svcType] ?? "#a89880";

  return (
    <div className="rounded-lg border border-surface-3 bg-surface-1 p-4">
      <div className="flex items-center gap-4">
        {/* Service node */}
        <div
          className="flex shrink-0 flex-col items-center rounded-lg border-2 px-4 py-2"
          style={{ borderColor: typeColor }}
        >
          <Network size={16} style={{ color: typeColor }} />
          <span className="mt-1 font-mono text-xs text-[#e8ddd0]">{svc.metadata.name}</span>
          <span className="text-[9px] text-[#a89880]">
            {svc.spec.clusterIP}
          </span>
        </div>

        {/* Arrow */}
        <div className="flex items-center">
          <div className="h-px w-8" style={{ backgroundColor: typeColor, opacity: 0.4 }} />
          <ArrowRight size={12} style={{ color: typeColor, opacity: 0.6 }} />
        </div>

        {/* Target pods */}
        <div className="flex flex-wrap gap-1.5">
          {matchingPods.slice(0, 8).map((p) => (
            <div
              key={p.metadata.uid}
              className="flex items-center gap-1 rounded bg-surface-2 px-2 py-1 text-[10px]"
            >
              <span
                className="h-2 w-2 rounded-full"
                style={{
                  backgroundColor:
                    p.status?.phase === "Running" ? "#7ec850" :
                    p.status?.phase === "Pending" ? "#f5c842" : "#c85a5a",
                }}
              />
              <span className="text-[#e8ddd0]">{p.metadata.name}</span>
              {p.status?.podIP && (
                <span className="text-[#5a4a3a]">{p.status.podIP}</span>
              )}
            </div>
          ))}
          {matchingPods.length > 8 && (
            <span className="px-2 py-1 text-[10px] text-[#a89880]">
              +{matchingPods.length - 8} more
            </span>
          )}
        </div>
      </div>
    </div>
  );
}

export function NetworkingView() {
  const ns = useUIStore((s) => s.selectedNamespace);
  const navigate = useNavigate();

  const { data: servicesData, isLoading: svcLoading } = useK8sList<Service>(
    "", "v1", "services", ns || undefined,
  );
  const { data: podsData } = useK8sList<Pod>("", "v1", "pods", ns || undefined);
  const { data: ingressData } = useK8sList<K8sResource>(
    "networking.k8s.io", "v1", "ingresses", ns || undefined,
  );
  const { data: netpolData } = useK8sList<K8sResource>(
    "networking.k8s.io", "v1", "networkpolicies", ns || undefined,
  );

  useK8sWatch("", "v1", "services", ns || undefined);

  const services = servicesData?.items ?? [];
  const pods = podsData?.items ?? [];
  const ingresses = ingressData?.items ?? [];
  const netpols = netpolData?.items ?? [];

  // Calculate matching pods per service
  const svcPodCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const svc of services) {
      const selector = svc.spec.selector ?? {};
      if (Object.keys(selector).length === 0) {
        counts[svc.metadata.uid ?? svc.metadata.name] = 0;
        continue;
      }
      counts[svc.metadata.uid ?? svc.metadata.name] = pods.filter((p) => {
        const labels = p.metadata.labels ?? {};
        return Object.entries(selector).every(([k, v]) => labels[k] === v);
      }).length;
    }
    return counts;
  }, [services, pods]);

  if (svcLoading) {
    return <div className="flex items-center justify-center py-16 text-[#a89880]">Loading networking...</div>;
  }

  // Zero state
  if (services.length === 0 && ingresses.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-24">
        <Network size={48} className="mb-4 text-[#5a4a3a]" />
        <h2 className="font-retro text-xl text-walle-yellow">No network resources</h2>
        <p className="mt-2 text-sm text-[#a89880]">Services and ingresses will appear here</p>
        <button
          onClick={() => navigate("/create")}
          className="mt-4 flex items-center gap-2 rounded-md bg-accent px-4 py-2 text-sm font-medium text-surface-0 hover:bg-accent-hover"
        >
          <Plus size={16} />
          Create Service
        </button>
      </div>
    );
  }

  // Services with selectors that have matching pods
  const servicesWithPods = services.filter((svc) => {
    const selector = svc.spec.selector ?? {};
    return Object.keys(selector).length > 0;
  });

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="font-retro text-2xl text-walle-yellow">Networking</h1>
          <p className="text-sm text-[#a89880]">
            {services.length} services &middot; {ingresses.length} ingresses &middot; {netpols.length} network policies
          </p>
        </div>
        <button
          onClick={() => navigate("/topology")}
          className="flex items-center gap-1.5 rounded-md border border-surface-3 px-3 py-1.5 text-xs text-[#a89880] hover:border-accent/30 hover:text-accent"
        >
          View Topology
        </button>
      </div>

      {/* Summary by type */}
      <div className="flex gap-4">
        {Object.entries(TYPE_COLORS).map(([type, color]) => {
          const count = services.filter((s) => (s.spec.type ?? "ClusterIP") === type).length;
          if (count === 0) return null;
          return (
            <div key={type} className="flex items-center gap-2 rounded-lg border border-surface-3 bg-surface-1 px-3 py-2">
              <span className="h-3 w-3 rounded-full" style={{ backgroundColor: color }} />
              <span className="text-xs text-[#a89880]">{type}</span>
              <span className="font-mono text-sm text-[#e8ddd0]">{count}</span>
            </div>
          );
        })}
      </div>

      {/* Service -> Pod topology diagrams */}
      {servicesWithPods.length > 0 && (
        <div>
          <h3 className="mb-3 text-sm font-medium text-[#e8ddd0]">Service Routing</h3>
          <div className="space-y-2">
            {servicesWithPods.slice(0, 6).map((svc) => (
              <ServiceTopology key={svc.metadata.uid} svc={svc} pods={pods} />
            ))}
          </div>
        </div>
      )}

      {/* Service cards */}
      <div>
        <h3 className="mb-3 text-sm font-medium text-[#e8ddd0]">
          Services ({services.length})
        </h3>
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {services.map((svc) => (
            <ServiceCard
              key={svc.metadata.uid ?? svc.metadata.name}
              svc={svc}
              matchingPods={svcPodCounts[svc.metadata.uid ?? svc.metadata.name] ?? 0}
            />
          ))}
        </div>
      </div>

      {/* Ingresses */}
      {ingresses.length > 0 && (
        <div>
          <h3 className="mb-3 text-sm font-medium text-[#e8ddd0]">
            Ingresses ({ingresses.length})
          </h3>
          <div className="overflow-x-auto rounded-lg border border-surface-3">
            <table className="w-full text-left text-sm">
              <thead>
                <tr className="border-b border-surface-3 bg-surface-2">
                  <th className="px-3 py-2 text-xs font-medium uppercase tracking-wider text-[#a89880]">Name</th>
                  <th className="px-3 py-2 text-xs font-medium uppercase tracking-wider text-[#a89880]">Namespace</th>
                  <th className="px-3 py-2 text-xs font-medium uppercase tracking-wider text-[#a89880] text-right">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-surface-3">
                {ingresses.map((ing) => (
                  <tr key={ing.metadata.uid} className="hover:bg-surface-2">
                    <td className="px-3 py-2">
                      <button
                        onClick={() =>
                          navigate(`/resources/${encodeURIComponent("networking.k8s.io/v1/ingresses")}/${ing.metadata.namespace}/${ing.metadata.name}`)
                        }
                        className="font-mono text-[#e8ddd0] hover:text-rust-light"
                      >
                        {ing.metadata.name}
                      </button>
                    </td>
                    <td className="px-3 py-2 text-[#a89880]">{ing.metadata.namespace}</td>
                    <td className="px-3 py-2 text-right">
                      <button
                        onClick={() =>
                          navigate(`/resources/${encodeURIComponent("networking.k8s.io/v1/ingresses")}/${ing.metadata.namespace}/${ing.metadata.name}`)
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

      {/* Network Policies */}
      {netpols.length > 0 && (
        <div>
          <h3 className="mb-3 text-sm font-medium text-[#e8ddd0]">
            Network Policies ({netpols.length})
          </h3>
          <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
            {netpols.map((np) => (
              <button
                key={np.metadata.uid}
                onClick={() =>
                  navigate(`/resources/${encodeURIComponent("networking.k8s.io/v1/networkpolicies")}/${np.metadata.namespace}/${np.metadata.name}`)
                }
                className="flex items-center gap-2 rounded-lg border border-surface-3 bg-surface-1 px-3 py-2 text-left hover:border-accent/20"
              >
                <Shield size={14} className="text-walle-yellow" />
                <div>
                  <div className="font-mono text-xs text-[#e8ddd0]">{np.metadata.name}</div>
                  <div className="text-[10px] text-[#a89880]">{np.metadata.namespace}</div>
                </div>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
