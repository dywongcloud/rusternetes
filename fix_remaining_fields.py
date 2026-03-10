#!/usr/bin/env python3
import re

def fix_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    original = content

    # Fix PodSpec that end with priority_class_name: None, followed by }
    pattern1 = r'(priority_class_name:\s*None,)\s*(\})'
    replacement1 = r'\1\n                automount_service_account_token: None,\n                ephemeral_containers: None,\n                overhead: None,\n                scheduler_name: None,\n                topology_spread_constraints: None,\n                resource_claims: None,\n            \2'
    content = re.sub(pattern1, replacement1, content)

    # Fix PodSpec that end with host_ipc: None, followed by }
    pattern2 = r'(host_ipc:\s*None,)\s*(\},)'
    replacement2 = r'\1\n                automount_service_account_token: None,\n                ephemeral_containers: None,\n                overhead: None,\n                scheduler_name: None,\n                topology_spread_constraints: None,\n                resource_claims: None,\n            \2'
    content = re.sub(pattern2, replacement2, content)

    # Fix PodSpec that end with host_ipc: None, followed by }, but not already having automount
    pattern3 = r'(host_ipc:\s*None,)\s*(\}[^,])'
    if 'automount_service_account_token' not in content:
        replacement3 = r'\1\n                automount_service_account_token: None,\n                ephemeral_containers: None,\n                overhead: None,\n                scheduler_name: None,\n                topology_spread_constraints: None,\n                resource_claims: None,\n            \2'
        content = re.sub(pattern3, replacement3, content)

    if content != original:
        with open(filepath, 'w') as f:
            f.write(content)
        print(f"Fixed: {filepath}")
        return True
    return False

# Files to fix
files = [
    "/Users/chrisalfonso/dev/rusternetes/crates/api-server/src/handlers/podtemplate.rs",
    "/Users/chrisalfonso/dev/rusternetes/crates/api-server/src/handlers/metrics.rs",
    "/Users/chrisalfonso/dev/rusternetes/crates/controller-manager/src/controllers/daemonset.rs",
    "/Users/chrisalfonso/dev/rusternetes/crates/controller-manager/src/controllers/endpoints.rs",
    "/Users/chrisalfonso/dev/rusternetes/crates/controller-manager/src/controllers/ttl_controller.rs",
    "/Users/chrisalfonso/dev/rusternetes/crates/controller-manager/tests/daemonset_controller_test.rs",
    "/Users/chrisalfonso/dev/rusternetes/crates/kubelet/tests/eviction_test.rs",
    "/Users/chrisalfonso/dev/rusternetes/crates/kubelet/tests/init_containers_test.rs",
]

for filepath in files:
    try:
        fix_file(filepath)
    except FileNotFoundError:
        print(f"Not found: {filepath}")
    except Exception as e:
        print(f"Error in {filepath}: {e}")
