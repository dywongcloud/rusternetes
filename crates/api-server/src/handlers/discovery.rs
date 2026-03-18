use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};

/// Version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub major: String,
    pub minor: String,
    #[serde(rename = "gitVersion")]
    pub git_version: String,
    #[serde(rename = "gitCommit")]
    pub git_commit: String,
    #[serde(rename = "gitTreeState")]
    pub git_tree_state: String,
    #[serde(rename = "buildDate")]
    pub build_date: String,
    #[serde(rename = "goVersion")]
    pub go_version: String,
    pub compiler: String,
    pub platform: String,
}

/// APIVersions lists the versions that are available, to allow clients to discover
/// the API at /api
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIVersions {
    pub kind: String,
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub versions: Vec<String>,
    #[serde(rename = "serverAddressByClientCIDRs")]
    pub server_address_by_client_cidrs: Vec<ServerAddressByClientCIDR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerAddressByClientCIDR {
    #[serde(rename = "clientCIDR")]
    pub client_cidr: String,
    #[serde(rename = "serverAddress")]
    pub server_address: String,
}

/// APIGroupList is a list of APIGroup, to allow clients to discover the API at /apis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIGroupList {
    pub kind: String,
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    pub groups: Vec<APIGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIGroup {
    pub name: String,
    pub versions: Vec<GroupVersionForDiscovery>,
    #[serde(rename = "preferredVersion")]
    pub preferred_version: GroupVersionForDiscovery,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupVersionForDiscovery {
    #[serde(rename = "groupVersion")]
    pub group_version: String,
    pub version: String,
}

/// GET /api
/// Returns the list of API versions available at /api/v1
pub async fn get_core_api() -> (StatusCode, Json<APIVersions>) {
    let api_versions = APIVersions {
        kind: "APIVersions".to_string(),
        api_version: "v1".to_string(),
        versions: vec!["v1".to_string()],
        server_address_by_client_cidrs: vec![ServerAddressByClientCIDR {
            client_cidr: "0.0.0.0/0".to_string(),
            server_address: "".to_string(),
        }],
    };

    (StatusCode::OK, Json(api_versions))
}

/// GET /apis
/// Returns the list of API groups available
pub async fn get_api_groups() -> (StatusCode, Json<APIGroupList>) {
    let groups = vec![
        // apps API group
        APIGroup {
            name: "apps".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "apps/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "apps/v1".to_string(),
                version: "v1".to_string(),
            },
        },
        // batch API group
        APIGroup {
            name: "batch".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "batch/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "batch/v1".to_string(),
                version: "v1".to_string(),
            },
        },
        // networking.k8s.io API group
        APIGroup {
            name: "networking.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "networking.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "networking.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            },
        },
        // rbac.authorization.k8s.io API group
        APIGroup {
            name: "rbac.authorization.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "rbac.authorization.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "rbac.authorization.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            },
        },
        // storage.k8s.io API group
        APIGroup {
            name: "storage.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "storage.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "storage.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            },
        },
        // snapshot.storage.k8s.io API group
        APIGroup {
            name: "snapshot.storage.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "snapshot.storage.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "snapshot.storage.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            },
        },
        // scheduling.k8s.io API group
        APIGroup {
            name: "scheduling.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "scheduling.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "scheduling.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            },
        },
        // apiextensions.k8s.io API group
        APIGroup {
            name: "apiextensions.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "apiextensions.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "apiextensions.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            },
        },
        // admissionregistration.k8s.io API group
        APIGroup {
            name: "admissionregistration.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "admissionregistration.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "admissionregistration.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            },
        },
        // coordination.k8s.io API group
        APIGroup {
            name: "coordination.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "coordination.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "coordination.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            },
        },
        // flowcontrol.apiserver.k8s.io API group
        APIGroup {
            name: "flowcontrol.apiserver.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "flowcontrol.apiserver.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "flowcontrol.apiserver.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            },
        },
        // certificates.k8s.io API group
        APIGroup {
            name: "certificates.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "certificates.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "certificates.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            },
        },
        // discovery.k8s.io API group
        APIGroup {
            name: "discovery.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "discovery.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "discovery.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            },
        },
        // node.k8s.io API group
        APIGroup {
            name: "node.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "node.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "node.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            },
        },
        // authentication.k8s.io API group
        APIGroup {
            name: "authentication.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "authentication.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "authentication.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            },
        },
        // authorization.k8s.io API group
        APIGroup {
            name: "authorization.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "authorization.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "authorization.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            },
        },
        // autoscaling API group
        APIGroup {
            name: "autoscaling".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "autoscaling/v2".to_string(),
                version: "v2".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "autoscaling/v2".to_string(),
                version: "v2".to_string(),
            },
        },
        // policy API group
        APIGroup {
            name: "policy".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "policy/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "policy/v1".to_string(),
                version: "v1".to_string(),
            },
        },
        // metrics.k8s.io API group
        APIGroup {
            name: "metrics.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "metrics.k8s.io/v1beta1".to_string(),
                version: "v1beta1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "metrics.k8s.io/v1beta1".to_string(),
                version: "v1beta1".to_string(),
            },
        },
        // custom.metrics.k8s.io API group
        APIGroup {
            name: "custom.metrics.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "custom.metrics.k8s.io/v1beta2".to_string(),
                version: "v1beta2".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "custom.metrics.k8s.io/v1beta2".to_string(),
                version: "v1beta2".to_string(),
            },
        },
        // resource.k8s.io API group
        APIGroup {
            name: "resource.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "resource.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "resource.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            },
        },
    ];

    let api_group_list = APIGroupList {
        kind: "APIGroupList".to_string(),
        api_version: "v1".to_string(),
        groups,
    };

    (StatusCode::OK, Json(api_group_list))
}

