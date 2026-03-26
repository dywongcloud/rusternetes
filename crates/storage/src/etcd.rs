use crate::{Storage, WatchEvent, WatchStream};
use async_trait::async_trait;
use etcd_client::{Client, Compare, CompareOp, GetOptions, TxnOp, WatchOptions};
use futures::StreamExt;
use rusternetes_common::{authz::AuthzStorage, Error, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};

/// EtcdStorage implements the Storage trait using etcd as the backend
pub struct EtcdStorage {
    client: Arc<tokio::sync::Mutex<Client>>,
}

impl EtcdStorage {
    /// Create a new EtcdStorage instance
    pub async fn new(endpoints: Vec<String>) -> Result<Self> {
        let client = Client::connect(endpoints, None)
            .await
            .map_err(|e| Error::Storage(format!("Failed to connect to etcd: {}", e)))?;

        info!("Connected to etcd successfully");

        Ok(Self {
            client: Arc::new(tokio::sync::Mutex::new(client)),
        })
    }

    /// Helper to serialize a value to JSON
    fn serialize<T: Serialize>(value: &T) -> Result<String> {
        serde_json::to_string(value).map_err(|e| Error::Serialization(e))
    }

    /// Helper to deserialize a value from JSON
    fn deserialize<T: DeserializeOwned>(data: &str) -> Result<T> {
        serde_json::from_str(data).map_err(|e| Error::Serialization(e))
    }
}

#[async_trait]
impl Storage for EtcdStorage {
    async fn create<T>(&self, key: &str, value: &T) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
    {
        let mut client = self.client.lock().await;
        // Ensure metadata.generation is set to 1 on creation
        let json = {
            let mut raw = Self::serialize(value)?;
            if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(metadata) = v.get_mut("metadata") {
                    if metadata.get("generation").map_or(true, |g| g.is_null()) {
                        metadata["generation"] = serde_json::json!(1);
                        raw = serde_json::to_string(&v).unwrap_or(raw);
                    }
                }
            }
            raw
        };

        // Use a transaction to ensure the key doesn't already exist
        let txn = etcd_client::Txn::new()
            .when(vec![Compare::version(key, CompareOp::Equal, 0)])
            .and_then(vec![TxnOp::put(key, json.clone(), None)])
            .or_else(vec![]);

        let txn_resp = client
            .txn(txn)
            .await
            .map_err(|e| Error::Storage(format!("Failed to create resource: {}", e)))?;

        if !txn_resp.succeeded() {
            return Err(Error::AlreadyExists(key.to_string()));
        }

        debug!("Created resource at key: {}", key);

        // Get the resource back to populate resourceVersion from etcd mod_revision
        let get_resp = client
            .get(key, None)
            .await
            .map_err(|e| Error::Storage(format!("Failed to get created resource: {}", e)))?;

