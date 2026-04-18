import { useMemo } from "react";
import { useK8sList } from "../hooks/useK8sList";
import type { Pod, Node, Service } from "../engine/types";
import { useNavigate } from "react-router-dom";

/** Colors matching the retro theme. */
const COLORS = {
  node: "#4a90b8",
  nodeBg: "#4a90b820",
  podRunning: "#7ec850",
  podPending: "#f5c842",
  podFailed: "#c85a5a",
  podUnknown: "#a89880",
  service: "#e8722a",
  serviceLine: "#e8722a40",
  text: "#e8ddd0",
  textDim: "#a89880",
  grid: "#2a211820",
};

function podColor(phase?: string) {
  switch (phase) {
    case "Running": return COLORS.podRunning;
    case "Pending": return COLORS.podPending;
    case "Failed": return COLORS.podFailed;
    default: return COLORS.podUnknown;
  }
}

interface LayoutNode {
  id: string;
  name: string;
  x: number;
  y: number;
  pods: LayoutPod[];
  ready: boolean;
  capacity: { cpu?: string; memory?: string };
}

interface LayoutPod {
  id: string;
  name: string;
  namespace: string;
  phase: string;
  x: number;
  y: number;
  restarts: number;
  containers: number;
  readyContainers: number;
}

interface LayoutService {
  id: string;
  name: string;
  namespace: string;
  type: string;
  clusterIP: string;
  x: number;
  y: number;
  targetPods: string[]; // pod IDs matched by selector
}

