import { useMemo, useState, useEffect, useRef, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { useK8sList } from "../hooks/useK8sList";
import { useQuery } from "@tanstack/react-query";
import type { Pod, Node, Service, K8sResource } from "../engine/types";
import {
  Layers,
  ZoomIn,
  ZoomOut,
  Maximize2,
  Eye,
} from "lucide-react";

// --- Theme colors ---
const C = {
  node: "#4a90b8",
  nodeBg: "#4a90b818",
  nodeBorder: "#4a90b840",
  podRunning: "#7ec850",
  podPending: "#f5c842",
  podFailed: "#c85a5a",
  podSucceeded: "#4aaaa0",
  podUnknown: "#a89880",
  service: "#e8722a",
  serviceGlow: "#e8722a30",
  ingress: "#f5c842",
  netpol: "#c85a5a",
  particle: "#e8722a",
  text: "#e8ddd0",
  textDim: "#a89880",
  textFaint: "#5a4a3a",
  grid: "#2a211815",
  nsBg: ["#4a90b808", "#7ec85008", "#e8722a08", "#f5c84208", "#4aaaa008", "#c85a5a08"],
};

function podColor(phase?: string) {
  switch (phase) {
    case "Running": return C.podRunning;
    case "Pending": return C.podPending;
    case "Failed": return C.podFailed;
    case "Succeeded": return C.podSucceeded;
    default: return C.podUnknown;
  }
}

// --- Types ---
interface PodMetrics { metadata: { name: string; namespace: string }; containers: { name: string; usage: { cpu?: string; memory?: string } }[] }
interface NodeMetrics { metadata: { name: string }; usage: { cpu?: string; memory?: string } }

interface LayoutPod {
  id: string; name: string; namespace: string; phase: string;
  x: number; y: number;
  cpuUsage: number; memUsage: number; // 0-1 intensity
  restarts: number; containers: number; readyContainers: number;
  ports: { port: number; protocol: string; name?: string }[];
  ip?: string;
}

interface LayoutNode {
  id: string; name: string; x: number; y: number; width: number; height: number;
  ready: boolean; pods: LayoutPod[];
  cpuPct: number; memPct: number;
}

interface LayoutService {
  id: string; name: string; namespace: string; type: string; clusterIP: string;
  x: number; y: number;
  ports: { port: number; targetPort: number | string; protocol: string; name?: string; nodePort?: number }[];
  targetPodIds: string[];
}

interface Particle {
  id: number; svcId: string; podId: string;
  progress: number; // 0-1
  speed: number;
  color: string;
  port?: number;
  protocol?: string;
}

// --- Helpers ---
function parseCpuMillis(q?: string): number {
  if (!q) return 0;
  if (q.endsWith("n")) return parseInt(q) / 1_000_000;
  if (q.endsWith("m")) return parseInt(q);
  return parseFloat(q) * 1000;
}
function parseMemMi(q?: string): number {
  if (!q) return 0;
  if (q.endsWith("Ki")) return parseInt(q) / 1024;
  if (q.endsWith("Mi")) return parseInt(q);
  if (q.endsWith("Gi")) return parseInt(q) * 1024;
  return parseInt(q) / (1024 * 1024);
}

const PROTOCOL_COLORS: Record<string, string> = {
  TCP: "#4a90b8",
  UDP: "#7ec850",
  SCTP: "#f5c842",
  HTTP: "#e8722a",
  HTTPS: "#4aaaa0",
  gRPC: "#c85a5a",
};

function protocolColor(protocol?: string): string {
  return PROTOCOL_COLORS[protocol ?? "TCP"] ?? C.particle;
}

// --- Components ---

/** Port badge showing protocol and port number. */
function PortBadge({ port, protocol, targetPort }: { port: number; protocol: string; targetPort?: number | string }) {
  const color = protocolColor(protocol);
  return (
    <g>
      <rect rx={3} fill={`${color}20`} stroke={color} strokeWidth={0.5} />
      <text fill={color} fontSize={7} fontFamily="'Space Mono', monospace">
        {port}{targetPort && targetPort !== port ? `→${targetPort}` : ""}/{protocol}
      </text>
    </g>
  );
}

export function TopologyView() {
  const navigate = useNavigate();
  const svgRef = useRef<SVGSVGElement>(null);
  const [zoom, setZoom] = useState(1);
  const [selectedPod, setSelectedPod] = useState<string | null>(null);
  const [selectedService, setSelectedService] = useState<string | null>(null);
  const [showNamespaces, setShowNamespaces] = useState(true);
  const [showProtocols, setShowProtocols] = useState(true);
  const [particles, setParticles] = useState<Particle[]>([]);
  const particleIdRef = useRef(0);
  const animRef = useRef<number>(0);

  // --- Data fetching ---
  const { data: nodesData } = useK8sList<Node>("", "v1", "nodes", undefined, { refetchInterval: 15_000 });
  const { data: podsData } = useK8sList<Pod>("", "v1", "pods", undefined, { refetchInterval: 10_000 });
  const { data: servicesData } = useK8sList<Service>("", "v1", "services", undefined, { refetchInterval: 30_000 });
  const { data: ingressData } = useK8sList<K8sResource>("networking.k8s.io", "v1", "ingresses", undefined, { refetchInterval: 60_000 });
  const { data: netpolData } = useK8sList<K8sResource>("networking.k8s.io", "v1", "networkpolicies", undefined, { refetchInterval: 60_000 });

  const { data: podMetricsData } = useQuery<{ items: PodMetrics[] }>({
    queryKey: ["k8s", "pod-metrics"],
    queryFn: async () => {
      const h: Record<string, string> = { Accept: "application/json" };
      const t = sessionStorage.getItem("rusternetes-token");
      if (t) h["Authorization"] = `Bearer ${t}`;
      const r = await fetch("/apis/metrics.k8s.io/v1beta1/pods", { headers: h });
      return r.ok ? r.json() : { items: [] };
    },
    refetchInterval: 30_000,
  });

  const { data: nodeMetricsData } = useQuery<{ items: NodeMetrics[] }>({
    queryKey: ["k8s", "node-metrics-topo"],
    queryFn: async () => {
      const h: Record<string, string> = { Accept: "application/json" };
      const t = sessionStorage.getItem("rusternetes-token");
      if (t) h["Authorization"] = `Bearer ${t}`;
      const r = await fetch("/apis/metrics.k8s.io/v1beta1/nodes", { headers: h });
      return r.ok ? r.json() : { items: [] };
    },
    refetchInterval: 15_000,
  });

  const nodes = nodesData?.items ?? [];
  const pods = podsData?.items ?? [];
  const services = servicesData?.items ?? [];
  const ingresses = ingressData?.items ?? [];
  const netpols = netpolData?.items ?? [];

  // --- Build metrics maps ---
  const podMetricsMap = useMemo(() => {
    const m = new Map<string, { cpu: number; mem: number }>();
    for (const pm of podMetricsData?.items ?? []) {
      let cpu = 0, mem = 0;
      for (const c of pm.containers) {
        cpu += parseCpuMillis(c.usage?.cpu);
        mem += parseMemMi(c.usage?.memory);
      }
      m.set(`${pm.metadata.namespace}/${pm.metadata.name}`, { cpu, mem });
    }
    return m;
  }, [podMetricsData]);

  const nodeMetricsMap = useMemo(() => {
    const m = new Map<string, { cpu: number; mem: number }>();
    for (const nm of nodeMetricsData?.items ?? []) {
      m.set(nm.metadata.name, {
        cpu: parseCpuMillis(nm.usage?.cpu),
        mem: parseMemMi(nm.usage?.memory),
      });
    }
    return m;
  }, [nodeMetricsData]);

  // --- Layout ---
  const layout = useMemo(() => {
    const NODE_W = 300;
    const NODE_PAD = 40;
    const POD_SZ = 16;
    const POD_GAP = 4;
    const PODS_PER_ROW = 12;
    const NODE_HDR = 60;
    const SVC_AREA_H = 100;
    const INGRESS_H = 50;

    // Collect namespaces
    const namespaces = new Set<string>();
    pods.forEach((p) => namespaces.add(p.metadata.namespace ?? "default"));
    services.forEach((s) => namespaces.add(s.metadata.namespace ?? "default"));
    const nsColors = new Map<string, string>();
    let nsIdx = 0;
    for (const ns of namespaces) {
      nsColors.set(ns, C.nsBg[nsIdx % C.nsBg.length]!);
      nsIdx++;
    }

    // Layout nodes
    const layoutNodes: LayoutNode[] = nodes.map((n, i) => {
      const nodePods = pods.filter((p) => p.spec.nodeName === n.metadata.name);
      const podRows = Math.ceil(nodePods.length / PODS_PER_ROW) || 1;
      const nodeH = NODE_HDR + podRows * (POD_SZ + POD_GAP) + NODE_PAD;
      const x = NODE_PAD + i * (NODE_W + NODE_PAD);
      const y = INGRESS_H + SVC_AREA_H + 20;
      const ready = n.status?.conditions?.some((c) => c.type === "Ready" && c.status === "True") ?? false;

      const cpuCap = parseCpuMillis(n.status?.allocatable?.["cpu"] ?? n.status?.capacity?.["cpu"]);
      const memCap = parseMemMi(n.status?.allocatable?.["memory"] ?? n.status?.capacity?.["memory"]);
      const nm = nodeMetricsMap.get(n.metadata.name);
      const cpuPct = cpuCap > 0 && nm ? (nm.cpu / cpuCap) * 100 : 0;
      const memPct = memCap > 0 && nm ? (nm.mem / memCap) * 100 : 0;

      const layoutPods: LayoutPod[] = nodePods.map((p, pi) => {
        const row = Math.floor(pi / PODS_PER_ROW);
        const col = pi % PODS_PER_ROW;
        const pm = podMetricsMap.get(`${p.metadata.namespace}/${p.metadata.name}`);
        const maxCpu = 500; // normalize to 500m for intensity
        const maxMem = 512; // normalize to 512Mi
        return {
          id: p.metadata.uid ?? p.metadata.name,
          name: p.metadata.name,
          namespace: p.metadata.namespace ?? "default",
          phase: p.status?.phase ?? "Unknown",
          x: x + 15 + col * (POD_SZ + POD_GAP),
          y: y + NODE_HDR + row * (POD_SZ + POD_GAP),
          cpuUsage: pm ? Math.min(pm.cpu / maxCpu, 1) : 0,
          memUsage: pm ? Math.min(pm.mem / maxMem, 1) : 0,
          restarts: p.status?.containerStatuses?.reduce((s, c) => s + c.restartCount, 0) ?? 0,
          containers: p.spec.containers.length,
          readyContainers: p.status?.containerStatuses?.filter((c) => c.ready).length ?? 0,
          ports: p.spec.containers.flatMap((c) =>
            (c.ports ?? []).map((pt) => ({ port: pt.containerPort, protocol: pt.protocol ?? "TCP", name: pt.name }))
          ),
          ip: (p.status as Record<string, unknown> | undefined)?.podIP as string | undefined,
        };
      });

      return { id: n.metadata.uid ?? n.metadata.name, name: n.metadata.name, x, y, width: NODE_W, height: nodeH, ready, pods: layoutPods, cpuPct, memPct };
    });

    // Layout services
    const layoutServices: LayoutService[] = services.map((svc, i) => {
      const selector = svc.spec.selector ?? {};
      const targetPodIds = pods
        .filter((p) => {
          const labels = p.metadata.labels ?? {};
          return Object.keys(selector).length > 0 && Object.entries(selector).every(([k, v]) => labels[k] === v);
        })
        .map((p) => p.metadata.uid ?? p.metadata.name);

      return {
        id: svc.metadata.uid ?? svc.metadata.name,
        name: svc.metadata.name,
        namespace: svc.metadata.namespace ?? "default",
        type: svc.spec.type ?? "ClusterIP",
        clusterIP: svc.spec.clusterIP ?? "",
        x: 30 + i * 160,
        y: INGRESS_H + 30,
        ports: (svc.spec.ports ?? []).map((p) => ({
          port: p.port,
          targetPort: p.targetPort ?? p.port,
          protocol: p.protocol ?? "TCP",
          name: p.name,
          nodePort: p.nodePort,
        })),
        targetPodIds,
      };
    });

    const maxX = Math.max(
      layoutNodes.reduce((m, n) => Math.max(m, n.x + n.width), 500),
      layoutServices.length * 160 + 60,
    );
    const maxY = layoutNodes.reduce((m, n) => Math.max(m, n.y + n.height), 400) + 40;

    return {
      nodes: layoutNodes,
      services: layoutServices,
      namespaces: nsColors,
      ingressCount: ingresses.length,
      netpolCount: netpols.length,
      width: maxX + 40,
      height: maxY,
      podSize: POD_SZ,
      ingressH: INGRESS_H,
    };
  }, [nodes, pods, services, ingresses, netpols, podMetricsMap, nodeMetricsMap]);

  // --- Particle animation ---
  const spawnParticles = useCallback(() => {
    const newParticles: Particle[] = [];
    for (const svc of layout.services) {
      if (svc.targetPodIds.length === 0) continue;
      // Spawn 1-2 particles per service per cycle
      const count = Math.min(svc.targetPodIds.length, 2);
      for (let i = 0; i < count; i++) {
        const podId = svc.targetPodIds[Math.floor(Math.random() * svc.targetPodIds.length)]!;
        const port = svc.ports[0];
        newParticles.push({
          id: particleIdRef.current++,
          svcId: svc.id,
          podId,
          progress: 0,
          speed: 0.008 + Math.random() * 0.012,
          color: protocolColor(port?.protocol),
          port: port?.port,
          protocol: port?.protocol,
        });
      }
    }
    return newParticles;
  }, [layout.services]);

  useEffect(() => {
    let lastSpawn = 0;
    const animate = (time: number) => {
      // Spawn new particles every 2 seconds
      if (time - lastSpawn > 2000) {
        setParticles((prev) => [...prev.filter((p) => p.progress < 1), ...spawnParticles()]);
        lastSpawn = time;
      }
      // Advance particles
      setParticles((prev) =>
        prev
          .map((p) => ({ ...p, progress: p.progress + p.speed }))
          .filter((p) => p.progress < 1.2)
      );
      animRef.current = requestAnimationFrame(animate);
    };
    animRef.current = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(animRef.current);
  }, [spawnParticles]);

  // --- Helpers for rendering ---
  const findPodPos = (podId: string) => {
    for (const n of layout.nodes) {
      const p = n.pods.find((p) => p.id === podId);
      if (p) return { x: p.x + layout.podSize / 2, y: p.y + layout.podSize / 2 };
    }
    return null;
  };

  const selectedPodData = selectedPod
    ? layout.nodes.flatMap((n) => n.pods).find((p) => p.id === selectedPod)
    : null;

  const selectedSvcData = selectedService
    ? layout.services.find((s) => s.id === selectedService)
    : null;

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="font-retro text-2xl text-walle-yellow">Cluster Topology</h1>
          <p className="text-sm text-[#a89880]">
            {nodes.length} nodes &middot; {pods.length} pods &middot; {services.length} services
            &middot; {ingresses.length} ingresses &middot; {netpols.length} network policies
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setShowNamespaces(!showNamespaces)}
            className={`rounded-md px-2 py-1 text-xs ${showNamespaces ? "bg-accent/15 text-rust-light" : "text-[#a89880] hover:text-[#e8ddd0]"}`}
          >
            <Layers size={12} className="mr-1 inline" />
            Namespaces
          </button>
          <button
            onClick={() => setShowProtocols(!showProtocols)}
            className={`rounded-md px-2 py-1 text-xs ${showProtocols ? "bg-accent/15 text-rust-light" : "text-[#a89880] hover:text-[#e8ddd0]"}`}
          >
            Protocols
          </button>
          <div className="flex items-center gap-1 rounded-md border border-surface-3 bg-surface-2 px-1">
            <button onClick={() => setZoom((z) => Math.max(0.3, z - 0.15))} className="p-1 text-[#a89880] hover:text-[#e8ddd0]">
              <ZoomOut size={14} />
            </button>
            <span className="min-w-[3ch] text-center text-[10px] text-[#a89880]">{Math.round(zoom * 100)}%</span>
            <button onClick={() => setZoom((z) => Math.min(2, z + 0.15))} className="p-1 text-[#a89880] hover:text-[#e8ddd0]">
              <ZoomIn size={14} />
            </button>
            <button onClick={() => setZoom(1)} className="p-1 text-[#a89880] hover:text-[#e8ddd0]">
              <Maximize2 size={14} />
            </button>
          </div>
        </div>
      </div>

      {/* Legend */}
      <div className="flex flex-wrap items-center gap-4 text-[10px] text-[#a89880]">
        <span className="flex items-center gap-1"><span className="h-2.5 w-2.5 rounded-sm" style={{ backgroundColor: C.podRunning }} /> Running</span>
        <span className="flex items-center gap-1"><span className="h-2.5 w-2.5 rounded-sm" style={{ backgroundColor: C.podPending }} /> Pending</span>
        <span className="flex items-center gap-1"><span className="h-2.5 w-2.5 rounded-sm" style={{ backgroundColor: C.podFailed }} /> Failed</span>
        <span className="flex items-center gap-1"><span className="h-2.5 w-6 rounded-sm" style={{ backgroundColor: C.service }} /> Service</span>
        <span className="text-[#5a4a3a]">|</span>
        {Object.entries(PROTOCOL_COLORS).map(([proto, color]) => (
          <span key={proto} className="flex items-center gap-1">
            <span className="h-1.5 w-4 rounded-full" style={{ backgroundColor: color }} />
            {proto}
          </span>
        ))}
        <span className="text-[#5a4a3a]">|</span>
        <span>Brightness = CPU usage</span>
      </div>

      {/* SVG Canvas */}
      <div className="overflow-auto rounded-lg border border-surface-3 bg-surface-0" style={{ maxHeight: "70vh" }}>
        <svg
          ref={svgRef}
          width={layout.width * zoom}
          height={layout.height * zoom}
          viewBox={`0 0 ${layout.width} ${layout.height}`}
          className="min-w-full"
        >
          <defs>
            <pattern id="topo-grid" width="20" height="20" patternUnits="userSpaceOnUse">
              <path d="M 20 0 L 0 0 0 20" fill="none" stroke={C.grid} strokeWidth="0.5" />
            </pattern>
            <filter id="glow-svc"><feGaussianBlur stdDeviation="4" result="g" /><feMerge><feMergeNode in="g" /><feMergeNode in="SourceGraphic" /></feMerge></filter>
            <filter id="glow-pod"><feGaussianBlur stdDeviation="2" result="g" /><feMerge><feMergeNode in="g" /><feMergeNode in="SourceGraphic" /></feMerge></filter>
            <style>{`
              @keyframes dash-flow { to { stroke-dashoffset: -24; } }
              .flow-line { animation: dash-flow 1.5s linear infinite; }
              @keyframes pulse-slow { 0%,100% { opacity: 0.5; } 50% { opacity: 1; } }
              .pulse-slow { animation: pulse-slow 3s ease-in-out infinite; }
              @keyframes ripple { 0% { r: 0; opacity: 0.6; } 100% { r: 30; opacity: 0; } }
            `}</style>
          </defs>

          <rect width="100%" height="100%" fill="url(#topo-grid)" />

          {/* Namespace background zones */}
          {showNamespaces && layout.nodes.map((node) => {
            const nsGroups = new Map<string, { minX: number; minY: number; maxX: number; maxY: number }>();
            for (const pod of node.pods) {
              const ns = pod.namespace;
              const g = nsGroups.get(ns) ?? { minX: pod.x, minY: pod.y, maxX: pod.x + layout.podSize, maxY: pod.y + layout.podSize };
              g.minX = Math.min(g.minX, pod.x);
              g.minY = Math.min(g.minY, pod.y);
              g.maxX = Math.max(g.maxX, pod.x + layout.podSize);
              g.maxY = Math.max(g.maxY, pod.y + layout.podSize);
              nsGroups.set(ns, g);
            }
            return [...nsGroups.entries()].map(([ns, bounds]) => (
              <g key={`${node.id}-ns-${ns}`}>
                <rect
                  x={bounds.minX - 4} y={bounds.minY - 4}
                  width={bounds.maxX - bounds.minX + 8} height={bounds.maxY - bounds.minY + 8}
                  rx={4} fill={layout.namespaces.get(ns) ?? "#ffffff05"}
                  stroke={C.textFaint} strokeWidth={0.5} strokeDasharray="2 2"
                />
                <text x={bounds.minX - 2} y={bounds.minY - 6} fill={C.textFaint} fontSize={7} fontFamily="'Space Mono', monospace">
                  {ns}
                </text>
              </g>
            ));
          })}

          {/* Service → Pod connection lines */}
          {layout.services.map((svc) =>
            svc.targetPodIds.map((podId) => {
              const pos = findPodPos(podId);
              if (!pos) return null;
              const highlight = selectedService === svc.id || selectedPod === podId;
              return (
                <line
                  key={`${svc.id}-${podId}`}
                  x1={svc.x + 60} y1={svc.y + 20}
                  x2={pos.x} y2={pos.y}
                  stroke={highlight ? C.service : `${C.service}25`}
                  strokeWidth={highlight ? 1.5 : 0.8}
                  strokeDasharray="4 4"
                  className="flow-line"
                />
              );
            }),
          )}

          {/* Animated particles flowing along connections */}
          {particles.map((p) => {
            const svc = layout.services.find((s) => s.id === p.svcId);
            const podPos = findPodPos(p.podId);
            if (!svc || !podPos) return null;
            const x1 = svc.x + 60, y1 = svc.y + 20;
            const x = x1 + (podPos.x - x1) * p.progress;
            const y = y1 + (podPos.y - y1) * p.progress;
            const opacity = p.progress > 0.9 ? (1 - p.progress) * 10 : Math.min(p.progress * 5, 1);
            return (
              <circle
                key={p.id}
                cx={x} cy={y} r={2}
                fill={p.color}
                opacity={opacity * 0.8}
                filter="url(#glow-pod)"
              />
            );
          })}

          {/* Services */}
          {layout.services.map((svc) => {
            const isSelected = selectedService === svc.id;
            return (
              <g
                key={svc.id}
                className="cursor-pointer"
                onClick={() => setSelectedService(isSelected ? null : svc.id)}
              >
                <rect
                  x={svc.x} y={svc.y} width={140} height={showProtocols ? 50 : 40} rx={6}
                  fill={isSelected ? `${C.service}25` : C.serviceGlow}
                  stroke={C.service} strokeWidth={isSelected ? 2 : 1}
                  filter="url(#glow-svc)"
                />
                <text x={svc.x + 70} y={svc.y + 14} textAnchor="middle" fill={C.service} fontSize={10} fontWeight="bold" fontFamily="'Space Mono', monospace">
                  {svc.name.length > 16 ? svc.name.slice(0, 16) + "..." : svc.name}
                </text>
                <text x={svc.x + 70} y={svc.y + 26} textAnchor="middle" fill={C.textDim} fontSize={8} fontFamily="'Space Mono', monospace">
                  {svc.clusterIP} &middot; {svc.type}
                </text>
                {/* Port badges */}
                {showProtocols && svc.ports.slice(0, 3).map((port, pi) => {
                  const pc = protocolColor(port.protocol);
                  return (
                    <g key={pi}>
                      <rect
                        x={svc.x + 4 + pi * 46} y={svc.y + 32}
                        width={44} height={14} rx={3}
                        fill={`${pc}20`} stroke={pc} strokeWidth={0.5}
                      />
                      <text
                        x={svc.x + 26 + pi * 46} y={svc.y + 42}
                        textAnchor="middle" fill={pc} fontSize={7} fontFamily="'Space Mono', monospace"
                      >
                        {port.port}→{port.targetPort}/{port.protocol}
                      </text>
                    </g>
                  );
                })}
                {/* Endpoint count */}
                <text x={svc.x + 140 - 4} y={svc.y + 14} textAnchor="end" fill={C.textFaint} fontSize={8}>
                  {svc.targetPodIds.length}ep
                </text>
              </g>
            );
          })}

          {/* Nodes */}
          {layout.nodes.map((node) => (
            <g key={node.id}>
              {/* Node background */}
              <rect
                x={node.x} y={node.y} width={node.width} height={node.height} rx={8}
                fill={C.nodeBg} stroke={node.ready ? C.nodeBorder : C.podFailed}
                strokeWidth={1.5} strokeDasharray={node.ready ? "none" : "4 4"}
              />
              {/* Node header */}
              <text x={node.x + 12} y={node.y + 18} fill={C.text} fontSize={11} fontWeight="bold" fontFamily="'Space Mono', monospace">
                {node.name}
              </text>
              <text x={node.x + 12} y={node.y + 32} fill={C.textDim} fontSize={8} fontFamily="'Space Mono', monospace">
                {node.pods.length} pods
              </text>
              {/* CPU/Mem utilization mini-bars */}
              <rect x={node.x + node.width - 80} y={node.y + 10} width={60} height={4} rx={2} fill="#3d3024" />
              <rect x={node.x + node.width - 80} y={node.y + 10} width={Math.max(node.cpuPct * 0.6, 0)} height={4} rx={2}
                fill={node.cpuPct > 85 ? C.podFailed : node.cpuPct > 60 ? C.podPending : C.node}
              />
              <text x={node.x + node.width - 82} y={node.y + 14} textAnchor="end" fill={C.textFaint} fontSize={7}>CPU</text>

              <rect x={node.x + node.width - 80} y={node.y + 18} width={60} height={4} rx={2} fill="#3d3024" />
              <rect x={node.x + node.width - 80} y={node.y + 18} width={Math.max(node.memPct * 0.6, 0)} height={4} rx={2}
                fill={node.memPct > 85 ? C.podFailed : node.memPct > 60 ? C.podPending : C.podRunning}
              />
              <text x={node.x + node.width - 82} y={node.y + 22} textAnchor="end" fill={C.textFaint} fontSize={7}>MEM</text>

              {/* Ready indicator */}
              <circle cx={node.x + node.width - 12} y={node.y + 32} cy={node.y + 32} r={4}
                fill={node.ready ? C.podRunning : C.podFailed}
                className={node.ready ? "" : "pulse-slow"}
              />

              {/* Pods — brightness based on CPU usage */}
              {node.pods.map((pod) => {
                const isSelected = selectedPod === pod.id;
                const brightness = 0.4 + pod.cpuUsage * 0.6;
                const baseColor = podColor(pod.phase);
                return (
                  <g
                    key={pod.id}
                    className="cursor-pointer"
                    onClick={(e) => {
                      e.stopPropagation();
                      setSelectedPod(isSelected ? null : pod.id);
                      setSelectedService(null);
                    }}
                  >
                    <rect
                      x={pod.x} y={pod.y}
                      width={layout.podSize} height={layout.podSize}
                      rx={2}
                      fill={baseColor}
                      opacity={brightness}
                      stroke={isSelected ? C.text : "none"}
                      strokeWidth={isSelected ? 1.5 : 0}
                      filter={pod.cpuUsage > 0.7 ? "url(#glow-pod)" : undefined}
                    >
                      <title>
                        {pod.name} ({pod.namespace})
                        {"\n"}Phase: {pod.phase}
                        {"\n"}CPU: {Math.round(pod.cpuUsage * 100)}%
                        {"\n"}Memory: {Math.round(pod.memUsage * 100)}%
                        {"\n"}Containers: {pod.readyContainers}/{pod.containers}
                        {pod.restarts > 0 ? `\nRestarts: ${pod.restarts}` : ""}
                        {pod.ip ? `\nIP: ${pod.ip}` : ""}
                        {pod.ports.length > 0 ? `\nPorts: ${pod.ports.map((p) => `${p.port}/${p.protocol}`).join(", ")}` : ""}
                      </title>
                    </rect>
                    {/* Restart indicator */}
                    {pod.restarts > 0 && (
                      <circle cx={pod.x + layout.podSize} cy={pod.y} r={3}
                        fill={C.podFailed} stroke="#2a2118" strokeWidth={1}
                      />
                    )}
                  </g>
                );
              })}
            </g>
          ))}
        </svg>
      </div>

      {/* Detail panel */}
      {(selectedPodData || selectedSvcData) && (
        <div className="rounded-lg border border-surface-3 bg-surface-1 p-4">
          {selectedPodData && (
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <div>
                  <span className="font-mono text-sm text-[#e8ddd0]">{selectedPodData.name}</span>
                  <span className="ml-2 text-xs text-[#a89880]">{selectedPodData.namespace}</span>
                </div>
                <button
                  onClick={() =>
                    navigate(`/resources/${encodeURIComponent("core/v1/pods")}/${selectedPodData.namespace}/${selectedPodData.name}`)
                  }
                  className="flex items-center gap-1 rounded-md px-2 py-1 text-xs text-[#a89880] hover:bg-surface-3 hover:text-accent"
                >
                  <Eye size={12} /> Detail
                </button>
              </div>
              <div className="grid grid-cols-4 gap-3 text-xs">
                <div><span className="text-[#a89880]">Phase</span><div className="font-mono text-[#e8ddd0]">{selectedPodData.phase}</div></div>
                <div><span className="text-[#a89880]">CPU</span><div className="font-mono text-[#e8ddd0]">{Math.round(selectedPodData.cpuUsage * 500)}m</div></div>
                <div><span className="text-[#a89880]">Memory</span><div className="font-mono text-[#e8ddd0]">{Math.round(selectedPodData.memUsage * 512)}Mi</div></div>
                <div><span className="text-[#a89880]">Restarts</span><div className="font-mono text-[#e8ddd0]">{selectedPodData.restarts}</div></div>
              </div>
              {selectedPodData.ip && (
                <div className="text-xs"><span className="text-[#a89880]">Pod IP: </span><span className="font-mono text-container-teal">{selectedPodData.ip}</span></div>
              )}
              {selectedPodData.ports.length > 0 && (
                <div className="flex flex-wrap gap-1.5">
                  {selectedPodData.ports.map((p, i) => (
                    <span key={i} className="rounded-full px-2 py-0.5 text-[10px]" style={{ backgroundColor: `${protocolColor(p.protocol)}15`, color: protocolColor(p.protocol) }}>
                      {p.port}/{p.protocol}{p.name ? ` (${p.name})` : ""}
                    </span>
                  ))}
                </div>
              )}
            </div>
          )}
          {selectedSvcData && (
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <div>
                  <span className="font-mono text-sm text-[#e8ddd0]">{selectedSvcData.name}</span>
                  <span className="ml-2 text-xs text-[#a89880]">{selectedSvcData.namespace}</span>
                </div>
                <button
                  onClick={() =>
                    navigate(`/resources/${encodeURIComponent("core/v1/services")}/${selectedSvcData.namespace}/${selectedSvcData.name}`)
                  }
                  className="flex items-center gap-1 rounded-md px-2 py-1 text-xs text-[#a89880] hover:bg-surface-3 hover:text-accent"
                >
                  <Eye size={12} /> Detail
                </button>
              </div>
              <div className="grid grid-cols-3 gap-3 text-xs">
                <div><span className="text-[#a89880]">Type</span><div className="font-mono text-[#e8ddd0]">{selectedSvcData.type}</div></div>
                <div><span className="text-[#a89880]">Cluster IP</span><div className="font-mono text-container-teal">{selectedSvcData.clusterIP}</div></div>
                <div><span className="text-[#a89880]">Endpoints</span><div className="font-mono text-walle-yellow">{selectedSvcData.targetPodIds.length}</div></div>
              </div>
              {selectedSvcData.ports.length > 0 && (
                <div>
                  <span className="text-xs text-[#a89880]">Port Mappings:</span>
                  <div className="mt-1 flex flex-wrap gap-1.5">
                    {selectedSvcData.ports.map((p, i) => (
                      <span key={i} className="rounded-full px-2 py-0.5 text-[10px]" style={{ backgroundColor: `${protocolColor(p.protocol)}15`, color: protocolColor(p.protocol) }}>
                        {p.port}→{p.targetPort}/{p.protocol}
                        {p.nodePort ? ` (nodePort:${p.nodePort})` : ""}
                        {p.name ? ` [${p.name}]` : ""}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
