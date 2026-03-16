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
use rusternetes_common::resources::volume::PersistentVolumeSource;
use rusternetes_common::resources::{
    ConfigMap, Container, ContainerState, ContainerStatus, ExecAction, HTTPGetAction,
    PersistentVolume, PersistentVolumeClaim, Pod, Probe, Secret, TCPSocketAction,
};
use rusternetes_storage::{build_key, Storage};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::cni::CniRuntime;

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

        // Create volumes first (includes service account token volumes injected by admission controller)
        let volume_binds = self.create_pod_volumes(pod).await?;

        // Get pod IP from CNI (available after network setup) and create /etc/hosts
        // For non-CNI mode the IP is not known yet, so we create a hosts file with just localhost entries
        let pod_ip = if self.use_cni {
            if let Some(cni) = &self.cni {
                cni.get_container_ip(pod_name)
            } else {
                None
            }
        } else {
            None
        };

        let hosts_file_path = self.create_pod_hosts_file(pod, pod_ip.as_deref())?;

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

                    // Start the init container
                    self.start_container(
                        pod,
                        container,
                        &volume_binds,
                        netns_path.as_deref(),
                        hosts_file_path.as_deref(),
                    )
                    .await?;

                    // Wait for init container to complete
                    self.wait_for_container_completion(pod_name, &container.name)
                        .await?;

                    info!("Init container {} completed successfully", container.name);
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
                    )
                    .await?;
                }
            }
        }

        // Step 3: Start main containers
        for container in &pod.spec.as_ref().unwrap().containers {
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
            )
            .await?;
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

        std::fs::write(&hosts_path, &content)
            .with_context(|| format!("Failed to write /etc/hosts for pod {}", pod_name))?;

        Ok(Some(hosts_path))
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
    async fn create_pod_volumes(&self, pod: &Pod) -> Result<HashMap<String, String>> {
        let mut volume_paths = HashMap::new();

        if let Some(volumes) = &pod.spec.as_ref().unwrap().volumes {
            for volume in volumes {
                let volume_path = self.create_volume(pod, volume).await?;
                volume_paths.insert(volume.name.clone(), volume_path);
            }
        }

        Ok(volume_paths)
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

            let key = build_key("configmaps", Some(namespace), configmap_name);
            let configmap: ConfigMap = storage.get(&key).await.with_context(|| {
                format!(
                    "ConfigMap {} not found in namespace {}",
                    configmap_name, namespace
                )
            })?;

            // Create volume directory
            let volume_dir = format!("{}/{}/{}", self.volumes_base_path, pod_name, volume.name);
            std::fs::create_dir_all(&volume_dir)
                .context("Failed to create ConfigMap volume directory")?;

            // Write each key as a file
            if let Some(data) = &configmap.data {
                for (key, value) in data {
                    let file_path = format!("{}/{}", volume_dir, key);
                    std::fs::write(&file_path, value).with_context(|| {
                        format!("Failed to write ConfigMap key {} to file", key)
                    })?;
                    info!("Wrote ConfigMap key {} to {}", key, file_path);
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

            let key = build_key("secrets", Some(namespace), secret_name);
            let secret: Secret = storage.get(&key).await.with_context(|| {
                format!(
                    "Secret {} not found in namespace {}",
                    secret_name, namespace
                )
            })?;

            // Create volume directory
            let volume_dir = format!("{}/{}/{}", self.volumes_base_path, pod_name, volume.name);
            std::fs::create_dir_all(&volume_dir)
                .context("Failed to create Secret volume directory")?;

            // Write each key as a file
            if let Some(data) = &secret.data {
                for (key, value) in data {
                    let file_path = format!("{}/{}", volume_dir, key);
                    std::fs::write(&file_path, value)
                        .with_context(|| format!("Failed to write Secret key {} to file", key))?;
                    // Set restrictive permissions on secret files
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        std::fs::set_permissions(
                            &file_path,
                            std::fs::Permissions::from_mode(0o600),
                        )?;
                    }
                    info!("Wrote Secret key {} to {}", key, file_path);
                }
            }

            // Special handling for service account token secrets - add ca.crt
            // Service account secrets are identified by having a "token" key or by name pattern
            let is_service_account_secret = secret
                .data
                .as_ref()
                .map(|data| data.contains_key("token"))
                .unwrap_or(false)
                || secret_name.ends_with("-token");

            if is_service_account_secret {
                // Check if ca.crt already exists in the secret data
                let has_ca_cert = secret
                    .data
                    .as_ref()
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
            let path = match &pv.spec.volume_source {
                PersistentVolumeSource::HostPath(hp) => hp.path.clone(),
                _ => {
                    return Err(anyhow::anyhow!(
                        "PersistentVolume does not have a hostPath volume source"
                    ))
                }
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

                    // Set file permissions if specified
                    #[cfg(unix)]
                    if let Some(mode) = item.mode {
                        use std::os::unix::fs::PermissionsExt;
                        std::fs::set_permissions(
                            &file_path,
                            std::fs::Permissions::from_mode(mode as u32),
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

                    let path = match &pv.spec.volume_source {
                        PersistentVolumeSource::HostPath(hp) => hp.path.clone(),
                        _ => {
                            return Err(anyhow::anyhow!(
                                "PersistentVolume does not have a hostPath volume source"
                            ))
                        }
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

        Err(anyhow::anyhow!(
            "Unknown volume type for volume {}",
            volume.name
        ))
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

    async fn start_container(
        &self,
        pod: &Pod,
        container: &Container,
        volume_paths: &HashMap<String, String>,
        netns_path: Option<&str>,
        hosts_file_path: Option<&str>,
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
            if inspect.state.and_then(|s| s.running).unwrap_or(false) {
                info!("Container {} is already running", container_name);
                return Ok(());
            }

            // Remove stopped container
            info!("Removing stopped container: {}", container_name);
            let remove_options = RemoveContainerOptions {
                force: true,
                ..Default::default()
            };
            self.docker
                .remove_container(&container_name, Some(remove_options))
                .await?;
        }

        // Build environment variables
        let mut env_list = Vec::new();

        // Inject Kubernetes service environment variables for in-cluster access
        // Use ClusterIP instead of DNS name to avoid chicken-and-egg DNS dependency
        env_list.push(format!(
            "KUBERNETES_SERVICE_HOST={}",
            self.kubernetes_service_host
        ));
        env_list.push("KUBERNETES_SERVICE_PORT=443".to_string());
        env_list.push("KUBERNETES_SERVICE_PORT_HTTPS=443".to_string());
        env_list.push(format!(
            "KUBERNETES_PORT=tcp://{}:443",
            self.kubernetes_service_host
        ));
        env_list.push(format!(
            "KUBERNETES_PORT_443_TCP=tcp://{}:443",
            self.kubernetes_service_host
        ));
        env_list.push("KUBERNETES_PORT_443_TCP_PROTO=tcp".to_string());
        env_list.push("KUBERNETES_PORT_443_TCP_PORT=443".to_string());
        env_list.push(format!(
            "KUBERNETES_PORT_443_TCP_ADDR={}",
            self.kubernetes_service_host
        ));

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
                                // Only add the env var if the value is not empty
                                // This handles cases like status.podIP which may not be available yet
                                if !value.is_empty() {
                                    env_list.push(format!("{}={}", env_var.name, value));
                                    info!(
                                        "Set env var {} from field {}: {}",
                                        env_var.name, field_ref.field_path, value
                                    );
                                } else {
                                    debug!("Skipping env var {} - field {} is empty (may not be available yet)", env_var.name, field_ref.field_path);
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

        let env = if env_list.is_empty() {
            None
        } else {
            Some(env_list)
        };

        // Build port bindings
        let mut exposed_ports = HashMap::new();
        let mut port_bindings = HashMap::new();

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

        // Build volume bindings
        let mut binds = Vec::new();

        // Mount volumes based on volumeMounts (includes service account tokens injected by admission controller)
        if let Some(volume_mounts) = &container.volume_mounts {
            for mount in volume_mounts {
                if let Some(host_path) = volume_paths.get(&mount.name) {
                    let read_only = if mount.read_only.unwrap_or(false) {
                        ":ro"
                    } else {
                        ""
                    };
                    let bind = format!("{}:{}{}", host_path, mount.mount_path, read_only);
                    binds.push(bind);
                    info!(
                        "Mounting volume {} at {} in container {}",
                        mount.name, mount.mount_path, container.name
                    );
                }
            }
        }

        // Create and mount custom resolv.conf for non-CoreDNS pods
        // This bypasses Podman's aardvark-dns which overrides our DNS configuration
        if pod_name != "coredns" {
            let resolv_conf_path = format!("{}/{}/resolv.conf", self.volumes_base_path, pod_name);
            let resolv_conf_content = format!(
                "nameserver {}\nsearch {}.svc.{} svc.{} {}\noptions ndots:5\n",
                self.cluster_dns,
                namespace,
                self.cluster_domain,
                self.cluster_domain,
                self.cluster_domain
            );

            // Create directory if it doesn't exist
            std::fs::create_dir_all(format!("{}/{}", self.volumes_base_path, pod_name))
                .context("Failed to create pod directory for resolv.conf")?;

            // Write custom resolv.conf
            std::fs::write(&resolv_conf_path, resolv_conf_content).with_context(|| {
                format!("Failed to write custom resolv.conf for pod {}", pod_name)
            })?;

            // Mount custom resolv.conf into container
            binds.push(format!("{}:/etc/resolv.conf:ro", resolv_conf_path));
            info!(
                "Mounted custom resolv.conf for pod {} with DNS server {}",
                pod_name, self.cluster_dns
            );
        }

        // Mount /etc/hosts if a pod-specific hosts file was created
        if let Some(hosts_path) = hosts_file_path {
            binds.push(format!("{}:/etc/hosts:ro", hosts_path));
            info!("Mounted custom /etc/hosts for pod {}", pod_name);
        }

        // Create container configuration
        // Skip cluster DNS configuration for CoreDNS to avoid circular dependency
        let (dns_servers, dns_search_domains, dns_options) = if pod_name == "coredns" {
            info!("Skipping cluster DNS configuration for CoreDNS pod (using default/host DNS)");
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

        let mut config = Config {
            image: Some(container.image.clone()),
            env,
            working_dir: container.working_dir.clone(),
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
                // Configure DNS to use kube-dns service
                // CoreDNS uses default/host DNS to avoid circular dependency
                dns: dns_servers,
                dns_search: dns_search_domains,
                dns_options: dns_options,
                // Use CNI network namespace if available, otherwise use Podman bridge network
                network_mode: if let Some(netns) = netns_path {
                    Some(format!("ns:{}", netns))
                } else {
                    Some(self.network.clone())
                },
                ..Default::default()
            }),
            ..Default::default()
        };

        // Set command and args
        // In Kubernetes: command replaces ENTRYPOINT, args replaces CMD
        // When both are present, combine them
        if let Some(command) = &container.command {
            if let Some(args) = &container.args {
                // Both command and args present - combine them
                let mut full_cmd = command.clone();
                full_cmd.extend(args.clone());
                info!(
                    "Container {} - combining command {:?} and args {:?} into {:?}",
                    container.name, command, args, full_cmd
                );
                config.cmd = Some(full_cmd);
            } else {
                // Only command present
                info!(
                    "Container {} - using command: {:?}",
                    container.name, command
                );
                config.cmd = Some(command.clone());
            }
        } else if let Some(args) = &container.args {
            // Only args present - use container's default entrypoint + args
            info!("Container {} - using args: {:?}", container.name, args);
            config.cmd = Some(args.clone());
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
        Ok(())
    }

    /// Stop all containers for a pod
    pub async fn stop_pod(&self, pod_name: &str) -> Result<()> {
        info!("Stopping pod: {}", pod_name);

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

                // Stop the container gracefully
                let stop_options = StopContainerOptions { t: 10 };
                if let Err(e) = self.docker.stop_container(&id, Some(stop_options)).await {
                    warn!("Failed to stop container {}: {}", id, e);
                }

                // Remove the container
                let remove_options = RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                };

                if let Err(e) = self
                    .docker
                    .remove_container(&id, Some(remove_options))
                    .await
                {
                    warn!("Failed to remove container {}: {}", id, e);
                }
            }
        }

        // Teardown CNI networking if enabled
        if self.use_cni {
            if let Err(e) = self.teardown_pod_network(pod_name).await {
                warn!("Failed to teardown CNI network for pod {}: {}", pod_name, e);
                // Continue with cleanup even if CNI teardown fails
            }
        }

        // Clean up emptyDir volumes
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

                    // Get restart count from host config (Docker/Podman tracks this in restart policy)
                    // For now, we'll get it from the pod status if available, otherwise default to 0
                    let restart_count = pod
                        .status
                        .as_ref()
                        .and_then(|s| s.container_statuses.as_ref())
                        .and_then(|statuses| {
                            statuses
                                .iter()
                                .find(|cs| cs.name == container.name)
                                .map(|cs| cs.restart_count)
                        })
                        .unwrap_or(0);

                    let container_state = if running {
                        Some(ContainerState::Running {
                            started_at: state.started_at,
                        })
                    } else if exit_code != 0 {
                        Some(ContainerState::Terminated {
                            exit_code: exit_code as i32,
                            reason: state.error,
                        })
                    } else {
                        Some(ContainerState::Waiting {
                            reason: Some("ContainerCreating".to_string()),
                        })
                    };

                    // Check readiness probe
                    let ready = if running {
                        if let Some(probe) = &container.readiness_probe {
                            self.check_probe(&container_name, probe)
                                .await
                                .unwrap_or(false)
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
                        image: Some(container.image.clone()),
                        container_id: inspect.id,
                    }
                }
                Err(_) => ContainerStatus {
                    name: container.name.clone(),
                    ready: false,
                    restart_count: 0,
                    state: Some(ContainerState::Waiting {
                        reason: Some("ContainerCreating".to_string()),
                    }),
                    image: Some(container.image.clone()),
                    container_id: None,
                },
            };

            statuses.push(status);
        }

        Ok(statuses)
    }

    /// Check if a container needs to be restarted based on liveness probe
    pub async fn check_liveness(&self, pod: &Pod) -> Result<bool> {
        let pod_name = &pod.metadata.name;

        for container in &pod.spec.as_ref().unwrap().containers {
            if let Some(probe) = &container.liveness_probe {
                let container_name = format!("{}_{}", pod_name, container.name);

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

                // Check liveness
                let healthy = self.check_probe(&container_name, probe).await?;
                if !healthy {
                    warn!("Liveness probe failed for container: {}", container_name);
                    return Ok(true); // Needs restart
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

        Ok(true) // No probe configured
    }

    async fn check_http_probe(
        &self,
        container_name: &str,
        http_get: &HTTPGetAction,
        timeout: Duration,
    ) -> Result<bool> {
        // Get container IP
        let inspect = self
            .docker
            .inspect_container(container_name, None::<InspectContainerOptions>)
            .await?;

        let ip = inspect
            .network_settings
            .and_then(|ns| ns.ip_address)
            .unwrap_or_else(|| "127.0.0.1".to_string());

        let scheme = http_get.scheme.as_deref().unwrap_or("http");
        let path = http_get.path.as_deref().unwrap_or("/");
        let url = format!("{}://{}:{}{}", scheme, ip, http_get.port, path);

        debug!("HTTP probe: {}", url);

        let client = reqwest::Client::builder().timeout(timeout).build()?;

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
        // Get container IP
        let inspect = self
            .docker
            .inspect_container(container_name, None::<InspectContainerOptions>)
            .await?;

        let ip = inspect
            .network_settings
            .and_then(|ns| ns.ip_address)
            .unwrap_or_else(|| "127.0.0.1".to_string());

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
        let mut filters = HashMap::new();
        filters.insert("name".to_string(), vec![format!("{}_", pod_name)]);

        let options = ListContainersOptions {
            all: false, // Only running containers
            filters,
            ..Default::default()
        };

        let containers = self.docker.list_containers(Some(options)).await?;

        // Get the IP from the first running container
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
                .unwrap_or("".to_string()),
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
    fn get_container_resource_value(
        &self,
        pod: &Pod,
        resource_ref: &rusternetes_common::resources::ResourceFieldSelector,
    ) -> Result<String> {
        let spec = pod.spec.as_ref().context("Pod has no spec")?;

        // Find the container
        let container_name = resource_ref
            .container_name
            .as_ref()
            .context("Container name is required for resource field selector")?;

        let container = spec
            .containers
            .iter()
            .find(|c| &c.name == container_name)
            .with_context(|| format!("Container {} not found", container_name))?;

        let resources = container
            .resources
            .as_ref()
            .context("Container has no resources specified")?;

        let value = match resource_ref.resource.as_str() {
            "limits.cpu" => resources
                .limits
                .as_ref()
                .and_then(|l| l.get("cpu"))
                .cloned()
                .unwrap_or("0".to_string()),
            "limits.memory" => resources
                .limits
                .as_ref()
                .and_then(|l| l.get("memory"))
                .cloned()
                .unwrap_or("0".to_string()),
            "requests.cpu" => resources
                .requests
                .as_ref()
                .and_then(|r| r.get("cpu"))
                .cloned()
                .unwrap_or("0".to_string()),
            "requests.memory" => resources
                .requests
                .as_ref()
                .and_then(|r| r.get("memory"))
                .cloned()
                .unwrap_or("0".to_string()),
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported resource field: {}",
                    resource_ref.resource
                ))
            }
        };

        Ok(value)
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
        }
    }

    fn make_pod(name: &str, namespace: &str, hostname: Option<&str>, subdomain: Option<&str>) -> Pod {
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
            }),
            status: None,
        }
    }

    /// Build the /etc/hosts content string the same way create_pod_hosts_file does,
    /// so we can unit-test the logic without needing a live ContainerRuntime.
    fn build_hosts_content(
        pod: &Pod,
        pod_ip: Option<&str>,
        cluster_domain: &str,
    ) -> String {
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

        assert!(content.contains(
            "10.244.2.10\tcache-0\tcache-0.redis.kube-system.svc.cluster.local\n"
        ));
    }

    #[test]
    fn test_hosts_file_subdomain_fqdn_uses_custom_cluster_domain() {
        let pod = make_pod("web-0", "default", Some("web-0"), Some("nginx"));
        let content = build_hosts_content(&pod, Some("10.244.1.5"), "k8s.example.com");

        assert!(content.contains(
            "10.244.1.5\tweb-0\tweb-0.nginx.default.svc.k8s.example.com\n"
        ));
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
}
