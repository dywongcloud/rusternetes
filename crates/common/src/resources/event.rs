use crate::resources::ObjectReference;
use crate::types::ObjectMeta;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event represents a single event in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    #[serde(default = "default_api_version")]
    pub api_version: String,

    #[serde(default = "default_kind")]
    pub kind: String,

    pub metadata: ObjectMeta,

    /// InvolvedObject is the object this event is about
    pub involved_object: ObjectReference,

    /// Reason is a short, machine understandable string that gives the reason for this event
    pub reason: String,

    /// Message is a human-readable description of the status of this operation
    pub message: String,

    /// Source component generating the event
    pub source: EventSource,

    /// Type of this event (Normal, Warning), new types could be added in the future
    #[serde(rename = "type")]
    pub event_type: EventType,

    /// The time at which the event was first recorded
    pub first_timestamp: DateTime<Utc>,

    /// The time at which the most recent occurrence of this event was recorded
    pub last_timestamp: DateTime<Utc>,

    /// The number of times this event has occurred
    pub count: i32,

    /// Optional action that was taken/failed regarding the Regarding object
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,

    /// Optional related object (e.g., Node for Pod event)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related: Option<ObjectReference>,

    /// Event series data (for aggregated events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<EventSeries>,

    /// Time when this Event was first observed (MicroTime precision)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_time: Option<DateTime<Utc>>,

    /// Name of the controller that emitted this Event (e.g., "kubernetes.io/kubelet")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporting_component: Option<String>,

    /// ID of the controller instance (e.g., "kubelet-xyzf")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporting_instance: Option<String>,
}

impl Event {
    /// Create a new Event
    pub fn new(
        name: String,
        namespace: String,
        involved_object: ObjectReference,
        reason: String,
        message: String,
        event_type: EventType,
    ) -> Self {
        let now = Utc::now();
        Self {
            api_version: "v1".to_string(),
            kind: "Event".to_string(),
            metadata: ObjectMeta::new(name).with_namespace(namespace),
            involved_object,
            reason,
            message,
            source: EventSource {
                component: "rusternetes".to_string(),
                host: None,
            },
            event_type,
            first_timestamp: now,
            last_timestamp: now,
            count: 1,
            action: None,
            related: None,
            series: None,
            event_time: None,
            reporting_component: None,
            reporting_instance: None,
        }
    }

    /// Generate event name based on involved object and reason
    pub fn generate_name(involved_object: &ObjectReference, reason: &str) -> String {
        let obj_name = involved_object.name.as_deref().unwrap_or("unknown");
        let uid = involved_object.uid.as_deref().unwrap_or("unknown");
        let timestamp = Utc::now().timestamp();
        format!(
            "{}.{}.{}.{}",
            obj_name,
            reason.to_lowercase(),
            uid[..8.min(uid.len())].to_lowercase(),
            timestamp
        )
    }
}

/// EventSource contains information for an event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EventSource {
    /// Component from which the event is generated
    pub component: String,

    /// Node name on which the event is generated (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
}

/// EventType is the type of an event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    Normal,
    Warning,
}

impl Default for EventType {
    fn default() -> Self {
        EventType::Normal
    }
}

/// EventSeries contains information on series of events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EventSeries {
    /// Number of occurrences in this series up to the last heartbeat time
    pub count: i32,

    /// Time of the last occurrence observed
    pub last_observed_time: DateTime<Utc>,
}

/// EventList is a list of events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventList {
    #[serde(default = "default_api_version")]
    pub api_version: String,

    #[serde(default = "default_kind_list")]
    pub kind: String,

    pub items: Vec<Event>,
}

impl Default for EventList {
    fn default() -> Self {
        Self {
            api_version: "v1".to_string(),
            kind: "EventList".to_string(),
            items: Vec::new(),
        }
    }
}

fn default_api_version() -> String {
    "v1".to_string()
}

fn default_kind() -> String {
    "Event".to_string()
}

fn default_kind_list() -> String {
    "EventList".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let obj_ref = ObjectReference {
            kind: Some("Pod".to_string()),
            namespace: Some("default".to_string()),
            name: Some("test-pod".to_string()),
            uid: Some("abc123".to_string()),
            api_version: Some("v1".to_string()),
            resource_version: None,
            field_path: None,
        };

        let event = Event::new(
            "test-event".to_string(),
            "default".to_string(),
            obj_ref.clone(),
            "PodStarted".to_string(),
            "Pod test-pod started successfully".to_string(),
            EventType::Normal,
        );

        assert_eq!(event.metadata.name, "test-event");
        assert_eq!(event.metadata.namespace, Some("default".to_string()));
        assert_eq!(event.reason, "PodStarted");
        assert_eq!(event.message, "Pod test-pod started successfully");
        assert_eq!(event.event_type, EventType::Normal);
        assert_eq!(event.count, 1);
        assert_eq!(event.involved_object.kind, Some("Pod".to_string()));
    }

    #[test]
    fn test_generate_event_name() {
        let obj_ref = ObjectReference {
            kind: Some("Pod".to_string()),
            namespace: Some("default".to_string()),
            name: Some("test-pod".to_string()),
            uid: Some("abc123def456".to_string()),
            api_version: Some("v1".to_string()),
            resource_version: None,
            field_path: None,
        };

        let name = Event::generate_name(&obj_ref, "PodStarted");
        assert!(name.starts_with("test-pod.podstarted.abc123de"));
    }

    #[test]
    fn test_event_serialization() {
        let obj_ref = ObjectReference {
            kind: Some("Pod".to_string()),
            namespace: Some("default".to_string()),
            name: Some("test-pod".to_string()),
            uid: Some("abc123".to_string()),
            api_version: Some("v1".to_string()),
            resource_version: None,
            field_path: None,
        };

        let event = Event::new(
            "test-event".to_string(),
            "default".to_string(),
            obj_ref,
            "PodStarted".to_string(),
            "Pod test-pod started successfully".to_string(),
            EventType::Normal,
        );

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("PodStarted"));
        assert!(json.contains("Pod test-pod started successfully"));
        assert!(json.contains("Normal"));

        // Test deserialization
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.reason, event.reason);
        assert_eq!(deserialized.message, event.message);
        assert_eq!(deserialized.event_type, event.event_type);
    }

    #[test]
    fn test_event_list() {
        let list = EventList::default();
        assert_eq!(list.api_version, "v1");
        assert_eq!(list.kind, "EventList");
        assert_eq!(list.items.len(), 0);
    }
}
