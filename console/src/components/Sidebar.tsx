import { NavLink } from "react-router-dom";
import {
  LayoutDashboard,
  Box,
  Network,
  HardDrive,
  Settings,
  Calendar,
  Server,
  Shield,
  PanelLeftClose,
  PanelLeft,
  Compass,
  GitBranch,
  Plus,
} from "lucide-react";
import { useUIStore } from "../store/uiStore";

const NAV_SECTIONS = [
  {
    label: "Dashboard",
    items: [
      { to: "/", icon: LayoutDashboard, label: "Overview" },
      { to: "/topology", icon: GitBranch, label: "Topology" },
    ],
  },
  {
    label: "Resources",
    items: [
      { to: "/explore", icon: Compass, label: "Explore All" },
      { to: "/workloads", icon: Box, label: "Workloads" },
      { to: "/networking", icon: Network, label: "Networking" },
      { to: "/storage", icon: HardDrive, label: "Storage" },
      { to: "/nodes", icon: Server, label: "Nodes" },
      { to: "/config", icon: Settings, label: "Config" },
      { to: "/rbac", icon: Shield, label: "RBAC" },
      { to: "/events", icon: Calendar, label: "Events" },
    ],
  },
  {
    label: "Actions",
    items: [
      { to: "/create", icon: Plus, label: "Create" },
    ],
  },
];

export function Sidebar() {
  const collapsed = useUIStore((s) => s.sidebarCollapsed);
  const toggle = useUIStore((s) => s.toggleSidebar);

  return (
    <aside
      className={`flex flex-col border-r border-surface-3 bg-surface-1 transition-all ${
        collapsed ? "w-14" : "w-52"
      }`}
    >
      {/* Logo */}
      <div className="flex h-12 items-center gap-2 border-b border-surface-3 px-3">
        <svg className="h-7 w-7 shrink-0 text-accent" viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
          <rect x="12" y="0" width="8" height="5" fill="currentColor"/>
          <rect x="12" y="27" width="8" height="5" fill="currentColor"/>
          <rect x="0" y="12" width="5" height="8" fill="currentColor"/>
          <rect x="27" y="12" width="5" height="8" fill="currentColor"/>
          <rect x="23" y="2" width="6" height="6" rx="1" fill="currentColor" opacity="0.8"/>
          <rect x="3" y="2" width="6" height="6" rx="1" fill="currentColor" opacity="0.8"/>
          <rect x="23" y="24" width="6" height="6" rx="1" fill="currentColor" opacity="0.8"/>
          <rect x="3" y="24" width="6" height="6" rx="1" fill="currentColor" opacity="0.8"/>
          <circle cx="16" cy="16" r="12" fill="currentColor"/>
          <circle cx="16" cy="16" r="4" fill="#1a1410"/>
        </svg>
        {!collapsed && (
          <span className="font-retro text-lg tracking-tight text-walle-yellow">
            rūsternetes
          </span>
        )}
      </div>

      {/* Nav */}
      <nav className="flex-1 overflow-y-auto px-2 py-3">
        {NAV_SECTIONS.map((section) => (
          <div key={section.label} className="mb-3">
            {!collapsed && (
              <div className="mb-1 px-2.5 text-[10px] font-medium uppercase tracking-widest text-[#5a4a3a]">
                {section.label}
              </div>
            )}
            <div className="space-y-0.5">
              {section.items.map(({ to, icon: Icon, label }) => (
                <NavLink
                  key={to}
                  to={to}
                  end={to === "/"}
                  className={({ isActive }) =>
                    `flex items-center gap-2.5 rounded-md px-2.5 py-1.5 text-sm transition-colors ${
                      isActive
                        ? "bg-accent/15 text-rust-light font-medium"
                        : "text-[#a89880] hover:bg-surface-3 hover:text-[#e8ddd0]"
                    }`
                  }
                >
                  <Icon size={16} className="shrink-0" />
                  {!collapsed && <span>{label}</span>}
                </NavLink>
              ))}
            </div>
          </div>
        ))}
      </nav>

      {/* Collapse toggle */}
      <button
        onClick={toggle}
        className="flex h-10 items-center justify-center border-t border-surface-3 text-[#a89880] hover:text-[#e8ddd0]"
      >
        {collapsed ? <PanelLeft size={16} /> : <PanelLeftClose size={16} />}
      </button>
    </aside>
  );
}
