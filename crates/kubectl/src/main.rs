mod client;
mod commands;
mod kubeconfig;
mod types;
mod websocket;

use anyhow::Result;
use clap::{Parser, Subcommand};
use client::ApiClient;
use kubeconfig::KubeConfig;
use types::{RolloutCommands, TopCommands, AuthCommands, ConfigCommands};

#[derive(Parser)]
#[command(name = "kubectl")]
#[command(about = "Rusternetes kubectl - Command line tool for Rusternetes")]
struct Cli {
    /// Path to kubeconfig file
    #[arg(long, global = true)]
    kubeconfig: Option<String>,

    /// Context to use from kubeconfig
    #[arg(long, global = true)]
    context: Option<String>,

    /// API server address (overrides kubeconfig)
    #[arg(long, global = true)]
    server: Option<String>,

    /// Skip TLS certificate verification (insecure)
    #[arg(long, global = true)]
    insecure_skip_tls_verify: bool,

    /// Bearer token for authentication (overrides kubeconfig)
    #[arg(long, global = true)]
    token: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get resources
    Get {
        /// Resource type (pod, service, deployment, node, namespace)
        resource_type: String,

        /// Resource name (optional)
        name: Option<String>,

        /// Namespace (for namespaced resources)
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// List resources in all namespaces
        #[arg(short = 'A', long)]
        all_namespaces: bool,

        /// Output format (json, yaml, wide, name, custom-columns, jsonpath, go-template)
        #[arg(short = 'o', long)]
        output: Option<String>,

        /// Don't print headers (for table output)
        #[arg(long)]
        no_headers: bool,

        /// Label selector to filter on
        #[arg(short = 'l', long)]
        selector: Option<String>,

        /// Field selector to filter on
        #[arg(long)]
        field_selector: Option<String>,

        /// Watch for changes
        #[arg(short = 'w', long)]
        watch: bool,

        /// Show resource details in additional columns (wider output)
        #[arg(long)]
        show_labels: bool,
    },

