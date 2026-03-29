use crate::{Storage, WatchEvent, WatchStream};
use async_trait::async_trait;
use etcd_client::{Client, Compare, CompareOp, GetOptions, TxnOp, WatchOptions};
use futures::StreamExt;
use rusternetes_common::{authz::AuthzStorage, Error, Result};
use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, error, info};

/// EtcdStorage implements the Storage trait using etcd as the backend.
///
/// The etcd `Client` is `Clone` and internally uses gRPC/tonic which
/// multiplexes requests over a single HTTP/2 connection. No mutex is needed —
/// cloning the client is cheap and allows fully concurrent access.
pub struct EtcdStorage {
    client: Client,
}

impl EtcdStorage {
    /// Create a new EtcdStorage instance
    pub async fn new(endpoints: Vec<String>) -> Result<Self> {
        let client = Client::connect(endpoints, None)
            .await
            .map_err(|e| Error::Storage(format!("Failed to connect to etcd: {}", e)))?;

        info!("Connected to etcd successfully");

        Ok(Self { client })
    }

    /// Helper to serialize a value to JSON
    fn serialize<T: Serialize>(value: &T) -> Result<String> {
        serde_json::to_string(value).map_err(|e| Error::Serialization(e))
    }

    /// Inject resourceVersion into a JSON string by modifying the metadata object directly.
    /// This avoids a full parse→modify→reserialize cycle for the common case.
    fn inject_resource_version(json: &str, mod_revision: i64) -> String {
        let rv_str = crate::concurrency::mod_revision_to_resource_version(mod_revision);
        // Fast path: find "metadata":{...} and inject/replace resourceVersion
        if let Some(meta_start) = json.find("\"metadata\"") {
            if let Some(brace_pos) = json[meta_start..].find('{') {
                let insert_pos = meta_start + brace_pos + 1;
                // Check if resourceVersion already exists in metadata
                let meta_end = find_matching_brace(json, meta_start + brace_pos);
                let meta_section = &json[insert_pos..meta_end];
                if let Some(rv_start) = meta_section.find("\"resourceVersion\"") {
                    // Replace existing resourceVersion value
                    let abs_rv_start = insert_pos + rv_start;
                    // Find the colon after "resourceVersion"
                    if let Some(colon_offset) = json[abs_rv_start..].find(':') {
                        let value_start = abs_rv_start + colon_offset + 1;
                        // Skip whitespace
                        let trimmed_start = value_start + json[value_start..].len() - json[value_start..].trim_start().len();
                        // Find end of value (next comma or closing brace)
                        let value_end = find_json_value_end(json, trimmed_start);
                        return format!(
                            "{}\"{}\"{}",
                            &json[..trimmed_start],
                            rv_str,
                            &json[value_end..]
                        );
                    }
                }
                // No existing resourceVersion, inject at start of metadata object
                return format!(
                    "{}\"resourceVersion\":\"{}\"{}{}",
                    &json[..insert_pos],
                    rv_str,
                    if meta_section.trim().is_empty() { "" } else { "," },
                    &json[insert_pos..]
                );
            }
        }
        // Fallback: parse and modify (handles edge cases)
        if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(json) {
            if let Some(metadata) = v.get_mut("metadata") {
                metadata["resourceVersion"] = serde_json::json!(rv_str);
            }
            serde_json::to_string(&v).unwrap_or_else(|_| json.to_string())
        } else {
            json.to_string()
        }
    }
}

