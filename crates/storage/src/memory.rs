use crate::{Storage, WatchStream};
use async_trait::async_trait;
use rusternetes_common::{Error, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// In-memory storage implementation for testing
#[derive(Clone)]
pub struct MemoryStorage {
    data: Arc<RwLock<HashMap<String, String>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Clear all data (useful for test cleanup)
    pub fn clear(&self) {
        self.data.write().unwrap().clear();
    }

    /// Get the number of stored items
    pub fn len(&self) -> usize {
        self.data.read().unwrap().len()
    }

    /// Check if storage is empty
    pub fn is_empty(&self) -> bool {
        self.data.read().unwrap().is_empty()
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn create<T>(&self, key: &str, value: &T) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
    {
        let serialized = serde_json::to_string(value)?;

        let mut data = self.data.write().unwrap();
        if data.contains_key(key) {
            return Err(Error::AlreadyExists(key.to_string()));
        }

        data.insert(key.to_string(), serialized.clone());

        Ok(serde_json::from_str(&serialized)?)
    }

    async fn get<T>(&self, key: &str) -> Result<T>
    where
        T: DeserializeOwned + Send + Sync,
    {
        let data = self.data.read().unwrap();
        let value = data
            .get(key)
            .ok_or_else(|| Error::NotFound(key.to_string()))?;

        Ok(serde_json::from_str(value)?)
    }

    async fn update<T>(&self, key: &str, value: &T) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
    {
        let serialized = serde_json::to_string(value)?;

        let mut data = self.data.write().unwrap();
        if !data.contains_key(key) {
            return Err(Error::NotFound(key.to_string()));
        }

        data.insert(key.to_string(), serialized.clone());

        Ok(serde_json::from_str(&serialized)?)
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let mut data = self.data.write().unwrap();
        data.remove(key)
            .ok_or_else(|| Error::NotFound(key.to_string()))?;

        Ok(())
    }

    async fn list<T>(&self, prefix: &str) -> Result<Vec<T>>
    where
        T: DeserializeOwned + Send + Sync,
    {
        let data = self.data.read().unwrap();
        let mut results = Vec::new();

        for (key, value) in data.iter() {
            if key.starts_with(prefix) {
                results.push(serde_json::from_str(value)?);
            }
        }

        Ok(results)
    }

    async fn watch(&self, _prefix: &str) -> Result<WatchStream> {
        // For testing, return an empty stream
        // In a real implementation, you could use channels to send events
        use futures::stream;
        Ok(Box::pin(stream::empty()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestResource {
        name: String,
        value: i32,
    }

    #[tokio::test]
    async fn test_create_and_get() {
        let storage = MemoryStorage::new();
        let resource = TestResource {
            name: "test".to_string(),
            value: 42,
        };

        storage.create("/test/key", &resource).await.unwrap();
        let retrieved: TestResource = storage.get("/test/key").await.unwrap();

        assert_eq!(resource, retrieved);
    }

    #[tokio::test]
    async fn test_create_duplicate_fails() {
        let storage = MemoryStorage::new();
        let resource = TestResource {
            name: "test".to_string(),
            value: 42,
        };

        storage.create("/test/key", &resource).await.unwrap();
        let result = storage.create("/test/key", &resource).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update() {
        let storage = MemoryStorage::new();
        let resource = TestResource {
            name: "test".to_string(),
            value: 42,
        };

        storage.create("/test/key", &resource).await.unwrap();

        let updated = TestResource {
            name: "updated".to_string(),
            value: 100,
        };
        storage.update("/test/key", &updated).await.unwrap();

        let retrieved: TestResource = storage.get("/test/key").await.unwrap();
        assert_eq!(updated, retrieved);
    }

    #[tokio::test]
    async fn test_delete() {
        let storage = MemoryStorage::new();
        let resource = TestResource {
            name: "test".to_string(),
            value: 42,
        };

        storage.create("/test/key", &resource).await.unwrap();
        storage.delete("/test/key").await.unwrap();

        let result: Result<TestResource> = storage.get("/test/key").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list() {
        let storage = MemoryStorage::new();

        storage.create("/test/ns1/pod1", &TestResource { name: "pod1".to_string(), value: 1 }).await.unwrap();
        storage.create("/test/ns1/pod2", &TestResource { name: "pod2".to_string(), value: 2 }).await.unwrap();
        storage.create("/test/ns2/pod3", &TestResource { name: "pod3".to_string(), value: 3 }).await.unwrap();

        let ns1_pods: Vec<TestResource> = storage.list("/test/ns1/").await.unwrap();
        assert_eq!(ns1_pods.len(), 2);

        let all_pods: Vec<TestResource> = storage.list("/test/").await.unwrap();
        assert_eq!(all_pods.len(), 3);
    }

    #[tokio::test]
    async fn test_clear() {
        let storage = MemoryStorage::new();

        storage.create("/test/key1", &TestResource { name: "test1".to_string(), value: 1 }).await.unwrap();
        storage.create("/test/key2", &TestResource { name: "test2".to_string(), value: 2 }).await.unwrap();

        assert_eq!(storage.len(), 2);

        storage.clear();

        assert_eq!(storage.len(), 0);
        assert!(storage.is_empty());
    }
}
