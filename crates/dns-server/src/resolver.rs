use hickory_proto::rr::{Name, RData, Record};
use hickory_proto::rr::rdata::{A, AAAA, SRV};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::sync::RwLock;
use tracing::{debug, warn};

/// DNS record types stored in the resolver
#[derive(Debug, Clone)]
struct DnsRecord {
    name: String,
    rdata: RecordData,
}

#[derive(Debug, Clone)]
enum RecordData {
    A(Ipv4Addr),
    AAAA(Ipv6Addr),
    SRV {
        priority: u16,
        weight: u16,
        port: u16,
        target: String,
    },
}

/// Kubernetes DNS resolver
/// Handles DNS queries for:
/// - Services: <service>.<namespace>.svc.<cluster-domain>
/// - Pods: <pod-ip>.<namespace>.pod.<cluster-domain>
/// - SRV records for headless services
pub struct KubernetesResolver {
    cluster_domain: String,
    ttl: u32,
    records: RwLock<HashMap<String, Vec<DnsRecord>>>,
}

impl KubernetesResolver {
    pub fn new(cluster_domain: String, ttl: u32) -> Self {
        Self {
            cluster_domain,
            ttl,
            records: RwLock::new(HashMap::new()),
        }
    }

    /// Update service DNS records
    pub fn update_service(&self, name: &str, namespace: &str, cluster_ip: Option<&str>, endpoints: Vec<ServiceEndpoint>) {
        let fqdn = format!("{}.{}.svc.{}", name, namespace, self.cluster_domain);

        let mut records_map = self.records.write().unwrap();

        // Clear existing records for this service
        records_map.remove(&fqdn);

        // Also remove any existing SRV records for this service
        records_map.retain(|key, _| !key.ends_with(&format!(".{}", fqdn)));

        let mut records = Vec::new();

        if let Some(ip_str) = cluster_ip {
            // ClusterIP service - return the service ClusterIP
            if let Ok(ip) = IpAddr::from_str(ip_str) {
                match ip {
                    IpAddr::V4(ipv4) => {
                        records.push(DnsRecord {
                            name: fqdn.clone(),
                            rdata: RecordData::A(ipv4),
                        });
                        debug!("Added A record: {} -> {}", fqdn, ipv4);
                    }
                    IpAddr::V6(ipv6) => {
                        records.push(DnsRecord {
                            name: fqdn.clone(),
                            rdata: RecordData::AAAA(ipv6),
                        });
                        debug!("Added AAAA record: {} -> {}", fqdn, ipv6);
                    }
                }
            }
        } else if !endpoints.is_empty() {
            // Headless service (no ClusterIP) - return pod IPs
            for endpoint in &endpoints {
                if let Ok(ip) = IpAddr::from_str(&endpoint.ip) {
                    match ip {
                        IpAddr::V4(ipv4) => {
                            records.push(DnsRecord {
                                name: fqdn.clone(),
                                rdata: RecordData::A(ipv4),
                            });
                            debug!("Added A record (headless): {} -> {}", fqdn, ipv4);
                        }
                        IpAddr::V6(ipv6) => {
                            records.push(DnsRecord {
                                name: fqdn.clone(),
                                rdata: RecordData::AAAA(ipv6),
                            });
                            debug!("Added AAAA record (headless): {} -> {}", fqdn, ipv6);
                        }
                    }
                }
            }

            // Add SRV records for headless services (stored separately)
            for endpoint in &endpoints {
                if let Some(pod_name) = &endpoint.pod_name {
                    let srv_name = format!("_{}._{}.{}", endpoint.port_name.as_deref().unwrap_or("http"), endpoint.protocol.as_deref().unwrap_or("tcp"), fqdn);
                    let target = format!("{}.{}.pod.{}", pod_name, namespace, self.cluster_domain);

                    let srv_record = DnsRecord {
                        name: srv_name.clone(),
                        rdata: RecordData::SRV {
                            priority: 0,
                            weight: 100,
                            port: endpoint.port,
                            target: target.clone(),
                        },
                    };

                    // Store SRV records under their own name, not the service name
                    records_map.entry(srv_name.clone()).or_insert_with(Vec::new).push(srv_record);
                    debug!("Added SRV record: {} -> {}:{}", srv_name, target, endpoint.port);
                }
            }
        }

        if !records.is_empty() {
            records_map.insert(fqdn.clone(), records);
        }
    }