    /// Create a resource from a file
    Create {
        /// Path to YAML file
        #[arg(short = 'f', long)]
        file: Option<String>,

        /// Namespace for the resource
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// Inline resource type and name (e.g., kubectl create namespace foo)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Delete a resource
    Delete {
        /// Resource type (pod, service, deployment, node, namespace)
        resource_type: Option<String>,

        /// Resource name
        name: Option<String>,

        /// Delete from file
        #[arg(short = 'f', long)]
        file: Option<String>,

        /// Namespace (for namespaced resources)
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// Label selector to filter resources to delete
        #[arg(short = 'l', long)]
        selector: Option<String>,

        /// Delete all resources of the specified type
        #[arg(long)]
        all: bool,

        /// Force deletion (skip graceful delete)
        #[arg(long)]
        force: bool,

        /// Grace period in seconds
        #[arg(long)]
        grace_period: Option<i64>,
    },

    /// Apply a configuration to a resource
    Apply {
        /// Path to YAML file
        #[arg(short = 'f', long)]
        file: String,

        /// Namespace for the resource
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// Perform a dry run
        #[arg(long)]
        dry_run: Option<String>,

        /// Use server-side apply
        #[arg(long)]
        server_side: bool,

        /// Force apply (for conflicts)
        #[arg(long)]
        force: bool,
    },

    /// Describe a resource
    Describe {
        /// Resource type (pod, service, deployment, node, namespace)
        resource_type: String,

        /// Resource name
        name: Option<String>,

        /// Namespace (for namespaced resources)
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// Label selector to filter resources
        #[arg(short = 'l', long)]
        selector: Option<String>,

        /// List resources in all namespaces
        #[arg(short = 'A', long)]
        all_namespaces: bool,
    },

    /// Get logs from a pod
    Logs {
        /// Pod name
        pod_name: String,

        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// Container name (optional, for multi-container pods)
        #[arg(short = 'c', long)]
        container: Option<String>,

        /// Follow the logs
        #[arg(short = 'f', long)]
        follow: bool,

        /// Number of lines to show from the end of the logs
        #[arg(long)]
        tail: Option<i64>,

        /// Show timestamps
        #[arg(long)]
        timestamps: bool,

        /// Show logs since a specific time (RFC3339)
        #[arg(long)]
        since_time: Option<String>,

        /// Show logs since a duration ago (e.g., 5s, 2m, 1h)
        #[arg(long)]
        since: Option<String>,

        /// Show logs for previous container instance
        #[arg(short = 'p', long)]
        previous: bool,
    },

    /// Execute a command in a container
    Exec {
        /// Pod name
        pod_name: String,

        /// Container name (optional, for multi-container pods)
        #[arg(short = 'c', long)]
        container: Option<String>,

        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// Allocate a TTY
        #[arg(short = 't', long)]
        tty: bool,

        /// Pass stdin to the container
        #[arg(short = 'i', long)]
        stdin: bool,

        /// Command to execute
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        command: Vec<String>,
    },

    /// Forward one or more local ports to a pod
    PortForward {
        /// Pod name
        pod_name: String,

        /// Port mappings (e.g., 8080:80 or 8080)
        ports: Vec<String>,

        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// Address to bind to
        #[arg(long, default_value = "localhost")]
        address: String,
    },

    /// Copy files to/from containers
    Cp {
        /// Source (pod:path or local path)
        source: String,

        /// Destination (pod:path or local path)
        destination: String,

        /// Container name
        #[arg(short = 'c', long)]
        container: Option<String>,

        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,
    },

    /// Edit a resource
    Edit {
        /// Resource type
        resource_type: String,

        /// Resource name
        name: String,

        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// Output format (json or yaml)
        #[arg(short = 'o', long, default_value = "yaml")]
        output: String,
    },

    /// Patch a resource
    Patch {
        /// Resource type
        resource_type: String,

        /// Resource name
        name: String,

        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// Patch content
        #[arg(short = 'p', long)]
        patch: Option<String>,

        /// Patch file
        #[arg(long)]
        patch_file: Option<String>,

        /// Patch type (json, merge, strategic)
        #[arg(long, default_value = "strategic")]
        patch_type: String,
    },

    /// Scale a resource
    Scale {
        /// Resource type
        resource_type: String,

        /// Resource name
        name: String,

        /// Number of replicas
        #[arg(long)]
        replicas: i32,

        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,
    },

    /// Rollout management commands
    Rollout {
        #[command(subcommand)]
        command: RolloutCommands,
    },

    /// Display resource usage (CPU/memory)
    Top {
        #[command(subcommand)]
        command: TopCommands,
    },

    /// Update labels on a resource
    Label {
        /// Resource type
        resource_type: String,

        /// Resource name
        name: String,

        /// Labels to set (key=value) or remove (key-)
        labels: Vec<String>,

        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// Overwrite existing labels
        #[arg(long)]
        overwrite: bool,
    },

    /// Update annotations on a resource
    Annotate {
        /// Resource type
        resource_type: String,

        /// Resource name
        name: String,

        /// Annotations to set (key=value) or remove (key-)
        annotations: Vec<String>,

        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// Overwrite existing annotations
        #[arg(long)]
        overwrite: bool,
    },

    /// Explain resource documentation
    Explain {
        /// Resource type (e.g., pod, pod.spec, pod.spec.containers)
        resource: String,

        /// API version to use
        #[arg(long)]
        api_version: Option<String>,

        /// Show recursive schema
        #[arg(long)]
        recursive: bool,
    },

    /// Wait for a specific condition on resources
    Wait {
        /// Resource type and name (e.g., pod/mypod)
        resource: Vec<String>,

        /// Condition to wait for
        #[arg(long)]
        for_condition: Option<String>,

        /// Wait for deletion
        #[arg(long)]
        for_delete: bool,

        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// Timeout duration
        #[arg(long, default_value = "30s")]
        timeout: String,

        /// Label selector
        #[arg(short = 'l', long)]
        selector: Option<String>,
    },

    /// Show difference between current and applied configuration
    Diff {
        /// Path to YAML file
        #[arg(short = 'f', long)]
        file: String,

        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,
    },

    /// Authentication commands
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },

