/// Pod admission controllers for ResourceQuota and LimitRange enforcement
use rusternetes_common::{
    resources::{LimitRange, Pod, ResourceQuota},
    types::ResourceRequirements,
};
use rusternetes_storage::Storage;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;

/// Check if pod creation would exceed ResourceQuota limits
pub async fn check_resource_quota<S: Storage>(
    storage: &Arc<S>,
    namespace: &str,
    pod: &Pod,
) -> anyhow::Result<bool> {
    // Get all quotas for this namespace
    let quota_prefix = format!("/registry/resourcequotas/{}/", namespace);
    let quotas: Vec<ResourceQuota> = storage.list(&quota_prefix).await?;

    if quotas.is_empty() {
        // No quota to enforce
        return Ok(true);
    }

    // Calculate current usage
    let current_usage = calculate_namespace_usage(storage, namespace).await?;

    // Calculate pod resource requirements
    let pod_requests = calculate_pod_requests(pod);

    // Check each quota
    for quota in quotas {
        if let Some(hard) = &quota.spec.hard {
            // Check pod count
            if let Some(pod_limit) = hard.get("pods") {
                let current_pods = current_usage
                    .get("pods")
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0);
                let limit = pod_limit
                    .parse::<i64>()
                    .map_err(|e| anyhow::anyhow!("Invalid pod limit: {}", e))?;

                if current_pods + 1 > limit {
                    warn!(
                        "Pod creation would exceed quota {}/{}: pods {} + 1 > {}",
                        namespace, quota.metadata.name, current_pods, limit
                    );
                    return Ok(false);
                }
            }

            // Check CPU requests
            if let Some(cpu_limit) = hard.get("requests.cpu") {
                let current_cpu = current_usage
                    .get("requests.cpu")
                    .and_then(|s| parse_cpu_to_millicores(s).ok())
                    .unwrap_or(0);
                let pod_cpu = pod_requests.get("cpu").copied().unwrap_or(0);
                let limit = parse_cpu_to_millicores(cpu_limit)?;

                if current_cpu + pod_cpu > limit {
                    warn!(
                        "Pod creation would exceed CPU quota {}/{}: {}m + {}m > {}m",
                        namespace, quota.metadata.name, current_cpu, pod_cpu, limit
                    );
                    return Ok(false);
                }
            }

            // Check memory requests
            if let Some(mem_limit) = hard.get("requests.memory") {
                let current_mem = current_usage
                    .get("requests.memory")
                    .and_then(|s| parse_memory_to_bytes(s).ok())
                    .unwrap_or(0);
                let pod_mem = pod_requests.get("memory").copied().unwrap_or(0);
                let limit = parse_memory_to_bytes(mem_limit)?;

                if current_mem + pod_mem > limit {
                    warn!(
                        "Pod creation would exceed memory quota {}/{}: {} + {} > {}",
                        namespace, quota.metadata.name, current_mem, pod_mem, limit
                    );
                    return Ok(false);
                }
            }
        }
    }

    Ok(true)
}

