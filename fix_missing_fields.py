#!/usr/bin/env python3
import re
import sys

def fix_container_struct(content):
    """Add restart_policy: None to Container struct initializations."""
    # Pattern to match Container struct ending with security_context: None,
    pattern = r'(Container\s*\{[^}]*?security_context:\s*None,)\s*(\})'
    replacement = r'\1\n                restart_policy: None,\n            \2'
    content = re.sub(pattern, replacement, content, flags=re.DOTALL)
    return content

def fix_podspec_struct(content):
    """Add missing fields to PodSpec struct initializations."""
    # Find PodSpec structs that end with host_ipc: None,
    pattern = r'(PodSpec\s*\{[^}]*?host_ipc:\s*None,)\s*(\}|,\s*\})'

    def replacement_func(match):
        indent = "            "  # Default indent
        # Detect indentation from the context
        before = match.group(1)
        lines = before.split('\n')
        if len(lines) > 1:
            last_line = lines[-1]
            spaces = len(last_line) - len(last_line.lstrip())
            indent = ' ' * spaces

        missing_fields = f"""
{indent}automount_service_account_token: None,
{indent}ephemeral_containers: None,
{indent}overhead: None,
{indent}scheduler_name: None,
{indent}topology_spread_constraints: None,
{indent}resource_claims: None,"""

        return match.group(1) + missing_fields + '\n        ' + match.group(2)

    content = re.sub(pattern, replacement_func, content, flags=re.DOTALL)
    return content

def main():
    files_to_fix = [
        # Test files
        "crates/api-server/tests/admission_test.rs",
        "crates/api-server/tests/e2e_workflow_test.rs",
        "crates/controller-manager/tests/deployment_controller_test.rs",
        "crates/controller-manager/tests/garbage_collector_test.rs",
        "crates/controller-manager/tests/daemonset_controller_test.rs",
        "crates/controller-manager/tests/resource_quota_test.rs",
        "crates/controller-manager/tests/statefulset_controller_test.rs",
        "crates/kubelet/tests/eviction_test.rs",
        "crates/kubelet/tests/init_containers_test.rs",
        "crates/kubelet/tests/sidecar_containers_test.rs",
        # Source files
        "crates/api-server/src/handlers/metrics.rs",
        "crates/api-server/src/handlers/podtemplate.rs",
        "crates/controller-manager/src/controllers/daemonset.rs",
        "crates/controller-manager/src/controllers/endpoints.rs",
        "crates/controller-manager/src/controllers/events.rs",
        "crates/controller-manager/src/controllers/ttl_controller.rs",
    ]

    for filepath in files_to_fix:
        full_path = f"/Users/chrisalfonso/dev/rusternetes/{filepath}"
        try:
            with open(full_path, 'r') as f:
                content = f.read()

            # Apply fixes
            original_content = content
            content = fix_container_struct(content)
            content = fix_podspec_struct(content)

            if content != original_content:
                with open(full_path, 'w') as f:
                    f.write(content)
                print(f"Fixed: {filepath}")
            else:
                print(f"No changes needed: {filepath}")
        except FileNotFoundError:
            print(f"File not found: {filepath}")
        except Exception as e:
            print(f"Error processing {filepath}: {e}")

if __name__ == "__main__":
    main()
