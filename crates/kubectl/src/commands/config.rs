use crate::kubeconfig::KubeConfig;
use crate::types::ConfigCommands;
use anyhow::Result;
use std::path::PathBuf;

/// Execute config commands for kubeconfig management
pub async fn execute(command: ConfigCommands, kubeconfig_path: Option<&str>) -> Result<()> {
    let config_path = if let Some(path) = kubeconfig_path {
        PathBuf::from(path)
    } else {
        KubeConfig::default_path()?
    };

    match command {
        ConfigCommands::CurrentContext {} => {
            let config = KubeConfig::load_from_file(&config_path)?;
            println!("{}", config.current_context);
        }
        ConfigCommands::View { minify, flatten } => {
            let config = KubeConfig::load_from_file(&config_path)?;
            let yaml = serde_yaml::to_string(&config)?;
            println!("{}", yaml);
        }
        ConfigCommands::UseContext { name } => {
            let mut config = KubeConfig::load_from_file(&config_path)?;

            // Verify the context exists
            if !config.contexts.iter().any(|ctx| ctx.name == name) {
                anyhow::bail!("Context '{}' not found in kubeconfig", name);
            }

            config.current_context = name.clone();

            // Save the updated config
            let yaml = serde_yaml::to_string(&config)?;
            std::fs::write(&config_path, yaml)?;

            println!("Switched to context \"{}\"", name);
        }
        ConfigCommands::GetContexts { output } => {
            let config = KubeConfig::load_from_file(&config_path)?;
            println!("CURRENT   NAME                CLUSTER             AUTHINFO");
            for ctx in &config.contexts {
                let current = if ctx.name == config.current_context {
                    "*"
                } else {
                    " "
                };
                println!(
                    "{}         {:<20} {:<20} {}",
                    current, ctx.name, ctx.context.cluster, ctx.context.user
                );
            }
        }
        ConfigCommands::GetClusters {} => {
            let config = KubeConfig::load_from_file(&config_path)?;
            println!("NAME");
            for cluster in &config.clusters {
                println!("{}", cluster.name);
            }
        }
        ConfigCommands::Set { property, value } => {
            let mut config = KubeConfig::load_from_file(&config_path)?;

            // Parse property path (e.g., "current-context", "contexts.default.namespace")
            let parts: Vec<&str> = property.split('.').collect();

            match parts.as_slice() {
                ["current-context"] => {
                    config.current_context = value.clone();
                    println!("Property \"current-context\" set");
                }
                ["contexts", ctx_name, "namespace"] => {
                    // Find and update context namespace
                    let mut found = false;
                    for ctx in &mut config.contexts {
                        if ctx.name == *ctx_name {
                            ctx.context.namespace = value.clone();
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        anyhow::bail!("Context '{}' not found", ctx_name);
                    }
                    println!("Property \"contexts.{}.namespace\" set", ctx_name);
                }
                ["contexts", ctx_name, "cluster"] => {
                    let mut found = false;
                    for ctx in &mut config.contexts {
                        if ctx.name == *ctx_name {
                            ctx.context.cluster = value.clone();
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        anyhow::bail!("Context '{}' not found", ctx_name);
                    }
                    println!("Property \"contexts.{}.cluster\" set", ctx_name);
                }
                ["contexts", ctx_name, "user"] => {
                    let mut found = false;
                    for ctx in &mut config.contexts {
                        if ctx.name == *ctx_name {
                            ctx.context.user = value.clone();
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        anyhow::bail!("Context '{}' not found", ctx_name);
                    }
                    println!("Property \"contexts.{}.user\" set", ctx_name);
                }
                ["clusters", cluster_name, "server"] => {
                    let mut found = false;
                    for cluster in &mut config.clusters {
                        if cluster.name == *cluster_name {
                            cluster.cluster.server = value.clone();
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        anyhow::bail!("Cluster '{}' not found", cluster_name);
                    }
                    println!("Property \"clusters.{}.server\" set", cluster_name);
                }
                _ => {
                    anyhow::bail!("Unsupported property path: {}. Supported: current-context, contexts.<name>.namespace, contexts.<name>.cluster, contexts.<name>.user, clusters.<name>.server", property);
                }
            }

            // Save the updated config
            let yaml = serde_yaml::to_string(&config)?;
            std::fs::write(&config_path, yaml)?;
        }
        ConfigCommands::Unset { property } => {
            let mut config = KubeConfig::load_from_file(&config_path)?;

            // Parse property path
            let parts: Vec<&str> = property.split('.').collect();

            match parts.as_slice() {
                ["contexts", ctx_name, "namespace"] => {
                    let mut found = false;
                    for ctx in &mut config.contexts {
                        if ctx.name == *ctx_name {
                            ctx.context.namespace = String::new();
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        anyhow::bail!("Context '{}' not found", ctx_name);
                    }
                    println!("Property \"contexts.{}.namespace\" unset", ctx_name);
                }
                _ => {
                    anyhow::bail!("Unsupported property path for unset: {}. Supported: contexts.<name>.namespace", property);
                }
            }

            // Save the updated config
            let yaml = serde_yaml::to_string(&config)?;
            std::fs::write(&config_path, yaml)?;
        }
    }

    Ok(())
}
