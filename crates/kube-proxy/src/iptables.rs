use anyhow::{Context, Result};
use std::process::Command;
use tracing::{debug, error, info, warn};

/// IptablesManager handles iptables rule programming for service networking
pub struct IptablesManager {
    /// Chain names we create
    services_chain: String,
    nodeports_chain: String,
}

impl IptablesManager {
    pub fn new() -> Self {
        Self {
            services_chain: "RUSTERNETES-SERVICES".to_string(),
            nodeports_chain: "RUSTERNETES-NODEPORTS".to_string(),
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
            "kubernetes service portals"
        )?;

        self.ensure_jump_rule(
            "nat",
            "OUTPUT",
            &self.services_chain,
            "kubernetes service portals"
        )?;

        // Add jump rule for NodePort services
        self.ensure_jump_rule(
            "nat",
            "PREROUTING",
            &self.nodeports_chain,
            "kubernetes service node ports"
        )?;

        info!("Iptables chains initialized successfully");
        Ok(())
    }

    /// Ensure an iptables chain exists
    fn ensure_chain(&self, table: &str, chain: &str) -> Result<()> {
        // Try to create the chain, ignore error if it already exists
        let output = Command::new("iptables")
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
    fn ensure_jump_rule(&self, table: &str, from_chain: &str, to_chain: &str, comment: &str) -> Result<()> {
        // Check if jump rule already exists
        let check = Command::new("iptables")
            .args(["-t", table, "-C", from_chain, "-j", to_chain, "-m", "comment", "--comment", comment])
            .output();

        match check {
            Ok(output) if output.status.success() => {
                debug!("Jump rule from {} to {} already exists", from_chain, to_chain);
                return Ok(());
            }
            _ => {}
        }

        // Add the jump rule
        let output = Command::new("iptables")
            .args(["-t", table, "-A", from_chain, "-j", to_chain, "-m", "comment", "--comment", comment])
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

    /// Flush all rules in our custom chains
    pub fn flush_rules(&self) -> Result<()> {
        info!("Flushing kube-proxy iptables rules");

        self.flush_chain("nat", &self.services_chain)?;
        self.flush_chain("nat", &self.nodeports_chain)?;

        Ok(())
    }

    /// Flush all rules in a specific chain
    fn flush_chain(&self, table: &str, chain: &str) -> Result<()> {
        let output = Command::new("iptables")
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

    /// Add rules for a ClusterIP service
    pub fn add_clusterip_rules(&self, service_ip: &str, port: u16, endpoints: &[(String, u16)], protocol: &str) -> Result<()> {
        if endpoints.is_empty() {
            debug!("No endpoints for service {}:{}, skipping rules", service_ip, port);
            return Ok(());
        }

        info!("Adding ClusterIP rules for {}:{} ({}) with {} endpoints",
              service_ip, port, protocol, endpoints.len());

        let proto = protocol.to_lowercase();

        // For each endpoint, add a DNAT rule with probability for load balancing
        let probability = 1.0 / endpoints.len() as f64;

        for (idx, (endpoint_ip, endpoint_port)) in endpoints.iter().enumerate() {
            let is_last = idx == endpoints.len() - 1;

            let port_str = port.to_string();
            let mut args = vec![
                "-t", "nat",
                "-A", &self.services_chain,
                "-d", service_ip,
                "-p", &proto,
                "--dport", &port_str,
            ];

            // Add probability for load balancing (except for the last endpoint)
            let prob_str;
            if !is_last {
                let prob = probability;
                prob_str = format!("{:.10}", prob);
                args.extend_from_slice(&[
                    "-m", "statistic",
                    "--mode", "random",
                    "--probability", &prob_str,
                ]);
            }

            // Add DNAT to endpoint
            let dnat_target = format!("{}:{}", endpoint_ip, endpoint_port);
            let comment = format!("rusternetes service {}:{}", service_ip, port);
            args.extend_from_slice(&[
                "-j", "DNAT",
                "--to-destination", &dnat_target,
                "-m", "comment",
                "--comment", &comment,
            ]);

            let output = Command::new("iptables")
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

        Ok(())
    }

    /// Add rules for a NodePort service
    pub fn add_nodeport_rules(&self, node_port: u16, endpoints: &[(String, u16)], protocol: &str) -> Result<()> {
        if endpoints.is_empty() {
            debug!("No endpoints for NodePort {}, skipping rules", node_port);
            return Ok(());
        }

        info!("Adding NodePort rules for port {} ({}) with {} endpoints",
              node_port, protocol, endpoints.len());

        let proto = protocol.to_lowercase();

        // For each endpoint, add a DNAT rule with probability for load balancing
        let probability = 1.0 / endpoints.len() as f64;

        for (idx, (endpoint_ip, endpoint_port)) in endpoints.iter().enumerate() {
            let is_last = idx == endpoints.len() - 1;

            let node_port_str = node_port.to_string();
            let mut args = vec![
                "-t", "nat",
                "-A", &self.nodeports_chain,
                "-p", &proto,
                "--dport", &node_port_str,
            ];

            // Add probability for load balancing (except for the last endpoint)
            let prob_str;
            if !is_last {
                let prob = probability;
                prob_str = format!("{:.10}", prob);
                args.extend_from_slice(&[
                    "-m", "statistic",
                    "--mode", "random",
                    "--probability", &prob_str,
                ]);
            }

            // Add DNAT to endpoint
            let dnat_target = format!("{}:{}", endpoint_ip, endpoint_port);
            let comment = format!("rusternetes nodeport {}", node_port);
            args.extend_from_slice(&[
                "-j", "DNAT",
                "--to-destination", &dnat_target,
                "-m", "comment",
                "--comment", &comment,
            ]);

            let output = Command::new("iptables")
                .args(&args)
                .output()
                .context("Failed to add iptables DNAT rule for NodePort")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                error!("Failed to add NodePort DNAT rule for {}: {}", endpoint_ip, stderr);
            } else {
                debug!("Added NodePort DNAT rule: {} -> {}", node_port, dnat_target);
            }
        }

        Ok(())
    }

    /// Clean up all kube-proxy iptables rules
    pub fn cleanup(&self) -> Result<()> {
        info!("Cleaning up kube-proxy iptables rules");

        // Flush our chains
        self.flush_rules()?;

        // Remove jump rules
        self.remove_jump_rule("nat", "PREROUTING", &self.services_chain)?;
        self.remove_jump_rule("nat", "OUTPUT", &self.services_chain)?;
        self.remove_jump_rule("nat", "PREROUTING", &self.nodeports_chain)?;

        // Delete our chains
        self.delete_chain("nat", &self.services_chain)?;
        self.delete_chain("nat", &self.nodeports_chain)?;

        info!("Kube-proxy iptables cleanup complete");
        Ok(())
    }

    fn remove_jump_rule(&self, table: &str, from_chain: &str, to_chain: &str) -> Result<()> {
        let output = Command::new("iptables")
            .args(["-t", table, "-D", from_chain, "-j", to_chain])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                debug!("Removed jump rule from {} to {}", from_chain, to_chain);
            }
            _ => {
                debug!("Jump rule from {} to {} may not exist", from_chain, to_chain);
            }
        }

        Ok(())
    }

    fn delete_chain(&self, table: &str, chain: &str) -> Result<()> {
        let output = Command::new("iptables")
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
