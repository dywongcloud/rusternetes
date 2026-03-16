// Filtering helpers for list operations
//
// Provides common logic for filtering resources by field selectors and label selectors

use rusternetes_common::field_selector::FieldSelector;
use rusternetes_common::label_selector::LabelSelector;
use serde::Serialize;
use std::collections::HashMap;

/// Apply field selector filtering to a list of resources
///
/// If field_selector_str is None or empty, no filtering is applied.
/// If parsing fails, returns an error.
pub fn apply_field_selector<T: Serialize>(
    resources: &mut Vec<T>,
    params: &HashMap<String, String>,
) -> Result<(), rusternetes_common::Error> {
    if let Some(field_selector_str) = params.get("fieldSelector") {
        let selector = FieldSelector::parse(field_selector_str).map_err(|e| {
            rusternetes_common::Error::InvalidResource(format!("Invalid field selector: {}", e))
        })?;

        if !selector.is_empty() {
            resources.retain(|resource| {
                let resource_json = serde_json::to_value(resource).unwrap_or_default();
                selector.matches(&resource_json)
            });
        }
    }

    Ok(())
}

/// Apply label selector filtering to a list of resources
///
/// If label_selector_str is None or empty, no filtering is applied.
/// If parsing fails, returns an error.
pub fn apply_label_selector<T: Serialize>(
    resources: &mut Vec<T>,
    params: &HashMap<String, String>,
) -> Result<(), rusternetes_common::Error> {
    if let Some(label_selector_str) = params.get("labelSelector") {
        let selector = LabelSelector::parse(label_selector_str).map_err(|e| {
            rusternetes_common::Error::InvalidResource(format!("Invalid label selector: {}", e))
        })?;

        if !selector.is_empty() {
            resources.retain(|resource| {
                let resource_json = serde_json::to_value(resource).unwrap_or_default();
                selector.matches_resource(&resource_json)
            });
        }
    }

    Ok(())
}

/// Apply both field and label selector filtering to a list of resources
///
/// This is a convenience function that applies both field and label selectors in sequence.
/// Field selectors are applied first, then label selectors.
pub fn apply_selectors<T: Serialize>(
    resources: &mut Vec<T>,
    params: &HashMap<String, String>,
) -> Result<(), rusternetes_common::Error> {
    apply_field_selector(resources, params)?;
    apply_label_selector(resources, params)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Serialize)]
    struct TestResource {
        metadata: Metadata,
        status: Status,
    }

    #[derive(Serialize)]
    struct Metadata {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        labels: Option<HashMap<String, String>>,
    }

    #[derive(Serialize)]
    struct Status {
        phase: String,
    }

    #[test]
    fn test_apply_field_selector_empty() {
        let mut resources = vec![
            TestResource {
                metadata: Metadata {
                    name: "test1".to_string(),
                    labels: None,
                },
                status: Status {
                    phase: "Running".to_string(),
                },
            },
            TestResource {
                metadata: Metadata {
                    name: "test2".to_string(),
                    labels: None,
                },
                status: Status {
                    phase: "Pending".to_string(),
                },
            },
        ];

        let params = HashMap::new();
        apply_field_selector(&mut resources, &params).unwrap();

        assert_eq!(resources.len(), 2);
    }

    #[test]
    fn test_apply_field_selector_filters() {
        let mut resources = vec![
            TestResource {
                metadata: Metadata {
                    name: "test1".to_string(),
                    labels: None,
                },
                status: Status {
                    phase: "Running".to_string(),
                },
            },
            TestResource {
                metadata: Metadata {
                    name: "test2".to_string(),
                    labels: None,
                },
                status: Status {
                    phase: "Pending".to_string(),
                },
            },
        ];

        let mut params = HashMap::new();
        params.insert(
            "fieldSelector".to_string(),
            "status.phase=Running".to_string(),
        );

        apply_field_selector(&mut resources, &params).unwrap();

        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].status.phase, "Running");
    }

    #[test]
    fn test_apply_label_selector_filters() {
        let mut labels1 = HashMap::new();
        labels1.insert("app".to_string(), "nginx".to_string());

        let mut labels2 = HashMap::new();
        labels2.insert("app".to_string(), "redis".to_string());

        let mut resources = vec![
            TestResource {
                metadata: Metadata {
                    name: "test1".to_string(),
                    labels: Some(labels1),
                },
                status: Status {
                    phase: "Running".to_string(),
                },
            },
            TestResource {
                metadata: Metadata {
                    name: "test2".to_string(),
                    labels: Some(labels2),
                },
                status: Status {
                    phase: "Running".to_string(),
                },
            },
        ];

        let mut params = HashMap::new();
        params.insert("labelSelector".to_string(), "app=nginx".to_string());

        apply_label_selector(&mut resources, &params).unwrap();

        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].metadata.name, "test1");
    }

    #[test]
    fn test_apply_both_selectors() {
        let mut labels1 = HashMap::new();
        labels1.insert("app".to_string(), "nginx".to_string());

        let mut labels2 = HashMap::new();
        labels2.insert("app".to_string(), "nginx".to_string());

        let mut resources = vec![
            TestResource {
                metadata: Metadata {
                    name: "test1".to_string(),
                    labels: Some(labels1),
                },
                status: Status {
                    phase: "Running".to_string(),
                },
            },
            TestResource {
                metadata: Metadata {
                    name: "test2".to_string(),
                    labels: Some(labels2),
                },
                status: Status {
                    phase: "Pending".to_string(),
                },
            },
        ];

        let mut params = HashMap::new();
        params.insert(
            "fieldSelector".to_string(),
            "status.phase=Running".to_string(),
        );
        params.insert("labelSelector".to_string(), "app=nginx".to_string());

        apply_selectors(&mut resources, &params).unwrap();

        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].metadata.name, "test1");
        assert_eq!(resources[0].status.phase, "Running");
    }
}
