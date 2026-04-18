import { useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { useK8sList } from "../hooks/useK8sList";
import { useK8sWatch } from "../hooks/useK8sWatch";
import { k8sPatch, buildApiPath } from "../engine/query";
import { StatusBadge } from "../components/StatusBadge";
import { useQueryClient } from "@tanstack/react-query";
import type { Node, Pod } from "../engine/types";
import { Server, Shield, Eye, Ban, CheckCircle } from "lucide-react";

function age(timestamp?: string): string {
  if (!timestamp) return "-";
  const ms = Date.now() - new Date(timestamp).getTime();
  if (ms < 3_600_000) return `${Math.floor(ms / 60_000)}m`;
  if (ms < 86_400_000) return `${Math.floor(ms / 3_600_000)}h`;
  return `${Math.floor(ms / 86_400_000)}d`;
}

function parseQuantity(q?: string): number {
  if (!q) return 0;
  if (q.endsWith("Ki")) return parseInt(q) / (1024 * 1024);
  if (q.endsWith("Mi")) return parseInt(q) / 1024;
  if (q.endsWith("Gi")) return parseInt(q);
  if (q.endsWith("m")) return parseInt(q) / 1000;
  return parseInt(q) || 0;
}

/** Capacity gauge bar. */
function Gauge({
  label,
  used,
  total,
  unit,
  color,
}: {
  label: string;
  used: number;
  total: number;
  unit: string;
  color: string;
}) {
  const pct = total > 0 ? Math.min((used / total) * 100, 100) : 0;
  return (
    <div>
      <div className="flex items-center justify-between text-[10px]">
        <span className="text-[#a89880]">{label}</span>
        <span className="font-mono text-[#e8ddd0]">
          {used}{unit} / {total}{unit}
        </span>
      </div>
      <div className="mt-0.5 h-2 w-full rounded-full bg-surface-3">
        <div
          className="h-2 rounded-full transition-all duration-500"
          style={{
            width: `${pct}%`,
            backgroundColor: pct > 85 ? "#c85a5a" : pct > 60 ? "#f5c842" : color,
          }}
        />
      </div>
    </div>
  );
}

/** Node card with capacity gauges and actions. */
function NodeCard({
  node,
  podCount,
}: {
  node: Node;
  podCount: number;
}) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const ready = node.status?.conditions?.some(
    (c) => c.type === "Ready" && c.status === "True",
  ) ?? false;

  const roles = Object.keys(node.metadata.labels ?? {})
    .filter((l) => l.startsWith("node-role.kubernetes.io/"))
    .map((l) => l.replace("node-role.kubernetes.io/", ""));

  const taints = node.spec.taints ?? [];
  const cpuCapacity = parseInt(node.status?.capacity?.["cpu"] ?? "0");
  const cpuAllocatable = parseInt(node.status?.allocatable?.["cpu"] ?? "0");
  const memCapacity = parseQuantity(node.status?.capacity?.["memory"]);
  const memAllocatable = parseQuantity(node.status?.allocatable?.["memory"]);

  const handleCordon = async () => {
    const path = buildApiPath("", "v1", "nodes", undefined, node.metadata.name);
    await k8sPatch(
      path,
      { spec: { unschedulable: !node.spec.unschedulable } },
      "application/strategic-merge-patch+json",
    );
    queryClient.invalidateQueries({ queryKey: ["k8s", "list", "", "v1", "nodes"] });
  };

  return (
    <div className={`rounded-lg border bg-surface-1 p-4 transition-colors ${
      ready ? "border-surface-3 hover:border-walle-eye/30" : "border-container-red/30"
    }`}>
      {/* Header */}
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-2">
          <Server size={16} className={ready ? "text-walle-eye" : "text-container-red"} />
          <div>
            <button
              onClick={() =>
                navigate(`/resources/${encodeURIComponent("core/v1/nodes")}/${node.metadata.name}`)
              }
              className="font-mono text-sm text-[#e8ddd0] hover:text-rust-light"
            >
              {node.metadata.name}
            </button>
            <div className="flex items-center gap-1.5 mt-0.5">
              {roles.map((r) => (
                <span key={r} className="rounded bg-container-teal/10 px-1.5 py-0.5 text-[9px] text-container-teal">
                  {r}
                </span>
              ))}
              {node.spec.unschedulable && (
                <span className="rounded bg-container-red/10 px-1.5 py-0.5 text-[9px] text-container-red">
                  cordoned
                </span>
              )}
            </div>
          </div>
        </div>
        <StatusBadge status={ready ? "Ready" : "NotReady"} />
      </div>

      {/* Info */}
      <div className="mt-3 grid grid-cols-2 gap-x-4 gap-y-1 text-[10px]">
        <div className="flex justify-between">
          <span className="text-[#a89880]">Version</span>
          <span className="text-[#e8ddd0]">{node.status?.nodeInfo?.kubeletVersion ?? "-"}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-[#a89880]">OS</span>
          <span className="text-[#e8ddd0]">
            {node.status?.nodeInfo?.operatingSystem ?? "-"}/{node.status?.nodeInfo?.architecture ?? "-"}
          </span>
        </div>
        <div className="flex justify-between">
          <span className="text-[#a89880]">Pods</span>
          <span className="font-mono text-walle-yellow">{podCount}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-[#a89880]">Age</span>
          <span className="text-[#e8ddd0]">{age(node.metadata.creationTimestamp)}</span>
        </div>
      </div>

      {/* Capacity gauges */}
      <div className="mt-3 space-y-2">
        <Gauge
          label="CPU"
          used={cpuCapacity - cpuAllocatable}
          total={cpuCapacity}
          unit=" cores"
          color="#4a90b8"
        />
        <Gauge
          label="Memory"
          used={memCapacity - memAllocatable}
          total={memCapacity}
          unit="Gi"
          color="#7ec850"
        />
      </div>

      {/* Taints */}
      {taints.length > 0 && (
        <div className="mt-3">
          <div className="flex flex-wrap gap-1">
            {taints.map((t, i) => (
              <span key={i} className="flex items-center gap-1 rounded bg-walle-yellow/10 px-1.5 py-0.5 text-[9px] text-walle-yellow">
                <Shield size={8} />
                {t.key}={t.value ?? ""}:{t.effect}
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Actions */}
      <div className="mt-3 flex items-center justify-end gap-1">
        <button
          onClick={() =>
            navigate(`/resources/${encodeURIComponent("core/v1/nodes")}/${node.metadata.name}`)
          }
          className="rounded p-1 text-[#a89880] hover:bg-surface-3 hover:text-container-blue"
          title="View details"
        >
          <Eye size={13} />
        </button>
        <button
          onClick={handleCordon}
          className={`rounded p-1 text-[#a89880] hover:bg-surface-3 ${
            node.spec.unschedulable ? "hover:text-walle-eye" : "hover:text-walle-yellow"
          }`}
          title={node.spec.unschedulable ? "Uncordon" : "Cordon"}
        >
          {node.spec.unschedulable ? <CheckCircle size={13} /> : <Ban size={13} />}
        </button>
      </div>
    </div>
  );
}

export function NodesView() {
  const navigate = useNavigate();
  const { data: nodesData, isLoading } = useK8sList<Node>("", "v1", "nodes");
  const { data: podsData } = useK8sList<Pod>("", "v1", "pods");

  useK8sWatch("", "v1", "nodes");

  const nodes = nodesData?.items ?? [];
  const pods = podsData?.items ?? [];

  const podsByNode = useMemo(() => {
    const map: Record<string, number> = {};
    for (const p of pods) {
      const n = p.spec.nodeName ?? "__unscheduled__";
      map[n] = (map[n] ?? 0) + 1;
    }
    return map;
  }, [pods]);

  const readyCount = nodes.filter((n) =>
    n.status?.conditions?.some((c) => c.type === "Ready" && c.status === "True"),
  ).length;

  if (isLoading) {
    return <div className="flex items-center justify-center py-16 text-[#a89880]">Loading nodes...</div>;
  }

  // Zero state
  if (nodes.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-24">
        <Server size={48} className="mb-4 text-[#5a4a3a]" />
        <h2 className="font-retro text-xl text-walle-yellow">No nodes registered</h2>
        <p className="mt-2 text-sm text-[#a89880]">Nodes appear when kubelets connect to the API server</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="font-retro text-2xl text-walle-yellow">Nodes</h1>
          <p className="text-sm text-[#a89880]">
            {readyCount}/{nodes.length} ready &middot; {pods.length} pods scheduled
          </p>
        </div>
        <button
          onClick={() => navigate("/topology")}
          className="flex items-center gap-1.5 rounded-md border border-surface-3 px-3 py-1.5 text-xs text-[#a89880] hover:border-accent/30 hover:text-accent"
        >
          View Topology
        </button>
      </div>

      {/* Node cards grid */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {nodes.map((node) => (
          <NodeCard
            key={node.metadata.uid ?? node.metadata.name}
            node={node}
            podCount={podsByNode[node.metadata.name] ?? 0}
          />
        ))}
      </div>
    </div>
  );
}
