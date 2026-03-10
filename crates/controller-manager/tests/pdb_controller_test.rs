use rusternetes_controller_manager::controllers::pod_disruption_budget::PodDisruptionBudgetController;
use rusternetes_common::resources::{
    IntOrString, PodDisruptionBudget, PodDisruptionBudgetSpec, PodDisruptionBudgetStatus,
    PodDisruptionBudgetCondition,
};
use rusternetes_common::types::LabelSelector;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_pdb_disruption_prevention() {
    let controller = Arc::new(PodDisruptionBudgetController::new());

    // Create PDB with minAvailable=2 (out of 3 pods)
    let spec = PodDisruptionBudgetSpec {
        min_available: Some(IntOrString::Int(2)),
        max_unavailable: None,
        selector: LabelSelector {
            match_labels: Some(HashMap::from([
                ("app".to_string(), "web".to_string()),
            ])),
            match_expressions: None,
        },
        unhealthy_pod_eviction_policy: None,
    };

    let mut pdb = PodDisruptionBudget::new("web-pdb", "default", spec);

    // Simulate status with 3 healthy pods, 1 disruption allowed
    pdb.status = Some(PodDisruptionBudgetStatus {
        current_healthy: 3,
        desired_healthy: 2,
        disruptions_allowed: 1, // Can evict 1 pod (3 - 2 = 1)
        expected_pods: 3,
        observed_generation: Some(1),
        conditions: None,
    });

    controller.create_pdb(pdb).await.unwrap();

    // Test eviction of pod with matching labels - should be allowed (1 disruption available)
    let pod_labels = HashMap::from([
        ("app".to_string(), "web".to_string()),
    ]);

    let allowed = controller.is_eviction_allowed("default", &pod_labels).await;
    assert!(allowed, "First eviction should be allowed (disruptions_allowed=1)");
}

#[tokio::test]
async fn test_pdb_blocks_excessive_evictions() {
    let controller = Arc::new(PodDisruptionBudgetController::new());

    let spec = PodDisruptionBudgetSpec {
        min_available: Some(IntOrString::Int(3)),
        max_unavailable: None,
        selector: LabelSelector {
            match_labels: Some(HashMap::from([
                ("app".to_string(), "critical".to_string()),
            ])),
            match_expressions: None,
        },
        unhealthy_pod_eviction_policy: None,
    };

    let mut pdb = PodDisruptionBudget::new("critical-pdb", "default", spec);

    // All 3 pods must be available - no disruptions allowed
    pdb.status = Some(PodDisruptionBudgetStatus {
        current_healthy: 3,
        desired_healthy: 3,
        disruptions_allowed: 0, // Cannot evict any pods
        expected_pods: 3,
        observed_generation: Some(1),
        conditions: None,
    });

    controller.create_pdb(pdb).await.unwrap();

    let pod_labels = HashMap::from([
        ("app".to_string(), "critical".to_string()),
    ]);

    let allowed = controller.is_eviction_allowed("default", &pod_labels).await;
    assert!(!allowed, "Eviction should be blocked when disruptions_allowed=0");
}

#[tokio::test]
async fn test_pdb_selector_matching() {
    let controller = Arc::new(PodDisruptionBudgetController::new());

    // Create PDB that only matches pods with app=web AND tier=frontend
    let spec = PodDisruptionBudgetSpec {
        min_available: Some(IntOrString::Int(2)),
        max_unavailable: None,
        selector: LabelSelector {
            match_labels: Some(HashMap::from([
                ("app".to_string(), "web".to_string()),
                ("tier".to_string(), "frontend".to_string()),
            ])),
            match_expressions: None,
        },
        unhealthy_pod_eviction_policy: None,
    };

    let mut pdb = PodDisruptionBudget::new("web-frontend-pdb", "default", spec);
    pdb.status = Some(PodDisruptionBudgetStatus {
        current_healthy: 2,
        desired_healthy: 2,
        disruptions_allowed: 0,
        expected_pods: 2,
        observed_generation: Some(1),
        conditions: None,
    });

    controller.create_pdb(pdb).await.unwrap();

    // Pod with matching labels - eviction should be blocked
    let matching_pod = HashMap::from([
        ("app".to_string(), "web".to_string()),
        ("tier".to_string(), "frontend".to_string()),
    ]);

    let allowed_matching = controller.is_eviction_allowed("default", &matching_pod).await;
    assert!(!allowed_matching, "Should block eviction for matching pod");

    // Pod with different labels - eviction should be allowed (not covered by PDB)
    let non_matching_pod = HashMap::from([
        ("app".to_string(), "api".to_string()),
        ("tier".to_string(), "backend".to_string()),
    ]);

    let allowed_non_matching = controller.is_eviction_allowed("default", &non_matching_pod).await;
    assert!(allowed_non_matching, "Should allow eviction for non-matching pod");
}

#[tokio::test]
async fn test_pdb_namespace_isolation() {
    let controller = Arc::new(PodDisruptionBudgetController::new());

    // Create PDB in namespace "production"
    let spec = PodDisruptionBudgetSpec {
        min_available: Some(IntOrString::Int(3)),
        max_unavailable: None,
        selector: LabelSelector {
            match_labels: Some(HashMap::from([
                ("app".to_string(), "web".to_string()),
            ])),
            match_expressions: None,
        },
        unhealthy_pod_eviction_policy: None,
    };

    let mut pdb = PodDisruptionBudget::new("prod-pdb", "production", spec);
    pdb.status = Some(PodDisruptionBudgetStatus {
        current_healthy: 3,
        desired_healthy: 3,
        disruptions_allowed: 0,
        expected_pods: 3,
        observed_generation: Some(1),
        conditions: None,
    });

    controller.create_pdb(pdb).await.unwrap();

    let pod_labels = HashMap::from([
        ("app".to_string(), "web".to_string()),
    ]);

    // PDB in "production" should block eviction in "production"
    let allowed_prod = controller.is_eviction_allowed("production", &pod_labels).await;
    assert!(!allowed_prod, "Should block eviction in production namespace");

    // PDB in "production" should NOT affect "staging" namespace
    let allowed_staging = controller.is_eviction_allowed("staging", &pod_labels).await;
    assert!(allowed_staging, "Should allow eviction in staging namespace (different namespace)");
}

