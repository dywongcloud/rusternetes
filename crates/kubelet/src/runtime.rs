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
    Container, ContainerState, ContainerStatus, ExecAction, HTTPGetAction, Pod, Probe,
    TCPSocketAction,
};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// ContainerRuntime manages containers using Docker/Podman
pub struct ContainerRuntime {
    docker: Docker,
}

impl ContainerRuntime {
    pub async fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;
        Ok(Self { docker })
    }

    /// Pull an image if necessary based on the pull policy
    async fn ensure_image(
        &self,
        image: &str,
        pull_policy: Option<&str>,
    ) -> Result<()> {
        let policy = pull_policy.unwrap_or("IfNotPresent");
        debug!("Ensuring image {} with policy {}", image, policy);

        // Check if image exists locally
        let image_exists = match self.docker.inspect_image(image).await {
            Ok(_) => {
                debug!("Image {} exists locally", image);
                true
            }
            Err(e) => {
                debug!("Image {} not found locally: {}", image, e);
                false
            }
        };

        let should_pull = match policy {
            "Always" => true,
            "Never" => false,
            "IfNotPresent" => !image_exists,
            _ => !image_exists, // Default to IfNotPresent
        };

        debug!("Image {} - exists: {}, should_pull: {}", image, image_exists, should_pull);

        if should_pull {
            info!("Pulling image: {}", image);
            let options = CreateImageOptions {
                from_image: image,
                ..Default::default()
            };

            let mut stream = self.docker.create_image(Some(options), None, None);

            while let Some(result) = stream.next().await {
                match result {
                    Ok(info) => {
                        if let Some(status) = info.status {
                            debug!("Image pull: {}", status);
                        }
                        if let Some(error) = info.error {
                            return Err(anyhow::anyhow!("Image pull failed: {}", error));
                        }
                    }
                    Err(e) => return Err(e.into()),
                }
            }
            info!("Successfully pulled image: {}", image);
        } else {
            debug!("Image {} already exists locally, skipping pull", image);
        }

        Ok(())
    }

    /// Start all containers for a pod
    pub async fn start_pod(&self, pod: &Pod) -> Result<()> {
        let pod_name = &pod.metadata.name;
        info!("Starting pod: {}", pod_name);

        for container in &pod.spec.containers {
            // Ensure image is available
            if let Err(e) = self
                .ensure_image(
                    &container.image,
                    container.image_pull_policy.as_deref(),
                )
                .await
            {
                error!("Failed to pull image for container {}: {}", container.name, e);
                return Err(e);
            }

            // Start the container
            self.start_container(pod_name, container).await?;
        }

        Ok(())
    }

    async fn start_container(&self, pod_name: &str, container: &Container) -> Result<()> {
        let container_name = format!("{}_{}", pod_name, container.name);

        info!("Starting container: {}", container_name);

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
        let env: Option<Vec<String>> = container.env.as_ref().map(|env_vars| {
            env_vars
                .iter()
                .filter_map(|e| e.value.as_ref().map(|v| format!("{}={}", e.name, v)))
                .collect()
        });

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

        // Create container configuration
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
                ..Default::default()
            }),
            ..Default::default()
        };

        // Set command and args
        if let Some(command) = &container.command {
            config.cmd = Some(command.clone());
        } else if let Some(args) = &container.args {
            config.cmd = Some(args.clone());
        }

        let options = CreateContainerOptions {
            name: container_name.clone(),
            ..Default::default()
        };

        // Create the container
        self.docker
            .create_container(Some(options), config)
            .await
            .context("Failed to create container")?;

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

                if let Err(e) = self.docker.remove_container(&id, Some(remove_options)).await {
                    warn!("Failed to remove container {}: {}", id, e);
                }
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
    pub async fn get_container_statuses(
        &self,
        pod: &Pod,
    ) -> Result<Vec<ContainerStatus>> {
        let mut statuses = Vec::new();
        let pod_name = &pod.metadata.name;

        for container in &pod.spec.containers {
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
                            self.check_probe(&container_name, probe).await.unwrap_or(false)
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

        for container in &pod.spec.containers {
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
                                if let Ok(started) = chrono::DateTime::parse_from_rfc3339(&started_at) {
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
            return self.check_http_probe(container_name, http_get, timeout).await;
        }

        // TCP Socket probe
        if let Some(tcp_socket) = &probe.tcp_socket {
            return self.check_tcp_probe(container_name, tcp_socket, timeout).await;
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

        let client = reqwest::Client::builder()
            .timeout(timeout)
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

    /// Restart a container
    pub async fn restart_container(&self, container_name: &str) -> Result<()> {
        info!("Restarting container: {}", container_name);

        let stop_options = StopContainerOptions { t: 10 };
        self.docker
            .stop_container(container_name, Some(stop_options))
            .await?;

        self.docker
            .start_container(container_name, None::<StartContainerOptions<String>>)
            .await?;

        Ok(())
    }

    /// Get the pod IP address from the first running container
    pub async fn get_pod_ip(&self, pod_name: &str) -> Result<Option<String>> {
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
                    if let Some(ip) = network_settings.ip_address {
                        if !ip.is_empty() && ip != "0.0.0.0" {
                            return Ok(Some(ip));
                        }
                    }
                }
            }
        }

        Ok(None)
    }
}
