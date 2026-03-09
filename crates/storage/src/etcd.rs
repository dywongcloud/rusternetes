use crate::{Storage, WatchEvent, WatchStream};
use async_trait::async_trait;
use etcd_client::{Client, Compare, CompareOp, GetOptions, TxnOp, WatchOptions};
use futures::StreamExt;
use rusternetes_common::{Error, Result};
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
        let json = Self::serialize(value)?;

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
        Self::deserialize(&json)
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
            Self::deserialize(json)
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

        // Check if the key exists first
        let get_resp = client
            .get(key, None)
            .await
            .map_err(|e| Error::Storage(format!("Failed to check resource: {}", e)))?;

        if get_resp.kvs().is_empty() {
            return Err(Error::NotFound(key.to_string()));
        }

        client
            .put(key, json.clone(), None)
            .await
            .map_err(|e| Error::Storage(format!("Failed to update resource: {}", e)))?;

        debug!("Updated resource at key: {}", key);
        Self::deserialize(&json)
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
        T: DeserializeOwned + Send + Sync,
    {
        let mut client = self.client.lock().await;

        let get_options = GetOptions::new().with_prefix();
        let resp = client
            .get(prefix, Some(get_options))
            .await
            .map_err(|e| Error::Storage(format!("Failed to list resources: {}", e)))?;

        let mut results = Vec::new();
        for kv in resp.kvs() {
            let json = kv
                .value_str()
                .map_err(|e| Error::Storage(format!("Invalid UTF-8 in value: {}", e)))?;
            match Self::deserialize(json) {
                Ok(value) => results.push(value),
                Err(e) => {
                    error!("Failed to deserialize value: {}", e);
                    continue;
                }
            }
        }

        debug!("Listed {} resources with prefix: {}", results.len(), prefix);
        Ok(results)
    }

    async fn watch(&self, prefix: &str) -> Result<WatchStream> {
        let mut client = self.client.lock().await;

        let watch_options = WatchOptions::new().with_prefix();
        let (mut watcher, stream) = client
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
                                let value = event
                                    .kv()
                                    .and_then(|kv| kv.value_str().ok())
                                    .unwrap_or("")
                                    .to_string();

                                // Check if this is a new key or an update
                                if event.kv().map(|kv| kv.version()).unwrap_or(0) == 1 {
                                    Ok(WatchEvent::Added(key, value))
                                } else {
                                    Ok(WatchEvent::Modified(key, value))
                                }
                            }
                            etcd_client::EventType::Delete => Ok(WatchEvent::Deleted(key)),
                        };
                    }
                    Err(Error::Storage("Empty watch response".to_string()))
                }
                Err(e) => Err(Error::Storage(format!("Watch error: {}", e))),
            }
        });

        Ok(Box::pin(watch_stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let created = storage.create("/test/key", &data).await.unwrap();
        assert_eq!(created, data);

        let retrieved: TestData = storage.get("/test/key").await.unwrap();
        assert_eq!(retrieved, data);

        storage.delete("/test/key").await.unwrap();
    }
}
