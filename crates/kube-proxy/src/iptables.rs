use anyhow::{Context, Result};
use std::process::Command;
use tracing::{debug, error, info, warn};

/// Detect the correct iptables command to use.
/// Docker Desktop uses nftables backend (`iptables`), while Podman uses `iptables-legacy`.
/// We must match the backend that the container runtime uses, otherwise DNAT rules
/// won't apply to container traffic.
fn detect_iptables_cmd() -> &'static str {
    // Try `iptables` first (nftables backend, used by Docker Desktop)
    if let Ok(output) = Command::new("/usr/sbin/iptables")
        .args(["-t", "nat", "-L", "PREROUTING", "-n"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // If the DOCKER chain jump exists in iptables (nftables), Docker is using this backend
        if stdout.contains("DOCKER") {
            info!("Detected Docker nftables backend, using /usr/sbin/iptables");
            return "/usr/sbin/iptables";
        }
    }
    // Fall back to iptables-legacy (Podman, older systems)
    info!("Using /usr/sbin/iptables-legacy (Podman/legacy backend)");
    "/usr/sbin/iptables-legacy"
}

/// IptablesManager handles iptables rule programming for service networking
pub struct IptablesManager {
    /// Chain names we create
    services_chain: String,
    nodeports_chain: String,
    /// The iptables command to use (detected at init)
    iptables_cmd: String,
    /// Track per-endpoint chains (KUBE-SEP-*) so we can clean them up on flush
    sep_chains: std::sync::Mutex<Vec<String>>,
    /// Whether the xt_recent kernel module is available
    recent_available: bool,
}

impl IptablesManager {
    pub fn new() -> Self {
        let iptables_cmd = detect_iptables_cmd().to_string();
        // Probe whether the xt_recent module is available by trying a dummy check rule.
        // If the module is missing, stderr contains "Couldn't load" or "No such file".
        // If the rule just doesn't exist, stderr contains "does a matching rule exist"
        // which means the module loaded fine.
        let recent_available = Command::new(&iptables_cmd)
            .args([
                "-t",
                "nat",
                "-C",
                "OUTPUT",
                "-m",
                "recent",
                "--name",
                "__probe__",
                "--rcheck",
                "-j",
                "RETURN",
            ])
            .output()
            .map(|o| {
                let stderr = String::from_utf8_lossy(&o.stderr);
                !stderr.contains("Couldn't load") && !stderr.contains("No such file")
            })
            .unwrap_or(false);
        if recent_available {
            info!("xt_recent module is available, session affinity will use it");
        } else {
            warn!(
                "xt_recent module is NOT available, session affinity will fall back to direct DNAT"
            );
        }
        Self {
            services_chain: "RUSTERNETES-SERVICES".to_string(),
            nodeports_chain: "RUSTERNETES-NODEPORTS".to_string(),
            iptables_cmd,
            sep_chains: std::sync::Mutex::new(Vec::new()),
            recent_available,
        }
    }