        if let Some(kv) = get_resp.kvs().first() {
            let mod_revision = kv.mod_revision();
            let mut resource: serde_json::Value =
                serde_json::from_str(&json).map_err(|e| Error::Serialization(e))?;

            // Set resourceVersion in metadata if it exists
            if let Some(metadata) = resource.get_mut("metadata") {
                metadata["resourceVersion"] = serde_json::json!(
                    crate::concurrency::mod_revision_to_resource_version(mod_revision)
                );
            }

            serde_json::from_value(resource).map_err(|e| Error::Serialization(e))
        } else {
            Self::deserialize(&json)
        }
    }

    async fn get<T>(&self, key: &str) -> Result<T>
    where
        T: DeserializeOwned + Send + Sync,
    {
        let mut client = self.client.lock().await;

        let resp = client
            .get(key, None)
            .await
            .map_err(|e| Error::Storage(format!("Failed to get resource: {}", e)))?;

        if let Some(kv) = resp.kvs().first() {
            let json = kv
                .value_str()
                .map_err(|e| Error::Storage(format!("Invalid UTF-8 in value: {}", e)))?;

            // Add resourceVersion from etcd mod_revision
            let mod_revision = kv.mod_revision();
            let mut resource: serde_json::Value =
                serde_json::from_str(json).map_err(|e| Error::Serialization(e))?;

            if let Some(metadata) = resource.get_mut("metadata") {
                metadata["resourceVersion"] = serde_json::json!(
                    crate::concurrency::mod_revision_to_resource_version(mod_revision)
                );
            }

            serde_json::from_value(resource).map_err(|e| Error::Serialization(e))
        } else {
            Err(Error::NotFound(key.to_string()))
        }
    }

    async fn update<T>(&self, key: &str, value: &T) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
    {
        let mut client = self.client.lock().await;
        let json = Self::serialize(value)?;

        // Check if the key exists and get current resourceVersion
        let get_resp = client
            .get(key, None)
            .await
            .map_err(|e| Error::Storage(format!("Failed to check resource: {}", e)))?;

        if get_resp.kvs().is_empty() {
            return Err(Error::NotFound(key.to_string()));
        }

        let current_kv = get_resp.kvs().first().unwrap();
        let current_mod_revision = current_kv.mod_revision();

        // Extract resourceVersion from the incoming resource
        let incoming_resource: serde_json::Value =
            serde_json::from_str(&json).map_err(|e| Error::Serialization(e))?;
        let incoming_rv = crate::concurrency::extract_resource_version(
            incoming_resource
                .get("metadata")
                .unwrap_or(&serde_json::json!({})),
        );

        // Validate optimistic concurrency if resourceVersion is provided
        if let Some(incoming_rv) = incoming_rv.as_deref() {
            let current_rv =
                crate::concurrency::mod_revision_to_resource_version(current_mod_revision);
            crate::concurrency::validate_resource_version(Some(incoming_rv), Some(&current_rv))?;

            // Use a transaction to ensure atomic update with version check
            let expected_mod_revision =
                crate::concurrency::resource_version_to_mod_revision(incoming_rv)?;
            let txn = etcd_client::Txn::new()
                .when(vec![Compare::mod_revision(
                    key,
                    CompareOp::Equal,
                    expected_mod_revision,
                )])
                .and_then(vec![TxnOp::put(key, json.clone(), None)])
                .or_else(vec![]);

            let txn_resp = client
                .txn(txn)
                .await
                .map_err(|e| Error::Storage(format!("Failed to update resource: {}", e)))?;

            if !txn_resp.succeeded() {
                return Err(Error::Conflict(format!(
                    "resourceVersion mismatch: resource was modified (expected: {}, current: {})",
                    incoming_rv, current_rv
                )));
            }
        } else {
            // No resourceVersion provided, allow update without optimistic lock
            client
                .put(key, json.clone(), None)
                .await
                .map_err(|e| Error::Storage(format!("Failed to update resource: {}", e)))?;
        }

        debug!("Updated resource at key: {}", key);

        // Get the updated resource to return the new resourceVersion
        let get_resp = client
            .get(key, None)
            .await
            .map_err(|e| Error::Storage(format!("Failed to get updated resource: {}", e)))?;

        if let Some(kv) = get_resp.kvs().first() {
            let mod_revision = kv.mod_revision();
            let mut resource: serde_json::Value =
                serde_json::from_str(&json).map_err(|e| Error::Serialization(e))?;

            if let Some(metadata) = resource.get_mut("metadata") {
                metadata["resourceVersion"] = serde_json::json!(
                    crate::concurrency::mod_revision_to_resource_version(mod_revision)
                );
            }

            serde_json::from_value(resource).map_err(|e| Error::Serialization(e))
        } else {
            Self::deserialize(&json)
        }
    }

    async fn update_raw(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        let mut client = self.client.lock().await;
        let json = serde_json::to_string(value).map_err(|e| Error::Serialization(e))?;

        // Check if the key exists first
        let get_resp = client
            .get(key, None)
            .await
            .map_err(|e| Error::Storage(format!("Failed to check resource: {}", e)))?;

        if get_resp.kvs().is_empty() {
            return Err(Error::NotFound(key.to_string()));
        }

        client
            .put(key, json, None)
            .await
            .map_err(|e| Error::Storage(format!("Failed to update resource: {}", e)))?;

        debug!("Updated resource (raw) at key: {}", key);
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let mut client = self.client.lock().await;

        let resp = client
            .delete(key, None)
            .await
            .map_err(|e| Error::Storage(format!("Failed to delete resource: {}", e)))?;

        if resp.deleted() == 0 {
            return Err(Error::NotFound(key.to_string()));
        }

        debug!("Deleted resource at key: {}", key);
        Ok(())
    }

    async fn list<T>(&self, prefix: &str) -> Result<Vec<T>>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
    {
        let mut client = self.client.lock().await;

        // Paginate etcd list calls to avoid hitting the default 4MB gRPC
        // message size limit. Fetch up to 500 keys per request.
        const PAGE_SIZE: i64 = 500;
        let mut results = Vec::new();
        let mut last_key: Option<Vec<u8>> = None;

        loop {
            let get_options = match &last_key {
                None => {
                    // First page: use prefix scan
                    GetOptions::new().with_prefix().with_limit(PAGE_SIZE)
                }
                Some(key) => {
                    // Subsequent pages: start from last_key (exclusive) with prefix
                    GetOptions::new()
                        .with_prefix()
                        .with_from_key()
                        .with_limit(PAGE_SIZE + 1) // +1 because from_key is inclusive
                }
            };

            let query_key: Vec<u8> = match &last_key {
                None => prefix.as_bytes().to_vec(),
                Some(key) => key.clone(),
            };

            let resp = client
                .get(query_key, Some(get_options))
                .await
                .map_err(|e| Error::Storage(format!("Failed to list resources: {}", e)))?;

            let kvs = resp.kvs();
            for kv in kvs {
                // Skip the last_key itself (from_key is inclusive)
                if let Some(ref lk) = last_key {
                    if kv.key() == lk.as_slice() {
                        continue;
                    }
                }

                // Ensure key still has the prefix (from_key may go beyond prefix)
                let key_str = kv
                    .key_str()
                    .map_err(|e| Error::Storage(format!("Invalid UTF-8 in key: {}", e)))?;
                if !key_str.starts_with(prefix) {
                    // We've gone past the prefix range, stop
                    debug!("Listed {} resources with prefix: {}", results.len(), prefix);
                    return Ok(results);
                }

                let json = kv
                    .value_str()
                    .map_err(|e| Error::Storage(format!("Invalid UTF-8 in value: {}", e)))?;
                let mod_revision = kv.mod_revision();

                // Add resourceVersion from etcd mod_revision
                let mut resource: serde_json::Value = match serde_json::from_str(json) {
                    Ok(value) => value,
                    Err(e) => {
                        error!("Failed to deserialize value: {}", e);
                        continue;
                    }
                };

                if let Some(metadata) = resource.get_mut("metadata") {
                    metadata["resourceVersion"] = serde_json::json!(
                        crate::concurrency::mod_revision_to_resource_version(mod_revision)
                    );
                }

                match serde_json::from_value::<T>(resource) {
                    Ok(value) => {
                        results.push(value);
                    }
                    Err(e) => {
                        error!("Failed to deserialize enhanced value: {}", e);
                        continue;
                    }
                }

            }

            // If we got fewer results than PAGE_SIZE, we've reached the end
            let total_kvs = kvs.len() as i64;
            let expected = if last_key.is_some() {
                PAGE_SIZE + 1
            } else {
                PAGE_SIZE
            };
            if total_kvs < expected {
                break;
            }

            // Set last_key to the last key we received for the next page
            if let Some(last_kv) = kvs.last() {
                last_key = Some(last_kv.key().to_vec());
            } else {
                break;
            }
        }

        debug!("Listed {} resources with prefix: {}", results.len(), prefix);
        Ok(results)
    }

    async fn watch_from_revision(&self, prefix: &str, revision: i64) -> Result<WatchStream> {
        let mut client = self.client.lock().await;
        let watch_options = WatchOptions::new()
            .with_prefix()
            .with_prev_key()
            .with_start_revision(revision);
        let (_watcher, stream) = client
            .watch(prefix, Some(watch_options))
            .await
            .map_err(|e| Error::Storage(format!("Failed to create watch from revision {}: {}", revision, e)))?;
        info!("Started watching prefix: {} from revision {}", prefix, revision);
        let watch_stream = stream.map(move |watch_resp| {
            match watch_resp {
                Ok(resp) => {
                    for event in resp.events() {
                        let key = event
                            .kv()
                            .map(|kv| kv.key_str().unwrap_or("").to_string())
                            .unwrap_or_default();
                        return match event.event_type() {
                            etcd_client::EventType::Put => {
                                let raw_value = event
                                    .kv()
                                    .map(|kv| String::from_utf8_lossy(kv.value()).to_string())
                                    .unwrap_or_default();
                                // Inject resourceVersion from etcd mod_revision
                                let mod_revision = event.kv().map(|kv| kv.mod_revision()).unwrap_or(0);
                                let value = if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&raw_value) {
                                    if let Some(metadata) = v.get_mut("metadata") {
                                        metadata["resourceVersion"] = serde_json::json!(
                                            crate::concurrency::mod_revision_to_resource_version(mod_revision)
                                        );
                                    }
                                    serde_json::to_string(&v).unwrap_or(raw_value)
                                } else {
                                    raw_value
                                };
                                if event.prev_kv().is_some() {
                                    Ok(WatchEvent::Modified(key, value))
                                } else {
                                    Ok(WatchEvent::Added(key, value))
                                }
                            }
                            etcd_client::EventType::Delete => {
                                let raw_prev = event
                                    .prev_kv()
                                    .map(|kv| String::from_utf8_lossy(kv.value()).to_string())
                                    .unwrap_or_default();
                                let mod_revision = event.kv().map(|kv| kv.mod_revision()).unwrap_or(0);
                                let prev_value = if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&raw_prev) {
                                    if let Some(metadata) = v.get_mut("metadata") {
                                        metadata["resourceVersion"] = serde_json::json!(
                                            crate::concurrency::mod_revision_to_resource_version(mod_revision)
                                        );
                                    }
                                    serde_json::to_string(&v).unwrap_or(raw_prev)
                                } else {
                                    raw_prev
                                };
                                Ok(WatchEvent::Deleted(key, prev_value))
                            }
                        };
                    }
                    // No events in this response — return a dummy Added event
                    // that the watch handler will ignore
                    Err(Error::Storage("empty watch response".to_string()))
                }
                Err(e) => Err(Error::Storage(format!("Watch error: {}", e))),
            }
        });
        Ok(Box::pin(watch_stream))
    }

    async fn watch(&self, prefix: &str) -> Result<WatchStream> {
        let mut client = self.client.lock().await;

        // Enable prev_kv to get the previous value on DELETE events (required for Kubernetes)
        let watch_options = WatchOptions::new().with_prefix().with_prev_key();
        let (_watcher, stream) = client
            .watch(prefix, Some(watch_options))
            .await
            .map_err(|e| Error::Storage(format!("Failed to create watch: {}", e)))?;

        info!("Started watching prefix: {}", prefix);

        // Convert etcd watch stream to our WatchStream
        let watch_stream = stream.map(move |watch_resp| {
            match watch_resp {
                Ok(resp) => {
                    for event in resp.events() {
                        let key = event
                            .kv()
                            .map(|kv| kv.key_str().unwrap_or("").to_string())
                            .unwrap_or_default();

                        return match event.event_type() {
                            etcd_client::EventType::Put => {
                                let raw_value = event
                                    .kv()
                                    .and_then(|kv| kv.value_str().ok())
                                    .unwrap_or("")
                                    .to_string();

                                // Inject resourceVersion from etcd mod_revision into the JSON value
                                let mod_revision = event.kv().map(|kv| kv.mod_revision()).unwrap_or(0);
                                let value = if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&raw_value) {
                                    if let Some(metadata) = v.get_mut("metadata") {
                                        metadata["resourceVersion"] = serde_json::json!(
                                            crate::concurrency::mod_revision_to_resource_version(mod_revision)
                                        );
                                    }
                                    serde_json::to_string(&v).unwrap_or(raw_value)
                                } else {
                                    raw_value
                                };

                                // Check if this is a new key or an update
                                if event.kv().map(|kv| kv.version()).unwrap_or(0) == 1 {
                                    Ok(WatchEvent::Added(key, value))
                                } else {
                                    Ok(WatchEvent::Modified(key, value))
                                }
                            }
                            etcd_client::EventType::Delete => {
                                // Get the previous value from prev_kv and inject resourceVersion
                                let raw_prev = event
                                    .prev_kv()
                                    .and_then(|kv| kv.value_str().ok())
                                    .unwrap_or("")
                                    .to_string();
                                let mod_revision = event.kv().map(|kv| kv.mod_revision()).unwrap_or(0);
                                let prev_value = if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&raw_prev) {
                                    if let Some(metadata) = v.get_mut("metadata") {
                                        metadata["resourceVersion"] = serde_json::json!(
                                            crate::concurrency::mod_revision_to_resource_version(mod_revision)
                                        );
                                    }
                                    serde_json::to_string(&v).unwrap_or(raw_prev)
                                } else {
                                    raw_prev
                                };
                                Ok(WatchEvent::Deleted(key, prev_value))
                            }
                        };
                    }
                    Err(Error::Storage("Empty watch response".to_string()))
                }
                Err(e) => Err(Error::Storage(format!("Watch error: {}", e))),
            }
        });

        Ok(Box::pin(watch_stream))
    }

    async fn current_revision(&self) -> Result<i64> {
        let mut client = self.client.lock().await;
        // Get status to find current revision
        let resp = client
            .get("/", None)
            .await
            .map_err(|e| Error::Storage(format!("Failed to get current revision: {}", e)))?;
        Ok(resp.header().unwrap().revision())
    }

    async fn is_revision_compacted(&self, revision: i64) -> Result<bool> {
        let mut client = self.client.lock().await;
        // Try to get a key at the given revision; if compacted, etcd returns an error
        let opts = GetOptions::new().with_revision(revision);
        match client.get("/registry/", Some(opts)).await {
            Ok(_) => Ok(false), // revision still available
            Err(e) => {
                let err_msg = format!("{}", e);
                if err_msg.contains("compacted") || err_msg.contains("required revision has been compacted") {
                    Ok(true) // revision has been compacted
                } else {
                    // Other error — not a compaction issue
                    Ok(false)
                }
            }
        }
    }
}

