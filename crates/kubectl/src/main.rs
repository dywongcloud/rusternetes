mod client;
mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};
use client::ApiClient;

#[derive(Parser)]
#[command(name = "kubectl")]
#[command(about = "Rusternetes kubectl - Command line tool for Rusternetes")]
struct Cli {
    /// API server address
    #[arg(long, default_value = "https://localhost:6443", global = true)]
    server: String,

    /// Skip TLS certificate verification (insecure)
    #[arg(long, global = true)]
    insecure_skip_tls_verify: bool,

    /// Bearer token for authentication
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

        /// Output format (json, yaml, wide)
        #[arg(short = 'o', long)]
        output: Option<String>,

        /// Don't print headers (for table output)
        #[arg(long)]
        no_headers: bool,
    },

    /// Create a resource from a file
    Create {
        /// Path to YAML file
        #[arg(short = 'f', long)]
        file: String,
    },

    /// Delete a resource
    Delete {
        /// Resource type (pod, service, deployment, node, namespace)
        resource_type: String,

        /// Resource name
        name: String,

        /// Namespace (for namespaced resources)
        #[arg(short = 'n', long)]
        namespace: Option<String>,
    },

    /// Apply a configuration to a resource
    Apply {
        /// Path to YAML file
        #[arg(short = 'f', long)]
        file: String,
    },

    /// Describe a resource
    Describe {
        /// Resource type (pod, service, deployment, node, namespace)
        resource_type: String,

        /// Resource name
        name: String,

        /// Namespace (for namespaced resources)
        #[arg(short = 'n', long)]
        namespace: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = ApiClient::new(&cli.server, cli.insecure_skip_tls_verify, cli.token)?;

    match cli.command {
        Commands::Get {
            resource_type,
            name,
            namespace,
            output,
            no_headers,
        } => {
            commands::get::execute(&client, &resource_type, name.as_deref(), namespace.as_deref(), output.as_deref(), no_headers)
                .await?;
        }
        Commands::Create { file } => {
            commands::create::execute(&client, &file).await?;
        }
        Commands::Delete {
            resource_type,
            name,
            namespace,
        } => {
            commands::delete::execute(&client, &resource_type, &name, namespace.as_deref()).await?;
        }
        Commands::Apply { file } => {
            commands::apply::execute(&client, &file).await?;
        }
        Commands::Describe {
            resource_type,
            name,
            namespace,
        } => {
            commands::describe::execute(&client, &resource_type, &name, namespace.as_deref()).await?;
        }
    }

    Ok(())
}