    /// Initialize iptables chains and jump rules
    pub fn initialize(&self) -> Result<()> {
        info!("Initializing iptables chains for kube-proxy");

        // Create our custom chains if they don't exist
        self.ensure_chain("nat", &self.services_chain)?;
        self.ensure_chain("nat", &self.nodeports_chain)?;

        // Add jump rules from PREROUTING and OUTPUT to our chains
        self.ensure_jump_rule(
            "nat",
            "PREROUTING",
            &self.services_chain,
            "kubernetes service portals",
        )?;

        self.ensure_jump_rule(
            "nat",
            "OUTPUT",
            &self.services_chain,
            "kubernetes service portals",
        )?;

        // Add jump rules for NodePort services (both PREROUTING and OUTPUT)
        self.ensure_jump_rule(
            "nat",
            "PREROUTING",
            &self.nodeports_chain,
            "kubernetes service node ports",
        )?;

        self.ensure_jump_rule(
            "nat",
            "OUTPUT",
            &self.nodeports_chain,
            "kubernetes service node ports",
        )?;

        // Add MASQUERADE rule for hairpin NAT (container→ClusterIP→container on same bridge).
        // Without this, DNATed traffic within the Docker bridge doesn't have its source
        // rewritten, so the return path bypasses NAT and the connection fails.
        let masq_check = Command::new(&self.iptables_cmd)
            .args([
                "-t",
                "nat",
                "-C",
                "POSTROUTING",
                "-m",
                "comment",
                "--comment",
                "rusternetes service hairpin masquerade",
                "-s",
                "172.18.0.0/16",
                "-d",
                "172.18.0.0/16",
                "-j",
                "MASQUERADE",
            ])
            .output();
        if masq_check.map_or(true, |o| !o.status.success()) {
            let output = Command::new(&self.iptables_cmd)
                .args([
                    "-t",
                    "nat",
                    "-A",
                    "POSTROUTING",
                    "-m",
                    "comment",
                    "--comment",
                    "rusternetes service hairpin masquerade",
                    "-s",
                    "172.18.0.0/16",
                    "-d",
                    "172.18.0.0/16",
                    "-j",
                    "MASQUERADE",
                ])
                .output()
                .context("Failed to add hairpin MASQUERADE rule")?;
            if output.status.success() {
                info!("Added hairpin MASQUERADE rule for service traffic within Docker network");
            } else {
                warn!(
                    "Failed to add hairpin MASQUERADE: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        // Add MASQUERADE for NodePort traffic from local sources.
        // When a process on the node itself connects to a NodePort, the source IP
        // is local. Without MASQUERADE the backend pod replies directly, bypassing
        // conntrack, and the connection breaks (asymmetric routing).
        let nodeport_masq_check = Command::new(&self.iptables_cmd)
            .args([
                "-t",
                "nat",
                "-C",
                "POSTROUTING",
                "-m",
                "comment",
                "--comment",
                "rusternetes nodeport masquerade",
                "-m",
                "addrtype",
                "--src-type",
                "LOCAL",
                "-j",
                "MASQUERADE",
            ])
            .output();
        if nodeport_masq_check.map_or(true, |o| !o.status.success()) {
            let output = Command::new(&self.iptables_cmd)
                .args([
                    "-t",
                    "nat",
                    "-A",
                    "POSTROUTING",
                    "-m",
                    "comment",
                    "--comment",
                    "rusternetes nodeport masquerade",
                    "-m",
                    "addrtype",
                    "--src-type",
                    "LOCAL",
                    "-j",
                    "MASQUERADE",
                ])
                .output()
                .context("Failed to add NodePort MASQUERADE rule")?;
            if output.status.success() {
                info!("Added MASQUERADE rule for NodePort traffic (local source)");
            } else {
                warn!(
                    "Failed to add NodePort MASQUERADE: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        // Add general MASQUERADE for all DNAT'd traffic.
        // This covers both ClusterIP and NodePort: when traffic is DNATed to a pod
        // the source must be rewritten so the reply comes back through the node for
        // proper connection tracking.
        let dnat_masq_check = Command::new(&self.iptables_cmd)
            .args([
                "-t",
                "nat",
                "-C",
                "POSTROUTING",
                "-m",
                "comment",
                "--comment",
                "rusternetes DNAT traffic masquerade",
                "-m",
                "conntrack",
                "--ctstate",
                "DNAT",
                "-j",
                "MASQUERADE",
            ])
            .output();
        if dnat_masq_check.map_or(true, |o| !o.status.success()) {
            let output = Command::new(&self.iptables_cmd)
                .args([
                    "-t",
                    "nat",
                    "-A",
                    "POSTROUTING",
                    "-m",
                    "comment",
                    "--comment",
                    "rusternetes DNAT traffic masquerade",
                    "-m",
                    "conntrack",
                    "--ctstate",
                    "DNAT",
                    "-j",
                    "MASQUERADE",
                ])
                .output()
                .context("Failed to add DNAT MASQUERADE rule")?;
            if output.status.success() {
                info!("Added MASQUERADE rule for all DNAT'd traffic");
            } else {
                warn!(
                    "Failed to add DNAT MASQUERADE: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        // Ensure the service CIDR (10.96.0.0/12) is routable.
        // Without a route, packets to ClusterIPs are dropped before reaching
        // iptables PREROUTING/DNAT. We add a route pointing to the Docker bridge
        // so the kernel accepts the packets and lets iptables DNAT them.
        let route_check = Command::new("ip")
            .args(["route", "show", "10.96.0.0/12"])
            .output();
        if route_check.map_or(true, |o| o.stdout.is_empty()) {
            // Find the Docker bridge interface
            let bridge_iface = Command::new("ip")
                .args(["route", "show", "172.18.0.0/16"])
                .output()
                .ok()
                .and_then(|o| {
                    String::from_utf8_lossy(&o.stdout)
                        .split_whitespace()
                        .nth(2) // "dev <iface>"
                        .map(|s| s.to_string())
                });
            if let Some(iface) = bridge_iface {
                let output = Command::new("ip")
                    .args(["route", "add", "10.96.0.0/12", "dev", &iface])
                    .output();
                match output {
                    Ok(o) if o.status.success() => {
                        info!("Added route for service CIDR 10.96.0.0/12 via {}", iface);
                    }
                    Ok(o) => {
                        let stderr = String::from_utf8_lossy(&o.stderr);
                        if !stderr.contains("File exists") {
                            warn!("Failed to add service CIDR route: {}", stderr);
                        }
                    }
                    Err(e) => warn!("Failed to add service CIDR route: {}", e),
                }
            }
        }

        info!("Iptables chains initialized successfully");
        Ok(())
    }

    /// Ensure an iptables chain exists
    fn ensure_chain(&self, table: &str, chain: &str) -> Result<()> {
        // Try to create the chain, ignore error if it already exists
        let output = Command::new(&self.iptables_cmd)
            .args(["-t", table, "-N", chain])
            .output()
            .context("Failed to create iptables chain")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Chain already exists is not an error
            if !stderr.contains("Chain already exists") {
                warn!("Chain creation warning for {}: {}", chain, stderr);
            }
        }

        debug!("Ensured chain {} exists in table {}", chain, table);
        Ok(())
    }

    /// Ensure a jump rule exists
    fn ensure_jump_rule(
        &self,
        table: &str,
        from_chain: &str,
        to_chain: &str,
        comment: &str,
    ) -> Result<()> {
        // Check if jump rule already exists
        let check = Command::new(&self.iptables_cmd)
            .args([
                "-t",
                table,
                "-C",
                from_chain,
                "-j",
                to_chain,
                "-m",
                "comment",
                "--comment",
                comment,
            ])
            .output();

        match check {
            Ok(output) if output.status.success() => {
                debug!(
                    "Jump rule from {} to {} already exists",
                    from_chain, to_chain
                );
                return Ok(());
            }
            _ => {}
        }

        // Insert the jump rule at the top of the chain so our rules are
        // evaluated before any Docker/Podman rules that might RETURN early.
        let output = Command::new(&self.iptables_cmd)
            .args([
                "-t",
                table,
                "-I",
                from_chain,
                "1",
                "-j",
                to_chain,
                "-m",
                "comment",
                "--comment",
                comment,
            ])
            .output()
            .context("Failed to add iptables jump rule")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Failed to add jump rule: {}", stderr);
            return Err(anyhow::anyhow!("Failed to add jump rule: {}", stderr));
        }

        info!("Added jump rule from {} to {}", from_chain, to_chain);
        Ok(())
    }

    /// Flush all rules in our custom chains and clean up per-endpoint chains
    pub fn flush_rules(&self) -> Result<()> {
        info!("Flushing kube-proxy iptables rules");

        self.flush_chain("nat", &self.services_chain)?;
        self.flush_chain("nat", &self.nodeports_chain)?;

        // Clean up all per-endpoint (SEP) chains from previous sync.
        // These must be flushed and deleted, otherwise on resync:
        // - iptables -N fails (chain already exists)
        // - iptables -A appends duplicate rules to the existing chain
        // This causes stale/duplicate DNAT rules that break routing.
        let chains = {
            let mut guard = self.sep_chains.lock().unwrap();
            std::mem::take(&mut *guard)
        };
        for chain in &chains {
            let _ = Command::new(&self.iptables_cmd)
                .args(["-t", "nat", "-F", chain.as_str()])
                .output();
            let _ = Command::new(&self.iptables_cmd)
                .args(["-t", "nat", "-X", chain.as_str()])
                .output();
            debug!("Cleaned up SEP chain {}", chain);
        }

        // Also clean up any leftover KUBE-SEP / KUBE-NP-SEP chains that might
        // exist from a previous run (e.g., after a crash or restart).
        if let Ok(output) = Command::new(&self.iptables_cmd)
            .args(["-t", "nat", "-L", "-n"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Some(rest) = line
                    .strip_prefix("Chain KUBE-SEP-")
                    .or_else(|| line.strip_prefix("Chain KUBE-NP-SEP-"))
                {
                    // Extract chain name: "Chain KUBE-SEP-xxx (N references)"
                    let full_line = if line.starts_with("Chain KUBE-NP-SEP-") {
                        format!(
                            "KUBE-NP-SEP-{}",
                            rest.split_whitespace().next().unwrap_or("")
                        )
                    } else {
                        format!("KUBE-SEP-{}", rest.split_whitespace().next().unwrap_or(""))
                    };
                    let _ = Command::new(&self.iptables_cmd)
                        .args(["-t", "nat", "-F", full_line.as_str()])
                        .output();
                    let _ = Command::new(&self.iptables_cmd)
                        .args(["-t", "nat", "-X", full_line.as_str()])
                        .output();
                    debug!("Cleaned up leftover chain {}", full_line);
                }
            }
        }

        Ok(())
    }

    /// Flush all rules in a specific chain
    fn flush_chain(&self, table: &str, chain: &str) -> Result<()> {
        let output = Command::new(&self.iptables_cmd)
            .args(["-t", table, "-F", chain])
            .output()
            .context(format!("Failed to flush chain {}", chain))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to flush chain {}: {}", chain, stderr);
        } else {
            debug!("Flushed chain {}", chain);
        }

        Ok(())
    }

    /// Run an iptables command with error logging instead of silently discarding errors
    fn run_iptables_logged(&self, args: &[&str], description: &str) -> bool {
        match Command::new(&self.iptables_cmd).args(args).output() {
            Ok(output) if output.status.success() => {
                debug!("iptables {}: success", description);
                true
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // "Chain already exists" is expected for -N on existing chains
                if !stderr.contains("Chain already exists") {
                    warn!("iptables {} failed: {}", description, stderr.trim());
                }
                false
            }
            Err(e) => {
                error!("Failed to execute iptables for {}: {}", description, e);
                false
            }
        }
    }

    /// Track a SEP chain name so it can be cleaned up during flush
    fn track_sep_chain(&self, chain: &str) {
        let mut guard = self.sep_chains.lock().unwrap();
        guard.push(chain.to_string());
    }

    /// Add rules for a ClusterIP service.
    ///
    /// Session affinity implementation:
    /// - When `recent_available` is true, uses xt_recent module to track client IPs.
    ///   Each endpoint gets a per-endpoint chain (KUBE-SEP-*) with two separate rules:
    ///   1. `recent --set` to mark the source IP (always matches, does NOT terminate)
    ///   2. `DNAT` to redirect traffic (terminates)
    ///   The main chain has recent-check rules first (sticky routing), then
    ///   probability-based fallback rules for new connections.
    /// - When `recent_available` is false, falls back to direct DNAT rules without
    ///   session affinity (service is still reachable, just not sticky).
    ///
    /// Probability calculation uses the Kubernetes approach: rule i has probability
    /// 1/(N-i) so each endpoint gets an equal share of traffic.
    pub fn add_clusterip_rules(
        &self,
        service_ip: &str,
        port: u16,
        endpoints: &[(String, u16)],
        protocol: &str,
        session_affinity: bool,
        affinity_timeout: i32,
    ) -> Result<()> {
        if endpoints.is_empty() {
            debug!(
                "No endpoints for service {}:{}, skipping rules",
                service_ip, port
            );
            return Ok(());
        }

        info!(
            "Adding ClusterIP rules for {}:{} ({}) with {} endpoints (affinity={})",
            service_ip,
            port,
            protocol,
            endpoints.len(),
            session_affinity
        );

        let proto = protocol.to_lowercase();
        let n = endpoints.len();

        if session_affinity && n > 1 && self.recent_available {
            // Session affinity with xt_recent module available.
            // Create per-endpoint chains with separate recent-set and DNAT rules.
            let timeout_str = affinity_timeout.to_string();
            for (idx, (endpoint_ip, endpoint_port)) in endpoints.iter().enumerate() {
                let sep_chain =
                    format!("KUBE-SEP-{}-{}-{}", service_ip.replace('.', ""), port, idx);
                let recent_name =
                    format!("AFFINITY-{}-{}-{}", service_ip.replace('.', ""), port, idx);
                let dnat_target = format!("{}:{}", endpoint_ip, endpoint_port);

                // Create the per-endpoint chain
                self.run_iptables_logged(
                    &["-t", "nat", "-N", &sep_chain],
                    &format!("create SEP chain {}", sep_chain),
                );
                self.track_sep_chain(&sep_chain);

                // Rule 1 in SEP chain: set the recent mark (always matches, does NOT
                // terminate — no -j target, so processing continues to next rule)
                self.run_iptables_logged(
                    &[
                        "-t",
                        "nat",
                        "-A",
                        &sep_chain,
                        "-m",
                        "recent",
                        "--name",
                        &recent_name,
                        "--set",
                    ],
                    &format!("SEP {} recent set", sep_chain),
                );

                // Rule 2 in SEP chain: DNAT to the endpoint (terminates).
                // Must include -p proto — iptables requires a protocol for port DNAT.
                self.run_iptables_logged(
                    &[
                        "-t",
                        "nat",
                        "-A",
                        &sep_chain,
                        "-p",
                        &proto,
                        "-j",
                        "DNAT",
                        "--to-destination",
                        &dnat_target,
                    ],
                    &format!("SEP {} DNAT -> {}", sep_chain, dnat_target),
                );

                // In the main services chain: check if source was recently seen
                // for this endpoint -> jump to its SEP chain (sticky routing)
                let port_str = port.to_string();
                self.run_iptables_logged(
                    &[
                        "-t",
                        "nat",
                        "-A",
                        &self.services_chain,
                        "-d",
                        service_ip,
                        "-p",
                        &proto,
                        "--dport",
                        &port_str,
                        "-m",
                        "recent",
                        "--name",
                        &recent_name,
                        "--rcheck",
                        "--seconds",
                        &timeout_str,
                        "-j",
                        &sep_chain,
                    ],
                    &format!("affinity recent check for {}", sep_chain),
                );
            }

            // Add probability-based fallback rules for new connections
            // (sources not yet tracked by recent module)
            let port_str = port.to_string();
            for (idx, _) in endpoints.iter().enumerate() {
                let is_last = idx == n - 1;
                let sep_chain =
                    format!("KUBE-SEP-{}-{}-{}", service_ip.replace('.', ""), port, idx);
                let mut args = vec![
                    "-t",
                    "nat",
                    "-A",
                    &self.services_chain,
                    "-d",
                    service_ip,
                    "-p",
                    &proto,
                    "--dport",
                    &port_str,
                ];
                let prob_str;
                if !is_last {
                    // Kubernetes probability: 1/(N-idx) for uniform distribution
                    let prob = 1.0 / (n - idx) as f64;
                    prob_str = format!("{:.10}", prob);
                    args.extend_from_slice(&[
                        "-m",
                        "statistic",
                        "--mode",
                        "random",
                        "--probability",
                        &prob_str,
                    ]);
                }
                args.extend_from_slice(&["-j", &sep_chain]);
                self.run_iptables_logged(
                    &args,
                    &format!("affinity probability fallback for {}", sep_chain),
                );
            }
        } else {
            // No session affinity, or single endpoint, or recent module unavailable.
            // Use direct DNAT rules with proper probability distribution.
            let port_str = port.to_string();
            for (idx, (endpoint_ip, endpoint_port)) in endpoints.iter().enumerate() {
                let is_last = idx == n - 1;
                let dnat_target = format!("{}:{}", endpoint_ip, endpoint_port);

                let mut args = vec![
                    "-t",
                    "nat",
                    "-A",
                    &self.services_chain,
                    "-d",
                    service_ip,
                    "-p",
                    &proto,
                    "--dport",
                    &port_str,
                ];
                let prob_str;
                if !is_last {
                    // Kubernetes probability: 1/(N-idx)
                    let prob = 1.0 / (n - idx) as f64;
                    prob_str = format!("{:.10}", prob);
                    args.extend_from_slice(&[
                        "-m",
                        "statistic",
                        "--mode",
                        "random",
                        "--probability",
                        &prob_str,
                    ]);
                }
                let comment = format!("rusternetes service {}:{}", service_ip, port);
                args.extend_from_slice(&[
                    "-j",
                    "DNAT",
                    "--to-destination",
                    &dnat_target,
                    "-m",
                    "comment",
                    "--comment",
                    &comment,
                ]);
                let output = Command::new(&self.iptables_cmd)
                    .args(&args)
                    .output()
                    .context("Failed to add iptables DNAT rule")?;
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    error!("Failed to add DNAT rule for {}: {}", endpoint_ip, stderr);
                } else {
                    debug!("Added DNAT rule: {} -> {}", service_ip, dnat_target);
                }
            }
        }

        Ok(())
    }

    /// Add rules for a NodePort service.
    ///
    /// Supports session affinity using the same xt_recent approach as ClusterIP rules.
    /// Each endpoint gets a per-endpoint chain (KUBE-NP-SEP-*) with recent-set + DNAT.
    pub fn add_nodeport_rules(
        &self,
        node_port: u16,
        endpoints: &[(String, u16)],
        protocol: &str,
        session_affinity: bool,
        affinity_timeout: i32,
    ) -> Result<()> {
        if endpoints.is_empty() {
            debug!("No endpoints for NodePort {}, skipping rules", node_port);
            return Ok(());
        }

        info!(
            "Adding NodePort rules for port {} ({}) with {} endpoints (affinity={})",
            node_port,
            protocol,
            endpoints.len(),
            session_affinity
        );

        let proto = protocol.to_lowercase();
        let n = endpoints.len();

        if session_affinity && n > 1 && self.recent_available {
            // Session affinity for NodePort with xt_recent module
            let timeout_str = affinity_timeout.to_string();
            for (idx, (endpoint_ip, endpoint_port)) in endpoints.iter().enumerate() {
                let sep_chain = format!("KUBE-NP-SEP-{}-{}", node_port, idx);
                let recent_name = format!("NP-AFFINITY-{}-{}", node_port, idx);
                let dnat_target = format!("{}:{}", endpoint_ip, endpoint_port);

                // Create per-endpoint chain
                self.run_iptables_logged(
                    &["-t", "nat", "-N", &sep_chain],
                    &format!("create NP SEP chain {}", sep_chain),
                );
                self.track_sep_chain(&sep_chain);

                // Rule 1: set recent mark (non-terminating)
                self.run_iptables_logged(
                    &[
                        "-t",
                        "nat",
                        "-A",
                        &sep_chain,
                        "-m",
                        "recent",
                        "--name",
                        &recent_name,
                        "--set",
                    ],
                    &format!("NP SEP {} recent set", sep_chain),
                );

                // Rule 2: DNAT (terminating) — must include -p proto for port DNAT
                self.run_iptables_logged(
                    &[
                        "-t",
                        "nat",
                        "-A",
                        &sep_chain,
                        "-p",
                        &proto,
                        "-j",
                        "DNAT",
                        "--to-destination",
                        &dnat_target,
                    ],
                    &format!("NP SEP {} DNAT -> {}", sep_chain, dnat_target),
                );

                // Recent check in main nodeports chain
                let node_port_str = node_port.to_string();
                self.run_iptables_logged(
                    &[
                        "-t",
                        "nat",
                        "-A",
                        &self.nodeports_chain,
                        "-p",
                        &proto,
                        "--dport",
                        &node_port_str,
                        "-m",
                        "recent",
                        "--name",
                        &recent_name,
                        "--rcheck",
                        "--seconds",
                        &timeout_str,
                        "-j",
                        &sep_chain,
                    ],
                    &format!("NP affinity recent check for {}", sep_chain),
                );
            }

            // Probability-based fallback for new connections
            let node_port_str = node_port.to_string();
            for (idx, _) in endpoints.iter().enumerate() {
                let is_last = idx == n - 1;
                let sep_chain = format!("KUBE-NP-SEP-{}-{}", node_port, idx);
                let mut args = vec![
                    "-t",
                    "nat",
                    "-A",
                    &self.nodeports_chain,
                    "-p",
                    &proto,
                    "--dport",
                    &node_port_str,
                ];
                let prob_str;
                if !is_last {
                    let prob = 1.0 / (n - idx) as f64;
                    prob_str = format!("{:.10}", prob);
                    args.extend_from_slice(&[
                        "-m",
                        "statistic",
                        "--mode",
                        "random",
                        "--probability",
                        &prob_str,
                    ]);
                }
                args.extend_from_slice(&["-j", &sep_chain]);
                self.run_iptables_logged(
                    &args,
                    &format!("NP affinity probability fallback for {}", sep_chain),
                );
            }
        } else {
            // No session affinity or single endpoint — direct DNAT rules
            for (idx, (endpoint_ip, endpoint_port)) in endpoints.iter().enumerate() {
                let is_last = idx == n - 1;

                let node_port_str = node_port.to_string();
                let mut args = vec![
                    "-t",
                    "nat",
                    "-A",
                    &self.nodeports_chain,
                    "-p",
                    &proto,
                    "--dport",
                    &node_port_str,
                ];

                // Kubernetes probability: 1/(N-idx)
                let prob_str;
                if !is_last {
                    let prob = 1.0 / (n - idx) as f64;
                    prob_str = format!("{:.10}", prob);
                    args.extend_from_slice(&[
                        "-m",
                        "statistic",
                        "--mode",
                        "random",
                        "--probability",
                        &prob_str,
                    ]);
                }

                // Add DNAT to endpoint
                let dnat_target = format!("{}:{}", endpoint_ip, endpoint_port);
                let comment = format!("rusternetes nodeport {}", node_port);
                args.extend_from_slice(&[
                    "-j",
                    "DNAT",
                    "--to-destination",
                    &dnat_target,
                    "-m",
                    "comment",
                    "--comment",
                    &comment,
                ]);

                let output = Command::new(&self.iptables_cmd)
                    .args(&args)
                    .output()
                    .context("Failed to add iptables DNAT rule for NodePort")?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    error!(
                        "Failed to add NodePort DNAT rule for {}: {}",
                        endpoint_ip, stderr
                    );
                } else {
                    debug!("Added NodePort DNAT rule: {} -> {}", node_port, dnat_target);
                }
            }
        }

        Ok(())
    }

    /// Clean up all kube-proxy iptables rules
    pub fn cleanup(&self) -> Result<()> {
        info!("Cleaning up kube-proxy iptables rules");

        // Flush our chains (also cleans up SEP chains)
        self.flush_rules()?;

        // Remove jump rules
        self.remove_jump_rule("nat", "PREROUTING", &self.services_chain)?;
        self.remove_jump_rule("nat", "OUTPUT", &self.services_chain)?;
        self.remove_jump_rule("nat", "PREROUTING", &self.nodeports_chain)?;
        self.remove_jump_rule("nat", "OUTPUT", &self.nodeports_chain)?;

        // Delete our chains
        self.delete_chain("nat", &self.services_chain)?;
        self.delete_chain("nat", &self.nodeports_chain)?;

        info!("Kube-proxy iptables cleanup complete");
        Ok(())
    }

    fn remove_jump_rule(&self, table: &str, from_chain: &str, to_chain: &str) -> Result<()> {
        let output = Command::new(&self.iptables_cmd)
            .args(["-t", table, "-D", from_chain, "-j", to_chain])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                debug!("Removed jump rule from {} to {}", from_chain, to_chain);
            }
            _ => {
                debug!(
                    "Jump rule from {} to {} may not exist",
                    from_chain, to_chain
                );
            }
        }

        Ok(())
    }

    fn delete_chain(&self, table: &str, chain: &str) -> Result<()> {
        let output = Command::new(&self.iptables_cmd)
            .args(["-t", table, "-X", chain])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                debug!("Deleted chain {}", chain);
            }
            _ => {
                debug!("Chain {} may not exist", chain);
            }
        }

        Ok(())
    }
}