    /// Update pod DNS records
    pub fn update_pod(&self, pod_name: &str, namespace: &str, pod_ip: &str) {
        // Pod DNS format: <pod-name>.<namespace>.pod.<cluster-domain>
        let fqdn = format!("{}.{}.pod.{}", pod_name, namespace, self.cluster_domain);

        // Also support IP-based format: <ip-with-dashes>.<namespace>.pod.<cluster-domain>
        let ip_based_fqdn = format!("{}.{}.pod.{}",
            pod_ip.replace('.', "-"),
            namespace,
            self.cluster_domain
        );

        let mut records_map = self.records.write().unwrap();

        if let Ok(ip) = IpAddr::from_str(pod_ip) {
            let record = match ip {
                IpAddr::V4(ipv4) => DnsRecord {
                    name: fqdn.clone(),
                    rdata: RecordData::A(ipv4),
                },
                IpAddr::V6(ipv6) => DnsRecord {
                    name: fqdn.clone(),
                    rdata: RecordData::AAAA(ipv6),
                },
            };

            // Insert both name-based and IP-based records
            records_map.insert(fqdn.clone(), vec![record.clone()]);
            records_map.insert(ip_based_fqdn.clone(), vec![record]);

            debug!("Added pod record: {} -> {}", fqdn, pod_ip);
            debug!("Added pod record (IP-based): {} -> {}", ip_based_fqdn, pod_ip);
        } else {
            warn!("Invalid pod IP address: {}", pod_ip);
        }
    }

    /// Remove service DNS records
    #[allow(dead_code)]
    pub fn remove_service(&self, name: &str, namespace: &str) {
        let fqdn = format!("{}.{}.svc.{}", name, namespace, self.cluster_domain);
        let mut records_map = self.records.write().unwrap();
        records_map.remove(&fqdn);

        // Also remove any SRV records for this service
        records_map.retain(|key, _| !key.ends_with(&format!(".{}", fqdn)));

        debug!("Removed service record: {}", fqdn);
    }

    /// Remove pod DNS records
    #[allow(dead_code)]
    pub fn remove_pod(&self, pod_name: &str, namespace: &str, pod_ip: Option<&str>) {
        let fqdn = format!("{}.{}.pod.{}", pod_name, namespace, self.cluster_domain);
        let mut records_map = self.records.write().unwrap();
        records_map.remove(&fqdn);

        if let Some(ip) = pod_ip {
            let ip_based_fqdn = format!("{}.{}.pod.{}",
                ip.replace('.', "-"),
                namespace,
                self.cluster_domain
            );
            records_map.remove(&ip_based_fqdn);
        }

        debug!("Removed pod record: {}", fqdn);
    }

    /// Lookup DNS records by name
    pub fn lookup(&self, name: &Name) -> Option<Vec<Record>> {
        let query_name = name.to_lowercase().to_string();
        let records_map = self.records.read().unwrap();

        if let Some(dns_records) = records_map.get(&query_name) {
            let mut result = Vec::new();

            for record in dns_records {
                let record_name = match Name::from_str(&record.name) {
                    Ok(n) => n,
                    Err(e) => {
                        warn!("Failed to parse record name {}: {}", record.name, e);
                        continue;
                    }
                };

                let rdata = match &record.rdata {
                    RecordData::A(ipv4) => RData::A(A(*ipv4)),
                    RecordData::AAAA(ipv6) => RData::AAAA(AAAA(*ipv6)),
                    RecordData::SRV { priority, weight, port, target } => {
                        match Name::from_str(target) {
                            Ok(target_name) => RData::SRV(SRV::new(
                                *priority,
                                *weight,
                                *port,
                                target_name,
                            )),
                            Err(e) => {
                                warn!("Failed to parse SRV target {}: {}", target, e);
                                continue;
                            }
                        }
                    }
                };

                let dns_record = Record::from_rdata(
                    record_name,
                    self.ttl,
                    rdata,
                );

                result.push(dns_record);
            }

            if !result.is_empty() {
                debug!("Lookup {} returned {} records", query_name, result.len());
                return Some(result);
            }
        }

        debug!("No records found for {}", query_name);
        None
    }

    /// Get statistics about cached records
    pub fn stats(&self) -> (usize, usize) {
        let records_map = self.records.read().unwrap();
        let total_names = records_map.len();
        let total_records: usize = records_map.values().map(|v| v.len()).sum();
        (total_names, total_records)
    }
}

#[derive(Debug, Clone)]
pub struct ServiceEndpoint {
    pub ip: String,
    pub port: u16,
    pub port_name: Option<String>,
    pub protocol: Option<String>,
    pub pod_name: Option<String>,
}