// Implement AuthzStorage for EtcdStorage
#[async_trait]
impl AuthzStorage for EtcdStorage {
    async fn get<T>(&self, key: &str, namespace: Option<&str>) -> Result<T>
    where
        T: DeserializeOwned + Send + Sync,
    {
        // Build the full key based on the resource type and namespace
        // AuthzStorage expects just the resource name, so we need to infer the resource type
        // from the generic type T
        let full_key = match namespace {
            Some(ns) => {
                // For namespaced resources, the key pattern is /registry/{resource_type}/{namespace}/{name}
                // We need to determine the resource type from T
                // For now, we'll construct keys for RBAC resources
                if std::any::type_name::<T>().contains("Role")
                    && !std::any::type_name::<T>().contains("Cluster")
                {
                    format!("/registry/roles/{}/{}", ns, key)
                } else if std::any::type_name::<T>().contains("RoleBinding")
                    && !std::any::type_name::<T>().contains("Cluster")
                {
                    format!("/registry/rolebindings/{}/{}", ns, key)
                } else {
                    format!("/registry/unknown/{}/{}", ns, key)
                }
            }
            None => {
                // For cluster-scoped resources
                if std::any::type_name::<T>().contains("ClusterRole")
                    && !std::any::type_name::<T>().contains("Binding")
                {
                    format!("/registry/clusterroles/{}", key)
                } else if std::any::type_name::<T>().contains("ClusterRoleBinding") {
                    format!("/registry/clusterrolebindings/{}", key)
                } else {
                    format!("/registry/unknown/{}", key)
                }
            }
        };

        Storage::get(self, &full_key).await
    }