#[tokio::test]
async fn test_pdb_list_by_namespace() {
    let controller = Arc::new(PodDisruptionBudgetController::new());

    // Create PDBs in different namespaces
    for ns in &["default", "production", "staging"] {
        let spec = PodDisruptionBudgetSpec {
            min_available: Some(IntOrString::Int(1)),
            max_unavailable: None,
            selector: LabelSelector {
                match_labels: Some(HashMap::from([
                    ("app".to_string(), "test".to_string()),
                ])),
                match_expressions: None,
            },
            unhealthy_pod_eviction_policy: None,
        };

        let pdb = PodDisruptionBudget::new(format!("{}-pdb", ns), *ns, spec);
        controller.create_pdb(pdb).await.unwrap();
    }

    // List all PDBs
    let all_pdbs = controller.list_pdbs(None).await;
    assert_eq!(all_pdbs.len(), 3, "Should have 3 total PDBs");

    // List PDBs in default namespace
    let default_pdbs = controller.list_pdbs(Some("default")).await;
    assert_eq!(default_pdbs.len(), 1, "Should have 1 PDB in default namespace");
    assert_eq!(default_pdbs[0].metadata.name, "default-pdb");

    // List PDBs in production namespace
    let prod_pdbs = controller.list_pdbs(Some("production")).await;
    assert_eq!(prod_pdbs.len(), 1, "Should have 1 PDB in production namespace");
    assert_eq!(prod_pdbs[0].metadata.name, "production-pdb");
}

#[tokio::test]
async fn test_pdb_with_conditions() {
    let controller = Arc::new(PodDisruptionBudgetController::new());

    let spec = PodDisruptionBudgetSpec {
        min_available: Some(IntOrString::Int(3)),
        max_unavailable: None,
        selector: LabelSelector {
            match_labels: Some(HashMap::from([
                ("app".to_string(), "db".to_string()),
            ])),
            match_expressions: None,
        },
        unhealthy_pod_eviction_policy: None,
    };

    let mut pdb = PodDisruptionBudget::new("db-pdb", "default", spec);
    pdb.status = Some(PodDisruptionBudgetStatus {
        current_healthy: 5,
        desired_healthy: 3,
        disruptions_allowed: 2,
        expected_pods: 5,
        observed_generation: Some(1),
        conditions: Some(vec![
            PodDisruptionBudgetCondition {
                condition_type: "DisruptionAllowed".to_string(),
                status: "True".to_string(),
                last_transition_time: None,
                reason: Some("SufficientPods".to_string()),
                message: Some("2 disruptions allowed out of 5 pods".to_string()),
            }
        ]),
    });

    controller.create_pdb(pdb).await.unwrap();

    // Verify status was updated
    let final_pdb = controller.get_pdb("default", "db-pdb").await.unwrap();
    let status = final_pdb.status.unwrap();

    assert_eq!(status.current_healthy, 5);
    assert_eq!(status.desired_healthy, 3);
    assert_eq!(status.disruptions_allowed, 2);
    assert_eq!(status.expected_pods, 5);
    assert!(status.conditions.is_some());

    let conditions = status.conditions.unwrap();
    assert_eq!(conditions.len(), 1);
    assert_eq!(conditions[0].condition_type, "DisruptionAllowed");
    assert_eq!(conditions[0].status, "True");
    assert_eq!(conditions[0].reason, Some("SufficientPods".to_string()));
}

#[tokio::test]
async fn test_pdb_percentage_based_values() {
    let controller = Arc::new(PodDisruptionBudgetController::new());

    // Test percentage for minAvailable
    let spec1 = PodDisruptionBudgetSpec {
        min_available: Some(IntOrString::String("80%".to_string())),
        max_unavailable: None,
        selector: LabelSelector {
            match_labels: Some(HashMap::from([
                ("app".to_string(), "cache".to_string()),
            ])),
            match_expressions: None,
        },
        unhealthy_pod_eviction_policy: None,
    };

    let pdb1 = PodDisruptionBudget::new("cache-pdb", "default", spec1);
    controller.create_pdb(pdb1.clone()).await.unwrap();

    let retrieved1 = controller.get_pdb("default", "cache-pdb").await.unwrap();
    match &retrieved1.spec.min_available {
        Some(IntOrString::String(s)) => assert_eq!(s, "80%"),
        _ => panic!("Expected IntOrString::String for percentage"),
    }

    // Test percentage for maxUnavailable
    let spec2 = PodDisruptionBudgetSpec {
        min_available: None,
        max_unavailable: Some(IntOrString::String("30%".to_string())),
        selector: LabelSelector {
            match_labels: Some(HashMap::from([
                ("component".to_string(), "worker".to_string()),
            ])),
            match_expressions: None,
        },
        unhealthy_pod_eviction_policy: None,
    };

    let pdb2 = PodDisruptionBudget::new("worker-pdb", "default", spec2);
    controller.create_pdb(pdb2.clone()).await.unwrap();

    let retrieved2 = controller.get_pdb("default", "worker-pdb").await.unwrap();
    match &retrieved2.spec.max_unavailable {
        Some(IntOrString::String(s)) => assert_eq!(s, "30%"),
        _ => panic!("Expected IntOrString::String for percentage"),
    }
}
