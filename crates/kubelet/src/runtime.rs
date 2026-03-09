use anyhow::Result;
use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, RemoveContainerOptions,
    StartContainerOptions,
};
use bollard::Docker;
use rusternetes_common::resources::{Container, Pod};
use std::collections::HashMap;
use tracing::{debug, info};

/// ContainerRuntime manages containers using Docker
pub struct ContainerRuntime {
    docker: Docker,
}

impl ContainerRuntime {
    pub async fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;
        Ok(Self { docker })
    }

    /// Start all containers for a pod
    pub async fn start_pod(&self, pod: &Pod) -> Result<()> {
        info!("Starting pod: {}", pod.metadata.name);

        for container in &pod.spec.containers {
            self.start_container(&pod.metadata.name, container).await?;
        }

        Ok(())
    }

    async fn start_container(&self, pod_name: &str, container: &Container) -> Result<()> {
        let container_name = format!("{}_{}", pod_name, container.name);

        info!("Starting container: {}", container_name);

        // Build environment variables
        let env: Option<Vec<String>> = container.env.as_ref().map(|env_vars| {
            env_vars
                .iter()
                .filter_map(|e| {
                    e.value
                        .as_ref()
                        .map(|v| format!("{}={}", e.name, v))
                })
                .collect()
        });

        // Create container configuration
        let config = Config {
            image: Some(container.image.clone()),
            cmd: container.args.clone(),
            env,
            working_dir: container.working_dir.clone(),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: container_name.clone(),
            ..Default::default()
        };

        // Create the container
        self.docker.create_container(Some(options), config).await?;

        // Start the container
        self.docker
            .start_container(&container_name, None::<StartContainerOptions<String>>)
            .await?;

        debug!("Container {} started successfully", container_name);
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
            if let Some(names) = container.names {
                for name in names {
                    let container_name = name.trim_start_matches('/');
                    info!("Removing container: {}", container_name);

                    let remove_options = RemoveContainerOptions {
                        force: true,
                        ..Default::default()
                    };

                    self.docker
                        .remove_container(container_name, Some(remove_options))
                        .await?;
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
}