export function TopologyView() {
  const navigate = useNavigate();
  const { data: nodesData } = useK8sList<Node>("", "v1", "nodes", undefined, { refetchInterval: 15_000 });
  const { data: podsData } = useK8sList<Pod>("", "v1", "pods", undefined, { refetchInterval: 10_000 });
  const { data: servicesData } = useK8sList<Service>("", "v1", "services", undefined, { refetchInterval: 30_000 });

  const nodes = nodesData?.items ?? [];
  const pods = podsData?.items ?? [];
  const services = servicesData?.items ?? [];

  const layout = useMemo(() => {
    const NODE_WIDTH = 280;
    const NODE_PADDING = 30;
    const POD_SIZE = 14;
    const POD_GAP = 4;
    const PODS_PER_ROW = 10;
    const NODE_HEADER = 50;
    const SERVICE_AREA_HEIGHT = 80;

    // Layout nodes
    const layoutNodes: LayoutNode[] = nodes.map((n, i) => {
      const nodePods = pods.filter((p) => p.spec.nodeName === n.metadata.name);

      const x = NODE_PADDING + i * (NODE_WIDTH + NODE_PADDING);
      const y = SERVICE_AREA_HEIGHT + 20;

      const ready = n.status?.conditions?.some(
        (c) => c.type === "Ready" && c.status === "True",
      ) ?? false;

      const layoutPods: LayoutPod[] = nodePods.map((p, pi) => {
        const row = Math.floor(pi / PODS_PER_ROW);
        const col = pi % PODS_PER_ROW;
        return {
          id: p.metadata.uid ?? p.metadata.name,
          name: p.metadata.name,
          namespace: p.metadata.namespace ?? "default",
          phase: p.status?.phase ?? "Unknown",
          x: x + 15 + col * (POD_SIZE + POD_GAP),
          y: y + NODE_HEADER + row * (POD_SIZE + POD_GAP),
          restarts: p.status?.containerStatuses?.reduce((s, c) => s + c.restartCount, 0) ?? 0,
          containers: p.spec.containers.length,
          readyContainers: p.status?.containerStatuses?.filter((c) => c.ready).length ?? 0,
        };
      });

      return {
        id: n.metadata.uid ?? n.metadata.name,
        name: n.metadata.name,
        x,
        y,
        pods: layoutPods,
        ready,
        capacity: {
          cpu: n.status?.capacity?.["cpu"],
          memory: n.status?.capacity?.["memory"],
        },
      };
    });

    // Unscheduled pods (no nodeName)
    const unscheduled = pods.filter((p) => !p.spec.nodeName);

    // Layout services across the top
    const layoutServices: LayoutService[] = services.map((svc, i) => {
      const selector = svc.spec.selector ?? {};
      const matchingPods = pods
        .filter((p) => {
          const labels = p.metadata.labels ?? {};
          return Object.entries(selector).every(([k, v]) => labels[k] === v);
        })
        .map((p) => p.metadata.uid ?? p.metadata.name);

      return {
        id: svc.metadata.uid ?? svc.metadata.name,
        name: svc.metadata.name,
        namespace: svc.metadata.namespace ?? "default",
        type: svc.spec.type ?? "ClusterIP",
        clusterIP: svc.spec.clusterIP ?? "",
        x: 30 + i * 140,
        y: 30,
        targetPods: matchingPods,
      };
    });

    // Calculate SVG dimensions
    const maxNodeX = layoutNodes.reduce(
      (max, n) => Math.max(max, n.x + NODE_WIDTH),
      400,
    );
    const maxNodeY = layoutNodes.reduce((max, n) => {
      const podRows = Math.ceil(n.pods.length / PODS_PER_ROW) || 1;
      return Math.max(max, n.y + NODE_HEADER + podRows * (POD_SIZE + POD_GAP) + NODE_PADDING);
    }, 300);
    const svcWidth = layoutServices.length * 140 + 60;

    return {
      nodes: layoutNodes,
      services: layoutServices,
      unscheduled,
      width: Math.max(maxNodeX + 40, svcWidth),
      height: maxNodeY + 40,
      nodeWidth: NODE_WIDTH,
      nodeHeader: NODE_HEADER,
      podSize: POD_SIZE,
      podsPerRow: PODS_PER_ROW,
      podGap: POD_GAP,
      nodePadding: NODE_PADDING,
    };
  }, [nodes, pods, services]);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-lg font-semibold text-[#f5efe8]">Cluster Topology</h1>
          <p className="text-sm text-[#a89880]">
            {nodes.length} nodes &middot; {pods.length} pods &middot; {services.length} services
          </p>
        </div>
        {/* Legend */}
        <div className="flex items-center gap-4 text-xs text-[#a89880]">
          <span className="flex items-center gap-1">
            <span className="inline-block h-3 w-3 rounded-sm" style={{ backgroundColor: COLORS.podRunning }} />
            Running
          </span>
          <span className="flex items-center gap-1">
            <span className="inline-block h-3 w-3 rounded-sm" style={{ backgroundColor: COLORS.podPending }} />
            Pending
          </span>
          <span className="flex items-center gap-1">
            <span className="inline-block h-3 w-3 rounded-sm" style={{ backgroundColor: COLORS.podFailed }} />
            Failed
          </span>
          <span className="flex items-center gap-1">
            <span className="inline-block h-3 w-6 rounded-sm" style={{ backgroundColor: COLORS.service }} />
            Service
          </span>
        </div>
      </div>

      <div className="overflow-auto rounded-lg border border-surface-3 bg-surface-0">
        <svg
          width={layout.width}
          height={layout.height}
          viewBox={`0 0 ${layout.width} ${layout.height}`}
          className="min-w-full"
        >
          {/* Grid pattern */}
          <defs>
            <pattern id="grid" width="20" height="20" patternUnits="userSpaceOnUse">
              <path d="M 20 0 L 0 0 0 20" fill="none" stroke={COLORS.grid} strokeWidth="0.5" />
            </pattern>
            {/* Glow filter for services */}
            <filter id="svc-glow">
              <feGaussianBlur stdDeviation="3" result="glow" />
              <feMerge>
                <feMergeNode in="glow" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
            {/* Animated dash for network lines */}
            <style>{`
              @keyframes dash { to { stroke-dashoffset: -20; } }
              .network-line { animation: dash 1s linear infinite; }
              @keyframes pulse { 0%, 100% { opacity: 0.6; } 50% { opacity: 1; } }
              .pod-pulse { animation: pulse 2s ease-in-out infinite; }
            `}</style>
          </defs>
          <rect width="100%" height="100%" fill="url(#grid)" />

          {/* Service -> Pod connection lines */}
          {layout.services.map((svc) =>
            svc.targetPods.map((podId) => {
              const pod = layout.nodes
                .flatMap((n) => n.pods)
                .find((p) => p.id === podId);
              if (!pod) return null;
              return (
                <line
                  key={`${svc.id}-${podId}`}
                  x1={svc.x + 40}
                  y1={svc.y + 20}
                  x2={pod.x + layout.podSize / 2}
                  y2={pod.y + layout.podSize / 2}
                  stroke={COLORS.serviceLine}
                  strokeWidth="1"
                  strokeDasharray="4 4"
                  className="network-line"
                />
              );
            }),
          )}

          {/* Services */}
          {layout.services.map((svc) => (
            <g key={svc.id} className="cursor-pointer">
              <rect
                x={svc.x}
                y={svc.y}
                width={120}
                height={40}
                rx={6}
                fill={`${COLORS.service}15`}
                stroke={COLORS.service}
                strokeWidth={1.5}
                filter="url(#svc-glow)"
              />
              <text
                x={svc.x + 60}
                y={svc.y + 16}
                textAnchor="middle"
                fill={COLORS.service}
                fontSize={10}
                fontWeight="bold"
                fontFamily="'Space Mono', monospace"
              >
                {svc.name.length > 14 ? svc.name.slice(0, 14) + "..." : svc.name}
              </text>
              <text
                x={svc.x + 60}
                y={svc.y + 30}
                textAnchor="middle"
                fill={COLORS.textDim}
                fontSize={8}
                fontFamily="'Space Mono', monospace"
              >
                {svc.clusterIP} &middot; {svc.type}
              </text>
            </g>
          ))}

          {/* Nodes */}
          {layout.nodes.map((node) => {
            const podRows = Math.ceil(node.pods.length / layout.podsPerRow) || 1;
            const nodeHeight =
              layout.nodeHeader +
              podRows * (layout.podSize + layout.podGap) +
              layout.nodePadding;

            return (
              <g key={node.id}>
                {/* Node container */}
                <rect
                  x={node.x}
                  y={node.y}
                  width={layout.nodeWidth}
                  height={nodeHeight}
                  rx={8}
                  fill={COLORS.nodeBg}
                  stroke={node.ready ? COLORS.node : COLORS.podFailed}
                  strokeWidth={1.5}
                  strokeDasharray={node.ready ? "none" : "4 4"}
                />
                {/* Node header */}
                <text
                  x={node.x + 12}
                  y={node.y + 20}
                  fill={COLORS.text}
                  fontSize={11}
                  fontWeight="bold"
                  fontFamily="'Space Mono', monospace"
                >
                  {node.name}
                </text>
                <text
                  x={node.x + 12}
                  y={node.y + 35}
                  fill={COLORS.textDim}
                  fontSize={9}
                  fontFamily="'Space Mono', monospace"
                >
                  {node.pods.length} pods
                  {node.capacity.cpu && ` &middot; ${node.capacity.cpu} CPU`}
                </text>
                {/* Status dot */}
                <circle
                  cx={node.x + layout.nodeWidth - 15}
                  cy={node.y + 20}
                  r={4}
                  fill={node.ready ? COLORS.podRunning : COLORS.podFailed}
                  className={node.ready ? "" : "pod-pulse"}
                />

                {/* Pods */}
                {node.pods.map((pod) => (
                  <g
                    key={pod.id}
                    className="cursor-pointer"
                    onClick={() =>
                      navigate(
                        `/resources/${encodeURIComponent("core/v1/pods")}/${pod.namespace}/${pod.name}`,
                      )
                    }
                  >
                    <rect
                      x={pod.x}
                      y={pod.y}
                      width={layout.podSize}
                      height={layout.podSize}
                      rx={2}
                      fill={podColor(pod.phase)}
                      opacity={pod.phase === "Running" ? 0.85 : 1}
                      className={pod.phase === "Pending" ? "pod-pulse" : ""}
                    >
                      <title>
                        {pod.name} ({pod.namespace})
                        {"\n"}Phase: {pod.phase}
                        {"\n"}Containers: {pod.readyContainers}/{pod.containers}
                        {pod.restarts > 0 ? `\nRestarts: ${pod.restarts}` : ""}
                      </title>
                    </rect>
                    {/* Restart indicator */}
                    {pod.restarts > 0 && (
                      <circle
                        cx={pod.x + layout.podSize}
                        cy={pod.y}
                        r={3}
                        fill={COLORS.podFailed}
                        stroke={COLORS.nodeBg}
                        strokeWidth={1}
                      />
                    )}
                  </g>
                ))}
              </g>
            );
          })}

          {/* Unscheduled pods */}
          {layout.unscheduled.length > 0 && (
            <g>
              <text
                x={20}
                y={layout.height - 30}
                fill={COLORS.podPending}
                fontSize={10}
                fontFamily="'Space Mono', monospace"
              >
                {layout.unscheduled.length} unscheduled pods
              </text>
            </g>
          )}
        </svg>
      </div>
    </div>
  );
}
