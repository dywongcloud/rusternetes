use axum::{
    body::Body,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
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

/// Check if the Accept header SPECIFICALLY requests aggregated discovery format.
/// Only return aggregated format if the client explicitly prefers it
/// (i.e., the aggregated type appears BEFORE plain application/json).
/// Some clients (like sonobuoy) send both types in Accept but expect the standard format.
fn wants_aggregated_discovery(headers: &HeaderMap) -> bool {
    if let Some(accept) = headers.get("accept").and_then(|v| v.to_str().ok()) {
        if !accept.contains("apidiscovery.k8s.io") {
            return false;
        }
        // Only return aggregated discovery when the Accept header EXCLUSIVELY
        // requests it (no plain application/json alternative). This prevents
        // breaking clients like sonobuoy that list both types but can't parse aggregated.
        let has_plain_json = accept.split(',').any(|mt| {
            let mt = mt.trim();
            mt == "application/json" || (mt.starts_with("application/json") && !mt.contains("apidiscovery"))
        });
        if has_plain_json {
            return false; // Client also accepts plain JSON — prefer it for compatibility
        }
        return true;
    }
    false
}

/// GET /api
/// Returns the list of API versions available at /api/v1
pub async fn get_core_api(headers: HeaderMap) -> Response {
    if wants_aggregated_discovery(&headers) {
        // Return aggregated discovery format for /api (core API)
        let core_resources = get_aggregated_resources_for_group("", "v1");
        let discovery = serde_json::json!({
            "kind": "APIGroupDiscoveryList",
            "apiVersion": "apidiscovery.k8s.io/v2",
            "metadata": {},
            "items": [
                {
                    "metadata": {
                        "name": ""
                    },
                    "versions": [
                        {
                            "version": "v1",
                            "resources": core_resources,
                            "freshness": "Current"
                        }
                    ]
                }
            ]
        });
        return (
            StatusCode::OK,
            [
                ("content-type", "application/json;g=apidiscovery.k8s.io;v=v2;as=APIGroupDiscoveryList"),
            ],
            Json(discovery),
        ).into_response();
    }

    let api_versions = APIVersions {
        kind: "APIVersions".to_string(),
        api_version: "v1".to_string(),
        versions: vec!["v1".to_string()],
        server_address_by_client_cidrs: vec![ServerAddressByClientCIDR {
            client_cidr: "0.0.0.0/0".to_string(),
            server_address: "".to_string(),
        }],
    };

    let json_bytes = serde_json::to_vec(&api_versions).unwrap_or_default();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json_bytes))
        .unwrap()
}

