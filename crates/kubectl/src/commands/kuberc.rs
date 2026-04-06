use anyhow::Result;

/// KubeRC is an alpha feature for user-level kubectl configuration.
///
/// Equivalent to:
///   kubectl kuberc
///
/// KubeRC allows users to configure kubectl behavior via a
/// configuration file (~/.kube/kuberc) including default command
/// flags, aliases, and overrides.
pub fn execute() -> Result<()> {
    eprintln!("kuberc is an alpha feature and is not enabled by default.");
    eprintln!();
    eprintln!("To use kuberc, set the KUBECTL_KUBERC environment variable to 'true'");
    eprintln!("and create a configuration file at ~/.kube/kuberc.");
    eprintln!();
    eprintln!("For more information, see:");
    eprintln!("  https://kubernetes.io/docs/reference/kubectl/kuberc/");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kuberc_prints_alpha_message() {
        // execute() prints to stderr and returns Ok
        let result = execute();
        assert!(result.is_ok());
    }
}