impl Drop for IptablesManager {
    fn drop(&mut self) {
        // Best effort cleanup
        let _ = self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probability_calculation_uniform() {
        // Verify that the Kubernetes probability formula 1/(N-idx) produces
        // uniform distribution across all endpoints
        let n = 4;
        let mut remaining = 1.0_f64;
        let mut shares = Vec::new();
        for idx in 0..n {
            let is_last = idx == n - 1;
            if is_last {
                shares.push(remaining);
            } else {
                let prob = 1.0 / (n - idx) as f64;
                let share = remaining * prob;
                shares.push(share);
                remaining -= share;
            }
        }
        // Each endpoint should get ~0.25 (1/4)
        for share in &shares {
            assert!(
                (*share - 0.25).abs() < 1e-10,
                "Expected ~0.25, got {}",
                share
            );
        }
    }

    #[test]
    fn test_probability_calculation_two_endpoints() {
        let n = 2;
        let mut remaining = 1.0_f64;
        let mut shares = Vec::new();
        for idx in 0..n {
            let is_last = idx == n - 1;
            if is_last {
                shares.push(remaining);
            } else {
                let prob = 1.0 / (n - idx) as f64;
                let share = remaining * prob;
                shares.push(share);
                remaining -= share;
            }
        }
        assert!((shares[0] - 0.5).abs() < 1e-10);
        assert!((shares[1] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_probability_calculation_three_endpoints() {
        let n = 3;
        let mut remaining = 1.0_f64;
        let mut shares = Vec::new();
        for idx in 0..n {
            let is_last = idx == n - 1;
            if is_last {
                shares.push(remaining);
            } else {
                let prob = 1.0 / (n - idx) as f64;
                let share = remaining * prob;
                shares.push(share);
                remaining -= share;
            }
        }
        let expected = 1.0 / 3.0;
        for (i, share) in shares.iter().enumerate() {
            assert!(
                (*share - expected).abs() < 1e-10,
                "Endpoint {}: expected ~{}, got {}",
                i,
                expected,
                share
            );
        }
    }

    #[test]
    fn test_probability_single_endpoint() {
        // With a single endpoint, no probability module is needed (is_last=true immediately)
        let n = 1;
        let idx = 0;
        let is_last = idx == n - 1;
        assert!(is_last, "Single endpoint should be the last");
    }

    #[test]
    fn test_sep_chain_tracking() {
        // Verify that SEP chains are tracked and can be retrieved
        let chains: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
        {
            let mut guard = chains.lock().unwrap();
            guard.push("KUBE-SEP-10960001-80-0".to_string());
            guard.push("KUBE-SEP-10960001-80-1".to_string());
        }
        let taken = {
            let mut guard = chains.lock().unwrap();
            std::mem::take(&mut *guard)
        };
        assert_eq!(taken.len(), 2);
        assert_eq!(taken[0], "KUBE-SEP-10960001-80-0");
        assert_eq!(taken[1], "KUBE-SEP-10960001-80-1");

        // After take, the vec should be empty
        let guard = chains.lock().unwrap();
        assert!(guard.is_empty());
    }

    #[test]
    fn test_sep_chain_name_format() {
        // Verify chain name format for ClusterIP
        let service_ip = "10.96.0.1";
        let port: u16 = 80;
        let idx = 0;
        let chain = format!("KUBE-SEP-{}-{}-{}", service_ip.replace('.', ""), port, idx);
        assert_eq!(chain, "KUBE-SEP-109601-80-0");

        // Verify chain name format for NodePort
        let node_port: u16 = 30080;
        let np_chain = format!("KUBE-NP-SEP-{}-{}", node_port, idx);
        assert_eq!(np_chain, "KUBE-NP-SEP-30080-0");
    }
}