    async fn list<T>(&self, namespace: Option<&str>) -> Result<Vec<T>>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
    {
        // Build the prefix based on resource type and namespace
        let prefix = match namespace {
            Some(ns) => {
                if std::any::type_name::<T>().contains("Role")
                    && !std::any::type_name::<T>().contains("Cluster")
                {
                    format!("/registry/roles/{}/", ns)
                } else if std::any::type_name::<T>().contains("RoleBinding")
                    && !std::any::type_name::<T>().contains("Cluster")
                {
                    format!("/registry/rolebindings/{}/", ns)
                } else {
                    format!("/registry/unknown/{}/", ns)
                }
            }
            None => {
                if std::any::type_name::<T>().contains("ClusterRole")
                    && !std::any::type_name::<T>().contains("Binding")
                {
                    "/registry/clusterroles/".to_string()
                } else if std::any::type_name::<T>().contains("ClusterRoleBinding") {
                    "/registry/clusterrolebindings/".to_string()
                } else {
                    "/registry/unknown/".to_string()
                }
            }
        };

        Storage::list(self, &prefix).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    // Note: These tests require a running etcd instance
    // Run with: docker run -d -p 2379:2379 -e ALLOW_NONE_AUTHENTICATION=yes bitnami/etcd

    #[tokio::test]
    #[ignore] // Ignore by default since it requires etcd
    async fn test_create_and_get() {
        let storage = EtcdStorage::new(vec!["http://localhost:2379".to_string()])
            .await
            .unwrap();

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct TestData {
            name: String,
            value: i32,
        }

        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let created = Storage::create(&storage, "/test/key", &data).await.unwrap();
        assert_eq!(created, data);

        let retrieved: TestData = Storage::get(&storage, "/test/key").await.unwrap();
        assert_eq!(retrieved, data);

        storage.delete("/test/key").await.unwrap();
    }
}