/// Find the position of the matching closing brace for an opening brace at `open_pos`.
fn find_matching_brace(json: &str, open_pos: usize) -> usize {
    let bytes = json.as_bytes();
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;
    for i in open_pos..bytes.len() {
        if escape_next {
            escape_next = false;
            continue;
        }
        match bytes[i] {
            b'\\' if in_string => escape_next = true,
            b'"' => in_string = !in_string,
            b'{' if !in_string => depth += 1,
            b'}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    json.len()
}

/// Find the end of a JSON value starting at `start` (returns position after the value).
fn find_json_value_end(json: &str, start: usize) -> usize {
    let bytes = json.as_bytes();
    let mut i = start;
    if i >= bytes.len() {
        return i;
    }
    match bytes[i] {
        b'"' => {
            // String value — find closing quote
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'"' {
                    return i + 1;
                }
                i += 1;
            }
            i
        }
        b'{' => find_matching_brace(json, i) + 1,
        b'[' => {
            // Array
            let mut depth = 0;
            let mut in_string = false;
            let mut escape_next = false;
            for j in i..bytes.len() {
                if escape_next { escape_next = false; continue; }
                match bytes[j] {
                    b'\\' if in_string => escape_next = true,
                    b'"' => in_string = !in_string,
                    b'[' if !in_string => depth += 1,
                    b']' if !in_string => {
                        depth -= 1;
                        if depth == 0 { return j + 1; }
                    }
                    _ => {}
                }
            }
            bytes.len()
        }
        _ => {
            // Number, bool, null — find delimiter
            while i < bytes.len() && bytes[i] != b',' && bytes[i] != b'}' && bytes[i] != b']' && bytes[i] != b' ' && bytes[i] != b'\n' && bytes[i] != b'\r' && bytes[i] != b'\t' {
                i += 1;
            }
            i
        }
    }
}

#[async_trait]
impl Storage for EtcdStorage {
    async fn create<T>(&self, key: &str, value: &T) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
    {
        let mut client = self.client.clone();
        // Ensure metadata exists and generation is set to 1 on creation
        let json = {
            let mut raw = Self::serialize(value)?;
            if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&raw) {
                // Ensure metadata object exists
                if v.get("metadata").is_none() || v.get("metadata").map_or(false, |m| m.is_null()) {
                    v["metadata"] = serde_json::json!({});
                }
                if let Some(metadata) = v.get_mut("metadata") {
                    if metadata.get("generation").map_or(true, |g| g.is_null()) {
                        metadata["generation"] = serde_json::json!(1);
                    }
                }
                raw = serde_json::to_string(&v).unwrap_or(raw);
            }
            raw
        };

        // Use a transaction to ensure the key doesn't already exist
        // Request prev_kv to avoid a separate GET for the mod_revision
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

        // Get the mod_revision from the transaction response header
        let mod_revision = txn_resp.header().map(|h| h.revision()).unwrap_or(0);

        // Inject resourceVersion and deserialize
        let json_with_rv = Self::inject_resource_version(&json, mod_revision);
        serde_json::from_str(&json_with_rv).map_err(|e| Error::Serialization(e))
    }

    async fn get<T>(&self, key: &str) -> Result<T>
    where
        T: DeserializeOwned + Send + Sync,
    {
        let mut client = self.client.clone();

        let resp = client
            .get(key, None)
            .await
            .map_err(|e| Error::Storage(format!("Failed to get resource: {}", e)))?;

        if let Some(kv) = resp.kvs().first() {
            let json = kv
                .value_str()
                .map_err(|e| Error::Storage(format!("Invalid UTF-8 in value: {}", e)))?;

            let mod_revision = kv.mod_revision();
            let json_with_rv = Self::inject_resource_version(json, mod_revision);
            serde_json::from_str(&json_with_rv).map_err(|e| Error::Serialization(e))
        } else {
            Err(Error::NotFound(key.to_string()))
        }
    }

    async fn update<T>(&self, key: &str, value: &T) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
    {
        let mut client = self.client.clone();
        let json = Self::serialize(value)?;

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
                .or_else(vec![TxnOp::get(key, None)]);

            let txn_resp = client
                .txn(txn)
                .await
                .map_err(|e| Error::Storage(format!("Failed to update resource: {}", e)))?;

            if !txn_resp.succeeded() {
                // Get the current resourceVersion from the failed txn's else branch
                let current_rv = txn_resp
                    .op_responses()
                    .first()
                    .and_then(|resp| {
                        // The else branch returns a get response
                        if let etcd_client::TxnOpResponse::Get(get_resp) = resp {
                            get_resp.kvs().first().map(|kv| {
                                crate::concurrency::mod_revision_to_resource_version(kv.mod_revision())
                            })
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| "unknown".to_string());
                return Err(Error::Conflict(format!(
                    "resourceVersion mismatch: resource was modified (expected: {}, current: {})",
                    incoming_rv, current_rv
                )));
            }

            debug!("Updated resource at key: {}", key);

            // Get mod_revision from the transaction response header
            let mod_revision = txn_resp.header().map(|h| h.revision()).unwrap_or(0);
            let json_with_rv = Self::inject_resource_version(&json, mod_revision);
            serde_json::from_str(&json_with_rv).map_err(|e| Error::Serialization(e))
        } else {
            // No resourceVersion provided — check key exists, then put
            let get_resp = client
                .get(key, Some(GetOptions::new().with_keys_only()))
                .await
                .map_err(|e| Error::Storage(format!("Failed to check resource: {}", e)))?;

            if get_resp.kvs().is_empty() {
                return Err(Error::NotFound(key.to_string()));
            }

            let put_resp = client
                .put(key, json.clone(), None)
                .await
                .map_err(|e| Error::Storage(format!("Failed to update resource: {}", e)))?;

            debug!("Updated resource at key: {}", key);

            // Get mod_revision from put response header
            let mod_revision = put_resp.header().map(|h| h.revision()).unwrap_or(0);
            let json_with_rv = Self::inject_resource_version(&json, mod_revision);
            serde_json::from_str(&json_with_rv).map_err(|e| Error::Serialization(e))
        }
    }

    async fn update_raw(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        let mut client = self.client.clone();
        let json = serde_json::to_string(value).map_err(|e| Error::Serialization(e))?;

        // Check if the key exists first (keys_only to save bandwidth)
        let get_resp = client
            .get(key, Some(GetOptions::new().with_keys_only()))
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
        let mut client = self.client.clone();

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
        let mut client = self.client.clone();

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
                Some(_key) => {
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

                // Inject resourceVersion and deserialize in one step
                let json_with_rv = Self::inject_resource_version(json, mod_revision);
                match serde_json::from_str::<T>(&json_with_rv) {
                    Ok(value) => {
                        results.push(value);
                    }
                    Err(e) => {
                        error!("Failed to deserialize value at {}: {}", key_str, e);
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
        let mut client = self.client.clone();
        let watch_options = WatchOptions::new()
            .with_prefix()
            .with_prev_key()
            .with_start_revision(revision);
        let (_watcher, stream) = client
            .watch(prefix, Some(watch_options))
            .await
            .map_err(|e| Error::Storage(format!("Failed to create watch from revision {}: {}", revision, e)))?;
        info!("Started watching prefix: {} from revision {}", prefix, revision);
        // Use flat_map to handle multiple events per etcd watch response.
        // etcd can batch multiple events into a single response, and we must
        // emit all of them — not just the first one.
        let watch_stream = stream.flat_map(move |watch_resp| {
            let events: Vec<Result<WatchEvent>> = match watch_resp {
                Ok(resp) => {
                    resp.events().iter().map(|event| {
                        let key = event
                            .kv()
                            .map(|kv| kv.key_str().unwrap_or("").to_string())
                            .unwrap_or_default();
                        match event.event_type() {
                            etcd_client::EventType::Put => {
                                let raw_value = event
                                    .kv()
                                    .map(|kv| String::from_utf8_lossy(kv.value()).to_string())
                                    .unwrap_or_default();
                                let mod_revision = event.kv().map(|kv| kv.mod_revision()).unwrap_or(0);
                                let value = Self::inject_resource_version(&raw_value, mod_revision);
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
                                let prev_value = Self::inject_resource_version(&raw_prev, mod_revision);
                                Ok(WatchEvent::Deleted(key, prev_value))
                            }
                        }
                    }).collect()
                }
                Err(e) => vec![Err(Error::Storage(format!("Watch error: {}", e)))],
            };
            futures::stream::iter(events)
        });
        Ok(Box::pin(watch_stream))
    }

    async fn watch(&self, prefix: &str) -> Result<WatchStream> {
        let mut client = self.client.clone();

        // Enable prev_kv to get the previous value on DELETE events (required for Kubernetes)
        let watch_options = WatchOptions::new().with_prefix().with_prev_key();
        let (_watcher, stream) = client
            .watch(prefix, Some(watch_options))
            .await
            .map_err(|e| Error::Storage(format!("Failed to create watch: {}", e)))?;

        info!("Started watching prefix: {}", prefix);

        // Convert etcd watch stream to our WatchStream.
        // Use flat_map to handle multiple events per etcd watch response.
        let watch_stream = stream.flat_map(move |watch_resp| {
            let events: Vec<Result<WatchEvent>> = match watch_resp {
                Ok(resp) => {
                    resp.events().iter().map(|event| {
                        let key = event
                            .kv()
                            .map(|kv| kv.key_str().unwrap_or("").to_string())
                            .unwrap_or_default();

                        match event.event_type() {
                            etcd_client::EventType::Put => {
                                let raw_value = event
                                    .kv()
                                    .and_then(|kv| kv.value_str().ok())
                                    .unwrap_or("")
                                    .to_string();

                                let mod_revision = event.kv().map(|kv| kv.mod_revision()).unwrap_or(0);
                                let value = Self::inject_resource_version(&raw_value, mod_revision);

                                if event.kv().map(|kv| kv.version()).unwrap_or(0) == 1 {
                                    Ok(WatchEvent::Added(key, value))
                                } else {
                                    Ok(WatchEvent::Modified(key, value))
                                }
                            }
                            etcd_client::EventType::Delete => {
                                let raw_prev = event
                                    .prev_kv()
                                    .and_then(|kv| kv.value_str().ok())
                                    .unwrap_or("")
                                    .to_string();
                                let mod_revision = event.kv().map(|kv| kv.mod_revision()).unwrap_or(0);
                                let prev_value = Self::inject_resource_version(&raw_prev, mod_revision);
                                Ok(WatchEvent::Deleted(key, prev_value))
                            }
                        }
                    }).collect()
                }
                Err(e) => vec![Err(Error::Storage(format!("Watch error: {}", e)))],
            };
            futures::stream::iter(events)
        });

        Ok(Box::pin(watch_stream))
    }

    async fn current_revision(&self) -> Result<i64> {
        let mut client = self.client.clone();
        // Use keys_only to minimize data transfer
        let resp = client
            .get("/", Some(GetOptions::new().with_keys_only()))
            .await
            .map_err(|e| Error::Storage(format!("Failed to get current revision: {}", e)))?;
        Ok(resp.header().unwrap().revision())
    }

    async fn is_revision_compacted(&self, revision: i64) -> Result<bool> {
        let mut client = self.client.clone();
        // Try to get a key at the given revision; if compacted, etcd returns an error
        let opts = GetOptions::new().with_revision(revision).with_keys_only();
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
        let full_key = match namespace {
            Some(ns) => {
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

    #[test]
    fn test_inject_resource_version_with_existing() {
        let json = r#"{"metadata":{"name":"test","resourceVersion":"100"},"spec":{}}"#;
        let result = EtcdStorage::inject_resource_version(json, 200);
        assert!(result.contains("\"200\""));
        assert!(!result.contains("\"100\""));
    }

    #[test]
    fn test_inject_resource_version_without_existing() {
        let json = r#"{"metadata":{"name":"test"},"spec":{}}"#;
        let result = EtcdStorage::inject_resource_version(json, 42);
        assert!(result.contains("\"resourceVersion\":\"42\""));
    }

    #[test]
    fn test_inject_resource_version_empty_metadata() {
        let json = r#"{"metadata":{},"spec":{}}"#;
        let result = EtcdStorage::inject_resource_version(json, 99);
        assert!(result.contains("\"resourceVersion\":\"99\""));
    }

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
