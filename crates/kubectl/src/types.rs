use clap::Subcommand;

#[derive(Subcommand)]
pub enum CertificateCommands {
    /// Approve a certificate signing request
    Approve {
        /// CSR names to approve
        csr_names: Vec<String>,
        /// Update the CSR even if it is already approved
        #[arg(long)]
        force: bool,
    },
    /// Deny a certificate signing request
    Deny {
        /// CSR names to deny
        csr_names: Vec<String>,
        /// Update the CSR even if it is already denied
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
pub enum RolloutCommands {
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
pub enum TopCommands {
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
pub enum AuthCommands {
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
pub enum SetCommands {
    /// Update container images in a resource
    Image {
        /// Resource (TYPE/NAME, e.g., deployment/nginx)
        resource: String,
        /// Container=image pairs (e.g., nginx=nginx:1.9.1)
        container_images: Vec<String>,
        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,
    },
    /// Update environment variables on a resource
    Env {
        /// Resource (TYPE/NAME, e.g., deployment/registry)
        resource: String,
        /// Environment variables (KEY=VALUE or KEY-)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        env_vars: Vec<String>,
        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,
        /// Container name
        #[arg(short = 'c', long)]
        container: Option<String>,
        /// List environment variables
        #[arg(long)]
        list: bool,
    },
    /// Update resource requests/limits on a resource
    Resources {
        /// Resource (TYPE/NAME, e.g., deployment/nginx)
        resource: String,
        /// Namespace
        #[arg(short = 'n', long)]
        namespace: Option<String>,
        /// Container name
        #[arg(short = 'c', long)]
        container: Option<String>,
        /// Resource limits (cpu=200m,memory=512Mi)
        #[arg(long)]
        limits: Option<String>,
        /// Resource requests (cpu=100m,memory=256Mi)
        #[arg(long)]
        requests: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum PluginCommands {
    /// List all available plugin files on a user's PATH
    List {},
}

#[derive(Subcommand)]
pub enum ConfigCommands {
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