/// GET /apis
/// Returns the list of API groups available
pub async fn get_api_groups(headers: HeaderMap) -> Response {
    if wants_aggregated_discovery(&headers) {
        // Return aggregated discovery format for /apis with inline resources
        let groups: Vec<serde_json::Value> = get_api_group_names()
            .into_iter()
            .map(|(name, version)| {
                let resources = get_aggregated_resources_for_group(name, version);
                serde_json::json!({
                    "metadata": {
                        "name": name
                    },
                    "versions": [
                        {
                            "version": version,
                            "resources": resources,
                            "freshness": "Current"
                        }
                    ]
                })
            })
            .collect();

        let discovery = serde_json::json!({
            "kind": "APIGroupDiscoveryList",
            "apiVersion": "apidiscovery.k8s.io/v2",
            "metadata": {},
            "items": groups
        });
        return (
            StatusCode::OK,
            [
                ("content-type", "application/json;g=apidiscovery.k8s.io;v=v2;as=APIGroupDiscoveryList"),
            ],
            Json(discovery),
        ).into_response();
    }

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
            versions: vec![
                GroupVersionForDiscovery {
                    group_version: "autoscaling/v1".to_string(),
                    version: "v1".to_string(),
                },
                GroupVersionForDiscovery {
                    group_version: "autoscaling/v2".to_string(),
                    version: "v2".to_string(),
                },
            ],
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
        // events.k8s.io API group
        APIGroup {
            name: "events.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "events.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "events.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            },
        },
        // apiregistration.k8s.io API group
        APIGroup {
            name: "apiregistration.k8s.io".to_string(),
            versions: vec![GroupVersionForDiscovery {
                group_version: "apiregistration.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            }],
            preferred_version: GroupVersionForDiscovery {
                group_version: "apiregistration.k8s.io/v1".to_string(),
                version: "v1".to_string(),
            },
        },
    ];

    let api_group_list = APIGroupList {
        kind: "APIGroupList".to_string(),
        api_version: "v1".to_string(),
        groups,
    };

    (StatusCode::OK, Json(api_group_list)).into_response()
}

/// Helper to get all API group names and their preferred versions
fn get_api_group_names() -> Vec<(&'static str, &'static str)> {
    vec![
        ("apps", "v1"),
        ("batch", "v1"),
        ("networking.k8s.io", "v1"),
        ("rbac.authorization.k8s.io", "v1"),
        ("storage.k8s.io", "v1"),
        ("snapshot.storage.k8s.io", "v1"),
        ("scheduling.k8s.io", "v1"),
        ("apiextensions.k8s.io", "v1"),
        ("admissionregistration.k8s.io", "v1"),
        ("coordination.k8s.io", "v1"),
        ("flowcontrol.apiserver.k8s.io", "v1"),
        ("certificates.k8s.io", "v1"),
        ("discovery.k8s.io", "v1"),
        ("node.k8s.io", "v1"),
        ("authentication.k8s.io", "v1"),
        ("authorization.k8s.io", "v1"),
        ("autoscaling", "v2"),
        ("policy", "v1"),
        ("metrics.k8s.io", "v1beta1"),
        ("custom.metrics.k8s.io", "v1beta2"),
        ("resource.k8s.io", "v1"),
        ("events.k8s.io", "v1"),
        ("apiregistration.k8s.io", "v1"),
    ]
}

/// Build aggregated discovery resource entries for a given API group.
/// Returns a list of resource objects in the apidiscovery.k8s.io/v2 format.
fn get_aggregated_resources_for_group(
    group: &str,
    version: &str,
) -> Vec<serde_json::Value> {
    // Helper to build a single resource entry
    let res = |name: &str,
               singular: &str,
               kind: &str,
               namespaced: bool,
               verbs: &[&str]| {
        let scope = if namespaced { "Namespaced" } else { "Cluster" };
        serde_json::json!({
            "resource": name,
            "responseKind": {
                "group": "",
                "version": version,
                "kind": kind
            },
            "scope": scope,
            "singularResource": singular,
            "verbs": verbs
        })
    };

    let all_verbs: &[&str] = &[
        "create",
        "delete",
        "deletecollection",
        "get",
        "list",
        "patch",
        "update",
        "watch",
    ];
    let status_verbs: &[&str] = &["get", "patch", "update"];

    match group {
        "" => vec![
            res("namespaces", "namespace", "Namespace", false, all_verbs),
            res("namespaces/status", "", "Namespace", false, status_verbs),
            res("pods", "pod", "Pod", true, all_verbs),
            res("pods/status", "", "Pod", true, status_verbs),
            res("pods/log", "", "Pod", true, &["get"]),
            res("pods/exec", "", "PodExecOptions", true, &["get", "create"]),
            res("pods/attach", "", "PodAttachOptions", true, &["get", "create"]),
            res("pods/portforward", "", "PodPortForwardOptions", true, &["get", "create"]),
            res("pods/binding", "", "Binding", true, &["create"]),
            res("pods/eviction", "", "Eviction", true, &["create"]),
            res("pods/ephemeralcontainers", "", "Pod", true, &["get", "patch", "update"]),
            res("services", "service", "Service", true, all_verbs),
            res("services/status", "", "Service", true, status_verbs),
            res("endpoints", "endpoints", "Endpoints", true, all_verbs),
            res("nodes", "node", "Node", false, all_verbs),
            res("nodes/status", "", "Node", false, status_verbs),
            res("configmaps", "configmap", "ConfigMap", true, all_verbs),
            res("secrets", "secret", "Secret", true, all_verbs),
            res("serviceaccounts", "serviceaccount", "ServiceAccount", true, all_verbs),
            res("persistentvolumes", "persistentvolume", "PersistentVolume", false, all_verbs),
            res("persistentvolumes/status", "", "PersistentVolume", false, status_verbs),
            res("persistentvolumeclaims", "persistentvolumeclaim", "PersistentVolumeClaim", true, all_verbs),
            res("persistentvolumeclaims/status", "", "PersistentVolumeClaim", true, status_verbs),
            res("events", "event", "Event", true, all_verbs),
            res("resourcequotas", "resourcequota", "ResourceQuota", true, all_verbs),
            res("resourcequotas/status", "", "ResourceQuota", true, status_verbs),
            res("limitranges", "limitrange", "LimitRange", true, all_verbs),
            res("replicationcontrollers", "replicationcontroller", "ReplicationController", true, all_verbs),
            res("replicationcontrollers/status", "", "ReplicationController", true, status_verbs),
            res("replicationcontrollers/scale", "", "Scale", true, status_verbs),
            res("componentstatuses", "componentstatus", "ComponentStatus", false, &["get", "list"]),
            res("podtemplates", "podtemplate", "PodTemplate", true, all_verbs),
        ],
        "admissionregistration.k8s.io" => vec![
            res("validatingwebhookconfigurations", "validatingwebhookconfiguration", "ValidatingWebhookConfiguration", false, all_verbs),
            res("mutatingwebhookconfigurations", "mutatingwebhookconfiguration", "MutatingWebhookConfiguration", false, all_verbs),
            res("validatingadmissionpolicies", "validatingadmissionpolicy", "ValidatingAdmissionPolicy", false, all_verbs),
            res("validatingadmissionpolicies/status", "", "ValidatingAdmissionPolicy", false, status_verbs),
            res("validatingadmissionpolicybindings", "validatingadmissionpolicybinding", "ValidatingAdmissionPolicyBinding", false, all_verbs),
        ],
        "apps" => vec![
            res("deployments", "deployment", "Deployment", true, all_verbs),
            res("deployments/status", "", "Deployment", true, status_verbs),
            res("deployments/scale", "", "Scale", true, status_verbs),
            res("replicasets", "replicaset", "ReplicaSet", true, all_verbs),
            res("replicasets/status", "", "ReplicaSet", true, status_verbs),
            res("replicasets/scale", "", "Scale", true, status_verbs),
            res("daemonsets", "daemonset", "DaemonSet", true, all_verbs),
            res("daemonsets/status", "", "DaemonSet", true, status_verbs),
            res("statefulsets", "statefulset", "StatefulSet", true, all_verbs),
            res("statefulsets/status", "", "StatefulSet", true, status_verbs),
            res("statefulsets/scale", "", "Scale", true, status_verbs),
            res("controllerrevisions", "controllerrevision", "ControllerRevision", true, all_verbs),
        ],
        "batch" => vec![
            res("jobs", "job", "Job", true, all_verbs),
            res("jobs/status", "", "Job", true, status_verbs),
            res("cronjobs", "cronjob", "CronJob", true, all_verbs),
            res("cronjobs/status", "", "CronJob", true, status_verbs),
        ],
        "networking.k8s.io" => vec![
            res("networkpolicies", "networkpolicy", "NetworkPolicy", true, all_verbs),
            res("ingresses", "ingress", "Ingress", true, all_verbs),
            res("ingresses/status", "", "Ingress", true, status_verbs),
            res("ingressclasses", "ingressclass", "IngressClass", false, all_verbs),
        ],
        "rbac.authorization.k8s.io" => vec![
            res("roles", "role", "Role", true, all_verbs),
            res("rolebindings", "rolebinding", "RoleBinding", true, all_verbs),
            res("clusterroles", "clusterrole", "ClusterRole", false, all_verbs),
            res("clusterrolebindings", "clusterrolebinding", "ClusterRoleBinding", false, all_verbs),
        ],
        "storage.k8s.io" => vec![
            res("storageclasses", "storageclass", "StorageClass", false, all_verbs),
            res("volumeattachments", "volumeattachment", "VolumeAttachment", false, all_verbs),
            res("volumeattachments/status", "", "VolumeAttachment", false, status_verbs),
            res("csinodes", "csinode", "CSINode", false, all_verbs),
            res("csidrivers", "csidriver", "CSIDriver", false, all_verbs),
            res("csistoragecapacities", "csistoragecapacity", "CSIStorageCapacity", true, all_verbs),
        ],
        "scheduling.k8s.io" => vec![
            res("priorityclasses", "priorityclass", "PriorityClass", false, all_verbs),
        ],
        "apiextensions.k8s.io" => vec![
            res("customresourcedefinitions", "customresourcedefinition", "CustomResourceDefinition", false, all_verbs),
            res("customresourcedefinitions/status", "", "CustomResourceDefinition", false, status_verbs),
        ],
        "coordination.k8s.io" => vec![
            res("leases", "lease", "Lease", true, all_verbs),
        ],
        "certificates.k8s.io" => vec![
            res("certificatesigningrequests", "certificatesigningrequest", "CertificateSigningRequest", false, all_verbs),
            res("certificatesigningrequests/status", "", "CertificateSigningRequest", false, status_verbs),
            res("certificatesigningrequests/approval", "", "CertificateSigningRequest", false, &["get", "patch", "update"]),
        ],
        "discovery.k8s.io" => vec![
            res("endpointslices", "endpointslice", "EndpointSlice", true, all_verbs),
        ],
        "node.k8s.io" => vec![
            res("runtimeclasses", "runtimeclass", "RuntimeClass", false, all_verbs),
        ],
        "authentication.k8s.io" => vec![
            res("tokenreviews", "tokenreview", "TokenReview", false, &["create"]),
        ],
        "authorization.k8s.io" => vec![
            res("subjectaccessreviews", "subjectaccessreview", "SubjectAccessReview", false, &["create"]),
            res("localsubjectaccessreviews", "localsubjectaccessreview", "LocalSubjectAccessReview", true, &["create"]),
            res("selfsubjectaccessreviews", "selfsubjectaccessreview", "SelfSubjectAccessReview", false, &["create"]),
            res("selfsubjectrulesreviews", "selfsubjectrulesreview", "SelfSubjectRulesReview", false, &["create"]),
        ],
        "autoscaling" => vec![
            res("horizontalpodautoscalers", "horizontalpodautoscaler", "HorizontalPodAutoscaler", true, all_verbs),
            res("horizontalpodautoscalers/status", "", "HorizontalPodAutoscaler", true, status_verbs),
        ],
        "policy" => vec![
            res("poddisruptionbudgets", "poddisruptionbudget", "PodDisruptionBudget", true, all_verbs),
            res("poddisruptionbudgets/status", "", "PodDisruptionBudget", true, status_verbs),
        ],
        "flowcontrol.apiserver.k8s.io" => vec![
            res("flowschemas", "flowschema", "FlowSchema", false, all_verbs),
            res("flowschemas/status", "", "FlowSchema", false, status_verbs),
            res("prioritylevelconfigurations", "prioritylevelconfiguration", "PriorityLevelConfiguration", false, all_verbs),
            res("prioritylevelconfigurations/status", "", "PriorityLevelConfiguration", false, status_verbs),
        ],
        "events.k8s.io" => vec![
            res("events", "event", "Event", true, all_verbs),
        ],
        "snapshot.storage.k8s.io" => vec![
            res("volumesnapshots", "volumesnapshot", "VolumeSnapshot", true, all_verbs),
            res("volumesnapshotclasses", "volumesnapshotclass", "VolumeSnapshotClass", false, all_verbs),
            res("volumesnapshotcontents", "volumesnapshotcontent", "VolumeSnapshotContent", false, all_verbs),
        ],
        "metrics.k8s.io" => vec![
            res("nodes", "node", "NodeMetrics", false, &["get", "list"]),
            res("pods", "pod", "PodMetrics", true, &["get", "list"]),
        ],
        "custom.metrics.k8s.io" => vec![],
        "resource.k8s.io" => vec![
            res("resourceclaims", "resourceclaim", "ResourceClaim", true, all_verbs),
            res("resourceclaims/status", "", "ResourceClaim", true, status_verbs),
            res("resourceclaimtemplates", "resourceclaimtemplate", "ResourceClaimTemplate", true, all_verbs),
            res("resourceslices", "resourceslice", "ResourceSlice", false, all_verbs),
            res("deviceclasses", "deviceclass", "DeviceClass", false, all_verbs),
        ],
        "apiregistration.k8s.io" => vec![
            res("apiservices", "apiservice", "APIService", false, all_verbs),
            res("apiservices/status", "", "APIService", false, status_verbs),
        ],
        _ => vec![],
    }
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
    #[serde(
        rename = "storageVersionHash",
        skip_serializing_if = "Option::is_none"
    )]
    pub storage_version_hash: Option<String>,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
        },
        APIResource {
            name: "pods/log".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "Pod".to_string(),
            verbs: vec!["get"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
            storage_version_hash: None,
        },
        APIResource {
            name: "pods/exec".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "PodExecOptions".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
            storage_version_hash: None,
        },
        APIResource {
            name: "pods/attach".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "PodAttachOptions".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
            storage_version_hash: None,
        },
        APIResource {
            name: "pods/portforward".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "PodPortForwardOptions".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
            storage_version_hash: None,
        },
        APIResource {
            name: "pods/binding".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "Binding".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
            storage_version_hash: None,
        },
        APIResource {
            name: "pods/eviction".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "Eviction".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
        },
        APIResource {
            name: "componentstatuses".to_string(),
            singular_name: "componentstatus".to_string(),
            namespaced: false,
            kind: "ComponentStatus".to_string(),
            verbs: vec!["get", "list"].iter().map(|s| s.to_string()).collect(),
            short_names: Some(vec!["cs".to_string()]),
            categories: None,
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
        storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
        },
        APIResource {
            name: "validatingadmissionpolicies/status".to_string(),
            singular_name: "".to_string(),
            namespaced: false,
            kind: "ValidatingAdmissionPolicy".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
            storage_version_hash: None,
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
            storage_version_hash: None,
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
        storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
        storage_version_hash: None,
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
        storage_version_hash: None,
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
            storage_version_hash: None,
        },
        APIResource {
            name: "selfsubjectreviews".to_string(),
            singular_name: "selfsubjectreview".to_string(),
            namespaced: false,
            kind: "SelfSubjectReview".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
            storage_version_hash: None,
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
            storage_version_hash: None,
        },
        APIResource {
            name: "selfsubjectaccessreviews".to_string(),
            singular_name: "selfsubjectaccessreview".to_string(),
            namespaced: false,
            kind: "SelfSubjectAccessReview".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
            storage_version_hash: None,
        },
        APIResource {
            name: "localsubjectaccessreviews".to_string(),
            singular_name: "localsubjectaccessreview".to_string(),
            namespaced: true,
            kind: "LocalSubjectAccessReview".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
            storage_version_hash: None,
        },
        APIResource {
            name: "selfsubjectrulesreviews".to_string(),
            singular_name: "selfsubjectrulesreview".to_string(),
            namespaced: false,
            kind: "SelfSubjectRulesReview".to_string(),
            verbs: vec!["create"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
            storage_version_hash: None,
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
            storage_version_hash: None,
        },
        APIResource {
            name: "pods".to_string(),
            singular_name: "".to_string(),
            namespaced: true,
            kind: "PodMetrics".to_string(),
            verbs: vec!["get", "list"].iter().map(|s| s.to_string()).collect(),
            short_names: None,
            categories: None,
            storage_version_hash: None,
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
        storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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

/// GET /apis/autoscaling/v1
/// Returns the list of resources available in the autoscaling/v1 API
pub async fn get_autoscaling_v1_resources() -> (StatusCode, Json<APIResourceList>) {
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
            storage_version_hash: None,
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
            storage_version_hash: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "autoscaling/v1".to_string(),
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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
            storage_version_hash: None,
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

/// GET /apis/events.k8s.io/v1
/// Returns the list of resources available in the events.k8s.io/v1 API group
pub async fn get_events_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
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
            storage_version_hash: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "events.k8s.io/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}

/// GET /apis/apiregistration.k8s.io/v1
/// Returns the list of resources available in the apiregistration.k8s.io/v1 API group
pub async fn get_apiregistration_v1_resources() -> (StatusCode, Json<APIResourceList>) {
    let resources = vec![
        APIResource {
            name: "apiservices".to_string(),
            singular_name: "apiservice".to_string(),
            namespaced: false,
            kind: "APIService".to_string(),
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
            storage_version_hash: None,
        },
        APIResource {
            name: "apiservices/status".to_string(),
            singular_name: "".to_string(),
            namespaced: false,
            kind: "APIService".to_string(),
            verbs: vec!["get", "patch", "update"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            short_names: None,
            categories: None,
            storage_version_hash: None,
        },
    ];

    let resource_list = APIResourceList {
        kind: "APIResourceList".to_string(),
        api_version: "v1".to_string(),
        group_version: "apiregistration.k8s.io/v1".to_string(),
        resources,
    };

    (StatusCode::OK, Json(resource_list))
}
