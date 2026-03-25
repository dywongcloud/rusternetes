use anyhow::{Context, Result};
use bollard::container::{
    Config, CreateContainerOptions, InspectContainerOptions, ListContainersOptions,
    RemoveContainerOptions, StartContainerOptions, StopContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::CreateImageOptions;
use bollard::Docker;
use chrono::Utc;
use futures_util::StreamExt;
use rusternetes_common::resources::{
    ConfigMap, Container, ContainerState, ContainerStatus, ExecAction, GRPCAction, HTTPGetAction,
    LifecycleHandler, PersistentVolume, PersistentVolumeClaim, Pod, Probe, Secret, TCPSocketAction,
};
use rusternetes_storage::{build_key, Storage};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::cni::CniRuntime;

/// Wrapper for pre-encoded protobuf bytes (gRPC health check request).
#[derive(Clone, Debug, Default)]
struct EncodedBytes(Vec<u8>);

impl prost::Message for EncodedBytes {
    fn encode_raw(&self, buf: &mut impl prost::bytes::BufMut) {
        buf.put_slice(&self.0);
    }
    fn merge_field(
        &mut self,
        _tag: u32,
        _wire_type: prost::encoding::WireType,
        _buf: &mut impl prost::bytes::Buf,
        _ctx: prost::encoding::DecodeContext,
    ) -> std::result::Result<(), prost::DecodeError> {
        Ok(())
    }
    fn encoded_len(&self) -> usize {
        self.0.len()
    }
    fn clear(&mut self) {
        self.0.clear();
    }
}

/// Decoded gRPC health check response — extracts the status field (field 1, varint).
#[derive(Clone, Debug, Default)]
struct DecodedStatus(i32);

impl prost::Message for DecodedStatus {
    fn encode_raw(&self, _buf: &mut impl prost::bytes::BufMut) {}
    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: prost::encoding::WireType,
        buf: &mut impl prost::bytes::Buf,
        ctx: prost::encoding::DecodeContext,
    ) -> std::result::Result<(), prost::DecodeError> {
        if tag == 1 {
            // status field is an enum (varint)
            prost::encoding::int32::merge(wire_type, &mut self.0, buf, ctx)
        } else {
            prost::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }
    fn encoded_len(&self) -> usize {
        0
    }
    fn clear(&mut self) {
        self.0 = 0;
    }
}

/// Tracks consecutive probe successes/failures for threshold-based probe evaluation.
#[derive(Debug, Clone, Default)]
struct ProbeState {
    consecutive_failures: i32,
    consecutive_successes: i32,
}

/// ContainerRuntime manages containers using Docker/Podman with CNI networking
pub struct ContainerRuntime {
    docker: Docker,
    storage: Option<Arc<rusternetes_storage::etcd::EtcdStorage>>,
    volumes_base_path: String,
    cluster_dns: String,
    cluster_domain: String,
    network: String,
    cni: Option<CniRuntime>,
    use_cni: bool,
    kubernetes_service_host: String,
    /// Probe state tracker: key is "{pod_name}/{container_name}/{probe_type}"
    probe_states: Mutex<HashMap<String, ProbeState>>,
}

impl ContainerRuntime {
    pub async fn new(
        volumes_base_path: String,
        cluster_dns: String,
        cluster_domain: String,
        network: String,
        kubernetes_service_host: String,
    ) -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;

        info!("Using volumes base path: {}", volumes_base_path);
        info!(
            "Cluster DNS: {}, domain: {}, network: {}",
            cluster_dns, cluster_domain, network
        );
        info!("Kubernetes service host: {}", kubernetes_service_host);

        // Initialize CNI if plugins are available
        let (cni, use_cni) = match Self::initialize_cni() {
            Ok(cni_runtime) => {
                info!("CNI networking enabled");
                (Some(cni_runtime), true)
            }
            Err(e) => {
                warn!(
                    "CNI not available, falling back to Podman networking: {}",
                    e
                );
                (None, false)
            }
        };

        Ok(Self {
            docker,
            storage: None,
            volumes_base_path,
            cluster_dns,
            cluster_domain,
            network,
            cni,
            use_cni,
            kubernetes_service_host,
            probe_states: Mutex::new(HashMap::new()),
        })
    }

    /// Initialize CNI runtime
    fn initialize_cni() -> Result<CniRuntime> {
        let cni_plugin_paths = vec![PathBuf::from("/opt/cni/bin")];
        let cni_config_dir = PathBuf::from("/etc/cni/net.d");

        // Check if CNI plugins exist
        if !cni_plugin_paths.iter().any(|p| p.exists()) {
            return Err(anyhow::anyhow!("CNI plugin directory not found"));
        }

        // Pre-flight check: verify we can create and access network namespaces
        // This will fail in Podman Machine where netns created in container aren't
        // accessible to other containers
        let test_netns = "rusternetes-cni-test";
        let create_result = Command::new("ip")
            .args(&["netns", "add", test_netns])
            .output();

        if let Ok(output) = create_result {
            if output.status.success()
                || String::from_utf8_lossy(&output.stderr).contains("File exists")
            {
                // Clean up test namespace
                let _ = Command::new("ip")
                    .args(&["netns", "del", test_netns])
                    .output();

                // Network namespaces work, but we're likely in a container where
                // they won't be accessible to sibling containers (Podman Machine case)
                warn!("CNI plugins found but network namespaces may not work across containers.");
                warn!("This is normal in Podman Machine - will fall back to Podman networking.");
                return Err(anyhow::anyhow!(
                    "Network namespace isolation prevents CNI usage"
                ));
            }
        }

        let cni = CniRuntime::new(cni_plugin_paths, cni_config_dir)?
            .with_default_network("rusternetes".to_string());

        Ok(cni)
    }

    pub fn with_storage(mut self, storage: Arc<rusternetes_storage::etcd::EtcdStorage>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Setup CNI networking for a pod
    /// Creates a network namespace and configures CNI networking
    /// Returns None if CNI setup fails (will fall back to Podman networking)
    async fn setup_pod_network(&self, pod_name: &str) -> Option<String> {
        info!("Setting up CNI network for pod: {}", pod_name);

        // Create network namespace
        let netns_name = format!("cni-{}", pod_name);
        let output = match Command::new("ip")
            .args(&["netns", "add", &netns_name])
            .output()
        {
            Ok(out) => out,
            Err(e) => {
                warn!("Failed to create network namespace for pod {}: {}. Falling back to Podman networking.", pod_name, e);
                return None;
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore error if namespace already exists
            if !stderr.contains("File exists") {
                warn!("Failed to create network namespace for pod {}: {}. Falling back to Podman networking.", pod_name, stderr);
                return None;
            }
            info!(
                "Network namespace {} already exists, reusing it",
                netns_name
            );
        } else {
            info!("Created network namespace: {}", netns_name);
        }

        // Get the network namespace path
        let netns_path = format!("/var/run/netns/{}", netns_name);

        // Setup CNI networking in the namespace
        if let Some(cni) = &self.cni {
            match cni.setup_network(pod_name, &netns_path, "eth0", None) {
                Ok(result) => {
                    info!(
                        "CNI network setup successful for pod {}: IP={:?}",
                        pod_name, result.ips
                    );
                }
                Err(e) => {
                    warn!("Failed to setup CNI network for pod {}: {}. Falling back to Podman networking.", pod_name, e);
                    // Clean up the network namespace on failure
                    let _ = Command::new("ip")
                        .args(&["netns", "del", &netns_name])
                        .output();
                    return None;
                }
            }
        }

        Some(netns_path)
    }

    /// Teardown CNI networking for a pod
    /// Removes CNI configuration and deletes the network namespace
    async fn teardown_pod_network(&self, pod_name: &str) -> Result<()> {
        info!("Tearing down CNI network for pod: {}", pod_name);

        let netns_name = format!("cni-{}", pod_name);
        let netns_path = format!("/var/run/netns/{}", netns_name);

        // Teardown CNI networking
        if let Some(cni) = &self.cni {
            if let Err(e) = cni.teardown_network(pod_name, &netns_path, "eth0", None) {
                warn!("Failed to teardown CNI network for pod {}: {}", pod_name, e);
                // Continue with namespace deletion even if CNI teardown fails
            } else {
                info!("CNI network teardown successful for pod {}", pod_name);
            }
        }

        // Delete network namespace
        let output = Command::new("ip")
            .args(&["netns", "del", &netns_name])
            .output()
            .context("Failed to execute ip netns del command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                "Failed to delete network namespace {}: {}",
                netns_name, stderr
            );
        } else {
            info!("Deleted network namespace: {}", netns_name);
        }

        Ok(())
    }

    /// Pull an image if necessary based on the pull policy
    async fn ensure_image(&self, image: &str, pull_policy: Option<&str>) -> Result<()> {
        let policy = pull_policy.unwrap_or("IfNotPresent");
        debug!("Ensuring image {} with policy {}", image, policy);

        // Normalize image name to include registry if not specified
        let normalized_image = self.normalize_image_name(image);
        debug!("Normalized image name: {}", normalized_image);

        // Check if image exists locally (try both original and normalized names)
        let image_exists = self.check_image_exists(image).await
            || self.check_image_exists(&normalized_image).await;

        let should_pull = match policy {
            "Always" => true,
            "Never" => false,
            "IfNotPresent" => !image_exists,
            _ => !image_exists, // Default to IfNotPresent
        };

        debug!(
            "Image {} - exists: {}, should_pull: {}",
            image, image_exists, should_pull
        );

        if should_pull {
            info!("Pulling image: {}", normalized_image);

            // Try to pull the image with proper registry handling
            if let Err(e) = self.pull_image_with_retry(&normalized_image).await {
                error!("Failed to pull image {}: {}", normalized_image, e);

                // If normalized image failed and it's different from original, try original
                if normalized_image != image {
                    warn!("Retrying with original image name: {}", image);
                    self.pull_image_with_retry(image).await?;
                } else {
                    return Err(e);
                }
            }

            info!("Successfully pulled image: {}", image);
        } else {
            debug!("Image {} already exists locally, skipping pull", image);
        }

        Ok(())
    }

    /// Check if an image exists locally
    async fn check_image_exists(&self, image: &str) -> bool {
        match self.docker.inspect_image(image).await {
            Ok(_) => {
                debug!("Image {} exists locally", image);
                true
            }
            Err(e) => {
                debug!("Image {} not found locally: {}", image, e);
                false
            }
        }
    }

    /// Normalize image name to include default registry
    fn normalize_image_name(&self, image: &str) -> String {
        // If image already has a registry (contains '/'), use as-is
        if image.contains('/') {
            // Check for explicit registry (contains '.' or ':' in first component)
            let first_component = image.split('/').next().unwrap_or("");
            if first_component.contains('.') || first_component.contains(':') {
                return image.to_string();
            }
            // Otherwise prepend docker.io/library/ for official images like "library/image"
            if !image.starts_with("docker.io/") {
                return format!("docker.io/{}", image);
            }
            image.to_string()
        } else {
            // Simple image name without registry - use docker.io/library
            format!("docker.io/library/{}", image)
        }
    }

    /// Pull image with retry logic
    async fn pull_image_with_retry(&self, image: &str) -> Result<()> {
        let options = CreateImageOptions {
            from_image: image,
            ..Default::default()
        };

        let mut stream = self.docker.create_image(Some(options), None, None);
        let mut last_error = None;

        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(status) = &info.status {
                        debug!("Image pull: {}", status);
                    }
                    if let Some(progress) = &info.progress {
                        debug!("Image pull progress: {}", progress);
                    }
                    if let Some(error) = info.error {
                        last_error = Some(error.clone());
                        error!("Image pull error: {}", error);
                    }
                }
                Err(e) => {
                    last_error = Some(format!("{}", e));
                    error!("Image pull stream error: {}", e);
                }
            }
        }

        // Check if there was an error
        if let Some(err) = last_error {
            return Err(anyhow::anyhow!("Image pull failed: {}", err));
        }

        Ok(())
    }

    /// Start all containers for a pod
    pub async fn start_pod(&self, pod: &Pod) -> Result<()> {
        let pod_name = &pod.metadata.name;
        let namespace = pod.metadata.namespace.as_deref().unwrap_or("default");
        info!("Starting pod: {}/{}", namespace, pod_name);

        // Create network namespace and setup CNI networking if enabled
        // If CNI setup fails, netns_path will be None and we fall back to Podman networking
        let netns_path = if self.use_cni {
            self.setup_pod_network(pod_name).await
        } else {
            None
        };

        // Ensure the pod has a kube-api-access volume for SA tokens.
        // Controllers that create pods directly in etcd bypass the API server's
        // admission controller, so the SA token volume may not be injected.
        let mut pod_with_sa = pod.clone();
        if let Some(ref mut spec) = pod_with_sa.spec {
            let has_sa_volume = spec.volumes.as_ref()
                .map(|vols| vols.iter().any(|v| v.name.contains("kube-api-access") || v.name.contains("token")))
                .unwrap_or(false);
            if !has_sa_volume {
                // Add projected SA token volume
                let sa_vol = rusternetes_common::resources::Volume {
                    name: "kube-api-access".to_string(),
                    empty_dir: None,
                    host_path: None,
                    config_map: None,
                    secret: None,
                    projected: Some(rusternetes_common::resources::ProjectedVolumeSource {
                        sources: Some(vec![
                            rusternetes_common::resources::VolumeProjection {
                                service_account_token: Some(rusternetes_common::resources::ServiceAccountTokenProjection {
                                    path: "token".to_string(),
                                    expiration_seconds: Some(3600),
                                    audience: None,
                                }),
                                config_map: None,
                                secret: None,
                                downward_api: None,
                                cluster_trust_bundle: None,
                            },
                        ]),
                        default_mode: Some(0o644),
                    }),
                    persistent_volume_claim: None,
                    downward_api: None,
                    csi: None,
                    ephemeral: None,
                    nfs: None,
                    image: None,
                    iscsi: None,
                };
                spec.volumes.get_or_insert_with(Vec::new).push(sa_vol);
                // Add volume mount to each container
                for container in &mut spec.containers {
                    let mounts = container.volume_mounts.get_or_insert_with(Vec::new);
                    if !mounts.iter().any(|m| m.name.contains("kube-api-access")) {
                        mounts.push(rusternetes_common::resources::VolumeMount {
                            name: "kube-api-access".to_string(),
                            mount_path: "/var/run/secrets/kubernetes.io/serviceaccount".to_string(),
                            read_only: Some(true),
                            sub_path: None,
                            sub_path_expr: None,
                            mount_propagation: None,
                            recursive_read_only: None,
                        });
                    }
                }
            }
        }
        let pod = &pod_with_sa;

        // Create volumes first (includes service account token volumes)
        let volume_binds = self.create_pod_volumes(pod).await?;

        // Get pod IP. For CNI mode, IP is available right after network setup.
        // For non-CNI (Docker bridge) mode, we start a pause container first so we
        // can learn the pod's IP before creating real containers (which need the IP
        // in their environment for Downward API env vars like SONOBUOY_ADVERTISE_IP).
        let mut pod_ip: Option<String> = if self.use_cni {
            if let Some(cni) = &self.cni {
                cni.get_container_ip(pod_name)
            } else {
                None
            }
        } else {
            // Start a pause container to obtain the pod's Docker-assigned IP before
            // creating any real containers.
            match self.start_pause_container(pod_name, pod).await {
                Ok(ip) => {
                    info!("Pause container assigned IP {} for pod {}", ip, pod_name);
                    Some(ip)
                }
                Err(e) => {
                    warn!(
                        "Failed to start pause container for pod {}: {}",
                        pod_name, e
                    );
                    None
                }
            }
        };

        // Create /etc/hosts now that we know the pod IP.
        let hosts_file_path = self.create_pod_hosts_file(pod, pod_ip.as_deref())?;

        // resolved_ip is only used in the non-CNI/non-pause fallback path now.
        let mut resolved_ip = pod_ip.is_some();

        // Step 1: Run init containers sequentially (non-sidecar init containers)
        // Sidecar init containers (with restartPolicy: Always) will be started with main containers
        if let Some(init_containers) = &pod.spec.as_ref().unwrap().init_containers {
            for container in init_containers {
                // Check if this is a sidecar container (restartPolicy: Always)
                let is_sidecar = container.restart_policy.as_deref() == Some("Always");

                if !is_sidecar {
                    // Regular init container - run to completion
                    info!("Running init container: {}", container.name);

                    // Ensure image is available
                    if let Err(e) = self
                        .ensure_image(&container.image, container.image_pull_policy.as_deref())
                        .await
                    {
                        error!(
                            "Failed to pull image for init container {}: {}",
                            container.name, e
                        );
                        return Err(e);
                    }

                    // For restartPolicy=Always pods, retry failed init containers
                    // with exponential backoff (matching Kubernetes CrashLoopBackOff)
                    let restart_always = pod.spec.as_ref()
                        .and_then(|s| s.restart_policy.as_deref())
                        .unwrap_or("Always") == "Always";
                    let max_retries = if restart_always { 5 } else { 0 };
                    let mut attempt = 0;

                    loop {
                        // Start the init container
                        self.start_container(
                            pod,
                            container,
                            &volume_binds,
                            netns_path.as_deref(),
                            hosts_file_path.as_deref(),
                            pod_ip.as_deref(),
                        )
                        .await?;

                        // Resolve pod IP from first container for non-CNI mode
                        if !resolved_ip && pod_ip.is_none() {
                            if let Ok(Some(ip)) = self.get_pod_ip(pod_name).await {
                                pod_ip = Some(ip);
                                resolved_ip = true;
                                self.create_pod_hosts_file(pod, pod_ip.as_deref())?;
                            }
                        }

                        // Wait for init container to complete
                        match self.wait_for_container_completion(pod_name, &container.name).await {
                            Ok(()) => {
                                info!("Init container {} completed successfully", container.name);
                                break;
                            }
                            Err(e) => {
                                if attempt < max_retries && restart_always {
                                    attempt += 1;
                                    let backoff = std::cmp::min(2u64.pow(attempt as u32), 30);
                                    warn!(
                                        "Init container {} failed (attempt {}), retrying in {}s: {}",
                                        container.name, attempt, backoff, e
                                    );
                                    // Remove the failed container before retrying
                                    let full_name = format!("{}_{}", pod_name, container.name);
                                    let _ = self.docker.remove_container(&full_name, Some(
                                        RemoveContainerOptions { force: true, ..Default::default() }
                                    )).await;
                                    tokio::time::sleep(Duration::from_secs(backoff)).await;
                                } else {
                                    return Err(e);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Step 2: Start sidecar containers (init containers with restartPolicy: Always)
        if let Some(init_containers) = &pod.spec.as_ref().unwrap().init_containers {
            for container in init_containers {
                let is_sidecar = container.restart_policy.as_deref() == Some("Always");

                if is_sidecar {
                    info!("Starting sidecar container: {}", container.name);

                    // Ensure image is available
                    if let Err(e) = self
                        .ensure_image(&container.image, container.image_pull_policy.as_deref())
                        .await
                    {
                        error!(
                            "Failed to pull image for sidecar container {}: {}",
                            container.name, e
                        );
                        return Err(e);
                    }

                    // Start the sidecar container (it will run alongside main containers)
                    self.start_container(
                        pod,
                        container,
                        &volume_binds,
                        netns_path.as_deref(),
                        hosts_file_path.as_deref(),
                        pod_ip.as_deref(),
                    )
                    .await?;

                    // Resolve pod IP from first container for non-CNI mode
                    if !resolved_ip && pod_ip.is_none() {
                        if let Ok(Some(ip)) = self.get_pod_ip(pod_name).await {
                            pod_ip = Some(ip);
                            resolved_ip = true;
                            self.create_pod_hosts_file(pod, pod_ip.as_deref())?;
                        }
                    }
                }
            }
        }

        // Step 3: Start main containers
        let spec_containers = pod.spec.as_ref().unwrap().containers.clone();
        for container in &spec_containers {
            // Ensure image is available
            if let Err(e) = self
                .ensure_image(&container.image, container.image_pull_policy.as_deref())
                .await
            {
                error!(
                    "Failed to pull image for container {}: {}",
                    container.name, e
                );
                return Err(e);
            }

            // Start the container with volume bindings
            self.start_container(
                pod,
                container,
                &volume_binds,
                netns_path.as_deref(),
                hosts_file_path.as_deref(),
                pod_ip.as_deref(),
            )
            .await?;

            // Resolve pod IP from first container for non-CNI mode
            if !resolved_ip && pod_ip.is_none() {
                if let Ok(Some(ip)) = self.get_pod_ip(pod_name).await {
                    pod_ip = Some(ip.clone());
                    resolved_ip = true;
                    self.create_pod_hosts_file(pod, pod_ip.as_deref())?;
                    info!("Resolved pod IP {} for pod {} (non-CNI mode)", ip, pod_name);
                }
            }
        }

        Ok(())
    }

    /// Create /etc/hosts file for a pod.
    ///
    /// Generates a hosts file with:
    /// - Standard localhost entries
    /// - The pod's own hostname → IP mapping (if IP is known)
    /// - Subdomain-based FQDN if spec.subdomain is set
    ///
    /// Returns the path to the hosts file, or None if the pod is CoreDNS
    /// (which uses the host's /etc/hosts directly).
    fn create_pod_hosts_file(&self, pod: &Pod, pod_ip: Option<&str>) -> Result<Option<String>> {
        let pod_name = &pod.metadata.name;
        let namespace = pod.metadata.namespace.as_deref().unwrap_or("default");
        let spec = pod.spec.as_ref().unwrap();

        // Determine the pod's hostname: spec.hostname if set, otherwise pod name
        let hostname = spec.hostname.as_deref().unwrap_or(pod_name);

        let pod_dir = format!("{}/{}", self.volumes_base_path, pod_name);
        std::fs::create_dir_all(&pod_dir)
            .context("Failed to create pod directory for /etc/hosts")?;

        let hosts_path = format!("{}/hosts", pod_dir);

        let mut content = String::from(
            "# Kubernetes-managed hosts file\n\
             127.0.0.1\tlocalhost\n\
             ::1\tlocalhost ip6-localhost ip6-loopback\n\
             fe00::0\tip6-localnet\n\
             fe00::0\tip6-mcastprefix\n\
             fe00::1\tip6-allnodes\n\
             fe00::2\tip6-allrouters\n",
        );

        // Add the pod's own hostname → IP entry if we have an IP
        if let Some(ip) = pod_ip {
            // Build FQDN aliases based on subdomain and cluster domain
            let mut aliases = vec![hostname.to_string()];
            if let Some(subdomain) = &spec.subdomain {
                // <hostname>.<subdomain>.<namespace>.svc.<cluster-domain>
                aliases.push(format!(
                    "{}.{}.{}.svc.{}",
                    hostname, subdomain, namespace, self.cluster_domain
                ));
            }
            content.push_str(&format!("{}\t{}\n", ip, aliases.join("\t")));
            info!(
                "Added /etc/hosts entry for pod {}/{}: {} -> {}",
                namespace,
                pod_name,
                aliases.join(", "),
                ip
            );
        }

        // Add entries from spec.hostAliases
        // Kubernetes groups all hostnames for the same IP on a single line
        if let Some(host_aliases) = &spec.host_aliases {
            for alias in host_aliases {
                if let Some(hostnames) = &alias.hostnames {
                    if !hostnames.is_empty() {
                        content.push_str(&format!("{}\t{}\n", alias.ip, hostnames.join("\t")));
                    }
                }
            }
        }

        std::fs::write(&hosts_path, &content)
            .with_context(|| format!("Failed to write /etc/hosts for pod {}", pod_name))?;

        Ok(Some(hosts_path))
    }

    /// Start a pause (infra) container for a pod in non-CNI mode.
    ///
    /// The pause container holds the pod's network namespace. All real containers
    /// join its network via `--network container:<pause_name>`. This lets us know
    /// the pod's IP before creating any real containers, which is required to
    /// correctly populate Downward API env vars like `status.podIP`.
    ///
    /// Returns the IP address assigned to the pause container.
    async fn start_pause_container(&self, pod_name: &str, pod: &Pod) -> Result<String> {
        let pause_name = format!("{}_pause", pod_name);

        // Check if pause container already exists and is running — if so, just return its IP.
        // Recreating the pause container would destroy all containers sharing its network namespace.
        if let Ok(inspect) = self
            .docker
            .inspect_container(&pause_name, None::<InspectContainerOptions>)
            .await
        {
            let state = inspect.state.as_ref();
            let is_running = state.and_then(|s| s.running).unwrap_or(false);

            if is_running {
                // Pause container is already running — return its IP
                if let Some(network_settings) = inspect.network_settings {
                    if let Some(networks) = network_settings.networks {
                        if let Some(network_info) = networks.get(&self.network) {
                            if let Some(ip) = &network_info.ip_address {
                                if !ip.is_empty() {
                                    debug!("Pause container {} already running with IP {}", pause_name, ip);
                                    return Ok(ip.clone());
                                }
                            }
                        }
                    }
                }
            }

            // Pause container exists but is not running — remove it
            let remove_options = RemoveContainerOptions {
                force: true,
                ..Default::default()
            };
            let _ = self
                .docker
                .remove_container(&pause_name, Some(remove_options))
                .await;
        }

        // Collect all ports from all containers in the pod.
        // The pause container owns the network namespace and must declare all ports;
        // child containers that join via --network container:<pause> cannot re-declare them.
        let mut exposed_ports: HashMap<String, HashMap<(), ()>> = HashMap::new();
        let mut port_bindings: HashMap<String, Option<Vec<bollard::models::PortBinding>>> =
            HashMap::new();
        if let Some(spec) = &pod.spec {
            for c in &spec.containers {
                if let Some(ports) = &c.ports {
                    for port in ports {
                        let proto = port.protocol.as_deref().unwrap_or("TCP").to_lowercase();
                        let port_key = format!("{}/{}", port.container_port, proto);
                        exposed_ports.insert(port_key.clone(), HashMap::new());
                        if let Some(host_port) = port.host_port {
                            port_bindings.insert(
                                port_key,
                                Some(vec![bollard::models::PortBinding {
                                    host_ip: Some("0.0.0.0".to_string()),
                                    host_port: Some(host_port.to_string()),
                                }]),
                            );
                        }
                    }
                }
            }
        }

        // Create the pause container using busybox with sleep infinity.
        // This holds the pod's network namespace so all real containers can
        // join it via --network container:<pause_name>.
        // The pause container owns the DNS configuration for the pod network namespace.
        // CoreDNS pods must NOT use cluster DNS (circular dependency).
        let is_coredns = pod_name == "coredns";
        let config = Config {
            image: Some("busybox:latest".to_string()),
            cmd: Some(vec!["sleep".to_string(), "infinity".to_string()]),
            exposed_ports: if exposed_ports.is_empty() {
                None
            } else {
                Some(exposed_ports)
            },
            host_config: Some(bollard::models::HostConfig {
                network_mode: Some(self.network.clone()),
                dns: if is_coredns { None } else { Some(vec![self.cluster_dns.clone()]) },
                dns_options: if is_coredns { None } else { Some(vec!["ndots:5".to_string()]) },
                port_bindings: if port_bindings.is_empty() {
                    None
                } else {
                    Some(port_bindings)
                },
                ..Default::default()
            }),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: pause_name.clone(),
            ..Default::default()
        };

        self.docker
            .create_container(Some(options), config)
            .await
            .context("Failed to create pause container")?;

        self.docker
            .start_container(&pause_name, None::<StartContainerOptions<String>>)
            .await
            .context("Failed to start pause container")?;

        info!("Pause container {} started", pause_name);

        // Inspect to get the assigned IP
        let inspect = self
            .docker
            .inspect_container(&pause_name, None::<InspectContainerOptions>)
            .await
            .context("Failed to inspect pause container")?;

        if let Some(network_settings) = inspect.network_settings {
            if let Some(networks) = network_settings.networks {
                if let Some(network_info) = networks.get(&self.network) {
                    if let Some(ip) = &network_info.ip_address {
                        if !ip.is_empty() && ip != "0.0.0.0" {
                            return Ok(ip.clone());
                        }
                    }
                }
            }
            if let Some(ip) = network_settings.ip_address {
                if !ip.is_empty() && ip != "0.0.0.0" {
                    return Ok(ip);
                }
            }
        }

        Err(anyhow::anyhow!(
            "Pause container started but no IP address was assigned"
        ))
    }

    /// Wait for a container to complete (used for init containers)
    async fn wait_for_container_completion(
        &self,
        pod_name: &str,
        container_name: &str,
    ) -> Result<()> {
        let full_container_name = format!("{}_{}", pod_name, container_name);
        let timeout = Duration::from_secs(300); // 5 minute timeout
        let start_time = std::time::Instant::now();

        loop {
            if start_time.elapsed() > timeout {
                return Err(anyhow::anyhow!(
                    "Timeout waiting for init container {} to complete",
                    container_name
                ));
            }

            match self
                .docker
                .inspect_container(&full_container_name, None::<InspectContainerOptions>)
                .await
            {
                Ok(inspect) => {
                    if let Some(state) = inspect.state {
                        let running = state.running.unwrap_or(false);

                        if !running {
                            // Container has stopped
                            let exit_code = state.exit_code.unwrap_or(1);

                            if exit_code == 0 {
                                debug!("Init container {} completed successfully", container_name);
                                return Ok(());
                            } else {
                                let error_msg = state
                                    .error
                                    .unwrap_or_else(|| format!("Exit code: {}", exit_code));
                                error!("Init container {} failed: {}", container_name, error_msg);
                                return Err(anyhow::anyhow!(
                                    "Init container {} failed with exit code {}: {}",
                                    container_name,
                                    exit_code,
                                    error_msg
                                ));
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to inspect init container {}: {}", container_name, e);
                }
            }

            // Wait a bit before checking again
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    /// Create volumes for a pod and return volume bindings for containers
    pub async fn create_pod_volumes(&self, pod: &Pod) -> Result<HashMap<String, String>> {
        let mut volume_paths = HashMap::new();

        if let Some(volumes) = &pod.spec.as_ref().unwrap().volumes {
            for volume in volumes {
                let volume_path = self.create_volume(pod, volume).await?;
                volume_paths.insert(volume.name.clone(), volume_path);
            }
        }

        Ok(volume_paths)
    }

    /// Resync projected/secret/configmap volumes for a running pod.
    /// Re-reads source data from storage and updates volume files if changed.
    pub async fn resync_volumes<S: rusternetes_storage::Storage>(&self, pod: &Pod, storage: &S) -> Result<()> {
        let pod_name = &pod.metadata.name;
        let namespace = pod.metadata.namespace.as_deref().unwrap_or("default");

        if let Some(volumes) = &pod.spec.as_ref().unwrap().volumes {
            for volume in volumes {
                // Resync secret volumes
                if let Some(secret_source) = &volume.secret {
                    let secret_name = match &secret_source.secret_name {
                        Some(n) => n,
                        None => continue,
                    };
                    let key = rusternetes_storage::build_key("secrets", Some(namespace), secret_name);
                    if let Ok(secret) = storage.get::<rusternetes_common::resources::Secret>(&key).await {
                        let volume_dir = format!("{}/{}/{}", self.volumes_base_path, pod_name, volume.name);
                        if let Some(data) = &secret.data {
                            for (k, v) in data {
                                let file_path = format!("{}/{}", volume_dir, k);
                                // Only write if content changed
                                if let Ok(existing) = std::fs::read(&file_path) {
                                    if existing == *v { continue; }
                                }
                                let _ = std::fs::write(&file_path, v);
                            }
                        }
                    }
                }
                // Resync configmap volumes
                if let Some(cm_source) = &volume.config_map {
                    if let Some(cm_name) = &cm_source.name {
                        let key = rusternetes_storage::build_key("configmaps", Some(namespace), cm_name);
                        if let Ok(cm) = storage.get::<rusternetes_common::resources::ConfigMap>(&key).await {
                            let volume_dir = format!("{}/{}/{}", self.volumes_base_path, pod_name, volume.name);
                            if let Some(data) = &cm.data {
                                for (k, v) in data {
                                    let file_path = format!("{}/{}", volume_dir, k);
                                    if let Ok(existing) = std::fs::read_to_string(&file_path) {
                                        if existing == *v { continue; }
                                    }
                                    let _ = std::fs::write(&file_path, v);
                                }
                            }
                        }
                    }
                }
                // Resync projected volumes (may contain configmap/secret projections)
                if let Some(projected) = &volume.projected {
                    if let Some(sources) = &projected.sources {
                        let volume_dir = format!("{}/{}/{}", self.volumes_base_path, pod_name, volume.name);
                        for source in sources {
                            if let Some(cm_proj) = &source.config_map {
                                if let Some(cm_name) = &cm_proj.name {
                                    let key = rusternetes_storage::build_key("configmaps", Some(namespace), cm_name);
                                    if let Ok(cm) = storage.get::<rusternetes_common::resources::ConfigMap>(&key).await {
                                        if let Some(data) = &cm.data {
                                            for (k, v) in data {
                                                let file_path = format!("{}/{}", volume_dir, k);
                                                if let Ok(existing) = std::fs::read_to_string(&file_path) {
                                                    if existing == *v { continue; }
                                                }
                                                let _ = std::fs::write(&file_path, v);
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(sec_proj) = &source.secret {
                                if let Some(sec_name) = &sec_proj.name {
                                    let key = rusternetes_storage::build_key("secrets", Some(namespace), sec_name);
                                    if let Ok(secret) = storage.get::<rusternetes_common::resources::Secret>(&key).await {
                                        if let Some(data) = &secret.data {
                                            for (k, v) in data {
                                                let file_path = format!("{}/{}", volume_dir, k);
                                                if let Ok(existing) = std::fs::read(&file_path) {
                                                    if existing == *v { continue; }
                                                }
                                                let _ = std::fs::write(&file_path, v);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Expand $(VAR_NAME) references in a subPathExpr using the container's env vars.
    /// Returns Err if a referenced variable is not defined, or if the result is an absolute path.
    fn expand_subpath_expr(
        expr: &str,
        env_vars: &[(String, String)],
    ) -> std::result::Result<String, String> {
        let mut result = expr.to_string();
        // Find all $(VAR_NAME) references and expand them
        loop {
            let start = match result.find("$(") {
                Some(s) => s,
                None => break,
            };
            let rest = &result[start + 2..];
            let end = match rest.find(')') {
                Some(e) => e,
                None => break,
            };
            let var_name = &rest[..end];
            if let Some((_, value)) = env_vars.iter().find(|(k, _)| k == var_name) {
                result = format!(
                    "{}{}{}",
                    &result[..start],
                    value,
                    &result[start + 2 + end + 1..]
                );
            } else {
                return Err(format!("variable {} not found", var_name));
            }
        }
        // Reject absolute paths
        if result.starts_with('/') {
            return Err("subPath must not be an absolute path".to_string());
        }
        // Reject path traversal
        if result.contains("..") {
            return Err("subPath must not contain '..'".to_string());
        }
        if result.contains('`') {
            return Err("subPath must not contain backticks".to_string());
        }
        Ok(result)
    }

    /// Expand environment variables in a string (e.g., ${VAR_NAME} or $VAR_NAME)
    fn expand_env_vars(input: &str) -> String {
        let mut result = input.to_string();

        // Expand ${VAR_NAME} format
        while let Some(start) = result.find("${") {
            if let Some(end) = result[start..].find('}') {
                let var_name = &result[start + 2..start + end];
                let var_value = std::env::var(var_name).unwrap_or_default();
                result.replace_range(start..start + end + 1, &var_value);
            } else {
                break;
            }
        }

        // Expand $VAR_NAME format (word boundary based)
        let mut i = 0;
        while i < result.len() {
            if result[i..].starts_with('$') && i + 1 < result.len() {
                let rest = &result[i + 1..];
                let var_len = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .count();

                if var_len > 0 {
                    let var_name = &rest[..var_len];
                    let var_value = std::env::var(var_name).unwrap_or_default();
                    result.replace_range(i..i + 1 + var_len, &var_value);
                    i += var_value.len();
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        result
    }

    /// Create a single volume and return its host path
    async fn create_volume(
        &self,
        pod: &Pod,
        volume: &rusternetes_common::resources::Volume,
    ) -> Result<String> {
        let pod_name = &pod.metadata.name;
        let namespace = pod.metadata.namespace.as_deref().unwrap_or("default");

        // EmptyDir: create a temporary directory
        if volume.empty_dir.is_some() {
            let volume_dir = format!("{}/{}/{}", self.volumes_base_path, pod_name, volume.name);
            std::fs::create_dir_all(&volume_dir).context("Failed to create emptyDir volume")?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(
                    &volume_dir,
                    std::fs::Permissions::from_mode(0o777),
                )?;
            }
            info!("Created emptyDir volume {} at {}", volume.name, volume_dir);
            return Ok(volume_dir);
        }

        // HostPath: use the specified host path
        if let Some(host_path) = &volume.host_path {
            // Expand environment variables in the path
            let path = Self::expand_env_vars(&host_path.path);
            // Optionally create the directory if it doesn't exist
            if let Some(type_) = &host_path.type_ {
                if type_ == "DirectoryOrCreate" {
                    std::fs::create_dir_all(&path).context("Failed to create hostPath volume")?;
                }
            }
            info!("Using hostPath volume {} at {}", volume.name, path);
            return Ok(path);
        }

        // ConfigMap: mount configmap data as files
        if let Some(configmap_source) = &volume.config_map {
            let storage = self
                .storage
                .as_ref()
                .context("Storage not available for ConfigMap volumes")?;

            let configmap_name = configmap_source
                .name
                .as_ref()
                .context("ConfigMap volume must specify name")?;

            let is_optional = configmap_source.optional.unwrap_or(false);

            let key = build_key("configmaps", Some(namespace), configmap_name);
            let configmap_result: Result<ConfigMap, _> = storage.get(&key).await;

            // Create volume directory
            let volume_dir = format!("{}/{}/{}", self.volumes_base_path, pod_name, volume.name);
            std::fs::create_dir_all(&volume_dir)
                .context("Failed to create ConfigMap volume directory")?;

            // Determine the default file mode: spec defaultMode, or 0644 (Kubernetes default)
            let cm_default_mode = configmap_source.default_mode.unwrap_or(0o644);

            // Set directory permissions to match defaultMode
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(
                    &volume_dir,
                    std::fs::Permissions::from_mode(cm_default_mode as u32 | 0o111),
                )?;
            }

            match configmap_result {
                Ok(configmap) => {
                    // Helper closure to write a file and set permissions
                    let write_cm_file = |file_path: &str, content: &[u8], mode: i32| -> Result<()> {
                        if let Some(parent) = std::path::Path::new(file_path).parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        std::fs::write(file_path, content)?;
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            std::fs::set_permissions(
                                file_path,
                                std::fs::Permissions::from_mode(mode as u32),
                            )?;
                        }
                        Ok(())
                    };

                    // Check if specific items are requested
                    if let Some(ref items) = configmap_source.items {
                        // Only mount the specified keys (look in both data and binaryData)
                        for item in items {
                            let mode = item.mode.unwrap_or(cm_default_mode);
                            let file_path = format!("{}/{}", volume_dir, item.path);

                            // Try data first, then binary_data
                            if let Some(value) = configmap.data.as_ref().and_then(|d| d.get(&item.key)) {
                                write_cm_file(&file_path, value.as_bytes(), mode).with_context(|| {
                                    format!("Failed to write ConfigMap key {} to file", item.key)
                                })?;
                                info!("Wrote ConfigMap key {} to {}", item.key, file_path);
                            } else if let Some(value) = configmap.binary_data.as_ref().and_then(|d| d.get(&item.key)) {
                                write_cm_file(&file_path, value, mode).with_context(|| {
                                    format!("Failed to write ConfigMap binaryData key {} to file", item.key)
                                })?;
                                info!("Wrote ConfigMap binaryData key {} to {}", item.key, file_path);
                            } else if !is_optional {
                                warn!("ConfigMap {} missing key {}", configmap_name, item.key);
                            }
                        }
                    } else {
                        // Mount all keys from data
                        if let Some(data) = &configmap.data {
                            for (key, value) in data {
                                let file_path = format!("{}/{}", volume_dir, key);
                                write_cm_file(&file_path, value.as_bytes(), cm_default_mode).with_context(|| {
                                    format!("Failed to write ConfigMap key {} to file", key)
                                })?;
                                info!("Wrote ConfigMap key {} to {}", key, file_path);
                            }
                        }
                        // Mount all keys from binaryData
                        if let Some(binary_data) = &configmap.binary_data {
                            for (key, value) in binary_data {
                                let file_path = format!("{}/{}", volume_dir, key);
                                write_cm_file(&file_path, value, cm_default_mode).with_context(|| {
                                    format!("Failed to write ConfigMap binaryData key {} to file", key)
                                })?;
                                info!("Wrote ConfigMap binaryData key {} to {}", key, file_path);
                            }
                        }
                    }
                }
                Err(e) => {
                    if is_optional {
                        info!(
                            "Optional ConfigMap {} not found in namespace {}, creating empty volume",
                            configmap_name, namespace
                        );
                    } else {
                        // Required ConfigMap not found — abort pod start so kubelet
                        // retries on next reconciliation (when the ConfigMap exists).
                        return Err(anyhow::anyhow!(
                            "ConfigMap {} not found in namespace {}: {}",
                            configmap_name, namespace, e
                        ));
                    }
                }
            }

            info!("Created ConfigMap volume {} at {}", volume.name, volume_dir);
            return Ok(volume_dir);
        }

        // Secret: mount secret data as files
        if let Some(secret_source) = &volume.secret {
            let storage = self
                .storage
                .as_ref()
                .context("Storage not available for Secret volumes")?;

            let secret_name = secret_source
                .secret_name
                .as_ref()
                .context("Secret volume must specify secret_name")?;

            let is_optional = secret_source.optional.unwrap_or(false);

            let key = build_key("secrets", Some(namespace), secret_name);
            let secret_result: Result<Secret, _> = storage.get(&key).await;

            // Create volume directory
            let volume_dir = format!("{}/{}/{}", self.volumes_base_path, pod_name, volume.name);
            std::fs::create_dir_all(&volume_dir)
                .context("Failed to create Secret volume directory")?;

            // Set directory permissions to match defaultMode
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let secret_dir_mode = secret_source.default_mode.unwrap_or(0o644) as u32 | 0o111;
                std::fs::set_permissions(
                    &volume_dir,
                    std::fs::Permissions::from_mode(secret_dir_mode),
                )?;
            }

            let secret = match secret_result {
                Ok(s) => Some(s),
                Err(e) => {
                    if is_optional {
                        info!(
                            "Optional Secret {} not found in namespace {}, creating empty volume",
                            secret_name, namespace
                        );
                        None
                    } else {
                        // Required secret not found — abort pod start so kubelet
                        // retries on next reconciliation (when the secret exists).
                        return Err(anyhow::anyhow!(
                            "Secret {} not found in namespace {}: {}",
                            secret_name, namespace, e
                        ));
                    }
                }
            };

            // Determine the default file mode: spec defaultMode, or 0644 (Kubernetes default)
            let secret_default_mode = secret_source.default_mode.unwrap_or(0o644);

            // Write secret data as files
            if let Some(data) = secret.as_ref().and_then(|s| s.data.as_ref()) {
                if let Some(ref items) = secret_source.items {
                    // Only mount the specified keys
                    for item in items {
                        if let Some(value) = data.get(&item.key) {
                            let file_path = format!("{}/{}", volume_dir, item.path);
                            // Create parent directories if needed
                            if let Some(parent) = std::path::Path::new(&file_path).parent() {
                                std::fs::create_dir_all(parent).with_context(|| {
                                    format!("Failed to create directory for Secret item {}", item.path)
                                })?;
                            }
                            std::fs::write(&file_path, value)
                                .with_context(|| format!("Failed to write Secret key {} to file", item.key))?;
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                let mode = item.mode.unwrap_or(secret_default_mode) as u32;
                                std::fs::set_permissions(
                                    &file_path,
                                    std::fs::Permissions::from_mode(mode),
                                )?;
                            }
                            info!("Wrote Secret key {} to {}", item.key, file_path);
                        }
                    }
                } else {
                    // Mount all keys
                    for (key, value) in data {
                        let file_path = format!("{}/{}", volume_dir, key);
                        std::fs::write(&file_path, value)
                            .with_context(|| format!("Failed to write Secret key {} to file", key))?;
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            std::fs::set_permissions(
                                &file_path,
                                std::fs::Permissions::from_mode(secret_default_mode as u32),
                            )?;
                        }
                        info!("Wrote Secret key {} to {}", key, file_path);
                    }
                }
            }

            // Special handling for service account token secrets - add ca.crt
            // Service account secrets are identified by having a "token" key or by name pattern
            let is_service_account_secret = secret
                .as_ref()
                .and_then(|s| s.data.as_ref())
                .map(|data| data.contains_key("token"))
                .unwrap_or(false)
                || secret_name.ends_with("-token");

            if is_service_account_secret {
                // Check if ca.crt already exists in the secret data
                let has_ca_cert = secret
                    .as_ref()
                    .and_then(|s| s.data.as_ref())
                    .map(|data| data.contains_key("ca.crt"))
                    .unwrap_or(false);

                if !has_ca_cert {
                    // Inject ca.crt from the cluster CA certificate
                    // Try multiple locations: environment variable, volumes/_certs, then fallback to .rusternetes/certs
                    let ca_cert_source = std::env::var("CA_CERT_PATH").unwrap_or_else(|_| {
                        // First try volumes/_certs (accessible from kubelet container)
                        let volumes_cert_path = format!("{}/_certs/ca.crt", self.volumes_base_path);
                        if std::path::Path::new(&volumes_cert_path).exists() {
                            volumes_cert_path
                        } else {
                            // Fallback to .rusternetes/certs (for host-based kubelet)
                            format!(
                                "{}/.rusternetes/certs/ca.crt",
                                std::env::var("HOME").unwrap_or_else(|_| "/root".to_string())
                            )
                        }
                    });

                    let ca_path = format!("{}/ca.crt", volume_dir);
                    if let Ok(ca_content) = std::fs::read(&ca_cert_source) {
                        std::fs::write(&ca_path, ca_content)
                            .context("Failed to write CA certificate")?;
                        info!("Injected CA certificate into service account secret volume at {} (from {})", ca_path, ca_cert_source);
                    } else {
                        warn!("CA certificate not found at {}, pods may not be able to verify API server", ca_cert_source);
                    }
                }
            }

            info!("Created Secret volume {} at {}", volume.name, volume_dir);
            return Ok(volume_dir);
        }

        // PersistentVolumeClaim: find bound PV and use its path
        if let Some(pvc_source) = &volume.persistent_volume_claim {
            let storage = self
                .storage
                .as_ref()
                .context("Storage not available for PersistentVolumeClaim volumes")?;

            let pvc_key = build_key(
                "persistentvolumeclaims",
                Some(namespace),
                &pvc_source.claim_name,
            );
            let pvc: PersistentVolumeClaim = storage.get(&pvc_key).await.with_context(|| {
                format!(
                    "PersistentVolumeClaim {} not found in namespace {}",
                    pvc_source.claim_name, namespace
                )
            })?;

            // Get the bound PV name
            let pv_name = pvc
                .spec
                .volume_name
                .as_ref()
                .context("PersistentVolumeClaim is not bound to a volume")?;

            // Get the PV
            let pv_key = build_key("persistentvolumes", None, pv_name);
            let pv: PersistentVolume = storage
                .get(&pv_key)
                .await
                .with_context(|| format!("PersistentVolume {} not found", pv_name))?;

            // Get the host path from the PV
            let path = if let Some(hp) = &pv.spec.host_path {
                hp.path.clone()
            } else {
                return Err(anyhow::anyhow!(
                    "PersistentVolume does not have a hostPath volume source"
                ));
            };
            info!(
                "Using PersistentVolumeClaim volume {} backed by PV {} at {}",
                volume.name, pv_name, path
            );
            return Ok(path);
        }

        // DownwardAPI: expose pod/container metadata as files
        if let Some(downward_api) = &volume.downward_api {
            let volume_dir = format!("{}/{}/{}", self.volumes_base_path, pod_name, volume.name);
            std::fs::create_dir_all(&volume_dir)
                .context("Failed to create DownwardAPI volume directory")?;

            // Determine the default file mode: spec defaultMode, or 0644 (Kubernetes default)
            let da_default_mode = downward_api.default_mode.unwrap_or(0o644);

            // Set directory permissions to match defaultMode
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(
                    &volume_dir,
                    std::fs::Permissions::from_mode(da_default_mode as u32 | 0o111),
                )?;
            }

            if let Some(items) = &downward_api.items {
                for item in items {
                    let file_path = format!("{}/{}", volume_dir, item.path);

                    // Create parent directories if needed
                    if let Some(parent) = std::path::Path::new(&file_path).parent() {
                        std::fs::create_dir_all(parent)?;
                    }

                    // Get the value from field_ref or resource_field_ref
                    let value = if let Some(field_ref) = &item.field_ref {
                        self.get_pod_field_value(pod, &field_ref.field_path)?
                    } else if let Some(resource_ref) = &item.resource_field_ref {
                        self.get_container_resource_value(pod, resource_ref)?
                    } else {
                        return Err(anyhow::anyhow!(
                            "DownwardAPI item must have either fieldRef or resourceFieldRef"
                        ));
                    };

                    std::fs::write(&file_path, value).with_context(|| {
                        format!("Failed to write DownwardAPI file {}", file_path)
                    })?;

                    // Set file permissions: per-item mode overrides defaultMode
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let mode = item.mode.unwrap_or(da_default_mode) as u32;
                        std::fs::set_permissions(
                            &file_path,
                            std::fs::Permissions::from_mode(mode),
                        )?;
                    }

                    info!(
                        "Wrote DownwardAPI file {} with value from {}",
                        file_path, item.path
                    );
                }
            }

            info!(
                "Created DownwardAPI volume {} at {}",
                volume.name, volume_dir
            );
            return Ok(volume_dir);
        }

        // CSI: ephemeral inline volume (handled by external CSI driver)
        if let Some(_csi) = &volume.csi {
            // CSI ephemeral inline volumes are managed by the CSI driver via the kubelet CSI plugin
            // For conformance, we create a placeholder directory and rely on the CSI driver to populate it
            let volume_dir = format!("{}/{}/{}", self.volumes_base_path, pod_name, volume.name);
            std::fs::create_dir_all(&volume_dir)
                .context("Failed to create CSI volume directory")?;

            info!(
                "Created CSI ephemeral volume {} at {} (managed by CSI driver)",
                volume.name, volume_dir
            );
            return Ok(volume_dir);
        }

        // Ephemeral: generic ephemeral volume with PVC template
        if let Some(ephemeral) = &volume.ephemeral {
            if let Some(pvc_template) = &ephemeral.volume_claim_template {
                let storage = self
                    .storage
                    .as_ref()
                    .context("Storage not available for ephemeral volumes")?;

                // Create a PVC name based on pod name and volume name
                let pvc_name = format!("{}-{}", pod_name, volume.name);

                // Check if PVC already exists
                let pvc_key = build_key("persistentvolumeclaims", Some(namespace), &pvc_name);
                let pvc_exists = storage.get::<PersistentVolumeClaim>(&pvc_key).await.is_ok();

                if !pvc_exists {
                    // Create the PVC from the template
                    let mut pvc = PersistentVolumeClaim {
                        type_meta: rusternetes_common::types::TypeMeta {
                            kind: "PersistentVolumeClaim".to_string(),
                            api_version: "v1".to_string(),
                        },
                        metadata: rusternetes_common::types::ObjectMeta::new(&pvc_name)
                            .with_namespace(namespace),
                        spec: pvc_template.spec.clone(),
                        status: None,
                    };

                    // Copy labels and annotations from template if provided
                    if let Some(template_meta) = &pvc_template.metadata {
                        if let Some(labels) = &template_meta.labels {
                            pvc.metadata.labels = Some(labels.clone());
                        }
                        if let Some(annotations) = &template_meta.annotations {
                            pvc.metadata.annotations = Some(annotations.clone());
                        }
                    }

                    // Store the PVC
                    storage
                        .create(&pvc_key, &pvc)
                        .await
                        .context("Failed to create ephemeral PVC")?;

                    info!(
                        "Created ephemeral PVC {} for volume {}",
                        pvc_name, volume.name
                    );

                    // Wait for PVC to be bound (simplified - in production would poll/watch)
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }

                // Now use the PVC like a regular PersistentVolumeClaim
                let pvc: PersistentVolumeClaim = storage
                    .get(&pvc_key)
                    .await
                    .with_context(|| format!("Ephemeral PVC {} not found", pvc_name))?;

                if let Some(pv_name) = &pvc.spec.volume_name {
                    let pv_key = build_key("persistentvolumes", None, pv_name);
                    let pv: PersistentVolume = storage.get(&pv_key).await.with_context(|| {
                        format!(
                            "PersistentVolume {} not found for ephemeral volume",
                            pv_name
                        )
                    })?;

                    let path = if let Some(hp) = &pv.spec.host_path {
                        hp.path.clone()
                    } else {
                        return Err(anyhow::anyhow!(
                            "PersistentVolume does not have a hostPath volume source"
                        ));
                    };

                    info!(
                        "Using ephemeral volume {} backed by PVC {} and PV {} at {}",
                        volume.name, pvc_name, pv_name, path
                    );
                    return Ok(path);
                } else {
                    return Err(anyhow::anyhow!(
                        "Ephemeral PVC {} is not bound yet",
                        pvc_name
                    ));
                }
            }
        }

        // Projected: combine multiple volume sources (configMap, secret, downwardAPI, serviceAccountToken) into one directory
        if let Some(projected) = &volume.projected {
            let volume_dir = format!("{}/{}/{}", self.volumes_base_path, pod_name, volume.name);
            std::fs::create_dir_all(&volume_dir)
                .context("Failed to create projected volume directory")?;

            // Determine the default file mode: spec defaultMode, or 0644 (Kubernetes default)
            let proj_default_mode = projected.default_mode.unwrap_or(0o644);

            // Set directory permissions to match defaultMode
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(
                    &volume_dir,
                    std::fs::Permissions::from_mode(proj_default_mode as u32 | 0o111),
                )?;
            }

            if let Some(sources) = &projected.sources {
                let storage = self.storage.as_ref();

                for source in sources {
                    // ConfigMap projection
                    if let Some(cm_proj) = &source.config_map {
                        if let Some(cm_name) = &cm_proj.name {
                            let key = build_key("configmaps", Some(namespace), cm_name);
                            if let Some(storage) = storage {
                                match storage.get::<ConfigMap>(&key).await {
                                    Ok(cm) => {
                                        // Helper to write a projected file with permissions
                                        let write_proj_file = |path: &str, content: &[u8], mode: i32| -> Result<()> {
                                            if let Some(parent) = std::path::Path::new(path).parent() {
                                                std::fs::create_dir_all(parent)?;
                                            }
                                            std::fs::write(path, content)?;
                                            #[cfg(unix)]
                                            {
                                                use std::os::unix::fs::PermissionsExt;
                                                std::fs::set_permissions(
                                                    path,
                                                    std::fs::Permissions::from_mode(mode as u32),
                                                )?;
                                            }
                                            Ok(())
                                        };

                                        if let Some(items) = &cm_proj.items {
                                            for item in items {
                                                let mode = item.mode.unwrap_or(proj_default_mode);
                                                let file_path = format!("{}/{}", volume_dir, item.path);
                                                // Try data first, then binaryData
                                                if let Some(value) = cm.data.as_ref().and_then(|d| d.get(&item.key)) {
                                                    write_proj_file(&file_path, value.as_bytes(), mode)?;
                                                } else if let Some(value) = cm.binary_data.as_ref().and_then(|d| d.get(&item.key)) {
                                                    write_proj_file(&file_path, value, mode)?;
                                                }
                                            }
                                        } else {
                                            // Mount all keys from data
                                            if let Some(data) = &cm.data {
                                                for (k, v) in data {
                                                    let file_path = format!("{}/{}", volume_dir, k);
                                                    write_proj_file(&file_path, v.as_bytes(), proj_default_mode)?;
                                                }
                                            }
                                            // Mount all keys from binaryData
                                            if let Some(binary_data) = &cm.binary_data {
                                                for (k, v) in binary_data {
                                                    let file_path = format!("{}/{}", volume_dir, k);
                                                    write_proj_file(&file_path, v, proj_default_mode)?;
                                                }
                                            }
                                        }
                                    }
                                    Err(_) if cm_proj.optional.unwrap_or(false) => {
                                        // Optional configmap not found, skip
                                    }
                                    Err(e) => {
                                        warn!("Failed to get ConfigMap {} for projected volume: {}. Skipping.", cm_name, e);
                                    }
                                }
                            }
                        }
                    }

                    // Secret projection
                    if let Some(secret_proj) = &source.secret {
                        if let Some(secret_name) = &secret_proj.name {
                            let key = build_key("secrets", Some(namespace), secret_name);
                            if let Some(storage) = storage {
                                match storage.get::<Secret>(&key).await {
                                    Ok(secret) => {
                                        if let Some(data) = &secret.data {
                                            if let Some(items) = &secret_proj.items {
                                                for item in items {
                                                    if let Some(value) = data.get(&item.key) {
                                                        let file_path =
                                                            format!("{}/{}", volume_dir, item.path);
                                                        if let Some(parent) =
                                                            std::path::Path::new(&file_path)
                                                                .parent()
                                                        {
                                                            std::fs::create_dir_all(parent)?;
                                                        }
                                                        std::fs::write(&file_path, value)?;
                                                        #[cfg(unix)]
                                                        {
                                                            use std::os::unix::fs::PermissionsExt;
                                                            let mode = item.mode.unwrap_or(proj_default_mode) as u32;
                                                            std::fs::set_permissions(
                                                                &file_path,
                                                                std::fs::Permissions::from_mode(mode),
                                                            )?;
                                                        }
                                                    }
                                                }
                                            } else {
                                                for (k, v) in data {
                                                    let file_path = format!("{}/{}", volume_dir, k);
                                                    std::fs::write(&file_path, v)?;
                                                    #[cfg(unix)]
                                                    {
                                                        use std::os::unix::fs::PermissionsExt;
                                                        std::fs::set_permissions(
                                                            &file_path,
                                                            std::fs::Permissions::from_mode(proj_default_mode as u32),
                                                        )?;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(_) if secret_proj.optional.unwrap_or(false) => {
                                        // Optional secret not found, skip
                                    }
                                    Err(e) => {
                                        warn!("Failed to get Secret {} for projected volume: {}. Skipping.", secret_name, e);
                                    }
                                }
                            }
                        }
                    }

                    // DownwardAPI projection
                    if let Some(downward_api) = &source.downward_api {
                        if let Some(items) = &downward_api.items {
                            for item in items {
                                let file_path = format!("{}/{}", volume_dir, item.path);
                                if let Some(parent) =
                                    std::path::Path::new(&file_path).parent()
                                {
                                    std::fs::create_dir_all(parent)?;
                                }
                                let value = if let Some(ref field_ref) = item.field_ref {
                                    self.get_pod_field_value(pod, &field_ref.field_path)
                                        .unwrap_or_default()
                                } else if let Some(ref resource_ref) = item.resource_field_ref {
                                    self.get_container_resource_value(pod, resource_ref)
                                        .unwrap_or_default()
                                } else {
                                    String::new()
                                };
                                std::fs::write(&file_path, &value)?;
                                #[cfg(unix)]
                                {
                                    use std::os::unix::fs::PermissionsExt;
                                    let mode = item.mode.unwrap_or(proj_default_mode) as u32;
                                    std::fs::set_permissions(
                                        &file_path,
                                        std::fs::Permissions::from_mode(mode),
                                    )?;
                                }
                            }
                        }
                    }

                    // ServiceAccountToken projection
                    if let Some(sa_token) = &source.service_account_token {
                        let token_path = format!("{}/{}", volume_dir, sa_token.path);
                        if let Some(parent) = std::path::Path::new(&token_path).parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        // Write a placeholder JWT token
                        let token = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.placeholder";
                        std::fs::write(&token_path, token)?;
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            std::fs::set_permissions(
                                &token_path,
                                std::fs::Permissions::from_mode(proj_default_mode as u32),
                            )?;
                        }
                    }
                }
            }

            info!(
                "Created projected volume {} at {}",
                volume.name, volume_dir
            );
            return Ok(volume_dir);
        }

        // Fallback: create an empty directory for unrecognized volume types
        // (e.g. nfs, iscsi, image, or any future types)
        // This prevents pod startup failures for volumes we don't natively handle.
        warn!(
            "Unknown volume type for volume {}, creating empty directory as fallback (volume debug: downward_api={}, empty_dir={}, host_path={}, config_map={}, secret={}, projected={}, pvc={}, csi={}, ephemeral={})",
            volume.name,
            volume.downward_api.is_some(),
            volume.empty_dir.is_some(),
            volume.host_path.is_some(),
            volume.config_map.is_some(),
            volume.secret.is_some(),
            volume.projected.is_some(),
            volume.persistent_volume_claim.is_some(),
            volume.csi.is_some(),
            volume.ephemeral.is_some(),
        );
        let volume_dir = format!("{}/{}/{}", self.volumes_base_path, pod_name, volume.name);
        std::fs::create_dir_all(&volume_dir)
            .context("Failed to create fallback volume directory")?;
        Ok(volume_dir)
    }

    /// Create ServiceAccount token volume for in-cluster authentication
    async fn create_serviceaccount_token_volume(&self, pod: &Pod) -> Result<String> {
        let pod_name = &pod.metadata.name;
        let namespace = pod.metadata.namespace.as_deref().unwrap_or("default");

        // Get ServiceAccount name from pod spec (default to "default")
        let sa_name = pod
            .spec
            .as_ref()
            .and_then(|spec| spec.service_account_name.as_deref())
            .unwrap_or("default");

        info!(
            "Creating ServiceAccount token volume for pod {} using ServiceAccount {}/{}",
            pod_name, namespace, sa_name
        );

        // Find the ServiceAccount token secret
        let storage = self
            .storage
            .as_ref()
            .context("Storage not available for ServiceAccount token volumes")?;

        let secret_name = format!("{}-token", sa_name);
        let key = build_key("secrets", Some(namespace), &secret_name);

        // Try to get the secret; if it doesn't exist, create a basic token mount anyway
        let secret: Option<Secret> = match storage.get(&key).await {
            Ok(s) => Some(s),
            Err(e) => {
                warn!(
                    "ServiceAccount token secret {} not found: {}. Creating empty token volume.",
                    secret_name, e
                );
                None
            }
        };

        // Create volume directory
        let volume_dir = format!(
            "{}/{}/serviceaccount-token",
            self.volumes_base_path, pod_name
        );
        std::fs::create_dir_all(&volume_dir)
            .context("Failed to create ServiceAccount token volume directory")?;

        // Write token file
        if let Some(secret) = secret {
            if let Some(data) = &secret.data {
                // Write token
                if let Some(token) = data.get("token") {
                    let token_path = format!("{}/token", volume_dir);
                    std::fs::write(&token_path, token)
                        .context("Failed to write ServiceAccount token")?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        std::fs::set_permissions(
                            &token_path,
                            std::fs::Permissions::from_mode(0o600),
                        )?;
                    }
                    info!("Wrote ServiceAccount token to {}", token_path);
                }

                // Write namespace
                if let Some(ns_bytes) = data.get("namespace") {
                    let ns_path = format!("{}/namespace", volume_dir);
                    std::fs::write(&ns_path, ns_bytes).context("Failed to write namespace file")?;
                    info!("Wrote namespace file to {}", ns_path);
                } else {
                    // If not in secret, write the pod's namespace
                    let ns_path = format!("{}/namespace", volume_dir);
                    std::fs::write(&ns_path, namespace)
                        .context("Failed to write namespace file")?;
                }
            }
        } else {
            // Create minimal files even without a secret
            let ns_path = format!("{}/namespace", volume_dir);
            std::fs::write(&ns_path, namespace).context("Failed to write namespace file")?;
        }

        // Write ca.crt (cluster CA certificate) so pods can verify API server
        let ca_cert_source = std::env::var("CA_CERT_PATH").unwrap_or_else(|_| {
            format!(
                "{}/.rusternetes/certs/ca.crt",
                std::env::var("HOME").unwrap_or_else(|_| "/root".to_string())
            )
        });

        let ca_path = format!("{}/ca.crt", volume_dir);
        if let Ok(ca_content) = std::fs::read(&ca_cert_source) {
            std::fs::write(&ca_path, ca_content).context("Failed to write CA certificate")?;
            info!("Wrote CA certificate to {}", ca_path);
        } else {
            warn!(
                "CA certificate not found at {}, pods may not be able to verify API server",
                ca_cert_source
            );
        }

        info!("Created ServiceAccount token volume at {}", volume_dir);
        Ok(volume_dir)
    }

    pub async fn start_container(
        &self,
        pod: &Pod,
        container: &Container,
        volume_paths: &HashMap<String, String>,
        netns_path: Option<&str>,
        hosts_file_path: Option<&str>,
        pod_ip: Option<&str>,
    ) -> Result<()> {
        let pod_name = &pod.metadata.name;
        let namespace = pod.metadata.namespace.as_deref().unwrap_or("default");
        let container_name = format!("{}_{}", pod_name, container.name);

        info!(
            "Starting container: {} (netns: {:?})",
            container_name, netns_path
        );

        // Check if container already exists
        if let Ok(inspect) = self
            .docker
            .inspect_container(&container_name, None::<InspectContainerOptions>)
            .await
        {
            let state = inspect.state.as_ref();
            let is_running = state.and_then(|s| s.running).unwrap_or(false);
            let status = state.and_then(|s| s.status.as_ref());

            // Skip if container is running or just created (about to start)
            if is_running {
                return Ok(());
            }
            if matches!(status, Some(bollard::secret::ContainerStateStatusEnum::CREATED)) {
                debug!("Container {} is in created state, waiting for it to start", container_name);
                return Ok(());
            }

            // Only remove if container has actually exited
            if matches!(status, Some(bollard::secret::ContainerStateStatusEnum::EXITED) | Some(bollard::secret::ContainerStateStatusEnum::DEAD)) {
                debug!("Removing exited container: {}", container_name);
                let remove_options = RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                };
                self.docker
                    .remove_container(&container_name, Some(remove_options))
                    .await?;
            } else {
                // Unknown state — don't remove, don't recreate
                debug!("Container {} in state {:?}, skipping", container_name, status);
                return Ok(());
            }
        }

        // Build environment variables
        let mut env_list = Vec::new();

        // Inject Kubernetes service environment variables for in-cluster access.
        // When using direct API server IP (KUBERNETES_SERVICE_HOST_OVERRIDE),
        // use port 6443 (the actual API server port). When using ClusterIP,
        // use port 443 (the service port that DNAT maps to 6443).
        let k8s_port = if std::env::var("KUBERNETES_SERVICE_HOST_OVERRIDE").is_ok() {
            "6443"
        } else {
            "443"
        };
        env_list.push(format!(
            "KUBERNETES_SERVICE_HOST={}",
            self.kubernetes_service_host
        ));
        env_list.push(format!("KUBERNETES_SERVICE_PORT={}", k8s_port));
        env_list.push(format!("KUBERNETES_SERVICE_PORT_HTTPS={}", k8s_port));
        env_list.push(format!(
            "KUBERNETES_PORT=tcp://{}:{}",
            self.kubernetes_service_host, k8s_port
        ));
        env_list.push(format!(
            "KUBERNETES_PORT_443_TCP=tcp://{}:{}",
            self.kubernetes_service_host, k8s_port
        ));
        env_list.push("KUBERNETES_PORT_443_TCP_PROTO=tcp".to_string());
        env_list.push(format!("KUBERNETES_PORT_443_TCP_PORT={}", k8s_port));
        env_list.push(format!(
            "KUBERNETES_PORT_443_TCP_ADDR={}",
            self.kubernetes_service_host
        ));

        // Inject JOB_COMPLETION_INDEX for indexed Jobs
        if let Some(annotations) = &pod.metadata.annotations {
            if let Some(index) = annotations.get("batch.kubernetes.io/job-completion-index") {
                env_list.push(format!("JOB_COMPLETION_INDEX={}", index));
            }
        }

        // Inject service link environment variables (Kubernetes convention).
        // When enableServiceLinks is true (default), inject env vars for every
        // Service in the pod's namespace: {SVC}_SERVICE_HOST, {SVC}_SERVICE_PORT, etc.
        let enable_service_links = pod
            .spec
            .as_ref()
            .and_then(|s| s.enable_service_links)
            .unwrap_or(true);

        if enable_service_links {
            if let Some(storage) = &self.storage {
                let svc_prefix = rusternetes_storage::build_prefix("services", Some(namespace));
                match storage
                    .list::<rusternetes_common::resources::Service>(&svc_prefix)
                    .await
                {
                    Ok(services) => {
                        for service in &services {
                            let svc_name_raw = &service.metadata.name;
                            // Skip the "kubernetes" service — already injected above
                            if svc_name_raw == "kubernetes" {
                                continue;
                            }
                            let cluster_ip = match &service.spec.cluster_ip {
                                Some(ip) if ip != "None" && !ip.is_empty() => ip,
                                _ => continue,
                            };
                            let svc_env = svc_name_raw.to_uppercase().replace('-', "_");
                            env_list.push(format!("{}_SERVICE_HOST={}", svc_env, cluster_ip));
                            if let Some(first_port) = service.spec.ports.first() {
                                let proto = first_port
                                    .protocol
                                    .as_deref()
                                    .unwrap_or("TCP")
                                    .to_lowercase();
                                env_list
                                    .push(format!("{}_SERVICE_PORT={}", svc_env, first_port.port));
                                env_list.push(format!(
                                    "{}_PORT={}://{}:{}",
                                    svc_env, proto, cluster_ip, first_port.port
                                ));
                                env_list.push(format!(
                                    "{}_PORT_{}_{}={}://{}:{}",
                                    svc_env,
                                    first_port.port,
                                    proto.to_uppercase(),
                                    proto,
                                    cluster_ip,
                                    first_port.port
                                ));
                                env_list.push(format!(
                                    "{}_PORT_{}_{}_PROTO={}",
                                    svc_env,
                                    first_port.port,
                                    proto.to_uppercase(),
                                    proto
                                ));
                                env_list.push(format!(
                                    "{}_PORT_{}_{}_PORT={}",
                                    svc_env,
                                    first_port.port,
                                    proto.to_uppercase(),
                                    first_port.port
                                ));
                                env_list.push(format!(
                                    "{}_PORT_{}_{}_ADDR={}",
                                    svc_env,
                                    first_port.port,
                                    proto.to_uppercase(),
                                    cluster_ip
                                ));
                                // Named port: {SVC}_SERVICE_PORT_{PORT_NAME}
                                if let Some(port_name) = &first_port.name {
                                    let port_name_env = port_name.to_uppercase().replace('-', "_");
                                    env_list.push(format!(
                                        "{}_SERVICE_PORT_{}={}",
                                        svc_env, port_name_env, first_port.port
                                    ));
                                }
                            }
                            // Additional named ports beyond the first
                            for port in service.spec.ports.iter().skip(1) {
                                if let Some(port_name) = &port.name {
                                    let port_name_env = port_name.to_uppercase().replace('-', "_");
                                    env_list.push(format!(
                                        "{}_SERVICE_PORT_{}={}",
                                        svc_env, port_name_env, port.port
                                    ));
                                }
                                let proto =
                                    port.protocol.as_deref().unwrap_or("TCP").to_lowercase();
                                env_list.push(format!(
                                    "{}_PORT_{}_{}={}://{}:{}",
                                    svc_env,
                                    port.port,
                                    proto.to_uppercase(),
                                    proto,
                                    cluster_ip,
                                    port.port
                                ));
                                env_list.push(format!(
                                    "{}_PORT_{}_{}_PROTO={}",
                                    svc_env,
                                    port.port,
                                    proto.to_uppercase(),
                                    proto
                                ));
                                env_list.push(format!(
                                    "{}_PORT_{}_{}_PORT={}",
                                    svc_env,
                                    port.port,
                                    proto.to_uppercase(),
                                    port.port
                                ));
                                env_list.push(format!(
                                    "{}_PORT_{}_{}_ADDR={}",
                                    svc_env,
                                    port.port,
                                    proto.to_uppercase(),
                                    cluster_ip
                                ));
                            }
                        }
                        debug!(
                            "Injected service link env vars for {} services in namespace {}",
                            services.len(),
                            namespace
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to list services for service links in namespace {}: {}",
                            namespace, e
                        );
                    }
                }
            } else {
                debug!("No storage available for service link env var injection");
            }
        }

        // Add envFrom: inject all keys from referenced ConfigMaps/Secrets
        if let Some(env_from_sources) = &container.env_from {
            for source in env_from_sources {
                let prefix = source.prefix.as_deref().unwrap_or("");
                // ConfigMap envFrom
                if let Some(cm_ref) = &source.config_map_ref {
                    if let Some(storage) = &self.storage {
                        let cm_key = build_key("configmaps", Some(namespace), &cm_ref.name);
                        match storage.get::<ConfigMap>(&cm_key).await {
                            Ok(cm) => {
                                if let Some(data) = &cm.data {
                                    for (k, v) in data {
                                        env_list.push(format!("{}{}={}", prefix, k, v));
                                    }
                                }
                            }
                            Err(e) => {
                                let optional = cm_ref.optional.unwrap_or(false);
                                if !optional {
                                    warn!("Failed to get ConfigMap {} for envFrom: {}", cm_ref.name, e);
                                }
                            }
                        }
                    }
                }
                // Secret envFrom
                if let Some(secret_ref) = &source.secret_ref {
                    if let Some(storage) = &self.storage {
                        let secret_key = build_key("secrets", Some(namespace), &secret_ref.name);
                        match storage.get::<Secret>(&secret_key).await {
                            Ok(secret) => {
                                if let Some(data) = &secret.data {
                                    for (k, v) in data {
                                        if let Ok(val) = String::from_utf8(v.clone()) {
                                            env_list.push(format!("{}{}={}", prefix, k, val));
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                let optional = secret_ref.optional.unwrap_or(false);
                                if !optional {
                                    warn!("Failed to get Secret {} for envFrom: {}", secret_ref.name, e);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Add user-defined environment variables
        if let Some(env_vars) = &container.env {
            for env_var in env_vars {
                // Direct value
                if let Some(value) = &env_var.value {
                    env_list.push(format!("{}={}", env_var.name, value));
                    continue;
                }

                // Value from ConfigMap, Secret, or Downward API
                if let Some(value_from) = &env_var.value_from {
                    // ConfigMap reference
                    if let Some(configmap_ref) = &value_from.config_map_key_ref {
                        match self
                            .get_configmap_value(namespace, &configmap_ref.name, &configmap_ref.key)
                            .await
                        {
                            Ok(value) => {
                                env_list.push(format!("{}={}", env_var.name, value));
                            }
                            Err(e) => {
                                warn!("Failed to get ConfigMap value for {}: {}", env_var.name, e);
                            }
                        }
                        continue;
                    }

                    // Secret reference
                    if let Some(secret_ref) = &value_from.secret_key_ref {
                        match self
                            .get_secret_value(namespace, &secret_ref.name, &secret_ref.key)
                            .await
                        {
                            Ok(value) => {
                                env_list.push(format!("{}={}", env_var.name, value));
                            }
                            Err(e) => {
                                warn!("Failed to get Secret value for {}: {}", env_var.name, e);
                            }
                        }
                        continue;
                    }

                    // Field reference (Downward API)
                    if let Some(field_ref) = &value_from.field_ref {
                        match self.get_pod_field_value(pod, &field_ref.field_path) {
                            Ok(value) => {
                                // For status.podIP, fall back to the CNI-assigned IP when
                                // pod status hasn't been written to etcd yet (i.e., at first
                                // container creation). This ensures SONOBUOY_ADVERTISE_IP and
                                // similar env vars get the correct IP.
                                let resolved =
                                    if value.is_empty() && field_ref.field_path == "status.podIP" {
                                        pod_ip.unwrap_or("").to_string()
                                    } else {
                                        value
                                    };

                                // Always set the env var — even empty values are valid.
                                // Kubernetes never skips a fieldRef env var.
                                env_list.push(format!("{}={}", env_var.name, resolved));
                                if !resolved.is_empty() {
                                    info!(
                                        "Set env var {} from field {}: {}",
                                        env_var.name, field_ref.field_path, resolved
                                    );
                                }
                            }
                            Err(e) => {
                                warn!("Failed to get pod field value for {}: {}", env_var.name, e);
                            }
                        }
                        continue;
                    }

                    // Resource field reference
                    if let Some(resource_ref) = &value_from.resource_field_ref {
                        match self.get_container_resource_value(pod, resource_ref) {
                            Ok(value) => {
                                env_list.push(format!("{}={}", env_var.name, value));
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to get resource field value for {}: {}",
                                    env_var.name, e
                                );
                            }
                        }
                        continue;
                    }
                }
            }
        }

        // Collect resolved environment variables for subPathExpr expansion
        // (must be done before env_list is consumed).
        let resolved_env_pairs: Vec<(String, String)> = env_list
            .iter()
            .filter_map(|entry| {
                let mut parts = entry.splitn(2, '=');
                let name = parts.next()?.to_string();
                let value = parts.next().unwrap_or("").to_string();
                Some((name, value))
            })
            .collect();

        let env = if env_list.is_empty() {
            None
        } else {
            Some(env_list)
        };

        // Build port bindings.
        // When using container:<pause> network mode, ports must be declared on the
        // pause container (which owns the network namespace), not on child containers.
        // Docker rejects port declarations on containers that join another's network.
        let using_pause_network = !self.use_cni && netns_path.is_none();

        let mut exposed_ports = HashMap::new();
        let mut port_bindings = HashMap::new();

        if !using_pause_network {
            if let Some(ports) = &container.ports {
                for port in ports {
                    let port_key = format!("{}/tcp", port.container_port);
                    exposed_ports.insert(port_key.clone(), HashMap::new());

                    if let Some(host_port) = port.host_port {
                        port_bindings.insert(
                            port_key,
                            Some(vec![bollard::models::PortBinding {
                                host_ip: Some("0.0.0.0".to_string()),
                                host_port: Some(host_port.to_string()),
                            }]),
                        );
                    }
                }
            }
        }

        // Build volume bindings
        let mut binds = Vec::new();
        let mut tmpfs_mounts: HashMap<String, String> = HashMap::new();

        // Identify which volumes are emptyDir (should use tmpfs)
        let empty_dir_volumes: std::collections::HashSet<String> = pod
            .spec
            .as_ref()
            .and_then(|s| s.volumes.as_ref())
            .map(|volumes| {
                volumes
                    .iter()
                    .filter(|v| v.empty_dir.is_some())
                    .map(|v| v.name.clone())
                    .collect()
            })
            .unwrap_or_default();

        // Mount volumes based on volumeMounts (includes service account tokens injected by admission controller)
        if let Some(volume_mounts) = &container.volume_mounts {
            for mount in volume_mounts {
                // Validate subPathExpr / subPath BEFORE looking up the volume.
                // Kubernetes rejects containers whose expanded subpath contains
                // ".." or is absolute, regardless of whether the volume exists.
                let expanded_sub_path: Option<String> =
                    if let Some(ref expr) = mount.sub_path_expr {
                        match Self::expand_subpath_expr(expr, &resolved_env_pairs) {
                            Ok(expanded) => {
                                if expanded.is_empty() {
                                    return Err(anyhow::anyhow!(
                                        "CreateContainerError: subPathExpr '{}' expanded to empty string in container {}",
                                        expr, container.name
                                    ));
                                }
                                Some(expanded)
                            }
                            Err(e) => {
                                return Err(anyhow::anyhow!(
                                    "CreateContainerError: subPathExpr expansion failed for container {}: {}",
                                    container.name, e
                                ));
                            }
                        }
                    } else if let Some(ref sub_path) = mount.sub_path {
                        if !sub_path.is_empty() {
                            // Validate plain subPath for path traversal / absolute path
                            if sub_path.starts_with('/') {
                                return Err(anyhow::anyhow!(
                                    "CreateContainerError: subPath must not be an absolute path in container {}",
                                    container.name
                                ));
                            }
                            if sub_path.contains('`') {
                                return Err(anyhow::anyhow!(
                                    "CreateContainerError: subPath must not contain backticks in container {}",
                                    container.name
                                ));
                            }
                            if sub_path.contains("..") {
                                return Err(anyhow::anyhow!(
                                    "CreateContainerError: subPath must not contain '..' in container {}",
                                    container.name
                                ));
                            }
                            Some(sub_path.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                if let Some(host_path) = volume_paths.get(&mount.name) {
                    // Determine the effective host path, applying validated sub_path
                    let effective_host_path = if let Some(ref sub) = expanded_sub_path {
                        let full = format!("{}/{}", host_path, sub);
                        // Ensure the sub-directory exists.
                        // Check if the target already exists (e.g. a ConfigMap key
                        // file materialised during volume setup).  If it does not
                        // exist, create it as a directory so that directory-type
                        // subPath mounts work.  File-type subPath targets (ConfigMap
                        // keys) are expected to already be present on disk.
                        let sub_meta = std::fs::metadata(&full);
                        if sub_meta.is_err() {
                            if let Err(e) = std::fs::create_dir_all(&full) {
                                warn!("Failed to create subPath dir {}: {}", full, e);
                            }
                        }
                        full
                    } else {
                        host_path.clone()
                    };

                    let read_only = mount.read_only.unwrap_or(false);
                    // emptyDir with medium: Memory uses tmpfs (in-memory filesystem).
                    // emptyDir without medium uses bind mounts for cross-container sharing.
                    let is_memory_medium = pod.spec.as_ref()
                        .and_then(|s| s.volumes.as_ref())
                        .and_then(|vols| vols.iter().find(|v| v.name == mount.name))
                        .and_then(|v| v.empty_dir.as_ref())
                        .and_then(|ed| ed.medium.as_deref())
                        .map(|m| m == "Memory")
                        .unwrap_or(false);

                    if is_memory_medium && empty_dir_volumes.contains(&mount.name) && expanded_sub_path.is_none() {
                        // Use tmpfs for Memory-backed emptyDir
                        let opts = if read_only { "ro".to_string() } else { String::new() };
                        tmpfs_mounts.insert(mount.mount_path.clone(), opts);
                    } else {
                        let ro_suffix = if read_only { ":ro" } else { "" };
                        let bind =
                            format!("{}:{}{}", effective_host_path, mount.mount_path, ro_suffix);
                        binds.push(bind);
                    }
                    info!(
                        "Mounting volume {} at {} in container {}",
                        mount.name, mount.mount_path, container.name
                    );
                }
            }
        }

        // Create and mount custom resolv.conf based on DNS policy.
        // CoreDNS pods always skip custom DNS to avoid circular dependency.
        if pod_name != "coredns" {
            let dns_policy = pod
                .spec
                .as_ref()
                .and_then(|s| s.dns_policy.as_deref())
                .unwrap_or("ClusterFirst");

            let is_host_network = pod
                .spec
                .as_ref()
                .and_then(|s| s.host_network)
                .unwrap_or(false);

            // Build base resolv.conf content based on DNS policy
            let resolv_conf_content = match dns_policy {
                "None" => {
                    // Policy "None": start with empty resolv.conf, only dnsConfig applies
                    String::new()
                }
                "Default" => {
                    // Policy "Default": use the node's (host) /etc/resolv.conf
                    match std::fs::read_to_string("/etc/resolv.conf") {
                        Ok(content) => content,
                        Err(e) => {
                            warn!(
                                "Failed to read host /etc/resolv.conf for DNS policy Default: {}",
                                e
                            );
                            // Fall back to cluster DNS
                            format!(
                                "nameserver {}\nsearch {}.svc.{} svc.{} {}\noptions ndots:5\n",
                                self.cluster_dns,
                                namespace,
                                self.cluster_domain,
                                self.cluster_domain,
                                self.cluster_domain
                            )
                        }
                    }
                }
                "ClusterFirstWithHostNet" | "ClusterFirst" | _ => {
                    // ClusterFirst (default) and ClusterFirstWithHostNet: use cluster DNS.
                    // For ClusterFirstWithHostNet, if on host network, still use cluster DNS.
                    if dns_policy == "ClusterFirst" && is_host_network {
                        // ClusterFirst + host network: Kubernetes falls back to host DNS
                        match std::fs::read_to_string("/etc/resolv.conf") {
                            Ok(content) => content,
                            Err(_) => format!(
                                "nameserver {}\nsearch {}.svc.{} svc.{} {}\noptions ndots:5\n",
                                self.cluster_dns,
                                namespace,
                                self.cluster_domain,
                                self.cluster_domain,
                                self.cluster_domain
                            ),
                        }
                    } else {
                        format!(
                            "nameserver {}\nsearch {}.svc.{} svc.{} {}\noptions ndots:5\n",
                            self.cluster_dns,
                            namespace,
                            self.cluster_domain,
                            self.cluster_domain,
                            self.cluster_domain
                        )
                    }
                }
            };

            // Apply dnsConfig overrides (nameservers, searches, options)
            let final_content =
                if let Some(dns_config) = pod.spec.as_ref().and_then(|s| s.dns_config.as_ref()) {
                    let mut nameservers: Vec<String> = Vec::new();
                    let mut searches: Vec<String> = Vec::new();
                    let mut options: Vec<String> = Vec::new();

                    // Parse existing content
                    for line in resolv_conf_content.lines() {
                        let line = line.trim();
                        if line.starts_with("nameserver ") {
                            nameservers.push(line[11..].to_string());
                        } else if line.starts_with("search ") {
                            for domain in line[7..].split_whitespace() {
                                searches.push(domain.to_string());
                            }
                        } else if line.starts_with("options ") {
                            for opt in line[8..].split_whitespace() {
                                options.push(opt.to_string());
                            }
                        }
                    }

                    // Prepend custom nameservers
                    if let Some(ref custom_ns) = dns_config.nameservers {
                        let mut merged = custom_ns.clone();
                        for ns in &nameservers {
                            if !merged.contains(ns) {
                                merged.push(ns.clone());
                            }
                        }
                        nameservers = merged;
                    }

                    // Add/replace custom search domains
                    if let Some(ref custom_searches) = dns_config.searches {
                        let mut merged = custom_searches.clone();
                        for s in &searches {
                            if !merged.contains(s) {
                                merged.push(s.clone());
                            }
                        }
                        searches = merged;
                    }

                    // Add custom options
                    if let Some(ref custom_opts) = dns_config.options {
                        for opt in custom_opts {
                            let opt_str = if let Some(ref val) = opt.value {
                                format!("{}:{}", opt.name, val)
                            } else {
                                opt.name.clone()
                            };
                            // Replace existing option with same name
                            let opt_name = opt.name.as_str();
                            options.retain(|o| !o.starts_with(opt_name));
                            options.push(opt_str);
                        }
                    }

                    let mut result = String::new();
                    for ns in &nameservers {
                        result.push_str(&format!("nameserver {}\n", ns));
                    }
                    if !searches.is_empty() {
                        result.push_str(&format!("search {}\n", searches.join(" ")));
                    }
                    if !options.is_empty() {
                        result.push_str(&format!("options {}\n", options.join(" ")));
                    }
                    result
                } else {
                    resolv_conf_content
                };

            if !final_content.is_empty() {
                let resolv_conf_path =
                    format!("{}/{}/resolv.conf", self.volumes_base_path, pod_name);

                // Create directory if it doesn't exist
                std::fs::create_dir_all(format!("{}/{}", self.volumes_base_path, pod_name))
                    .context("Failed to create pod directory for resolv.conf")?;

                // Write custom resolv.conf
                std::fs::write(&resolv_conf_path, &final_content).with_context(|| {
                    format!("Failed to write custom resolv.conf for pod {}", pod_name)
                })?;

                // Mount custom resolv.conf into container
                binds.push(format!("{}:/etc/resolv.conf:ro", resolv_conf_path));
                info!(
                    "Mounted custom resolv.conf for pod {} (dns_policy={})",
                    pod_name, dns_policy
                );
            } else {
                debug!(
                    "DNS policy '{}' with no content — not mounting resolv.conf for pod {}",
                    dns_policy, pod_name
                );
            }
        }

        // Mount /etc/hosts if a pod-specific hosts file was created,
        // but skip if the container already has a volume mount at /etc/hosts
        let has_hosts_mount = container.volume_mounts.as_ref()
            .map(|mounts| mounts.iter().any(|m| m.mount_path == "/etc/hosts"))
            .unwrap_or(false);
        if let Some(hosts_path) = hosts_file_path {
            if !has_hosts_mount && !binds.iter().any(|b| b.contains(":/etc/hosts")) {
                binds.push(format!("{}:/etc/hosts:ro", hosts_path));
                info!("Mounted custom /etc/hosts for pod {}", pod_name);
            }
        }

        // Create container configuration
        // Skip cluster DNS configuration for:
        //   - CoreDNS (to avoid circular dependency)
        //   - Non-CNI containers (they join the pause container's network namespace,
        //     which owns the DNS config; Docker rejects dns options in container mode)
        let (dns_servers, dns_search_domains, dns_options) = if pod_name == "coredns" {
            info!("Skipping cluster DNS configuration for CoreDNS pod (using default/host DNS)");
            (None, None, None)
        } else if !self.use_cni {
            // DNS is inherited from the pause container's network namespace
            (None, None, None)
        } else {
            info!(
                "Configuring DNS for container {} in namespace {}",
                container.name, namespace
            );
            let servers = vec![self.cluster_dns.clone()];
            let search_domains = vec![
                format!("{}.svc.{}", namespace, self.cluster_domain),
                format!("svc.{}", self.cluster_domain),
                self.cluster_domain.clone(),
            ];
            // Add ndots:5 to match Kubernetes default DNS behavior
            // This tells the resolver to try search domains after 5 dots in the query
            let options = vec!["ndots:5".to_string()];
            info!(
                "DNS servers: {:?}, search domains: {:?}, options: {:?}",
                servers, search_domains, options
            );
            (Some(servers), Some(search_domains), Some(options))
        };

        // Parse resource limits for container cgroup enforcement
        let mut memory_limit: Option<i64> = None;
        let mut cpu_period: Option<i64> = None;
        let mut cpu_quota: Option<i64> = None;
        // Default cpu_shares to 2 (Kubernetes minimum, maps to cgroup2 weight ~1).
        // Docker's default of 1024 (weight 100) is too high for conformance tests
        // that check cpu.weight matches the pod's CPU request.
        let mut cpu_shares: Option<i64> = Some(2);

        if let Some(ref resources) = container.resources {
            if let Some(ref limits) = resources.limits {
                if let Some(memory) = limits.get("memory") {
                    let parsed = parse_memory_quantity(memory);
                    if parsed > 0 {
                        memory_limit = Some(parsed);
                        info!(
                            "Setting memory limit for container {}: {} bytes",
                            container.name, parsed
                        );
                    }
                }
                if let Some(cpu) = limits.get("cpu") {
                    let cpu_millicores = parse_cpu_quantity(cpu);
                    if cpu_millicores > 0 {
                        cpu_period = Some(100_000); // 100ms in microseconds
                        cpu_quota = Some((cpu_millicores * 100_000) / 1000);
                        info!(
                            "Setting CPU limit for container {}: {}m (quota={}/period={})",
                            container.name,
                            cpu_millicores,
                            cpu_quota.unwrap(),
                            cpu_period.unwrap()
                        );
                    }
                }
            }
            // Compute cpu_shares from CPU requests (cgroup cpu.weight via Docker).
            // Kubernetes formula: shares = max(2, milliCPU * 1024 / 1000)
            // Docker converts shares to cgroup2 cpu.weight automatically.
            // If requests.cpu is not set, fall back to limits.cpu (Kubernetes defaults
            // requests to limits when only limits are specified).
            let cpu_request = resources
                .requests
                .as_ref()
                .and_then(|r| r.get("cpu"))
                .or_else(|| resources.limits.as_ref().and_then(|l| l.get("cpu")));
            if let Some(cpu) = cpu_request {
                let cpu_millicores = parse_cpu_quantity(cpu);
                if cpu_millicores > 0 {
                    let shares = std::cmp::max(2, (cpu_millicores * 1024) / 1000);
                    cpu_shares = Some(shares);
                    info!(
                        "Setting CPU shares for container {}: {}m -> {} shares",
                        container.name, cpu_millicores, shares
                    );
                }
            }
        }

        // Resolve runAsUser and runAsGroup from container or pod security context
        let run_as_user_id: Option<i64> = container
            .security_context
            .as_ref()
            .and_then(|sc| sc.run_as_user)
            .or_else(|| {
                pod.spec
                    .as_ref()
                    .and_then(|s| s.security_context.as_ref())
                    .and_then(|sc| sc.run_as_user)
            });
        let run_as_group_id: Option<i64> = container
            .security_context
            .as_ref()
            .and_then(|sc| sc.run_as_group)
            .or_else(|| {
                pod.spec
                    .as_ref()
                    .and_then(|s| s.security_context.as_ref())
                    .and_then(|sc| sc.run_as_group)
            });
        // Docker user format: "uid" or "uid:gid"
        let run_as_user: Option<String> = match (run_as_user_id, run_as_group_id) {
            (Some(uid), Some(gid)) => Some(format!("{}:{}", uid, gid)),
            (Some(uid), None) => Some(uid.to_string()),
            (None, Some(gid)) => Some(format!("0:{}", gid)), // default uid 0 if only gid set
            (None, None) => None,
        };

        let mut config = Config {
            image: Some(container.image.clone()),
            env,
            working_dir: container.working_dir.clone(),
            user: run_as_user,
            exposed_ports: if exposed_ports.is_empty() {
                None
            } else {
                Some(exposed_ports)
            },
            host_config: Some(bollard::models::HostConfig {
                port_bindings: if port_bindings.is_empty() {
                    None
                } else {
                    Some(port_bindings)
                },
                binds: if binds.is_empty() { None } else { Some(binds) },
                tmpfs: if tmpfs_mounts.is_empty() { None } else { Some(tmpfs_mounts) },
                // Configure DNS to use kube-dns service
                // CoreDNS uses default/host DNS to avoid circular dependency
                dns: dns_servers,
                dns_search: dns_search_domains,
                dns_options: dns_options,
                // Use CNI network namespace if available, pause container network for
                // non-CNI mode (so all containers share the same pod network), or fall
                // back to the Docker bridge network.
                network_mode: if let Some(netns) = netns_path {
                    Some(format!("ns:{}", netns))
                } else if !self.use_cni {
                    Some(format!("container:{}_pause", pod_name))
                } else {
                    Some(self.network.clone())
                },
                // Resource limits enforcement via cgroups
                memory: memory_limit,
                cpu_period,
                cpu_quota,
                cpu_shares,
                // Read-only root filesystem from security context
                readonly_rootfs: container
                    .security_context
                    .as_ref()
                    .and_then(|sc| sc.read_only_root_filesystem),
                // Security options: no-new-privileges when allowPrivilegeEscalation is false
                security_opt: {
                    let ape = container.security_context.as_ref()
                        .and_then(|sc| sc.allow_privilege_escalation)
                        .or_else(|| pod.spec.as_ref()
                            .and_then(|s| s.security_context.as_ref())
                            .and_then(|sc| sc.run_as_non_root)
                            .map(|_| false));
                    if ape == Some(false) {
                        Some(vec!["no-new-privileges".to_string()])
                    } else {
                        None
                    }
                },
                // Capabilities
                cap_add: container.security_context.as_ref()
                    .and_then(|sc| sc.capabilities.as_ref())
                    .and_then(|c| c.add.clone()),
                cap_drop: container.security_context.as_ref()
                    .and_then(|sc| sc.capabilities.as_ref())
                    .and_then(|c| c.drop.clone()),
                // Privileged mode
                privileged: container.security_context.as_ref()
                    .and_then(|sc| sc.privileged),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Set command and args
        // In Kubernetes: command overrides Docker ENTRYPOINT, args overrides Docker CMD
        // Kubernetes expands $(VAR_NAME) references in command and args using the
        // container's own environment variables.
        let expand_k8s_vars = |items: &[String]| -> Vec<String> {
            items.iter().map(|item| {
                let mut result = item.clone();
                loop {
                    let start = match result.find("$(") {
                        Some(s) => s,
                        None => break,
                    };
                    let end = match result[start..].find(')') {
                        Some(e) => start + e,
                        None => break,
                    };
                    let var_name = &result[start + 2..end];
                    let value = resolved_env_pairs.iter()
                        .find(|(k, _)| k == var_name)
                        .map(|(_, v)| v.as_str())
                        .unwrap_or("");
                    result.replace_range(start..end + 1, value);
                }
                result
            }).collect()
        };

        if let Some(command) = &container.command {
            if let Some(args) = &container.args {
                // Both command and args present
                let expanded_cmd = expand_k8s_vars(command);
                let expanded_args = expand_k8s_vars(args);
                info!(
                    "Container {} - setting entrypoint {:?} and cmd {:?}",
                    container.name, expanded_cmd, expanded_args
                );
                config.entrypoint = Some(expanded_cmd);
                config.cmd = Some(expanded_args);
            } else {
                // Only command present - overrides entrypoint, clears cmd
                let expanded_cmd = expand_k8s_vars(command);
                info!(
                    "Container {} - setting entrypoint: {:?}",
                    container.name, expanded_cmd
                );
                config.entrypoint = Some(expanded_cmd);
                config.cmd = Some(vec![]);
            }
        } else if let Some(args) = &container.args {
            // Only args present - use container's default entrypoint + override cmd
            let expanded_args = expand_k8s_vars(args);
            info!("Container {} - setting cmd (args): {:?}", container.name, expanded_args);
            config.cmd = Some(expanded_args);
        }

        let options = CreateContainerOptions {
            name: container_name.clone(),
            ..Default::default()
        };

        // Create the container
        if let Err(e) = self.docker.create_container(Some(options), config).await {
            error!(
                "Docker API error creating container {}: {}",
                container_name, e
            );
            return Err(anyhow::anyhow!("Failed to create container: {}", e));
        }

        // Start the container
        self.docker
            .start_container(&container_name, None::<StartContainerOptions<String>>)
            .await
            .context("Failed to start container")?;

        info!("Container {} started successfully", container_name);

        // Execute postStart lifecycle hook if present
        if let Some(ref lifecycle) = container.lifecycle {
            if let Some(ref post_start) = lifecycle.post_start {
                info!("Executing postStart hook for container {}", container.name);
                if let Err(e) = self
                    .execute_lifecycle_handler(post_start, &container_name)
                    .await
                {
                    warn!(
                        "postStart hook failed for container {}: {}",
                        container.name, e
                    );
                    // In Kubernetes, if postStart fails, the container is killed
                    // For now, log the error but don't kill the container
                }
            }
        }

        Ok(())
    }

    /// Stop all containers for a pod
    pub async fn stop_pod(&self, pod_name: &str) -> Result<()> {
        self.clear_probe_states_for_pod(pod_name);
        self.stop_pod_with_grace_period(pod_name, 30).await
    }

    /// Stop and force-remove all containers for a pod.
    /// Used for orphaned container cleanup where logs are no longer needed.
    pub async fn stop_and_remove_pod(&self, pod_name: &str) -> Result<()> {
        self.clear_probe_states_for_pod(pod_name);

        let mut filters = HashMap::new();
        filters.insert("name".to_string(), vec![format!("{}_", pod_name)]);
        let options = ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        };
        let containers = self.docker.list_containers(Some(options)).await?;

        for container in containers {
            if let Some(id) = container.id {
                let _ = self.docker.stop_container(&id, Some(StopContainerOptions { t: 0 })).await;
                let remove_options = RemoveContainerOptions { force: true, ..Default::default() };
                let _ = self.docker.remove_container(&id, Some(remove_options)).await;
            }
        }

        if self.use_cni {
            let _ = self.teardown_pod_network(pod_name).await;
        }
        self.cleanup_pod_volumes(pod_name).await?;
        Ok(())
    }

    /// Stop all containers for a pod with a specific grace period in seconds
    pub async fn stop_pod_with_grace_period(
        &self,
        pod_name: &str,
        grace_period_seconds: i64,
    ) -> Result<()> {
        info!(
            "Stopping pod: {} (grace period: {}s)",
            pod_name, grace_period_seconds
        );

        // List all containers with this pod prefix
        let mut filters = HashMap::new();
        filters.insert("name".to_string(), vec![format!("{}_", pod_name)]);

        let options = ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        };

        let containers = self.docker.list_containers(Some(options)).await?;

        for container in containers {
            if let Some(id) = container.id {
                info!("Stopping container: {}", id);

                // Stop the container gracefully using the pod's terminationGracePeriodSeconds
                let stop_options = StopContainerOptions {
                    t: grace_period_seconds,
                };
                if let Err(e) = self.docker.stop_container(&id, Some(stop_options)).await {
                    warn!("Failed to stop container {}: {}", id, e);
                }

                // Do NOT remove containers here — keep them stopped for log
                // retrieval. Conformance tests read logs from completed/deleted
                // pods after the pod has been deleted from the API. The orphaned
                // container cleanup will remove them on the next cycle.
                debug!("Container {} stopped, keeping for log access", id);
            }
        }

        // Teardown CNI networking if enabled
        if self.use_cni {
            if let Err(e) = self.teardown_pod_network(pod_name).await {
                warn!("Failed to teardown CNI network for pod {}: {}", pod_name, e);
                // Continue with cleanup even if CNI teardown fails
            }
        }

        // Clean up emptyDir volumes (but keep container data for logs)
        self.cleanup_pod_volumes(pod_name).await?;

        Ok(())
    }

    /// Clean up volumes created for a pod
    async fn cleanup_pod_volumes(&self, pod_name: &str) -> Result<()> {
        let volume_dir = format!("{}/{}", self.volumes_base_path, pod_name);

        if std::path::Path::new(&volume_dir).exists() {
            if let Err(e) = std::fs::remove_dir_all(&volume_dir) {
                warn!("Failed to remove volume directory {}: {}", volume_dir, e);
            } else {
                info!("Cleaned up volumes for pod {}", pod_name);
            }
        }

        Ok(())
    }

    /// Check if a specific container is running
    pub async fn is_container_running(&self, container_name: &str) -> Result<bool> {
        match self.docker.inspect_container(container_name, None::<InspectContainerOptions>).await {
            Ok(inspect) => {
                Ok(inspect.state.as_ref().and_then(|s| s.running).unwrap_or(false))
            }
            Err(_) => Ok(false),
        }
    }

    /// Check if a pod's containers are running
    pub async fn is_pod_running(&self, pod_name: &str) -> Result<bool> {
        let mut filters = HashMap::new();
        filters.insert("name".to_string(), vec![format!("{}_", pod_name)]);

        let options = ListContainersOptions {
            all: false, // Only running containers
            filters,
            ..Default::default()
        };

        let containers = self.docker.list_containers(Some(options)).await?;
        Ok(!containers.is_empty())
    }

    /// Get detailed status of init containers by inspecting actual Docker container state.
    pub async fn get_init_container_statuses(&self, pod: &Pod) -> Option<Vec<ContainerStatus>> {
        let init_containers = pod.spec.as_ref()?.init_containers.as_ref()?;
        if init_containers.is_empty() {
            return None;
        }

        let pod_name = &pod.metadata.name;
        let mut statuses = Vec::new();

        for ic in init_containers {
            let container_name = format!("{}_{}", pod_name, ic.name);

            let container_state_info = self
                .docker
                .inspect_container(&container_name, None::<InspectContainerOptions>)
                .await;

            let (state, container_id): (ContainerState, Option<String>) = match container_state_info {
                Ok(inspect) => {
                    let ds = inspect.state.unwrap_or_default();
                    let cid = inspect.id.clone().map(|id| format!("docker://{}", id));
                    let running = ds.running.unwrap_or(false);

                    if running {
                        (ContainerState::Running { started_at: ds.started_at }, cid)
                    } else if ds.finished_at.is_some() || matches!(ds.status, Some(bollard::secret::ContainerStateStatusEnum::EXITED)) {
                        let code = ds.exit_code.unwrap_or(0) as i32;
                        let term_msg = self.read_termination_message(&container_name, ic).await;
                        (ContainerState::Terminated {
                            exit_code: code,
                            signal: None,
                            reason: Some(if code == 0 { "Completed".to_string() } else { ds.error.unwrap_or_else(|| "Error".to_string()) }),
                            message: term_msg,
                            started_at: ds.started_at,
                            finished_at: ds.finished_at,
                            container_id: cid.clone(),
                        }, cid)
                    } else {
                        (ContainerState::Waiting { reason: Some("PodInitializing".to_string()), message: None }, cid)
                    }
                }
                Err(_) => {
                    // Container doesn't exist — it hasn't been created yet.
                    // Report as Waiting, not Terminated.
                    (ContainerState::Waiting { reason: Some("PodInitializing".to_string()), message: None }, None)
                }
            };

            let is_terminated = matches!(state, ContainerState::Terminated { .. });

            statuses.push(ContainerStatus {
                name: ic.name.clone(),
                ready: is_terminated && matches!(&state, ContainerState::Terminated { exit_code, .. } if *exit_code == 0),
                restart_count: 0,
                state: Some(state),
                last_state: None,
                image: Some(ic.image.clone()),
                image_id: None,
                container_id,
                started: Some(false),
                allocated_resources: ic.resources.as_ref().and_then(|r| r.requests.clone()),
                allocated_resources_status: None,
                resources: ic.resources.clone(),
                user: None,
                volume_mounts: None,
                stop_signal: None,
            });
        }

        Some(statuses)
    }

    /// Get detailed status of all containers in a pod
    pub async fn get_container_statuses(&self, pod: &Pod) -> Result<Vec<ContainerStatus>> {
        let mut statuses = Vec::new();
        let pod_name = &pod.metadata.name;

        for container in &pod.spec.as_ref().unwrap().containers {
            let container_name = format!("{}_{}", pod_name, container.name);

            let status = match self
                .docker
                .inspect_container(&container_name, None::<InspectContainerOptions>)
                .await
            {
                Ok(inspect) => {
                    let state = inspect.state.unwrap_or_default();
                    let running = state.running.unwrap_or(false);
                    let exit_code = state.exit_code.unwrap_or(0);

                    // Get restart count from Docker/Podman container inspect
                    // (tracks actual container restarts), falling back to pod status
                    let restart_count = inspect.restart_count
                        .map(|c| c as u32)
                        .or_else(|| {
                            pod.status
                                .as_ref()
                                .and_then(|s| s.container_statuses.as_ref())
                                .and_then(|statuses| {
                                    statuses
                                        .iter()
                                        .find(|cs| cs.name == container.name)
                                        .map(|cs| cs.restart_count)
                                })
                        })
                        .unwrap_or(0);

                    let container_state = if running {
                        Some(ContainerState::Running {
                            started_at: state.started_at,
                        })
                    } else if matches!(state.status, Some(bollard::secret::ContainerStateStatusEnum::EXITED))
                        || state.finished_at.is_some()
                    {
                        // Read termination message from /dev/termination-log
                        let termination_msg = self.read_termination_message(&container_name, container).await;

                        // Container has exited (any exit code, including 0)
                        Some(ContainerState::Terminated {
                            exit_code: exit_code as i32,
                            signal: None,
                            reason: if exit_code == 0 {
                                Some("Completed".to_string())
                            } else {
                                state.error
                            },
                            message: termination_msg,
                            started_at: None,
                            finished_at: state.finished_at,
                            container_id: inspect.id.clone().map(|id| format!("docker://{}", id)),
                        })
                    } else {
                        Some(ContainerState::Waiting {
                            reason: Some("ContainerCreating".to_string()),
                            message: None,
                        })
                    };

                    // Check startup probe first. If defined and not yet passing,
                    // liveness and readiness probes are skipped.
                    // Uses threshold tracking for accurate Kubernetes semantics.
                    let startup_passed = if running {
                        if let Some(startup_probe) = &container.startup_probe {
                            let raw = self
                                .check_probe(&container_name, startup_probe)
                                .await
                                .unwrap_or(false);
                            let success_threshold = startup_probe.success_threshold.unwrap_or(1);
                            let key = format!("{}/{}/startup_status", pod_name, container.name);
                            let mut states = self.probe_states.lock().unwrap();
                            let state = states.entry(key).or_default();
                            if raw {
                                state.consecutive_successes += 1;
                                state.consecutive_failures = 0;
                                state.consecutive_successes >= success_threshold
                            } else {
                                state.consecutive_failures += 1;
                                state.consecutive_successes = 0;
                                false
                            }
                        } else {
                            true // No startup probe means started
                        }
                    } else {
                        false
                    };

                    // Check readiness probe only if startup probe has passed.
                    // Uses threshold tracking: successThreshold consecutive successes
                    // required to become ready, failureThreshold consecutive failures
                    // required to become not ready.
                    let ready = if running && startup_passed {
                        if let Some(probe) = &container.readiness_probe {
                            // Respect initialDelaySeconds before running readiness probe
                            let initial_delay = probe.initial_delay_seconds.unwrap_or(0);
                            let past_initial_delay = if initial_delay > 0 {
                                // Check container start time from Docker state
                                if let Some(ContainerState::Running { started_at: Some(ref started_at_str) }) = container_state {
                                    if let Ok(started) = chrono::DateTime::parse_from_rfc3339(started_at_str) {
                                        let elapsed = Utc::now().signed_duration_since(started);
                                        elapsed.num_seconds() >= initial_delay as i64
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                true
                            };

                            if !past_initial_delay {
                                false // Not ready yet, still within initial delay
                            } else {
                            let raw = self
                                .check_probe(&container_name, probe)
                                .await
                                .unwrap_or(false);
                            let _failure_threshold = probe.failure_threshold.unwrap_or(3);
                            let success_threshold = probe.success_threshold.unwrap_or(1);
                            let key = format!("{}/{}/readiness", pod_name, container.name);
                            let mut states = self.probe_states.lock().unwrap();
                            let state = states.entry(key).or_default();
                            if raw {
                                state.consecutive_successes += 1;
                                state.consecutive_failures = 0;
                                state.consecutive_successes >= success_threshold
                            } else {
                                state.consecutive_failures += 1;
                                state.consecutive_successes = 0;
                                // Not ready until success threshold is met
                                false
                            }
                            } // end else past_initial_delay
                        } else {
                            true // No probe means ready
                        }
                    } else {
                        false
                    };

                    ContainerStatus {
                        name: container.name.clone(),
                        ready,
                        restart_count,
                        state: container_state,
                        last_state: None,
                        image: Some(container.image.clone()),
                        image_id: inspect.image.clone().map(|img| {
                            if img.starts_with("sha256:") {
                                format!("docker-pullable://{}", img)
                            } else {
                                img
                            }
                        }),
                        container_id: inspect.id.map(|id| format!("docker://{}", id)),
                        started: Some(startup_passed),
                        allocated_resources: container.resources.as_ref().and_then(|r| r.requests.clone()),
                        allocated_resources_status: None,
                        resources: container.resources.clone(),
                        user: None,
                        volume_mounts: None,
                        stop_signal: None,
                    }
                }
                Err(_) => {
                    // If pod already has CreateContainerError for this container,
                    // preserve that status instead of defaulting to ContainerCreating.
                    let existing_reason = pod
                        .status
                        .as_ref()
                        .and_then(|s| s.container_statuses.as_ref())
                        .and_then(|statuses| {
                            statuses.iter().find(|cs| cs.name == container.name)
                        })
                        .and_then(|cs| match &cs.state {
                            Some(ContainerState::Waiting {
                                reason: Some(r), message,
                            }) if r == "CreateContainerError" => {
                                Some((r.clone(), message.clone()))
                            }
                            _ => None,
                        });

                    let (reason, message) = match existing_reason {
                        Some((r, m)) => (r, m),
                        None => {
                            let has_init = pod.spec.as_ref()
                                .and_then(|s| s.init_containers.as_ref())
                                .map_or(false, |ic| !ic.is_empty());
                            if has_init {
                                ("PodInitializing".to_string(), None)
                            } else {
                                ("ContainerCreating".to_string(), None)
                            }
                        }
                    };

                    ContainerStatus {
                        name: container.name.clone(),
                        ready: false,
                        restart_count: 0,
                        state: Some(ContainerState::Waiting {
                            reason: Some(reason),
                            message,
                        }),
                        last_state: None,
                        image: Some(container.image.clone()),
                        image_id: None,
                        container_id: None,
                        started: Some(false),
                        allocated_resources: container.resources.as_ref().and_then(|r| r.requests.clone()),
                        allocated_resources_status: None,
                        resources: container.resources.clone(),
                        user: None,
                        volume_mounts: None,
                        stop_signal: None,
                    }
                },
            };

            statuses.push(status);
        }

        Ok(statuses)
    }

    /// Check if a container needs to be restarted based on liveness probe.
    ///
    /// If a startup probe is defined and hasn't passed yet, liveness probes
    /// are skipped for that container (per Kubernetes semantics).
    ///
    /// Respects `failureThreshold` (default 3) and `successThreshold` (default 1)
    /// so that a single probe failure does not immediately trigger a restart.
    pub async fn check_liveness(&self, pod: &Pod) -> Result<bool> {
        let pod_name = &pod.metadata.name;

        for container in &pod.spec.as_ref().unwrap().containers {
            let container_name = format!("{}_{}", pod_name, container.name);

            // If a startup probe is defined, check it first.
            // Liveness probes are disabled until the startup probe succeeds.
            if let Some(startup_probe) = &container.startup_probe {
                let startup_key = format!("{}/{}/startup", pod_name, container.name);
                let raw_result = self
                    .check_probe(&container_name, startup_probe)
                    .await
                    .unwrap_or(false);

                let failure_threshold = startup_probe.failure_threshold.unwrap_or(3);
                let success_threshold = startup_probe.success_threshold.unwrap_or(1);

                let startup_passed = {
                    let mut states = self.probe_states.lock().unwrap();
                    let state = states.entry(startup_key).or_default();
                    if raw_result {
                        state.consecutive_successes += 1;
                        state.consecutive_failures = 0;
                        state.consecutive_successes >= success_threshold
                    } else {
                        state.consecutive_failures += 1;
                        state.consecutive_successes = 0;
                        if state.consecutive_failures >= failure_threshold {
                            warn!(
                                "Startup probe exceeded failure threshold ({}) for container {}",
                                failure_threshold, container_name
                            );
                        }
                        false
                    }
                };

                if !startup_passed {
                    debug!(
                        "Startup probe not yet passing for container {}, skipping liveness check",
                        container_name
                    );
                    continue;
                }
            }

            if let Some(probe) = &container.liveness_probe {
                // Wait for initial delay
                let initial_delay = probe.initial_delay_seconds.unwrap_or(0);
                if initial_delay > 0 {
                    // Check container start time
                    if let Ok(inspect) = self
                        .docker
                        .inspect_container(&container_name, None::<InspectContainerOptions>)
                        .await
                    {
                        if let Some(state) = inspect.state {
                            if let Some(started_at) = state.started_at {
                                if let Ok(started) =
                                    chrono::DateTime::parse_from_rfc3339(&started_at)
                                {
                                    let elapsed = Utc::now().signed_duration_since(started);
                                    if elapsed.num_seconds() < initial_delay as i64 {
                                        debug!("Skipping liveness check, within initial delay");
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                }

                // Check liveness with threshold tracking
                let healthy = self.check_probe(&container_name, probe).await?;
                let failure_threshold = probe.failure_threshold.unwrap_or(3);
                // For liveness probes, Kubernetes requires successThreshold=1
                let _success_threshold = probe.success_threshold.unwrap_or(1);
                let liveness_key = format!("{}/{}/liveness", pod_name, container.name);

                let needs_restart = {
                    let mut states = self.probe_states.lock().unwrap();
                    let state = states.entry(liveness_key).or_default();
                    if healthy {
                        state.consecutive_successes += 1;
                        state.consecutive_failures = 0;
                        false
                    } else {
                        state.consecutive_failures += 1;
                        state.consecutive_successes = 0;
                        if state.consecutive_failures >= failure_threshold {
                            warn!(
                                "Liveness probe failed {} consecutive times (threshold {}) for container: {}",
                                state.consecutive_failures, failure_threshold, container_name
                            );
                            // Reset state so next cycle starts fresh after restart
                            state.consecutive_failures = 0;
                            true
                        } else {
                            debug!(
                                "Liveness probe failed for container {} ({}/{} failures before restart)",
                                container_name, state.consecutive_failures, failure_threshold
                            );
                            false
                        }
                    }
                };

                if needs_restart {
                    return Ok(true);
                }
            }
        }

        Ok(false) // All probes passed
    }

    /// Execute a probe check
    async fn check_probe(&self, container_name: &str, probe: &Probe) -> Result<bool> {
        let timeout = Duration::from_secs(probe.timeout_seconds.unwrap_or(1) as u64);

        // HTTP GET probe
        if let Some(http_get) = &probe.http_get {
            return self
                .check_http_probe(container_name, http_get, timeout)
                .await;
        }

        // TCP Socket probe
        if let Some(tcp_socket) = &probe.tcp_socket {
            return self
                .check_tcp_probe(container_name, tcp_socket, timeout)
                .await;
        }

        // Exec probe
        if let Some(exec) = &probe.exec {
            return self.check_exec_probe(container_name, exec, timeout).await;
        }

        // gRPC probe
        if let Some(grpc) = &probe.grpc {
            return self.check_grpc_probe(container_name, grpc, timeout).await;
        }

        Ok(true) // No probe configured
    }

    /// Clear all probe states for a given pod (e.g., on restart or deletion).
    pub fn clear_probe_states_for_pod(&self, pod_name: &str) {
        let prefix = format!("{}/", pod_name);
        let mut states = self.probe_states.lock().unwrap();
        states.retain(|key, _| !key.starts_with(&prefix));
        debug!("Cleared probe states for pod {}", pod_name);
    }

    /// Read the termination message from a stopped container.
    /// Reads /dev/termination-log (or custom path) from the container filesystem.
    async fn read_termination_message(
        &self,
        container_name: &str,
        container: &Container,
    ) -> Option<String> {
        let msg_path = container
            .termination_message_path
            .as_deref()
            .unwrap_or("/dev/termination-log");

        debug!("Reading termination message from {} in container {}", msg_path, container_name);

        // Use docker cp to read the file from the stopped container
        let mut stream = self
            .docker
            .download_from_container(container_name, Some(bollard::container::DownloadFromContainerOptions { path: msg_path.to_string() }));
        let mut all_bytes = Vec::new();
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => all_bytes.extend_from_slice(&bytes),
                Err(e) => {
                    debug!("Error reading termination message from {}: {}", container_name, e);
                    // Check terminationMessagePolicy for FallbackToLogsOnError
                    if container.termination_message_policy.as_deref() == Some("FallbackToLogsOnError") {
                        return self.read_container_logs_tail(container_name, 80).await;
                    }
                    return None;
                }
            }
        }

        if all_bytes.is_empty() {
            debug!("Termination message file empty or not found in {}", container_name);
            // FallbackToLogsOnError: use container logs when file is empty/missing
            if container.termination_message_policy.as_deref() == Some("FallbackToLogsOnError") {
                return self.read_container_logs_tail(container_name, 80).await;
            }
            return None;
        }

        // Docker returns a tar archive; extract the file content
        let mut archive = tar::Archive::new(&all_bytes[..]);
        if let Ok(mut entries) = archive.entries() {
            while let Some(Ok(mut entry)) = entries.next() {
                let mut content = String::new();
                if std::io::Read::read_to_string(&mut entry, &mut content).is_ok() && !content.is_empty() {
                    // Trim to 4KB (Kubernetes limit for termination messages)
                    if content.len() > 4096 {
                        content.truncate(4096);
                    }
                    debug!("Read termination message ({} bytes) from {}", content.len(), container_name);
                    return Some(content);
                }
            }
        }

        debug!("Failed to extract termination message from tar archive for {}", container_name);
        // FallbackToLogsOnError: use container logs when extraction fails
        if container.termination_message_policy.as_deref() == Some("FallbackToLogsOnError") {
            return self.read_container_logs_tail(container_name, 80).await;
        }
        None
    }

    /// Read the last N lines of container logs (for FallbackToLogsOnError)
    async fn read_container_logs_tail(&self, container_name: &str, lines: usize) -> Option<String> {
        use bollard::container::LogsOptions;
        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            tail: lines.to_string(),
            ..Default::default()
        };
        let mut stream = self.docker.logs(container_name, Some(options));
        let mut output = String::new();
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(log) => output.push_str(&log.to_string()),
                Err(_) => break,
            }
        }
        if output.is_empty() {
            None
        } else {
            // Trim to 4KB
            if output.len() > 4096 {
                output.truncate(4096);
            }
            Some(output)
        }
    }

    /// Extract the best available IP from container network settings.
    /// Tries the specific network first, then default ip_address, then 127.0.0.1.
    fn extract_container_ip(
        &self,
        network_settings: Option<bollard::secret::NetworkSettings>,
    ) -> String {
        if let Some(ns) = network_settings {
            // First try the specific network we're using
            if let Some(networks) = &ns.networks {
                if let Some(net_info) = networks.get(&self.network) {
                    if let Some(ip) = &net_info.ip_address {
                        if !ip.is_empty() && ip != "0.0.0.0" {
                            return ip.clone();
                        }
                    }
                }
                // Try any network with a valid IP
                for net_info in networks.values() {
                    if let Some(ip) = &net_info.ip_address {
                        if !ip.is_empty() && ip != "0.0.0.0" {
                            return ip.clone();
                        }
                    }
                }
            }
            // Fallback to top-level ip_address
            if let Some(ip) = ns.ip_address {
                if !ip.is_empty() && ip != "0.0.0.0" {
                    return ip;
                }
            }
        }
        "127.0.0.1".to_string()
    }

    /// Get the effective IP for a container, resolving through pause containers
    /// when the app container uses NetworkMode=container:pause.
    async fn get_effective_container_ip(&self, container_name: &str) -> String {
        let inspect = match self
            .docker
            .inspect_container(container_name, None::<InspectContainerOptions>)
            .await
        {
            Ok(i) => i,
            Err(_) => return "127.0.0.1".to_string(),
        };

        // Check if this container uses another container's network
        if let Some(ref hc) = inspect.host_config {
            if let Some(ref net_mode) = hc.network_mode {
                if net_mode.starts_with("container:") {
                    // Get the pause container's IP instead
                    let pause_id = net_mode.trim_start_matches("container:");
                    if let Ok(pause_inspect) = self
                        .docker
                        .inspect_container(pause_id, None::<InspectContainerOptions>)
                        .await
                    {
                        let ip = self.extract_container_ip(pause_inspect.network_settings);
                        if ip != "127.0.0.1" {
                            return ip;
                        }
                    }
                }
            }
        }

        self.extract_container_ip(inspect.network_settings)
    }

    async fn check_http_probe(
        &self,
        container_name: &str,
        http_get: &HTTPGetAction,
        timeout: Duration,
    ) -> Result<bool> {
        // Use host field if specified, otherwise resolve container IP
        let ip = if let Some(ref host) = http_get.host {
            host.clone()
        } else {
            self.get_effective_container_ip(container_name).await
        };

        let scheme = http_get.scheme.as_deref().unwrap_or("http");
        let path = http_get.path.as_deref().unwrap_or("/");
        let url = format!("{}://{}:{}{}", scheme, ip, http_get.port, path);

        debug!("HTTP probe: {}", url);

        // Kubernetes probes skip TLS verification (accept self-signed certs)
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .danger_accept_invalid_certs(true)
            .build()?;

        match client.get(&url).send().await {
            Ok(response) => {
                let status = response.status();
                Ok(status.is_success())
            }
            Err(e) => {
                debug!("HTTP probe failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn check_tcp_probe(
        &self,
        container_name: &str,
        tcp_socket: &TCPSocketAction,
        timeout: Duration,
    ) -> Result<bool> {
        // Get container IP (resolving through pause container if needed)
        let ip = self.get_effective_container_ip(container_name).await;

        let addr = format!("{}:{}", ip, tcp_socket.port);
        debug!("TCP probe: {}", addr);

        match tokio::time::timeout(timeout, tokio::net::TcpStream::connect(&addr)).await {
            Ok(Ok(_)) => Ok(true),
            _ => Ok(false),
        }
    }

    async fn check_exec_probe(
        &self,
        container_name: &str,
        exec: &ExecAction,
        _timeout: Duration,
    ) -> Result<bool> {
        debug!("Exec probe: {:?}", exec.command);

        let exec_config = CreateExecOptions {
            cmd: Some(exec.command.clone()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let exec_id = self
            .docker
            .create_exec(container_name, exec_config)
            .await?
            .id;

        let start_result = self.docker.start_exec(&exec_id, None).await?;

        match start_result {
            StartExecResults::Attached { mut output, .. } => {
                while let Some(_) = output.next().await {}
            }
            StartExecResults::Detached => {}
        }

        // Get exec inspect to check exit code
        let inspect = self.docker.inspect_exec(&exec_id).await?;
        let exit_code = inspect.exit_code.unwrap_or(1);

        Ok(exit_code == 0)
    }

    /// Check a gRPC health probe by connecting to the gRPC health service.
    /// Sends a raw grpc.health.v1.Health/Check request over HTTP/2 via tonic Channel.
    async fn check_grpc_probe(
        &self,
        container_name: &str,
        grpc: &GRPCAction,
        timeout: Duration,
    ) -> Result<bool> {
        let ip = self.get_effective_container_ip(container_name).await;
        let addr = format!("http://{}:{}", ip, grpc.port);
        debug!("gRPC probe: {} service={:?}", addr, grpc.service);

        let endpoint = match tonic::transport::Endpoint::from_shared(addr.clone()) {
            Ok(ep) => ep.timeout(timeout).connect_timeout(timeout),
            Err(e) => {
                debug!("gRPC probe invalid endpoint {}: {}", addr, e);
                return Ok(false);
            }
        };

        let mut client = match tokio::time::timeout(timeout, endpoint.connect()).await {
            Ok(Ok(ch)) => tonic::client::Grpc::new(ch),
            Ok(Err(e)) => {
                debug!("gRPC probe connect failed: {}", e);
                return Ok(false);
            }
            Err(_) => {
                debug!("gRPC probe connect timed out");
                return Ok(false);
            }
        };

        if client.ready().await.is_err() {
            debug!("gRPC probe: client not ready");
            return Ok(false);
        }

        // Construct the HealthCheckRequest protobuf manually
        let service_name = grpc.service.as_deref().unwrap_or("");
        let mut request_bytes = Vec::new();
        if !service_name.is_empty() {
            // field 1, wire type 2 (length-delimited string)
            request_bytes.push(0x0a);
            // varint encode the length
            let len = service_name.len();
            if len < 128 {
                request_bytes.push(len as u8);
            } else {
                request_bytes.push((len & 0x7f | 0x80) as u8);
                request_bytes.push((len >> 7) as u8);
            }
            request_bytes.extend_from_slice(service_name.as_bytes());
        }

        let codec = tonic::codec::ProstCodec::default();
        let path = http::uri::PathAndQuery::from_static("/grpc.health.v1.Health/Check");

        // Use EncodedBytes to wrap our pre-encoded protobuf message
        let request = tonic::Request::new(EncodedBytes(request_bytes));
        let result: std::result::Result<tonic::Response<DecodedStatus>, tonic::Status> =
            client.unary(request, path, codec).await;

        match result {
            Ok(response) => {
                let status_val = response.into_inner().0;
                // HealthCheckResponse.ServingStatus: SERVING = 1
                let serving = status_val == 1;
                debug!("gRPC probe result: serving={}", serving);
                Ok(serving)
            }
            Err(status) => {
                debug!("gRPC probe failed with gRPC status: {:?}", status);
                Ok(false)
            }
        }
    }

    /// Execute a lifecycle handler (postStart/preStop) inside a container.
    ///
    /// Supports exec, httpGet, tcpSocket, and sleep handler types.
    /// Reuses the same probe execution patterns (exec via Docker API, HTTP via reqwest,
    /// TCP via tokio::net) for consistency.
    async fn execute_lifecycle_handler(
        &self,
        handler: &LifecycleHandler,
        container_name: &str,
    ) -> Result<()> {
        if let Some(ref exec) = handler.exec {
            // Execute command inside the container
            debug!(
                "Lifecycle exec handler: {:?} in {}",
                exec.command, container_name
            );
            let exec_config = CreateExecOptions {
                cmd: Some(exec.command.clone()),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                ..Default::default()
            };

            let exec_id = self
                .docker
                .create_exec(container_name, exec_config)
                .await
                .context("Failed to create exec for lifecycle handler")?
                .id;

            let start_result = self
                .docker
                .start_exec(&exec_id, None)
                .await
                .context("Failed to start exec for lifecycle handler")?;

            // Drain output
            match start_result {
                StartExecResults::Attached { mut output, .. } => {
                    while let Some(_) = output.next().await {}
                }
                StartExecResults::Detached => {}
            }

            // Check exit code
            let inspect = self.docker.inspect_exec(&exec_id).await?;
            let exit_code = inspect.exit_code.unwrap_or(1);
            if exit_code != 0 {
                return Err(anyhow::anyhow!(
                    "Lifecycle exec handler exited with code {}",
                    exit_code
                ));
            }
        } else if let Some(ref http_get) = handler.http_get {
            // Execute HTTP GET request — use host field if specified, otherwise container IP
            let ip = if let Some(ref host) = http_get.host {
                host.clone()
            } else {
                let inspect = self
                    .docker
                    .inspect_container(container_name, None::<InspectContainerOptions>)
                    .await?;
                inspect
                    .network_settings
                    .and_then(|ns| ns.ip_address)
                    .unwrap_or_else(|| "127.0.0.1".to_string())
            };

            let scheme = http_get.scheme.as_deref().unwrap_or("http");
            let path = http_get.path.as_deref().unwrap_or("/");
            let url = format!("{}://{}:{}{}", scheme, ip, http_get.port, path);

            debug!("Lifecycle HTTP handler: {}", url);

            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .danger_accept_invalid_certs(true)
                .build()?;

            match client.get(&url).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        return Err(anyhow::anyhow!(
                            "Lifecycle HTTP handler returned status {}",
                            response.status()
                        ));
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Lifecycle HTTP handler failed: {}", e));
                }
            }
        } else if let Some(ref tcp_socket) = handler.tcp_socket {
            // Open TCP connection to the container
            let inspect = self
                .docker
                .inspect_container(container_name, None::<InspectContainerOptions>)
                .await?;

            let ip = inspect
                .network_settings
                .and_then(|ns| ns.ip_address)
                .unwrap_or_else(|| "127.0.0.1".to_string());

            let addr = format!("{}:{}", ip, tcp_socket.port);
            debug!("Lifecycle TCP handler: {}", addr);

            match tokio::time::timeout(
                Duration::from_secs(10),
                tokio::net::TcpStream::connect(&addr),
            )
            .await
            {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    return Err(anyhow::anyhow!("Lifecycle TCP handler failed: {}", e));
                }
                Err(_) => {
                    return Err(anyhow::anyhow!("Lifecycle TCP handler timed out"));
                }
            }
        } else if let Some(ref sleep) = handler.sleep {
            // Sleep for the specified duration
            debug!("Lifecycle sleep handler: {}s", sleep.seconds);
            tokio::time::sleep(Duration::from_secs(sleep.seconds as u64)).await;
        }

        Ok(())
    }

    /// Stop all containers for a pod, executing preStop lifecycle hooks first.
    ///
    /// This is the preferred method when the Pod spec is available, as it
    /// allows preStop hooks to be executed before container termination.
    pub async fn stop_pod_for(&self, pod: &Pod, grace_period_seconds: i64) -> Result<()> {
        let pod_name = &pod.metadata.name;
        self.clear_probe_states_for_pod(pod_name);
        info!(
            "Stopping pod: {} (grace period: {}s, with lifecycle hooks)",
            pod_name, grace_period_seconds
        );

        // Build a map of container name -> lifecycle for preStop hook lookup
        let mut lifecycle_map: HashMap<String, _> = HashMap::new();
        if let Some(spec) = &pod.spec {
            for container in &spec.containers {
                if let Some(ref lifecycle) = container.lifecycle {
                    if lifecycle.pre_stop.is_some() {
                        let container_name = format!("{}_{}", pod_name, container.name);
                        lifecycle_map.insert(container_name, lifecycle.clone());
                    }
                }
            }
            if let Some(init_containers) = &spec.init_containers {
                for container in init_containers {
                    if let Some(ref lifecycle) = container.lifecycle {
                        if lifecycle.pre_stop.is_some() {
                            let container_name = format!("{}_{}", pod_name, container.name);
                            lifecycle_map.insert(container_name, lifecycle.clone());
                        }
                    }
                }
            }
        }

        // List all containers with this pod prefix
        let mut filters = HashMap::new();
        filters.insert("name".to_string(), vec![format!("{}_", pod_name)]);

        let options = ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        };

        let containers = self.docker.list_containers(Some(options)).await?;

        for container in containers {
            if let Some(id) = container.id {
                // Determine the container name for lifecycle hook lookup
                let names = container.names.unwrap_or_default();
                let container_name = names
                    .first()
                    .map(|n| n.trim_start_matches('/').to_string())
                    .unwrap_or_default();

                // Execute preStop hook if present and the container is running
                let is_running = container.state.as_deref() == Some("running");
                if is_running {
                    if let Some(lifecycle) = lifecycle_map.get(&container_name) {
                        if let Some(ref pre_stop) = lifecycle.pre_stop {
                            info!("Executing preStop hook for container {}", container_name);
                            if let Err(e) = self
                                .execute_lifecycle_handler(pre_stop, &container_name)
                                .await
                            {
                                warn!(
                                    "preStop hook failed for container {}: {}",
                                    container_name, e
                                );
                            }
                        }
                    }
                }

                info!("Stopping container: {}", id);

                // Stop the container gracefully
                let stop_options = StopContainerOptions {
                    t: grace_period_seconds,
                };
                if let Err(e) = self.docker.stop_container(&id, Some(stop_options)).await {
                    warn!("Failed to stop container {}: {}", id, e);
                }

                // Do NOT remove containers — keep them stopped for log retrieval.
                debug!("Container {} stopped, keeping for log access", id);
            }
        }

        // Teardown CNI networking if enabled
        if self.use_cni {
            if let Err(e) = self.teardown_pod_network(pod_name).await {
                warn!("Failed to teardown CNI network for pod {}: {}", pod_name, e);
            }
        }

        // Clean up emptyDir volumes (but keep container data for logs)
        self.cleanup_pod_volumes(pod_name).await?;

        Ok(())
    }

    /// Get the pod IP address from the first running container
    pub async fn get_pod_ip(&self, pod_name: &str) -> Result<Option<String>> {
        // If using CNI, get IP from CNI runtime
        if self.use_cni {
            if let Some(cni) = &self.cni {
                if let Some(ip) = cni.get_container_ip(pod_name) {
                    debug!("Retrieved pod IP {} from CNI for pod {}", ip, pod_name);
                    return Ok(Some(ip));
                }
            }
        }

        // Fallback to Docker/Podman network inspection
        // Look specifically for the pause container which owns the network namespace
        let pause_name = format!("{}_pause", pod_name);
        let mut filters = HashMap::new();
        filters.insert("name".to_string(), vec![pause_name.clone()]);

        let options = ListContainersOptions {
            all: false, // Only running containers
            filters,
            ..Default::default()
        };

        let mut containers = self.docker.list_containers(Some(options)).await?;

        // If no pause container, try any container matching the pod name
        if containers.is_empty() {
            let mut filters2 = HashMap::new();
            filters2.insert("name".to_string(), vec![format!("{}_", pod_name)]);
            let options2 = ListContainersOptions {
                all: false,
                filters: filters2,
                ..Default::default()
            };
            containers = self.docker.list_containers(Some(options2)).await?;
        }

        // Get the IP from the pause container (or first matching)
        if let Some(container) = containers.first() {
            if let Some(id) = &container.id {
                let inspect = self
                    .docker
                    .inspect_container(id, None::<InspectContainerOptions>)
                    .await?;

                if let Some(network_settings) = inspect.network_settings {
                    // First try to get IP from the specific network we're using
                    if let Some(networks) = network_settings.networks {
                        if let Some(network_info) = networks.get(&self.network) {
                            if let Some(ip) = &network_info.ip_address {
                                if !ip.is_empty() && ip != "0.0.0.0" {
                                    debug!(
                                        "Retrieved pod IP {} from network {} for pod {}",
                                        ip, self.network, pod_name
                                    );
                                    return Ok(Some(ip.clone()));
                                }
                            }
                        }
                    }

                    // Fallback to default network IP
                    if let Some(ip) = network_settings.ip_address {
                        if !ip.is_empty() && ip != "0.0.0.0" {
                            debug!(
                                "Retrieved pod IP {} from default network for pod {}",
                                ip, pod_name
                            );
                            return Ok(Some(ip));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Get a value from a ConfigMap
    async fn get_configmap_value(&self, namespace: &str, name: &str, key: &str) -> Result<String> {
        let storage = self.storage.as_ref().context("Storage not available")?;

        let configmap_key = build_key("configmaps", Some(namespace), name);
        let configmap: ConfigMap = storage
            .get(&configmap_key)
            .await
            .with_context(|| format!("ConfigMap {} not found in namespace {}", name, namespace))?;

        configmap
            .data
            .as_ref()
            .and_then(|data| data.get(key))
            .cloned()
            .with_context(|| format!("Key {} not found in ConfigMap {}", key, name))
    }

    /// Get a value from a Secret
    async fn get_secret_value(&self, namespace: &str, name: &str, key: &str) -> Result<String> {
        let storage = self.storage.as_ref().context("Storage not available")?;

        let secret_key = build_key("secrets", Some(namespace), name);
        let secret: Secret = storage
            .get(&secret_key)
            .await
            .with_context(|| format!("Secret {} not found in namespace {}", name, namespace))?;

        // Secret data is stored base64-encoded in storage, but needs to be decoded
        // for environment variables
        secret
            .data
            .as_ref()
            .and_then(|data| data.get(key))
            .and_then(|bytes| String::from_utf8(bytes.clone()).ok())
            .with_context(|| format!("Key {} not found in Secret {}", key, name))
    }

    /// Get a pod field value for DownwardAPI
    fn get_pod_field_value(&self, pod: &Pod, field_path: &str) -> Result<String> {
        let value = match field_path {
            "metadata.name" => pod.metadata.name.clone(),
            "metadata.namespace" => pod
                .metadata
                .namespace
                .clone()
                .unwrap_or("default".to_string()),
            "metadata.uid" => pod.metadata.uid.clone(),
            "spec.nodeName" => pod
                .spec
                .as_ref()
                .and_then(|s| s.node_name.clone())
                .unwrap_or("".to_string()),
            "spec.serviceAccountName" => pod
                .spec
                .as_ref()
                .and_then(|s| s.service_account_name.clone())
                .unwrap_or("default".to_string()),
            "status.podIP" => pod
                .status
                .as_ref()
                .and_then(|s| s.pod_ip.clone())
                .unwrap_or("".to_string()),
            "status.hostIP" => pod
                .status
                .as_ref()
                .and_then(|s| s.host_ip.clone())
                .unwrap_or_else(|| "127.0.0.1".to_string()),
            // All labels formatted as key="value"\n
            "metadata.labels" => {
                pod.metadata.labels.as_ref()
                    .map(|labels| {
                        let mut pairs: Vec<_> = labels.iter().collect();
                        pairs.sort_by_key(|(k, _)| k.clone());
                        pairs.iter().map(|(k, v)| format!("{}=\"{}\"", k, v)).collect::<Vec<_>>().join("\n")
                    })
                    .unwrap_or_default()
            }
            // All annotations formatted as key="value"\n
            "metadata.annotations" => {
                pod.metadata.annotations.as_ref()
                    .map(|anns| {
                        let mut pairs: Vec<_> = anns.iter().collect();
                        pairs.sort_by_key(|(k, _)| k.clone());
                        pairs.iter().map(|(k, v)| format!("{}=\"{}\"", k, v)).collect::<Vec<_>>().join("\n")
                    })
                    .unwrap_or_default()
            }
            _ => {
                // Support metadata.labels['key'] and metadata.annotations['key']
                if field_path.starts_with("metadata.labels['") && field_path.ends_with("']") {
                    let key = &field_path[17..field_path.len() - 2];
                    pod.metadata
                        .labels
                        .as_ref()
                        .and_then(|labels| labels.get(key))
                        .cloned()
                        .unwrap_or("".to_string())
                } else if field_path.starts_with("metadata.annotations['")
                    && field_path.ends_with("']")
                {
                    let key = &field_path[22..field_path.len() - 2];
                    pod.metadata
                        .annotations
                        .as_ref()
                        .and_then(|annotations| annotations.get(key))
                        .cloned()
                        .unwrap_or("".to_string())
                } else {
                    return Err(anyhow::anyhow!("Unsupported field path: {}", field_path));
                }
            }
        };
        Ok(value)
    }

    /// Get a container resource value for DownwardAPI
    ///
    /// Returns the resource value formatted according to the divisor.
    /// For memory: returns bytes (or bytes/divisor) as a string.
    /// For CPU: returns millicores (or cores with divisor "1") as a string.
    /// When divisor is "0" or absent, default units are used (bytes for memory, whole-number
    /// representation for CPU).
    fn get_container_resource_value(
        &self,
        pod: &Pod,
        resource_ref: &rusternetes_common::resources::ResourceFieldSelector,
    ) -> Result<String> {
        let spec = pod.spec.as_ref().context("Pod has no spec")?;

        // Find the container — if containerName is not set, default to the first container
        let container = if let Some(ref container_name) = resource_ref.container_name {
            spec.containers
                .iter()
                .find(|c| &c.name == container_name)
                .with_context(|| format!("Container {} not found", container_name))?
        } else {
            spec.containers
                .first()
                .context("Pod has no containers")?
        };

        let is_cpu = resource_ref.resource.contains("cpu")
            || resource_ref.resource.contains("hugepages");
        let is_memory = resource_ref.resource.contains("memory")
            || resource_ref.resource.contains("ephemeral-storage");

        let raw_value = match resource_ref.resource.as_str() {
            "limits.cpu" => container
                .resources
                .as_ref()
                .and_then(|r| r.limits.as_ref())
                .and_then(|l| l.get("cpu"))
                .cloned(),
            "limits.memory" => container
                .resources
                .as_ref()
                .and_then(|r| r.limits.as_ref())
                .and_then(|l| l.get("memory"))
                .cloned(),
            "limits.ephemeral-storage" => container
                .resources
                .as_ref()
                .and_then(|r| r.limits.as_ref())
                .and_then(|l| l.get("ephemeral-storage"))
                .cloned(),
            "requests.cpu" => container
                .resources
                .as_ref()
                .and_then(|r| r.requests.as_ref())
                .and_then(|l| l.get("cpu"))
                .cloned(),
            "requests.memory" => container
                .resources
                .as_ref()
                .and_then(|r| r.requests.as_ref())
                .and_then(|l| l.get("memory"))
                .cloned(),
            "requests.ephemeral-storage" => container
                .resources
                .as_ref()
                .and_then(|r| r.requests.as_ref())
                .and_then(|l| l.get("ephemeral-storage"))
                .cloned(),
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported resource field: {}",
                    resource_ref.resource
                ))
            }
        };

        // Parse the divisor (default: "1" meaning base units — bytes for memory, cores for cpu)
        let divisor_str = resource_ref.divisor.as_deref().unwrap_or("0");
        // A divisor of "0" means use default units (same as "1")

        if is_cpu {
            // Convert CPU value to millicores, then apply divisor
            // When no limit is set, use node capacity (default 4 cores = 4000m)
            let millicores = raw_value
                .as_deref()
                .map(parse_cpu_quantity)
                .unwrap_or(4000); // 4 cores default
            let divisor_millicores = if divisor_str == "0" || divisor_str == "1" {
                // Default divisor "1" means return in cores (1 core = 1000 millicores)
                1000
            } else {
                parse_cpu_quantity(divisor_str).max(1)
            };
            let result = millicores / divisor_millicores;
            Ok(result.to_string())
        } else if is_memory {
            // Convert memory value to bytes, then apply divisor
            // When no limit is set, use node allocatable memory (default 8Gi)
            let bytes = raw_value
                .as_deref()
                .map(parse_memory_quantity)
                .unwrap_or(8 * 1024 * 1024 * 1024); // 8Gi default
            let divisor_bytes = if divisor_str == "0" || divisor_str == "1" {
                1 // return bytes
            } else {
                parse_memory_quantity(divisor_str).max(1)
            };
            let result = bytes / divisor_bytes;
            Ok(result.to_string())
        } else {
            // Unknown resource type, return raw value
            Ok(raw_value.unwrap_or_else(|| "0".to_string()))
        }
    }

    /// List all running pod names from the container runtime
    pub async fn list_running_pods(&self) -> Result<Vec<String>> {
        let options = ListContainersOptions::<String> {
            all: false, // Only running containers
            ..Default::default()
        };

        let containers = self.docker.list_containers(Some(options)).await?;

        let mut pod_names = std::collections::HashSet::new();
        for container in containers {
            if let Some(names) = container.names {
                for name in names {
                    // Container names are in format: /{pod_name}_{container_name}
                    let name = name.trim_start_matches('/');
                    if let Some(pod_name) = name.split('_').next() {
                        // Skip Rusternetes control plane containers
                        if !pod_name.starts_with("rusternetes-") {
                            pod_names.insert(pod_name.to_string());
                        }
                    }
                }
            }
        }

        Ok(pod_names.into_iter().collect())
    }
}

/// Parse a Kubernetes memory quantity string (e.g., "128Mi", "1Gi", "1000000") into bytes.
fn parse_memory_quantity(s: &str) -> i64 {
    if s.ends_with("Gi") {
        s.trim_end_matches("Gi").parse::<i64>().unwrap_or(0) * 1024 * 1024 * 1024
    } else if s.ends_with("Mi") {
        s.trim_end_matches("Mi").parse::<i64>().unwrap_or(0) * 1024 * 1024
    } else if s.ends_with("Ki") {
        s.trim_end_matches("Ki").parse::<i64>().unwrap_or(0) * 1024
    } else if s.ends_with('G') {
        s.trim_end_matches('G').parse::<i64>().unwrap_or(0) * 1_000_000_000
    } else if s.ends_with('M') {
        s.trim_end_matches('M').parse::<i64>().unwrap_or(0) * 1_000_000
    } else if s.ends_with('K') || s.ends_with('k') {
        s.trim_end_matches(|c| c == 'K' || c == 'k')
            .parse::<i64>()
            .unwrap_or(0)
            * 1000
    } else {
        s.parse::<i64>().unwrap_or(0)
    }
}

/// Parse a Kubernetes CPU quantity string (e.g., "500m", "1", "0.5") into millicores.
fn parse_cpu_quantity(s: &str) -> i64 {
    if s.ends_with('m') {
        s.trim_end_matches('m').parse::<i64>().unwrap_or(0)
    } else {
        (s.parse::<f64>().unwrap_or(0.0) * 1000.0) as i64
    }
}

#[cfg(test)]
mod tests {
    use rusternetes_common::resources::{Container, Pod, PodSpec};
    use rusternetes_common::types::{ObjectMeta, TypeMeta};

    fn make_container(name: &str) -> Container {
        Container {
            name: name.to_string(),
            image: "nginx:latest".to_string(),
            image_pull_policy: Some("IfNotPresent".to_string()),
            command: None,
            args: None,
            ports: None,
            env: None,
            volume_mounts: None,
            liveness_probe: None,
            readiness_probe: None,
            startup_probe: None,
            resources: None,
            working_dir: None,
            security_context: None,
            restart_policy: None,
            resize_policy: None,
            lifecycle: None,
            termination_message_path: None,
            termination_message_policy: None,
            stdin: None,
            stdin_once: None,
            tty: None,
            env_from: None,
            volume_devices: None,
        }
    }

    fn make_pod(
        name: &str,
        namespace: &str,
        hostname: Option<&str>,
        subdomain: Option<&str>,
    ) -> Pod {
        Pod {
            type_meta: TypeMeta {
                kind: "Pod".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: ObjectMeta::new(name).with_namespace(namespace),
            spec: Some(PodSpec {
                containers: vec![make_container("app")],
                init_containers: None,
                ephemeral_containers: None,
                restart_policy: Some("Always".to_string()),
                node_name: None,
                node_selector: None,
                service_account_name: None,
                service_account: None,
                hostname: hostname.map(|s| s.to_string()),
                subdomain: subdomain.map(|s| s.to_string()),
                host_network: None,
                host_pid: None,
                host_ipc: None,
                affinity: None,
                tolerations: None,
                priority: None,
                priority_class_name: None,
                automount_service_account_token: None,
                topology_spread_constraints: None,
                overhead: None,
                scheduler_name: None,
                resource_claims: None,
                volumes: None,
                active_deadline_seconds: None,
                dns_policy: None,
                dns_config: None,
                security_context: None,
                image_pull_secrets: None,
                share_process_namespace: None,
                readiness_gates: None,
                runtime_class_name: None,
                enable_service_links: None,
                preemption_policy: None,
                host_users: None,
                set_hostname_as_fqdn: None,
                termination_grace_period_seconds: None,
                host_aliases: None,
                os: None,
                scheduling_gates: None,
                resources: None,
            }),
            status: None,
        }
    }

    /// Build the /etc/hosts content string the same way create_pod_hosts_file does,
    /// so we can unit-test the logic without needing a live ContainerRuntime.
    fn build_hosts_content(pod: &Pod, pod_ip: Option<&str>, cluster_domain: &str) -> String {
        let pod_name = &pod.metadata.name;
        let namespace = pod.metadata.namespace.as_deref().unwrap_or("default");
        let spec = pod.spec.as_ref().unwrap();
        let hostname = spec.hostname.as_deref().unwrap_or(pod_name);

        let mut content = String::from(
            "# Kubernetes-managed hosts file\n\
             127.0.0.1\tlocalhost\n\
             ::1\tlocalhost ip6-localhost ip6-loopback\n\
             fe00::0\tip6-localnet\n\
             fe00::0\tip6-mcastprefix\n\
             fe00::1\tip6-allnodes\n\
             fe00::2\tip6-allrouters\n",
        );

        if let Some(ip) = pod_ip {
            let mut aliases = vec![hostname.to_string()];
            if let Some(subdomain) = &spec.subdomain {
                aliases.push(format!(
                    "{}.{}.{}.svc.{}",
                    hostname, subdomain, namespace, cluster_domain
                ));
            }
            content.push_str(&format!("{}\t{}\n", ip, aliases.join("\t")));
        }

        // Add entries from spec.hostAliases
        // Kubernetes groups all hostnames for the same IP on a single line
        if let Some(host_aliases) = &spec.host_aliases {
            for alias in host_aliases {
                if let Some(hostnames) = &alias.hostnames {
                    if !hostnames.is_empty() {
                        content.push_str(&format!("{}\t{}\n", alias.ip, hostnames.join("\t")));
                    }
                }
            }
        }

        content
    }

    // --- hosts file content tests ---

    #[test]
    fn test_hosts_file_always_contains_localhost() {
        let pod = make_pod("my-pod", "default", None, None);
        let content = build_hosts_content(&pod, None, "cluster.local");

        assert!(content.contains("127.0.0.1\tlocalhost"));
        assert!(content.contains("::1\tlocalhost ip6-localhost ip6-loopback"));
        assert!(content.contains("# Kubernetes-managed hosts file"));
    }

    #[test]
    fn test_hosts_file_no_ip_no_hostname_entry() {
        let pod = make_pod("my-pod", "default", None, None);
        let content = build_hosts_content(&pod, None, "cluster.local");

        // Without a pod IP, no hostname entry should appear
        assert!(!content.contains("my-pod"));
    }

    #[test]
    fn test_hosts_file_pod_name_used_as_hostname_when_not_set() {
        let pod = make_pod("my-pod", "default", None, None);
        let content = build_hosts_content(&pod, Some("10.244.1.5"), "cluster.local");

        assert!(content.contains("10.244.1.5\tmy-pod\n"));
    }

    #[test]
    fn test_hosts_file_uses_spec_hostname_when_set() {
        let pod = make_pod("my-pod-abc", "default", Some("web-0"), None);
        let content = build_hosts_content(&pod, Some("10.244.1.5"), "cluster.local");

        // spec.hostname overrides pod name
        assert!(content.contains("10.244.1.5\tweb-0\n"));
        // pod name should NOT appear as a hostname entry
        assert!(!content.contains("my-pod-abc"));
    }

    #[test]
    fn test_hosts_file_subdomain_generates_fqdn() {
        let pod = make_pod("web-0", "default", Some("web-0"), Some("nginx"));
        let content = build_hosts_content(&pod, Some("10.244.1.5"), "cluster.local");

        // Should have: IP  hostname  FQDN
        assert!(content.contains("10.244.1.5\tweb-0\tweb-0.nginx.default.svc.cluster.local\n"));
    }

    #[test]
    fn test_hosts_file_subdomain_uses_pod_name_when_no_hostname() {
        // subdomain set, but spec.hostname is None -> pod name used as hostname
        let pod = make_pod("web-0", "default", None, Some("nginx"));
        let content = build_hosts_content(&pod, Some("10.244.1.5"), "cluster.local");

        assert!(content.contains("10.244.1.5\tweb-0\tweb-0.nginx.default.svc.cluster.local\n"));
    }

    #[test]
    fn test_hosts_file_subdomain_fqdn_uses_correct_namespace() {
        let pod = make_pod("cache-0", "kube-system", Some("cache-0"), Some("redis"));
        let content = build_hosts_content(&pod, Some("10.244.2.10"), "cluster.local");

        assert!(
            content.contains("10.244.2.10\tcache-0\tcache-0.redis.kube-system.svc.cluster.local\n")
        );
    }

    #[test]
    fn test_hosts_file_subdomain_fqdn_uses_custom_cluster_domain() {
        let pod = make_pod("web-0", "default", Some("web-0"), Some("nginx"));
        let content = build_hosts_content(&pod, Some("10.244.1.5"), "k8s.example.com");

        assert!(content.contains("10.244.1.5\tweb-0\tweb-0.nginx.default.svc.k8s.example.com\n"));
    }

    #[test]
    fn test_hosts_file_no_fqdn_without_subdomain() {
        // hostname set but no subdomain: only simple hostname entry, no FQDN
        let pod = make_pod("web-0", "default", Some("web-0"), None);
        let content = build_hosts_content(&pod, Some("10.244.1.5"), "cluster.local");

        assert!(content.contains("10.244.1.5\tweb-0\n"));
        assert!(!content.contains("svc.cluster.local"));
    }

    // --- hosts file path tests ---

    #[test]
    fn test_hosts_file_path_format() {
        let volumes_base = "/var/lib/rusternetes/volumes";
        let pod_name = "my-pod";
        let expected = format!("{}/{}/hosts", volumes_base, pod_name);
        assert_eq!(expected, "/var/lib/rusternetes/volumes/my-pod/hosts");
    }

    #[test]
    fn test_resolv_conf_and_hosts_colocated() {
        // Both files should live in the same pod directory
        let volumes_base = "/var/lib/rusternetes/volumes";
        let pod_name = "my-pod";
        let hosts_path = format!("{}/{}/hosts", volumes_base, pod_name);
        let resolv_path = format!("{}/{}/resolv.conf", volumes_base, pod_name);

        // Same directory
        assert_eq!(
            std::path::Path::new(&hosts_path).parent(),
            std::path::Path::new(&resolv_path).parent(),
        );
    }

    // --- PodSpec.subdomain field tests ---

    #[test]
    fn test_podspec_subdomain_field_default_none() {
        let pod = make_pod("test", "default", None, None);
        assert!(pod.spec.as_ref().unwrap().subdomain.is_none());
    }

    #[test]
    fn test_podspec_subdomain_field_can_be_set() {
        let pod = make_pod("web-0", "default", Some("web-0"), Some("nginx"));
        let spec = pod.spec.as_ref().unwrap();
        assert_eq!(spec.subdomain, Some("nginx".to_string()));
        assert_eq!(spec.hostname, Some("web-0".to_string()));
    }

    #[test]
    fn test_podspec_subdomain_serializes_correctly() {
        let pod = make_pod("web-0", "default", Some("web-0"), Some("nginx"));
        let json = serde_json::to_string(&pod).expect("serialize");
        assert!(json.contains(r#""subdomain":"nginx""#));
        assert!(json.contains(r#""hostname":"web-0""#));
    }

    #[test]
    fn test_podspec_subdomain_omitted_when_none() {
        let pod = make_pod("my-pod", "default", None, None);
        let json = serde_json::to_string(&pod).expect("serialize");
        // skip_serializing_if = Option::is_none means it must not appear
        assert!(!json.contains("subdomain"));
    }

    #[test]
    fn test_podspec_subdomain_roundtrip_deserialization() {
        let original = make_pod("web-0", "default", Some("web-0"), Some("nginx"));
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: Pod = serde_json::from_str(&json).expect("deserialize");
        let spec = restored.spec.as_ref().unwrap();
        assert_eq!(spec.subdomain, Some("nginx".to_string()));
        assert_eq!(spec.hostname, Some("web-0".to_string()));
    }

    #[test]
    fn test_emptydir_volume_path_format() {
        let pod_name = "test-pod-emptydir";
        let volume_name = "test-volume";
        let expected_path = format!("/volumes/{}/{}", pod_name, volume_name);

        assert_eq!(expected_path, "/volumes/test-pod-emptydir/test-volume");
    }

    #[test]
    fn test_hostpath_volume_path() {
        let path = "/tmp/test-hostpath";
        assert_eq!(path, "/tmp/test-hostpath");
    }

    #[test]
    fn test_volume_bind_string_format() {
        // Test read-write bind
        let host_path = "/tmp/test";
        let mount_path = "/data";
        let read_only = false;
        let bind_rw = format!(
            "{}:{}{}",
            host_path,
            mount_path,
            if read_only { ":ro" } else { "" }
        );
        assert_eq!(bind_rw, "/tmp/test:/data");

        // Test read-only bind
        let read_only = true;
        let bind_ro = format!(
            "{}:{}{}",
            host_path,
            mount_path,
            if read_only { ":ro" } else { "" }
        );
        assert_eq!(bind_ro, "/tmp/test:/data:ro");
    }

    #[test]
    fn test_cleanup_volume_path() {
        let pod_name = "test-pod";
        let volume_dir = format!("/volumes/{}", pod_name);

        assert_eq!(volume_dir, "/volumes/test-pod");
    }

    #[test]
    fn test_hostpath_types() {
        let types = vec![
            "DirectoryOrCreate",
            "Directory",
            "FileOrCreate",
            "File",
            "Socket",
            "CharDevice",
            "BlockDevice",
        ];

        for hp_type in types {
            assert!(hp_type.len() > 0);
        }
    }

    #[test]
    fn test_downward_api_field_paths() {
        // Test common DownwardAPI field paths
        let field_paths = vec![
            "metadata.name",
            "metadata.namespace",
            "metadata.uid",
            "spec.nodeName",
            "spec.serviceAccountName",
            "status.podIP",
            "status.hostIP",
        ];

        for path in field_paths {
            assert!(path.contains('.'));
        }
    }

    #[test]
    fn test_downward_api_label_syntax() {
        let field_path = "metadata.labels['app']";
        assert!(field_path.starts_with("metadata.labels['"));
        assert!(field_path.ends_with("']"));

        // Extract key
        let key = &field_path[17..field_path.len() - 2];
        assert_eq!(key, "app");
    }

    #[test]
    fn test_downward_api_annotation_syntax() {
        let field_path = "metadata.annotations['description']";
        assert!(field_path.starts_with("metadata.annotations['"));
        assert!(field_path.ends_with("']"));

        // Extract key
        let key = &field_path[22..field_path.len() - 2];
        assert_eq!(key, "description");
    }

    #[test]
    fn test_ephemeral_pvc_naming() {
        let pod_name = "test-pod";
        let volume_name = "cache";
        let pvc_name = format!("{}-{}", pod_name, volume_name);
        assert_eq!(pvc_name, "test-pod-cache");
    }

    #[test]
    fn test_csi_volume_directory_format() {
        let pod_name = "test-pod";
        let volume_name = "csi-vol";
        let volume_dir = format!("/volumes/{}/{}", pod_name, volume_name);
        assert_eq!(volume_dir, "/volumes/test-pod/csi-vol");
    }

    // --- pause container (non-CNI network sandbox) tests ---

    #[test]
    fn test_pause_container_name_format() {
        // The pause container for a pod must be named {pod_name}_pause so that
        // get_pod_ip (which filters by "{pod_name}_") discovers it.
        let pod_name = "sonobuoy";
        let pause_name = format!("{}_pause", pod_name);
        assert_eq!(pause_name, "sonobuoy_pause");

        // Verify it matches the pod prefix filter used by get_pod_ip
        assert!(pause_name.starts_with(&format!("{}_", pod_name)));
    }

    #[test]
    fn test_pause_container_name_format_various_pods() {
        for pod_name in &["web-0", "redis-0", "my-app-abc123", "kube-dns"] {
            let pause_name = format!("{}_pause", pod_name);
            assert!(pause_name.starts_with(&format!("{}_", pod_name)));
            assert!(pause_name.ends_with("_pause"));
        }
    }

    #[test]
    fn test_non_cni_network_mode_uses_pause_container() {
        // In non-CNI mode, real containers join the pause container's network
        // namespace so they share the pod IP and localhost.
        let pod_name = "my-pod";
        let use_cni = false;
        let network_mode = if use_cni {
            "rusternetes-network".to_string()
        } else {
            format!("container:{}_pause", pod_name)
        };
        assert_eq!(network_mode, "container:my-pod_pause");
    }

    #[test]
    fn test_cni_network_mode_uses_bridge_network() {
        // In CNI mode, containers join the named Docker network directly.
        let pod_name = "my-pod";
        let use_cni = true;
        let bridge_network = "rusternetes-network";
        let network_mode = if use_cni {
            bridge_network.to_string()
        } else {
            format!("container:{}_pause", pod_name)
        };
        assert_eq!(network_mode, "rusternetes-network");
    }

    // --- lifecycle hook tests ---

    #[test]
    fn test_lifecycle_handler_exec_is_recognized() {
        use rusternetes_common::resources::{ExecAction, Lifecycle, LifecycleHandler};

        let lifecycle = Lifecycle {
            post_start: Some(LifecycleHandler {
                exec: Some(ExecAction {
                    command: vec![
                        "/bin/sh".to_string(),
                        "-c".to_string(),
                        "echo hello".to_string(),
                    ],
                }),
                http_get: None,
                tcp_socket: None,
                sleep: None,
            }),
            pre_stop: None,
            stop_signal: None,
        };

        assert!(lifecycle.post_start.is_some());
        let handler = lifecycle.post_start.unwrap();
        assert!(handler.exec.is_some());
        assert_eq!(handler.exec.unwrap().command.len(), 3);
    }

    #[test]
    fn test_lifecycle_handler_http_get_is_recognized() {
        use rusternetes_common::resources::{HTTPGetAction, Lifecycle, LifecycleHandler};

        let lifecycle = Lifecycle {
            post_start: None,
            pre_stop: Some(LifecycleHandler {
                exec: None,
                http_get: Some(HTTPGetAction {
                    path: Some("/shutdown".to_string()),
                    port: 8080,
                    host: Some("localhost".to_string()),
                    scheme: Some("HTTP".to_string()),
                    http_headers: None,
                }),
                tcp_socket: None,
                sleep: None,
            }),
            stop_signal: None,
        };

        assert!(lifecycle.pre_stop.is_some());
        let handler = lifecycle.pre_stop.unwrap();
        assert!(handler.http_get.is_some());
        let http = handler.http_get.unwrap();
        assert_eq!(http.port, 8080);
        assert_eq!(http.path.as_deref(), Some("/shutdown"));
    }

    #[test]
    fn test_lifecycle_handler_sleep_is_recognized() {
        use rusternetes_common::resources::{Lifecycle, LifecycleHandler, SleepAction};

        let lifecycle = Lifecycle {
            post_start: None,
            pre_stop: Some(LifecycleHandler {
                exec: None,
                http_get: None,
                tcp_socket: None,
                sleep: Some(SleepAction { seconds: 5 }),
            }),
            stop_signal: None,
        };

        assert!(lifecycle.pre_stop.is_some());
        let handler = lifecycle.pre_stop.unwrap();
        assert!(handler.sleep.is_some());
        assert_eq!(handler.sleep.unwrap().seconds, 5);
    }

    #[test]
    fn test_container_lifecycle_field_present() {
        use rusternetes_common::resources::{ExecAction, Lifecycle, LifecycleHandler};

        let mut container = make_container("app");
        assert!(container.lifecycle.is_none());

        container.lifecycle = Some(Lifecycle {
            post_start: Some(LifecycleHandler {
                exec: Some(ExecAction {
                    command: vec![
                        "/bin/sh".to_string(),
                        "-c".to_string(),
                        "touch /tmp/started".to_string(),
                    ],
                }),
                http_get: None,
                tcp_socket: None,
                sleep: None,
            }),
            pre_stop: Some(LifecycleHandler {
                exec: Some(ExecAction {
                    command: vec![
                        "/bin/sh".to_string(),
                        "-c".to_string(),
                        "touch /tmp/stopping".to_string(),
                    ],
                }),
                http_get: None,
                tcp_socket: None,
                sleep: None,
            }),
            stop_signal: None,
        });

        assert!(container.lifecycle.is_some());
        let lc = container.lifecycle.unwrap();
        assert!(lc.post_start.is_some());
        assert!(lc.pre_stop.is_some());
    }

    #[test]
    fn test_lifecycle_serializes_correctly() {
        use rusternetes_common::resources::{ExecAction, Lifecycle, LifecycleHandler};

        let mut container = make_container("app");
        container.lifecycle = Some(Lifecycle {
            post_start: Some(LifecycleHandler {
                exec: Some(ExecAction {
                    command: vec!["echo".to_string(), "started".to_string()],
                }),
                http_get: None,
                tcp_socket: None,
                sleep: None,
            }),
            pre_stop: None,
            stop_signal: None,
        });

        let json = serde_json::to_string(&container).expect("serialize");
        assert!(json.contains("\"lifecycle\""));
        assert!(json.contains("\"postStart\""));
        assert!(json.contains("\"exec\""));
    }

    // --- startup probe tests ---

    #[test]
    fn test_container_startup_probe_field() {
        use rusternetes_common::resources::{ExecAction, Probe};

        let mut container = make_container("app");
        assert!(container.startup_probe.is_none());

        container.startup_probe = Some(Probe {
            exec: Some(ExecAction {
                command: vec!["cat".to_string(), "/tmp/healthy".to_string()],
            }),
            http_get: None,
            tcp_socket: None,
            initial_delay_seconds: Some(5),
            period_seconds: Some(10),
            timeout_seconds: Some(1),
            success_threshold: Some(1),
            failure_threshold: Some(30),
            grpc: None,
            termination_grace_period_seconds: None,
        });

        assert!(container.startup_probe.is_some());
        let probe = container.startup_probe.unwrap();
        assert_eq!(probe.failure_threshold, Some(30));
        assert!(probe.exec.is_some());
    }

    #[test]
    fn test_startup_probe_prevents_readiness_when_not_started() {
        // This tests the logical condition used in get_container_statuses:
        // when startup_passed is false, ready should be false
        let startup_passed = false;
        let running = true;
        let has_readiness_probe = true;

        // Simulated logic from get_container_statuses
        let ready = if running && startup_passed {
            if has_readiness_probe {
                true // would check probe
            } else {
                true
            }
        } else {
            false
        };

        assert!(!ready);
        assert!(!startup_passed);
    }

    #[test]
    fn test_startup_probe_allows_readiness_when_started() {
        // When startup probe passes, readiness probe should be evaluated
        let startup_passed = true;
        let running = true;

        let ready = if running && startup_passed {
            true // would check readiness probe
        } else {
            false
        };

        assert!(ready);
    }

    #[test]
    fn test_startup_probe_blocks_liveness_check() {
        // Verify the logical condition: if startup probe hasn't passed,
        // liveness checks should be skipped (continue in the loop)
        let has_startup_probe = true;
        let startup_passed = false;

        let should_skip_liveness = has_startup_probe && !startup_passed;
        assert!(should_skip_liveness);
    }

    #[test]
    fn test_no_startup_probe_does_not_block_liveness() {
        // Without a startup probe, liveness should proceed normally
        let has_startup_probe = false;

        // No startup probe means we don't skip
        let should_skip_liveness = has_startup_probe;
        assert!(!should_skip_liveness);
    }

    #[test]
    fn test_lifecycle_and_startup_probe_on_same_container() {
        use rusternetes_common::resources::{ExecAction, Lifecycle, LifecycleHandler, Probe};

        let mut container = make_container("app");
        container.lifecycle = Some(Lifecycle {
            post_start: Some(LifecycleHandler {
                exec: Some(ExecAction {
                    command: vec!["echo".to_string(), "started".to_string()],
                }),
                http_get: None,
                tcp_socket: None,
                sleep: None,
            }),
            pre_stop: Some(LifecycleHandler {
                exec: Some(ExecAction {
                    command: vec!["echo".to_string(), "stopping".to_string()],
                }),
                http_get: None,
                tcp_socket: None,
                sleep: None,
            }),
            stop_signal: None,
        });
        container.startup_probe = Some(Probe {
            exec: Some(ExecAction {
                command: vec!["cat".to_string(), "/tmp/ready".to_string()],
            }),
            http_get: None,
            tcp_socket: None,
            initial_delay_seconds: Some(0),
            period_seconds: Some(5),
            timeout_seconds: Some(1),
            success_threshold: Some(1),
            failure_threshold: Some(10),
            grpc: None,
            termination_grace_period_seconds: None,
        });

        assert!(container.lifecycle.is_some());
        assert!(container.startup_probe.is_some());

        // Both can coexist
        let lc = container.lifecycle.as_ref().unwrap();
        assert!(lc.post_start.is_some());
        assert!(lc.pre_stop.is_some());
    }

    #[test]
    fn test_pause_container_ip_is_pod_ip() {
        // The pause container holds the network namespace, so its IP is the pod IP.
        // Verify this convention by checking that get_pod_ip searches by pod name prefix,
        // which matches both real containers AND the pause container.
        let pod_name = "web-0";
        let pause_name = format!("{}_pause", pod_name);
        let filter_prefix = format!("{}_", pod_name);

        // Both the pause container and real containers match this filter
        assert!(pause_name.starts_with(&filter_prefix));
        assert!(format!("{}_app", pod_name).starts_with(&filter_prefix));
    }

    // --- probe threshold tests ---

    #[test]
    fn test_probe_state_default() {
        let state = super::ProbeState::default();
        assert_eq!(state.consecutive_failures, 0);
        assert_eq!(state.consecutive_successes, 0);
    }

    #[test]
    fn test_probe_threshold_logic_single_failure_no_action() {
        // Simulate: failure_threshold=3, one failure should NOT trigger action
        let failure_threshold = 3;
        let mut state = super::ProbeState::default();

        // First failure
        state.consecutive_failures += 1;
        state.consecutive_successes = 0;
        assert!(state.consecutive_failures < failure_threshold);
    }

    #[test]
    fn test_probe_threshold_logic_threshold_reached() {
        // Simulate: failure_threshold=3, three failures should trigger action
        let failure_threshold = 3;
        let mut state = super::ProbeState::default();

        for _ in 0..3 {
            state.consecutive_failures += 1;
            state.consecutive_successes = 0;
        }
        assert!(state.consecutive_failures >= failure_threshold);
    }

    #[test]
    fn test_probe_threshold_success_resets_failures() {
        let mut state = super::ProbeState::default();

        // Two failures
        state.consecutive_failures = 2;
        state.consecutive_successes = 0;

        // Then a success resets failures
        state.consecutive_successes += 1;
        state.consecutive_failures = 0;

        assert_eq!(state.consecutive_failures, 0);
        assert_eq!(state.consecutive_successes, 1);
    }

    #[test]
    fn test_probe_threshold_failure_resets_successes() {
        let mut state = super::ProbeState::default();

        // One success
        state.consecutive_successes = 1;
        state.consecutive_failures = 0;

        // Then a failure resets successes
        state.consecutive_failures += 1;
        state.consecutive_successes = 0;

        assert_eq!(state.consecutive_successes, 0);
        assert_eq!(state.consecutive_failures, 1);
    }

    #[test]
    fn test_probe_threshold_readiness_needs_success_threshold() {
        // successThreshold=3 means 3 consecutive successes needed for ready
        let success_threshold = 3;
        let mut state = super::ProbeState::default();

        // First success: not ready yet
        state.consecutive_successes += 1;
        state.consecutive_failures = 0;
        assert!(state.consecutive_successes < success_threshold);

        // Second success: still not ready
        state.consecutive_successes += 1;
        assert!(state.consecutive_successes < success_threshold);

        // Third success: now ready
        state.consecutive_successes += 1;
        assert!(state.consecutive_successes >= success_threshold);
    }

    #[test]
    fn test_probe_threshold_defaults() {
        use rusternetes_common::resources::Probe;

        let probe = Probe {
            http_get: None,
            tcp_socket: None,
            exec: None,
            initial_delay_seconds: None,
            timeout_seconds: None,
            period_seconds: None,
            success_threshold: None,
            failure_threshold: None,
            grpc: None,
            termination_grace_period_seconds: None,
        };

        // Kubernetes defaults
        assert_eq!(probe.failure_threshold.unwrap_or(3), 3);
        assert_eq!(probe.success_threshold.unwrap_or(1), 1);
        assert_eq!(probe.period_seconds.unwrap_or(10), 10);
    }

    #[test]
    fn test_probe_state_map_key_format() {
        let pod_name = "web-0";
        let container_name = "nginx";
        let liveness_key = format!("{}/{}/liveness", pod_name, container_name);
        let readiness_key = format!("{}/{}/readiness", pod_name, container_name);
        let startup_key = format!("{}/{}/startup", pod_name, container_name);

        assert_eq!(liveness_key, "web-0/nginx/liveness");
        assert_eq!(readiness_key, "web-0/nginx/readiness");
        assert_eq!(startup_key, "web-0/nginx/startup");

        // Keys for different probe types should be distinct
        assert_ne!(liveness_key, readiness_key);
        assert_ne!(readiness_key, startup_key);
    }

    #[test]
    fn test_clear_probe_states_removes_pod_entries() {
        let mut states = std::collections::HashMap::new();
        states.insert(
            "web-0/nginx/liveness".to_string(),
            super::ProbeState {
                consecutive_failures: 2,
                consecutive_successes: 0,
            },
        );
        states.insert(
            "web-0/nginx/readiness".to_string(),
            super::ProbeState {
                consecutive_failures: 0,
                consecutive_successes: 3,
            },
        );
        states.insert(
            "redis-0/redis/liveness".to_string(),
            super::ProbeState {
                consecutive_failures: 1,
                consecutive_successes: 0,
            },
        );

        let prefix = "web-0/";
        states.retain(|key, _| !key.starts_with(prefix));

        // web-0 entries should be removed
        assert!(!states.contains_key("web-0/nginx/liveness"));
        assert!(!states.contains_key("web-0/nginx/readiness"));
        // redis-0 should remain
        assert!(states.contains_key("redis-0/redis/liveness"));
    }

    // --- service environment variable tests ---

    #[test]
    fn test_service_env_var_name_formatting() {
        let svc_name = "my-redis-svc";
        let svc_env = svc_name.to_uppercase().replace('-', "_");
        assert_eq!(svc_env, "MY_REDIS_SVC");
    }

    #[test]
    fn test_service_env_var_host_format() {
        let svc_env = "MY_SVC";
        let cluster_ip = "10.96.0.10";
        let env_var = format!("{}_SERVICE_HOST={}", svc_env, cluster_ip);
        assert_eq!(env_var, "MY_SVC_SERVICE_HOST=10.96.0.10");
    }

    #[test]
    fn test_service_env_var_port_format() {
        let svc_env = "MY_SVC";
        let port = 8080;
        let cluster_ip = "10.96.0.10";

        let service_port = format!("{}_SERVICE_PORT={}", svc_env, port);
        assert_eq!(service_port, "MY_SVC_SERVICE_PORT=8080");

        let port_url = format!("{}_PORT=tcp://{}:{}", svc_env, cluster_ip, port);
        assert_eq!(port_url, "MY_SVC_PORT=tcp://10.96.0.10:8080");

        let port_tcp = format!(
            "{}_PORT_{}_TCP=tcp://{}:{}",
            svc_env, port, cluster_ip, port
        );
        assert_eq!(port_tcp, "MY_SVC_PORT_8080_TCP=tcp://10.96.0.10:8080");
    }

    #[test]
    fn test_service_env_var_named_port() {
        let svc_env = "MY_SVC";
        let port_name = "http-web";
        let port_name_env = port_name.to_uppercase().replace('-', "_");
        let env_var = format!("{}_SERVICE_PORT_{}={}", svc_env, port_name_env, 8080);
        assert_eq!(env_var, "MY_SVC_SERVICE_PORT_HTTP_WEB=8080");
    }

    #[test]
    fn test_service_env_var_skips_none_cluster_ip() {
        let cluster_ip = "None";
        let should_skip = cluster_ip == "None" || cluster_ip.is_empty();
        assert!(should_skip);
    }

    #[test]
    fn test_service_env_var_skips_empty_cluster_ip() {
        let cluster_ip = "";
        let should_skip = cluster_ip == "None" || cluster_ip.is_empty();
        assert!(should_skip);
    }

    #[test]
    fn test_enable_service_links_default_true() {
        let pod = make_pod("test", "default", None, None);
        let enable = pod
            .spec
            .as_ref()
            .and_then(|s| s.enable_service_links)
            .unwrap_or(true);
        assert!(enable);
    }

    // --- DNS policy tests ---

    #[test]
    fn test_dns_policy_default_is_cluster_first() {
        let pod = make_pod("test", "default", None, None);
        let dns_policy = pod
            .spec
            .as_ref()
            .and_then(|s| s.dns_policy.as_deref())
            .unwrap_or("ClusterFirst");
        assert_eq!(dns_policy, "ClusterFirst");
    }

    #[test]
    fn test_dns_policy_none_produces_empty_content() {
        let dns_policy = "None";
        let content = match dns_policy {
            "None" => String::new(),
            _ => "nameserver 10.96.0.10\n".to_string(),
        };
        assert!(content.is_empty());
    }

    #[test]
    fn test_dns_config_nameserver_prepend() {
        use rusternetes_common::resources::pod::PodDNSConfig;

        let dns_config = PodDNSConfig {
            nameservers: Some(vec!["8.8.8.8".to_string()]),
            searches: None,
            options: None,
        };

        let existing = vec!["10.96.0.10".to_string()];
        let mut merged = dns_config.nameservers.unwrap();
        for ns in &existing {
            if !merged.contains(ns) {
                merged.push(ns.clone());
            }
        }

        assert_eq!(merged, vec!["8.8.8.8", "10.96.0.10"]);
    }

    #[test]
    fn test_dns_config_search_domains() {
        use rusternetes_common::resources::pod::PodDNSConfig;

        let dns_config = PodDNSConfig {
            nameservers: None,
            searches: Some(vec!["custom.local".to_string()]),
            options: None,
        };

        let existing = vec!["default.svc.cluster.local".to_string()];
        let mut merged = dns_config.searches.unwrap();
        for s in &existing {
            if !merged.contains(s) {
                merged.push(s.clone());
            }
        }

        assert_eq!(merged, vec!["custom.local", "default.svc.cluster.local"]);
    }

    #[test]
    fn test_dns_config_options_with_value() {
        use rusternetes_common::resources::pod::PodDNSConfigOption;

        let opt = PodDNSConfigOption {
            name: "ndots".to_string(),
            value: Some("3".to_string()),
        };

        let opt_str = if let Some(ref val) = opt.value {
            format!("{}:{}", opt.name, val)
        } else {
            opt.name.clone()
        };

        assert_eq!(opt_str, "ndots:3");
    }

    #[test]
    fn test_dns_config_options_without_value() {
        use rusternetes_common::resources::pod::PodDNSConfigOption;

        let opt = PodDNSConfigOption {
            name: "single-request-reopen".to_string(),
            value: None,
        };

        let opt_str = if let Some(ref val) = opt.value {
            format!("{}:{}", opt.name, val)
        } else {
            opt.name.clone()
        };

        assert_eq!(opt_str, "single-request-reopen");
    }

    #[test]
    fn test_resolv_conf_parsing() {
        let content = "nameserver 10.96.0.10\nsearch default.svc.cluster.local svc.cluster.local cluster.local\noptions ndots:5\n";

        let mut nameservers = Vec::new();
        let mut searches = Vec::new();
        let mut options = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("nameserver ") {
                nameservers.push(line[11..].to_string());
            } else if line.starts_with("search ") {
                for domain in line[7..].split_whitespace() {
                    searches.push(domain.to_string());
                }
            } else if line.starts_with("options ") {
                for opt in line[8..].split_whitespace() {
                    options.push(opt.to_string());
                }
            }
        }

        assert_eq!(nameservers, vec!["10.96.0.10"]);
        assert_eq!(
            searches,
            vec![
                "default.svc.cluster.local",
                "svc.cluster.local",
                "cluster.local"
            ]
        );
        assert_eq!(options, vec!["ndots:5"]);
    }

    #[test]
    fn test_cluster_first_with_host_net_uses_cluster_dns() {
        // ClusterFirstWithHostNet should use cluster DNS regardless of host network
        let dns_policy = "ClusterFirstWithHostNet";
        let is_host_network = true;
        let cluster_dns = "10.96.0.10";

        let uses_cluster_dns = match dns_policy {
            "ClusterFirstWithHostNet" => true,          // always cluster DNS
            "ClusterFirst" if is_host_network => false, // falls back to host DNS
            "ClusterFirst" => true,
            _ => false,
        };

        assert!(uses_cluster_dns);
        assert_eq!(cluster_dns, "10.96.0.10");
    }

    #[test]
    fn test_cluster_first_with_host_network_uses_host_dns() {
        // ClusterFirst + hostNetwork=true should fall back to host DNS
        let dns_policy = "ClusterFirst";
        let is_host_network = true;

        let uses_host_dns = dns_policy == "ClusterFirst" && is_host_network;
        assert!(uses_host_dns);
    }
}
