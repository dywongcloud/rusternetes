use crate::types::{ObjectMeta, TypeMeta};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

/// ConfigMap holds configuration data for pods to consume
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigMap {
    #[serde(flatten)]
    pub type_meta: TypeMeta,

    pub metadata: ObjectMeta,

    /// Data contains the configuration data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<HashMap<String, String>>,

    /// BinaryData contains binary data
    #[serde(skip_serializing_if = "Option::is_none", alias = "binaryData")]
    pub binary_data: Option<HashMap<String, Vec<u8>>>,

    /// Immutable, if set, ensures that data stored in the ConfigMap cannot be updated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub immutable: Option<bool>,
}

impl ConfigMap {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            type_meta: TypeMeta {
                kind: "ConfigMap".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: ObjectMeta::new(name).with_namespace(namespace),
            data: None,
            binary_data: None,
            immutable: Some(false),
        }
    }

    pub fn with_data(mut self, data: HashMap<String, String>) -> Self {
        self.data = Some(data);
        self
    }

    pub fn with_immutable(mut self, immutable: bool) -> Self {
        self.immutable = Some(immutable);
        self
    }
}

/// Custom deserializer for Secret data field that handles base64-encoded strings
fn deserialize_secret_data<'de, D>(
    deserializer: D,
) -> Result<Option<HashMap<String, Vec<u8>>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<HashMap<String, String>> = Option::deserialize(deserializer)?;

    match opt {
        None => Ok(None),
        Some(map) => {
            let mut result = HashMap::new();
            for (k, v) in map {
                // Try to decode as base64, if it fails, just use the bytes as-is
                match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &v) {
                    Ok(decoded) => {
                        result.insert(k, decoded);
                    }
                    Err(_) => {
                        // If base64 decoding fails, use the string bytes
                        result.insert(k, v.into_bytes());
                    }
                }
            }
            Ok(Some(result))
        }
    }
}

/// Secret holds sensitive data such as passwords, tokens, or keys
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Secret {
    #[serde(flatten)]
    pub type_meta: TypeMeta,

    pub metadata: ObjectMeta,

    /// Type of secret (Opaque, kubernetes.io/service-account-token, etc.)
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub secret_type: Option<String>,

    /// Data contains the secret data (base64 encoded)
    #[serde(skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_secret_data")]
    pub data: Option<HashMap<String, Vec<u8>>>,

    /// StringData allows specifying non-binary secret data in string form
    #[serde(skip_serializing_if = "Option::is_none", alias = "stringData")]
    pub string_data: Option<HashMap<String, String>>,

    /// Immutable, if set, ensures that data stored in the Secret cannot be updated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub immutable: Option<bool>,
}

impl Secret {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            type_meta: TypeMeta {
                kind: "Secret".to_string(),
                api_version: "v1".to_string(),
            },
            metadata: ObjectMeta::new(name).with_namespace(namespace),
            secret_type: Some("Opaque".to_string()),
            data: None,
            string_data: None,
            immutable: Some(false),
        }
    }

    pub fn with_type(mut self, secret_type: impl Into<String>) -> Self {
        self.secret_type = Some(secret_type.into());
        self
    }

    pub fn with_data(mut self, data: HashMap<String, Vec<u8>>) -> Self {
        self.data = Some(data);
        self
    }

    pub fn with_string_data(mut self, string_data: HashMap<String, String>) -> Self {
        self.string_data = Some(string_data);
        self
    }

    pub fn with_immutable(mut self, immutable: bool) -> Self {
        self.immutable = Some(immutable);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configmap_creation() {
        let mut data = HashMap::new();
        data.insert("key1".to_string(), "value1".to_string());

        let cm = ConfigMap::new("test-config", "default").with_data(data);

        assert_eq!(cm.metadata.name, "test-config");
        assert_eq!(cm.metadata.namespace, Some("default".to_string()));
        assert!(cm.data.is_some());
    }

    #[test]
    fn test_secret_creation() {
        let mut data = HashMap::new();
        data.insert("password".to_string(), b"secret123".to_vec());

        let secret = Secret::new("test-secret", "default").with_data(data);

        assert_eq!(secret.metadata.name, "test-secret");
        assert_eq!(secret.metadata.namespace, Some("default".to_string()));
        assert!(secret.data.is_some());
        assert_eq!(secret.secret_type, Some("Opaque".to_string()));
    }
}