/// GET /version
/// Returns the version information for the server
pub async fn get_version() -> (StatusCode, Json<VersionInfo>) {
    let version_info = VersionInfo {
        major: "1".to_string(),
        minor: "35".to_string(),
        git_version: "v1.35.0".to_string(),
        git_commit: "rusternetes-v0.1.0".to_string(),
        git_tree_state: "clean".to_string(),
        build_date: "2026-03-10T00:00:00Z".to_string(),
        go_version: "rust1.83".to_string(),
        compiler: "rustc".to_string(),
        platform: "linux/amd64".to_string(),
    };

    (StatusCode::OK, Json(version_info))
}

/// APIResourceList is the list of APIResources available at a particular group/version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIResourceList {
    pub kind: String,
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    #[serde(rename = "groupVersion")]
    pub group_version: String,
    pub resources: Vec<APIResource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIResource {
    pub name: String,
    #[serde(rename = "singularName")]
    pub singular_name: String,
    pub namespaced: bool,
    pub kind: String,
    pub verbs: Vec<String>,
    #[serde(rename = "shortNames", skip_serializing_if = "Option::is_none")]
    pub short_names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,
}

/// GET /api/v1
/// Returns the list of resources available in the core v1 API
pub async fn get_core_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
        APIResource {
            name: "namespaces".to_string(),
            singular_name: "namespace".to_string(),
            namespaced: false,
            kind: "Namespace".to_string(),
            verbs: vec![
                "create", "delete", "get", "list", "patch", "update", "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["ns".to_string()]),
            categories: None,
        },
        APIResource {
            name: "namespaces/status".to_string(),
            singular_name: "".to_string(),
            namespaced: false,
            kind: "Namespace".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "pods".to_string(),
            singular_name: "pod".to_string(),
            namespaced: true,
            kind: "Pod".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["po".to_string()]),
            categories: Some(vec!["all".to_string()]),
        },
        APIResource {
            name: "pods/status".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "Pod".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "pods/log".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "Pod".to_string(),
            verbs: vec!["get"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "pods/exec".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "PodExecOptions".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "pods/attach".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "PodAttachOptions".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "pods/portforward".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "PodPortForwardOptions".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "pods/binding".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "Binding".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "pods/eviction".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "Eviction".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "services".to_string(),
            singular_name: "service".to_string(),
            namespaced: true,
            kind: "Service".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["svc".to_string()]),
            categories: Some(vec!["all".to_string()]),
        },
        APIResource {
            name: "services/status".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "Service".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "nodes".to_string(),
            singular_name: "node".to_string(),
            namespaced: false,
            kind: "Node".to_string(),
            verbs: vec![
                "create", "delete", "get", "list", "patch", "update", "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["no".to_string()]),
            categories: None,
        },
        APIResource {
            name: "nodes/status".to_string(),
            singular_name: "".to_string(),
            namespaced: false,
            kind: "Node".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "configmaps".to_string(),
            singular_name: "configmap".to_string(),
            namespaced: true,
            kind: "ConfigMap".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["cm".to_string()]),
            categories: None,
        },
        APIResource {
            name: "secrets".to_string(),
            singular_name: "secret".to_string(),
            namespaced: true,
            kind: "Secret".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "serviceaccounts".to_string(),
            singular_name: "serviceaccount".to_string(),
            namespaced: true,
            kind: "ServiceAccount".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["sa".to_string()]),
            categories: None,
        },
        APIResource {
            name: "persistentvolumes".to_string(),
            singular_name: "persistentvolume".to_string(),
            namespaced: false,
            kind: "PersistentVolume".to_string(),
            verbs: vec![
                "create", "delete", "get", "list", "patch", "update", "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["pv".to_string()]),
            categories: None,
        },
        APIResource {
            name: "persistentvolumeclaims".to_string(),
            singular_name: "persistentvolumeclaim".to_string(),
            namespaced: true,
            kind: "PersistentVolumeClaim".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["pvc".to_string()]),
            categories: None,
        },
        APIResource {
            name: "endpoints".to_string(),
            singular_name: "endpoints".to_string(),
            namespaced: true,
            kind: "Endpoints".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["ep".to_string()]),
            categories: None,
        },
        APIResource {
            name: "events".to_string(),
            singular_name: "event".to_string(),
            namespaced: true,
            kind: "Event".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["ev".to_string()]),
            categories: None,
        },
        APIResource {
            name: "resourcequotas".to_string(),
            singular_name: "resourcequota".to_string(),
            namespaced: true,
            kind: "ResourceQuota".to_string(),
            verbs: vec![
                "create", "delete", "get", "list", "patch", "update", "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["quota".to_string()]),
            categories: None,
        },
        APIResource {
            name: "limitranges".to_string(),
            singular_name: "limitrange".to_string(),
            namespaced: true,
            kind: "LimitRange".to_string(),
            verbs: vec![
                "create", "delete", "get", "list", "patch", "update", "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["limits".to_string()]),
            categories: None,
        },
        APIResource {
            name: "replicationcontrollers".to_string(),
            singular_name: "replicationcontroller".to_string(),
            namespaced: true,
            kind: "ReplicationController".to_string(),
            verbs: vec![
                "create", "delete", "get", "list", "patch", "update", "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["rc".to_string()]),
            categories: Some(vec!["all".to_string()]),
        },
        APIResource {
            name: "replicationcontrollers/status".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "ReplicationController".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "replicationcontrollers/scale".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "Scale".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "podtemplates".to_string(),
            singular_name: "podtemplate".to_string(),
            namespaced: true,
            kind: "PodTemplate".to_string(),
            verbs: vec![
                "create", "delete", "get", "list", "patch", "update", "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "componentstatuses".to_string(),
            singular_name: "componentstatus".to_string(),
            namespaced: false,
            kind: "ComponentStatus".to_string(),
            verbs: vec!["get", "list"].iter().map(|s| s.to_string()).collect(),
            short_names: Some(vec!["cs".to_string()]),
            categories: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/apps/v1
/// Returns the list of resources available in the apps/v1 API
pub async fn get_apps_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
        APIResource {
            name: "deployments".to_string(),
            singular_name: "deployment".to_string(),
            namespaced: true,
            kind: "Deployment".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["deploy".to_string()]),
            categories: Some(vec!["all".to_string()]),
        },
        APIResource {
            name: "deployments/status".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "Deployment".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "deployments/scale".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "Scale".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "replicasets".to_string(),
            singular_name: "replicaset".to_string(),
            namespaced: true,
            kind: "ReplicaSet".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["rs".to_string()]),
            categories: Some(vec!["all".to_string()]),
        },
        APIResource {
            name: "replicasets/status".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "ReplicaSet".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "replicasets/scale".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "Scale".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "statefulsets".to_string(),
            singular_name: "statefulset".to_string(),
            namespaced: true,
            kind: "StatefulSet".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["sts".to_string()]),
            categories: Some(vec!["all".to_string()]),
        },
        APIResource {
            name: "statefulsets/status".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "StatefulSet".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "statefulsets/scale".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "Scale".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "daemonsets".to_string(),
            singular_name: "daemonset".to_string(),
            namespaced: true,
            kind: "DaemonSet".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["ds".to_string()]),
            categories: Some(vec!["all".to_string()]),
        },
        APIResource {
            name: "daemonsets/status".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "DaemonSet".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "controllerrevisions".to_string(),
            singular_name: "controllerrevision".to_string(),
            namespaced: true,
            kind: "ControllerRevision".to_string(),
            verbs: vec![
                "create", "delete", "get", "list", "patch", "update", "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "apps/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/batch/v1
/// Returns the list of resources available in the batch/v1 API
pub async fn get_batch_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
        APIResource {
            name: "jobs".to_string(),
            singular_name: "job".to_string(),
            namespaced: true,
            kind: "Job".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: Some(vec!["all".to_string()]),
        },
        APIResource {
            name: "jobs/status".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "Job".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "cronjobs".to_string(),
            singular_name: "cronjob".to_string(),
            namespaced: true,
            kind: "CronJob".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["cj".to_string()]),
            categories: Some(vec!["all".to_string()]),
        },
        APIResource {
            name: "cronjobs/status".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "CronJob".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "batch/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/networking.k8s.io/v1
/// Returns the list of resources available in the networking.k8s.io/v1 API
pub async fn get_networking_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
        APIResource {
            name: "ingresses".to_string(),
            singular_name: "ingress".to_string(),
            namespaced: true,
            kind: "Ingress".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["ing".to_string()]),
            categories: None,
        },
        APIResource {
            name: "ingresses/status".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "Ingress".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "ingressclasses".to_string(),
            singular_name: "ingressclass".to_string(),
            namespaced: false,
            kind: "IngressClass".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "networkpolicies".to_string(),
            singular_name: "networkpolicy".to_string(),
            namespaced: true,
            kind: "NetworkPolicy".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["netpol".to_string()]),
            categories: None,
        },
        APIResource {
            name: "servicecidrs".to_string(),
            singular_name: "servicecidr".to_string(),
            namespaced: false,
            kind: "ServiceCIDR".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "ipaddresses".to_string(),
            singular_name: "ipaddress".to_string(),
            namespaced: false,
            kind: "IPAddress".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "networking.k8s.io/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/rbac.authorization.k8s.io/v1
/// Returns the list of resources available in the rbac.authorization.k8s.io/v1 API
pub async fn get_rbac_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
        APIResource {
            name: "roles".to_string(),
            singular_name: "role".to_string(),
            namespaced: true,
            kind: "Role".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "rolebindings".to_string(),
            singular_name: "rolebinding".to_string(),
            namespaced: true,
            kind: "RoleBinding".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "clusterroles".to_string(),
            singular_name: "clusterrole".to_string(),
            namespaced: false,
            kind: "ClusterRole".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "clusterrolebindings".to_string(),
            singular_name: "clusterrolebinding".to_string(),
            namespaced: false,
            kind: "ClusterRoleBinding".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "rbac.authorization.k8s.io/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/storage.k8s.io/v1
/// Returns the list of resources available in the storage.k8s.io/v1 API
pub async fn get_storage_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
        APIResource {
            name: "storageclasses".to_string(),
            singular_name: "storageclass".to_string(),
            namespaced: false,
            kind: "StorageClass".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["sc".to_string()]),
            categories: None,
        },
        APIResource {
            name: "csidrivers".to_string(),
            singular_name: "csidriver".to_string(),
            namespaced: false,
            kind: "CSIDriver".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "csinodes".to_string(),
            singular_name: "csinode".to_string(),
            namespaced: false,
            kind: "CSINode".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "csistoragecapacities".to_string(),
            singular_name: "csistoragecapacity".to_string(),
            namespaced: true,
            kind: "CSIStorageCapacity".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "volumeattachments".to_string(),
            singular_name: "volumeattachment".to_string(),
            namespaced: false,
            kind: "VolumeAttachment".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "volumeattachments/status".to_string(),
            singular_name: "".to_string(),
            namespaced: false,
            kind: "VolumeAttachment".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "volumeattributesclasses".to_string(),
            singular_name: "volumeattributesclass".to_string(),
            namespaced: false,
            kind: "VolumeAttributesClass".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "storage.k8s.io/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/scheduling.k8s.io/v1
/// Returns the list of resources available in the scheduling.k8s.io/v1 API
pub async fn get_scheduling_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![APIResource {
        name: "priorityclasses".to_string(),
        singular_name: "priorityclass".to_string(),
        namespaced: false,
        kind: "PriorityClass".to_string(),
        verbs: vec![
            "create",
            "delete",
            "deletecollection",
            "get",
            "list",
            "patch",
            "update",
            "watch",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        short_names: Some(vec!["pc".to_string()]),
        categories: None,
    }];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "scheduling.k8s.io/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/apiextensions.k8s.io/v1
/// Returns the list of resources available in the apiextensions.k8s.io/v1 API
pub async fn get_apiextensions_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
        APIResource {
            name: "customresourcedefinitions".to_string(),
            singular_name: "customresourcedefinition".to_string(),
            namespaced: false,
            kind: "CustomResourceDefinition".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["crd".to_string(), "crds".to_string()]),
            categories: None,
        },
        APIResource {
            name: "customresourcedefinitions/status".to_string(),
            singular_name: "".to_string(),
            namespaced: false,
            kind: "CustomResourceDefinition".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "apiextensions.k8s.io/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/admissionregistration.k8s.io/v1
/// Returns the list of resources available in the admissionregistration.k8s.io/v1 API
pub async fn get_admissionregistration_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
        APIResource {
            name: "validatingwebhookconfigurations".to_string(),
            singular_name: "validatingwebhookconfiguration".to_string(),
            namespaced: false,
            kind: "ValidatingWebhookConfiguration".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "mutatingwebhookconfigurations".to_string(),
            singular_name: "mutatingwebhookconfiguration".to_string(),
            namespaced: false,
            kind: "MutatingWebhookConfiguration".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "validatingadmissionpolicies".to_string(),
            singular_name: "validatingadmissionpolicy".to_string(),
            namespaced: false,
            kind: "ValidatingAdmissionPolicy".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "validatingadmissionpolicybindings".to_string(),
            singular_name: "validatingadmissionpolicybinding".to_string(),
            namespaced: false,
            kind: "ValidatingAdmissionPolicyBinding".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "admissionregistration.k8s.io/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/coordination.k8s.io/v1
/// Returns the list of resources available in the coordination.k8s.io/v1 API
pub async fn get_coordination_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![APIResource {
        name: "leases".to_string(),
        singular_name: "lease".to_string(),
        namespaced: true,
        kind: "Lease".to_string(),
        verbs: vec![
            "create",
            "delete",
            "deletecollection",
            "get",
            "list",
            "patch",
            "update",
            "watch",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        short_names: None,
        categories: None,
    }];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "coordination.k8s.io/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/flowcontrol.apiserver.k8s.io/v1
/// Returns the list of resources available in the flowcontrol.apiserver.k8s.io/v1 API
pub async fn get_flowcontrol_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
        APIResource {
            name: "flowschemas".to_string(),
            singular_name: "flowschema".to_string(),
            namespaced: false,
            kind: "FlowSchema".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "flowschemas/status".to_string(),
            singular_name: "".to_string(),
            namespaced: false,
            kind: "FlowSchema".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "prioritylevelconfigurations".to_string(),
            singular_name: "prioritylevelconfiguration".to_string(),
            namespaced: false,
            kind: "PriorityLevelConfiguration".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "prioritylevelconfigurations/status".to_string(),
            singular_name: "".to_string(),
            namespaced: false,
            kind: "PriorityLevelConfiguration".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "flowcontrol.apiserver.k8s.io/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/certificates.k8s.io/v1
/// Returns the list of resources available in the certificates.k8s.io/v1 API
pub async fn get_certificates_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
        APIResource {
            name: "certificatesigningrequests".to_string(),
            singular_name: "certificatesigningrequest".to_string(),
            namespaced: false,
            kind: "CertificateSigningRequest".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["csr".to_string()]),
            categories: None,
        },
        APIResource {
            name: "certificatesigningrequests/status".to_string(),
            singular_name: "".to_string(),
            namespaced: false,
            kind: "CertificateSigningRequest".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "certificatesigningrequests/approval".to_string(),
            singular_name: "".to_string(),
            namespaced: false,
            kind: "CertificateSigningRequest".to_string(),
            verbs: vec!["patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "certificates.k8s.io/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/snapshot.storage.k8s.io/v1
/// Returns the list of resources available in the snapshot.storage.k8s.io/v1 API
pub async fn get_snapshot_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
        APIResource {
            name: "volumesnapshotclasses".to_string(),
            singular_name: "volumesnapshotclass".to_string(),
            namespaced: false,
            kind: "VolumeSnapshotClass".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "volumesnapshots".to_string(),
            singular_name: "volumesnapshot".to_string(),
            namespaced: true,
            kind: "VolumeSnapshot".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "volumesnapshots/status".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "VolumeSnapshot".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "volumesnapshotcontents".to_string(),
            singular_name: "volumesnapshotcontent".to_string(),
            namespaced: false,
            kind: "VolumeSnapshotContent".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "volumesnapshotcontents/status".to_string(),
            singular_name: "".to_string(),
            namespaced: false,
            kind: "VolumeSnapshotContent".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "snapshot.storage.k8s.io/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/discovery.k8s.io/v1
/// Returns the list of resources available in the discovery.k8s.io/v1 API
pub async fn get_discovery_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![APIResource {
        name: "endpointslices".to_string(),
        singular_name: "endpointslice".to_string(),
        namespaced: true,
        kind: "EndpointSlice".to_string(),
        verbs: vec![
            "create", "delete", "get", "list", "patch", "update", "watch",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        short_names: None,
        categories: None,
    }];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "discovery.k8s.io/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/node.k8s.io/v1
/// Returns the list of resources available in the node.k8s.io/v1 API
pub async fn get_node_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![APIResource {
        name: "runtimeclasses".to_string(),
        singular_name: "runtimeclass".to_string(),
        namespaced: false,
        kind: "RuntimeClass".to_string(),
        verbs: vec![
            "create",
            "delete",
            "deletecollection",
            "get",
            "list",
            "patch",
            "update",
            "watch",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        short_names: None,
        categories: None,
    }];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "node.k8s.io/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/authentication.k8s.io/v1
/// Returns the list of resources available in the authentication.k8s.io/v1 API
pub async fn get_authentication_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
        APIResource {
            name: "tokenreviews".to_string(),
            singular_name: "tokenreview".to_string(),
            namespaced: false,
            kind: "TokenReview".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "selfsubjectreviews".to_string(),
            singular_name: "selfsubjectreview".to_string(),
            namespaced: false,
            kind: "SelfSubjectReview".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "authentication.k8s.io/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/authorization.k8s.io/v1
/// Returns the list of resources available in the authorization.k8s.io/v1 API
pub async fn get_authorization_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
        APIResource {
            name: "subjectaccessreviews".to_string(),
            singular_name: "subjectaccessreview".to_string(),
            namespaced: false,
            kind: "SubjectAccessReview".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "selfsubjectaccessreviews".to_string(),
            singular_name: "selfsubjectaccessreview".to_string(),
            namespaced: false,
            kind: "SelfSubjectAccessReview".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "localsubjectaccessreviews".to_string(),
            singular_name: "localsubjectaccessreview".to_string(),
            namespaced: true,
            kind: "LocalSubjectAccessReview".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "selfsubjectrulesreviews".to_string(),
            singular_name: "selfsubjectrulesreview".to_string(),
            namespaced: false,
            kind: "SelfSubjectRulesReview".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "authorization.k8s.io/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/metrics.k8s.io/v1beta1
/// Returns the list of resources available in the metrics.k8s.io/v1beta1 API
pub async fn get_metrics_v1beta1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
        APIResource {
            name: "nodes".to_string(),
            singular_name: "".to_string(),
            namespaced: false,
            kind: "NodeMetrics".to_string(),
            verbs: vec!["get", "list"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "pods".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "PodMetrics".to_string(),
            verbs: vec!["get", "list"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "metrics.k8s.io/v1beta1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/custom.metrics.k8s.io/v1beta2
/// Returns the list of resources available in the custom.metrics.k8s.io/v1beta2 API
pub async fn get_custom_metrics_v1beta2_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![APIResource {
        name: "*".to_string(),
        singular_name: "".to_string(),
        namespaced: true,
        kind: "MetricValueList".to_string(),
        verbs: vec!["get"].iter().map(|s| s.to_string()).collect(),
        short_names: None,
        categories: None,
    }];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1beta2".to_string(),
        group_version: "custom.metrics.k8s.io/v1beta2".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/resource.k8s.io/v1
/// Returns the list of resources available in the resource.k8s.io/v1 API
pub async fn get_resource_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
        APIResource {
            name: "resourceclaims".to_string(),
            singular_name: "resourceclaim".to_string(),
            namespaced: true,
            kind: "ResourceClaim".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "resourceclaims/status".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "ResourceClaim".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "resourceclaimtemplates".to_string(),
            singular_name: "resourceclaimtemplate".to_string(),
            namespaced: true,
            kind: "ResourceClaimTemplate".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "deviceclasses".to_string(),
            singular_name: "deviceclass".to_string(),
            namespaced: false,
            kind: "DeviceClass".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
        APIResource {
            name: "resourceslices".to_string(),
            singular_name: "resourceslice".to_string(),
            namespaced: false,
            kind: "ResourceSlice".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: None,
            categories: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "resource.k8s.io/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/autoscaling/v2
/// Returns the list of resources available in the autoscaling/v2 API
pub async fn get_autoscaling_v2_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
        APIResource {
            name: "horizontalpodautoscalers".to_string(),
            singular_name: "horizontalpodautoscaler".to_string(),
            namespaced: true,
            kind: "HorizontalPodAutoscaler".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["hpa".to_string()]),
            categories: Some(vec!["all".to_string()]),
        },
        APIResource {
            name: "horizontalpodautoscalers/status".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "HorizontalPodAutoscaler".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "autoscaling/v2".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/policy/v1
/// Returns the list of resources available in the policy/v1 API
pub async fn get_policy_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
        APIResource {
            name: "poddisruptionbudgets".to_string(),
            singular_name: "poddisruptionbudget".to_string(),
            namespaced: true,
            kind: "PodDisruptionBudget".to_string(),
            verbs: vec![
                "create",
                "delete",
                "deletecollection",
                "get",
                "list",
                "patch",
                "update",
                "watch",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            short_names: Some(vec!["pdb".to_string()]),
            categories: None,
        },
        APIResource {
            name: "poddisruptionbudgets/status".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "PodDisruptionBudget".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "policy/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}