    /// Display available API resources
    ApiResources {
        /// Show namespaced resources
        #[arg(long)]
        namespaced: Option<bool>,

        /// API group to filter
        #[arg(long)]
        api_group: Option<String>,

        /// Show only resource names
        #[arg(long)]
        no_headers: bool,

        /// Output format
        #[arg(short = 'o', long)]
        output: Option<String>,
    },

    /// Print supported API versions
    ApiVersions {},

    /// Manage kubeconfig files
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Display cluster information
    ClusterInfo {
        /// Dump detailed cluster information
        #[arg(long)]
        dump: bool,
    },

    /// Display Kubernetes version
    Version {
        /// Show client version only
        #[arg(long)]
        client: bool,

        /// Output format (json, yaml)
        #[arg(short = 'o', long)]
        output: Option<String>,
    },
}

// Rollout, Top, Auth, and Config commands are now in types.rs

/*
#[derive(Subcommand)]
enum RolloutCommands {
    /// Show rollout status
    Status {
        /// Resource type
        resource_type: String,
        /// Resource name
        name: String,
        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,
    },
    /// View rollout history
    History {
        /// Resource type
        resource_type: String,
        /// Resource name
        name: String,
        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,
        /// Show details of a specific revision
        #[arg(long)]
        revision: Option<i32>,
    },
    /// Rollback to a previous revision
    Undo {
        /// Resource type
        resource_type: String,
        /// Resource name
        name: String,
        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,
        /// Revision to rollback to
        #[arg(long)]
        to_revision: Option<i32>,
    },
    /// Restart a resource
    Restart {
        /// Resource type
        resource_type: String,
        /// Resource name
        name: String,
        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,
    },
    /// Pause a resource rollout
    Pause {
        /// Resource type
        resource_type: String,
        /// Resource name
        name: String,
        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,
    },
    /// Resume a paused resource
    Resume {
        /// Resource type
        resource_type: String,
        /// Resource name
        name: String,
        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,
    },
}

#[derive(Subcommand)]
enum TopCommands {
    /// Display resource usage of nodes
    Node {
        /// Node name (optional)
        name: Option<String>,
        /// Label selector
        #[arg(short = 'l', long)]
        selector: Option<String>,
    },
    /// Display resource usage of pods
    Pod {
        /// Pod name (optional)
        name: Option<String>,
        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,
        /// List pods in all namespaces
        #[arg(short = 'A', long)]
        all_namespaces: bool,
        /// Label selector
        #[arg(short = 'l', long)]
        selector: Option<String>,
        /// Show containers
        #[arg(long)]
        containers: bool,
    },
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Check if an action is allowed
    CanI {
        /// Verb (e.g., get, list, create, delete)
        verb: String,
        /// Resource (e.g., pods, deployments)
        resource: String,
        /// Resource name
        name: Option<String>,
        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,
        /// Check all namespaces
        #[arg(short = 'A', long)]
        all_namespaces: bool,
    },
    /// Experimental: Check who you are
    Whoami {
        /// Output format (json, yaml)
        #[arg(short = 'o', long)]
        output: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Display current context
    CurrentContext {},
    /// Display merged kubeconfig
    View {
        /// Minify output
        #[arg(long)]
        minify: bool,
        /// Flatten output
        #[arg(long)]
        flatten: bool,
    },
    /// Set a context
    UseContext {
        /// Context name
        name: String,
    },
    /// Get contexts
    GetContexts {
        /// Output format
        #[arg(short = 'o', long)]
        output: Option<String>,
    },
    /// Get clusters
    GetClusters {},
    /// Set a property in kubeconfig
    Set {
        /// Property path
        property: String,
        /// Value
        value: String,
    },
    /// Unset a property in kubeconfig
    Unset {
        /// Property path
        property: String,
    },
}
*/

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load kubeconfig or use CLI flags
    let (server, skip_tls, token, default_namespace) = if let Some(server_url) = cli.server {
        // CLI flags override kubeconfig
        (server_url, cli.insecure_skip_tls_verify, cli.token, "default".to_string())
    } else {
        // Try to load from kubeconfig
        let config = if let Some(path) = &cli.kubeconfig {
            KubeConfig::load_from_file(&std::path::PathBuf::from(path))?
        } else {
            KubeConfig::load_default().unwrap_or_else(|_| {
                eprintln!("Warning: Could not load kubeconfig, using defaults");
                // Return a minimal default config
                return KubeConfig {
                    api_version: Some("v1".to_string()),
                    kind: Some("Config".to_string()),
                    current_context: "default".to_string(),
                    contexts: vec![],
                    clusters: vec![],
                    users: vec![],
                    preferences: std::collections::HashMap::new(),
                };
            })
        };

        let server = config.get_server().unwrap_or_else(|_| "https://localhost:6443".to_string());
        let skip_tls = config.should_skip_tls_verify().unwrap_or(cli.insecure_skip_tls_verify);
        let token = cli.token.or_else(|| config.get_token().ok().flatten());
        let namespace = config.get_namespace().unwrap_or_else(|_| "default".to_string());

        (server, skip_tls, token, namespace)
    };

    let client = ApiClient::new(&server, skip_tls, token)?;

    match cli.command {
        Commands::Get {
            resource_type,
            name,
            namespace,
            all_namespaces,
            output,
            no_headers,
            selector,
            field_selector,
            watch,
            show_labels,
        } => {
            let ns = if all_namespaces {
                None
            } else {
                Some(namespace.as_deref().unwrap_or(&default_namespace))
            };
            commands::get::execute_enhanced(
                &client,
                &resource_type,
                name.as_deref(),
                ns,
                all_namespaces,
                output.as_deref(),
                no_headers,
                selector.as_deref(),
                field_selector.as_deref(),
                watch,
                show_labels,
            ).await?;
        }
        Commands::Create { file, namespace, args } => {
            if let Some(file_path) = file {
                commands::create::execute(&client, &file_path).await?;
            } else if !args.is_empty() {
                commands::create::execute_inline(&client, &args, namespace.as_deref().unwrap_or(&default_namespace)).await?;
            } else {
                anyhow::bail!("Either --file or resource arguments must be provided");
            }
        }
        Commands::Delete {
            resource_type,
            name,
            file,
            namespace,
            selector,
            all: _all,
            force,
            grace_period,
        } => {
            if let Some(file_path) = file {
                commands::delete::execute_from_file(&client, &file_path).await?;
            } else if let (Some(rt), Some(n)) = (resource_type.clone(), name) {
                commands::delete::execute_enhanced(
                    &client,
                    &rt,
                    &n,
                    namespace.as_deref().unwrap_or(&default_namespace),
                    force,
                    grace_period,
                ).await?;
            } else if let (Some(rt), Some(sel)) = (resource_type, selector) {
                commands::delete::execute_with_selector(&client, &rt, &sel, namespace.as_deref().unwrap_or(&default_namespace)).await?;
            } else {
                anyhow::bail!("Must provide either resource type/name, file, or selector");
            }
        }
        Commands::Apply { file, namespace, dry_run, server_side, force } => {
            commands::apply::execute_enhanced(
                &client,
                &file,
                namespace.as_deref(),
                dry_run.as_deref(),
                server_side,
                force,
            ).await?;
        }
        Commands::Describe {
            resource_type,
            name,
            namespace,
            selector,
            all_namespaces,
        } => {
            commands::describe::execute_enhanced(
                &client,
                &resource_type,
                name.as_deref(),
                namespace.as_deref().unwrap_or(&default_namespace),
                selector.as_deref(),
                all_namespaces,
            ).await?;
        }
        Commands::Logs {
            pod_name,
            namespace,
            container,
            follow,
            tail,
            timestamps,
            since_time,
            since,
            previous,
        } => {
            commands::logs::execute_enhanced(
                &client,
                &pod_name,
                namespace.as_deref().unwrap_or(&default_namespace),
                container.as_deref(),
                follow,
                tail,
                timestamps,
                since_time.as_deref(),
                since.as_deref(),
                previous,
            ).await?;
        }
        Commands::Exec {
            pod_name,
            container,
            namespace,
            tty,
            stdin,
            command,
        } => {
            commands::exec::execute(
                &client,
                &pod_name,
                namespace.as_deref().unwrap_or(&default_namespace),
                container.as_deref(),
                &command,
                tty,
                stdin,
            ).await?;
        }
        Commands::PortForward {
            pod_name,
            ports,
            namespace,
            address,
        } => {
            commands::port_forward::execute(
                &client,
                &pod_name,
                namespace.as_deref().unwrap_or(&default_namespace),
                &ports,
                &address,
            ).await?;
        }
        Commands::Cp {
            source,
            destination,
            container,
            namespace,
        } => {
            commands::cp::execute(
                &client,
                &source,
                &destination,
                namespace.as_deref().unwrap_or(&default_namespace),
                container.as_deref(),
            ).await?;
        }
        Commands::Edit {
            resource_type,
            name,
            namespace,
            output,
        } => {
            commands::edit::execute(
                &client,
                &resource_type,
                &name,
                namespace.as_deref().unwrap_or(&default_namespace),
                &output,
            ).await?;
        }
        Commands::Patch {
            resource_type,
            name,
            namespace,
            patch,
            patch_file,
            patch_type,
        } => {
            commands::patch::execute(
                &client,
                &resource_type,
                &name,
                namespace.as_deref().unwrap_or(&default_namespace),
                patch.as_deref(),
                patch_file.as_deref(),
                &patch_type,
            ).await?;
        }
        Commands::Scale {
            resource_type,
            name,
            replicas,
            namespace,
        } => {
            commands::scale::execute(
                &client,
                &resource_type,
                &name,
                namespace.as_deref().unwrap_or(&default_namespace),
                replicas,
            ).await?;
        }
        Commands::Rollout { command } => {
            commands::rollout::execute(&client, command, &default_namespace).await?;
        }
        Commands::Top { command } => {
            commands::top::execute(&client, command, &default_namespace).await?;
        }
        Commands::Label {
            resource_type,
            name,
            labels,
            namespace,
            overwrite,
        } => {
            commands::label::execute(
                &client,
                &resource_type,
                &name,
                namespace.as_deref().unwrap_or(&default_namespace),
                &labels,
                overwrite,
            ).await?;
        }
        Commands::Annotate {
            resource_type,
            name,
            annotations,
            namespace,
            overwrite,
        } => {
            commands::annotate::execute(
                &client,
                &resource_type,
                &name,
                namespace.as_deref().unwrap_or(&default_namespace),
                &annotations,
                overwrite,
            ).await?;
        }
        Commands::Explain {
            resource,
            api_version,
            recursive,
        } => {
            commands::explain::execute(&resource, api_version.as_deref(), recursive).await?;
        }
        Commands::Wait {
            resource,
            for_condition,
            for_delete,
            namespace,
            timeout,
            selector,
        } => {
            commands::wait::execute(
                &client,
                &resource,
                for_condition.as_deref(),
                for_delete,
                namespace.as_deref().unwrap_or(&default_namespace),
                &timeout,
                selector.as_deref(),
            ).await?;
        }
        Commands::Diff { file, namespace } => {
            commands::diff::execute(
                &client,
                &file,
                namespace.as_deref().unwrap_or(&default_namespace),
            ).await?;
        }
        Commands::Auth { command } => {
            commands::auth::execute(&client, command, &default_namespace).await?;
        }
        Commands::ApiResources {
            namespaced,
            api_group,
            no_headers,
            output,
        } => {
            commands::api_resources::execute(&client, namespaced, api_group.as_deref(), no_headers, output.as_deref()).await?;
        }
        Commands::ApiVersions {} => {
            commands::api_versions::execute(&client).await?;
        }
        Commands::Config { command } => {
            commands::config::execute(command, cli.kubeconfig.as_deref()).await?;
        }
        Commands::ClusterInfo { dump } => {
            commands::cluster_info::execute(&client, dump).await?;
        }
        Commands::Version { client: client_only, output } => {
            commands::version::execute(&client, client_only, output.as_deref()).await?;
        }
    }

    Ok(())
}