/// Apply LimitRange defaults and validate constraints
pub async fn apply_limit_range<S: Storage>(
    storage: &Arc<S>,
    namespace: &str,
    pod: &mut Pod,
) -> anyhow::Result<bool> {
    // Get all LimitRanges for this namespace
    let limit_prefix = format!("/registry/limitranges/{}/", namespace);
    let limit_ranges: Vec<LimitRange> = storage.list(&limit_prefix).await?;

    if limit_ranges.is_empty() {
        // No limits to apply
        return Ok(true);
    }

    // Apply defaults and validate for each container
    if let Some(spec) = &mut pod.spec {
        for container in &mut spec.containers {
            for limit_range in &limit_ranges {
                for limit_item in &limit_range.spec.limits {
                    // Only apply Container limits to containers
                    if limit_item.item_type == "Container" {
                        // Apply defaults if not specified
                        if container.resources.is_none() {
                            container.resources = Some(ResourceRequirements {
                                limits: None,
                                requests: None,
                            });
                        }

                        let resources = container.resources.as_mut().unwrap();

                        // Apply default limits
                        if let Some(default_limits) = &limit_item.default {
                            if resources.limits.is_none() {
                                resources.limits = Some(default_limits.clone());
                            } else {
                                // Merge with existing limits
                                let limits = resources.limits.as_mut().unwrap();
                                for (key, value) in default_limits {
                                    limits
                                        .entry(key.clone())
                                        .or_insert_with(|| value.clone());
                                }
                            }
                        }

                        // Apply default requests
                        if let Some(default_requests) = &limit_item.default_request {
                            if resources.requests.is_none() {
                                resources.requests = Some(default_requests.clone());
                            } else {
                                // Merge with existing requests
                                let requests = resources.requests.as_mut().unwrap();
                                for (key, value) in default_requests {
                                    requests
                                        .entry(key.clone())
                                        .or_insert_with(|| value.clone());
                                }
                            }
                        }

                        // Validate min constraints
                        if let Some(min) = &limit_item.min {
                            if !validate_min_resources(resources, min, &container.name)? {
                                return Ok(false);
                            }
                        }

                        // Validate max constraints
                        if let Some(max) = &limit_item.max {
                            if !validate_max_resources(resources, max, &container.name)? {
                                return Ok(false);
                            }
                        }

                        // Validate max limit/request ratio
                        if let Some(ratio) = &limit_item.max_limit_request_ratio {
                            if !validate_ratio(resources, ratio, &container.name)? {
                                return Ok(false);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(true)
}

// Helper functions

async fn calculate_namespace_usage<S: Storage>(
    storage: &Arc<S>,
    namespace: &str,
) -> anyhow::Result<HashMap<String, String>> {
    let mut usage = HashMap::new();

    // Count pods
    let pod_prefix = format!("/registry/pods/{}/", namespace);
    let pods: Vec<Pod> = storage.list(&pod_prefix).await?;
    usage.insert("pods".to_string(), pods.len().to_string());

    // Calculate CPU and memory requests
    let mut total_cpu_requests = 0i64;
    let mut total_memory_requests = 0i64;

    for pod in &pods {
        if let Some(spec) = &pod.spec {
            for container in &spec.containers {
                if let Some(resources) = &container.resources {
                    if let Some(requests) = &resources.requests {
                        if let Some(cpu) = requests.get("cpu") {
                            if let Ok(millis) = parse_cpu_to_millicores(cpu) {
                                total_cpu_requests += millis;
                            }
                        }
                        if let Some(memory) = requests.get("memory") {
                            if let Ok(bytes) = parse_memory_to_bytes(memory) {
                                total_memory_requests += bytes;
                            }
                        }
                    }
                }
            }
        }
    }

    if total_cpu_requests > 0 {
        usage.insert("requests.cpu".to_string(), format!("{}m", total_cpu_requests));
    }
    if total_memory_requests > 0 {
        usage.insert(
            "requests.memory".to_string(),
            bytes_to_memory_string(total_memory_requests),
        );
    }

    Ok(usage)
}

fn calculate_pod_requests(pod: &Pod) -> HashMap<String, i64> {
    let mut requests = HashMap::new();
    let mut total_cpu = 0i64;
    let mut total_memory = 0i64;

    if let Some(spec) = &pod.spec {
        for container in &spec.containers {
            if let Some(resources) = &container.resources {
                if let Some(reqs) = &resources.requests {
                    if let Some(cpu) = reqs.get("cpu") {
                        if let Ok(millis) = parse_cpu_to_millicores(cpu) {
                            total_cpu += millis;
                        }
                    }
                    if let Some(memory) = reqs.get("memory") {
                        if let Ok(bytes) = parse_memory_to_bytes(memory) {
                            total_memory += bytes;
                        }
                    }
                }
            }
        }
    }

    if total_cpu > 0 {
        requests.insert("cpu".to_string(), total_cpu);
    }
    if total_memory > 0 {
        requests.insert("memory".to_string(), total_memory);
    }

    requests
}

fn validate_min_resources(
    resources: &ResourceRequirements,
    min: &HashMap<String, String>,
    container_name: &str,
) -> anyhow::Result<bool> {
    if let Some(requests) = &resources.requests {
        for (resource, min_value) in min {
            if let Some(request_value) = requests.get(resource) {
                if resource == "cpu" {
                    let request = parse_cpu_to_millicores(request_value)?;
                    let minimum = parse_cpu_to_millicores(min_value)?;
                    if request < minimum {
                        warn!(
                            "Container {} has CPU request {} below minimum {}",
                            container_name, request_value, min_value
                        );
                        return Ok(false);
                    }
                } else if resource == "memory" {
                    let request = parse_memory_to_bytes(request_value)?;
                    let minimum = parse_memory_to_bytes(min_value)?;
                    if request < minimum {
                        warn!(
                            "Container {} has memory request {} below minimum {}",
                            container_name, request_value, min_value
                        );
                        return Ok(false);
                    }
                }
            }
        }
    }

    Ok(true)
}

fn validate_max_resources(
    resources: &ResourceRequirements,
    max: &HashMap<String, String>,
    container_name: &str,
) -> anyhow::Result<bool> {
    if let Some(limits) = &resources.limits {
        for (resource, max_value) in max {
            if let Some(limit_value) = limits.get(resource) {
                if resource == "cpu" {
                    let limit = parse_cpu_to_millicores(limit_value)?;
                    let maximum = parse_cpu_to_millicores(max_value)?;
                    if limit > maximum {
                        warn!(
                            "Container {} has CPU limit {} exceeding maximum {}",
                            container_name, limit_value, max_value
                        );
                        return Ok(false);
                    }
                } else if resource == "memory" {
                    let limit = parse_memory_to_bytes(limit_value)?;
                    let maximum = parse_memory_to_bytes(max_value)?;
                    if limit > maximum {
                        warn!(
                            "Container {} has memory limit {} exceeding maximum {}",
                            container_name, limit_value, max_value
                        );
                        return Ok(false);
                    }
                }
            }
        }
    }

    Ok(true)
}

fn validate_ratio(
    resources: &ResourceRequirements,
    max_ratio: &HashMap<String, String>,
    container_name: &str,
) -> anyhow::Result<bool> {
    if let (Some(limits), Some(requests)) = (&resources.limits, &resources.requests) {
        for (resource, max_ratio_str) in max_ratio {
            if let (Some(limit_value), Some(request_value)) =
                (limits.get(resource), requests.get(resource))
            {
                let ratio_limit = max_ratio_str.parse::<f64>()?;

                if resource == "cpu" {
                    let limit = parse_cpu_to_millicores(limit_value)? as f64;
                    let request = parse_cpu_to_millicores(request_value)? as f64;
                    if request > 0.0 {
                        let actual_ratio = limit / request;
                        if actual_ratio > ratio_limit {
                            warn!(
                                "Container {} has CPU limit/request ratio {:.2} exceeding maximum {:.2}",
                                container_name, actual_ratio, ratio_limit
                            );
                            return Ok(false);
                        }
                    }
                } else if resource == "memory" {
                    let limit = parse_memory_to_bytes(limit_value)? as f64;
                    let request = parse_memory_to_bytes(request_value)? as f64;
                    if request > 0.0 {
                        let actual_ratio = limit / request;
                        if actual_ratio > ratio_limit {
                            warn!(
                                "Container {} has memory limit/request ratio {:.2} exceeding maximum {:.2}",
                                container_name, actual_ratio, ratio_limit
                            );
                            return Ok(false);
                        }
                    }
                }
            }
        }
    }

    Ok(true)
}

fn parse_cpu_to_millicores(cpu: &str) -> anyhow::Result<i64> {
    if cpu.ends_with('m') {
        let millis = cpu.trim_end_matches('m').parse::<i64>()?;
        Ok(millis)
    } else {
        let cores = cpu.parse::<f64>()?;
        Ok((cores * 1000.0) as i64)
    }
}

fn parse_memory_to_bytes(memory: &str) -> anyhow::Result<i64> {
    let memory = memory.trim();

    if memory.ends_with("Gi") {
        let value = memory.trim_end_matches("Gi").parse::<f64>()?;
        Ok((value * 1024.0 * 1024.0 * 1024.0) as i64)
    } else if memory.ends_with("Mi") {
        let value = memory.trim_end_matches("Mi").parse::<f64>()?;
        Ok((value * 1024.0 * 1024.0) as i64)
    } else if memory.ends_with("Ki") {
        let value = memory.trim_end_matches("Ki").parse::<f64>()?;
        Ok((value * 1024.0) as i64)
    } else if memory.ends_with("G") {
        let value = memory.trim_end_matches("G").parse::<f64>()?;
        Ok((value * 1000.0 * 1000.0 * 1000.0) as i64)
    } else if memory.ends_with("M") {
        let value = memory.trim_end_matches("M").parse::<f64>()?;
        Ok((value * 1000.0 * 1000.0) as i64)
    } else if memory.ends_with("K") {
        let value = memory.trim_end_matches("K").parse::<f64>()?;
        Ok((value * 1000.0) as i64)
    } else {
        Ok(memory.parse::<i64>()?)
    }
}

fn bytes_to_memory_string(bytes: i64) -> String {
    const GI: i64 = 1024 * 1024 * 1024;
    const MI: i64 = 1024 * 1024;
    const KI: i64 = 1024;

    if bytes >= GI && bytes % GI == 0 {
        format!("{}Gi", bytes / GI)
    } else if bytes >= MI && bytes % MI == 0 {
        format!("{}Mi", bytes / MI)
    } else if bytes >= KI && bytes % KI == 0 {
        format!("{}Ki", bytes / KI)
    } else {
        format!("{}", bytes)
    }
}
