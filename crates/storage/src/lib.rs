use async_trait::async_trait;
use rusternetes_common::Result;
use serde::{de::DeserializeOwned, Serialize};

pub mod concurrency;
pub mod etcd;
pub mod memory;

// Re-export MemoryStorage for convenient testing
pub use memory::MemoryStorage;

/// Storage trait for persisting Kubernetes resources
#[async_trait]
pub trait Storage: Send + Sync {
    /// Create a new resource
    async fn create<T>(&self, key: &str, value: &T) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync;

    /// Get a resource by key
    async fn get<T>(&self, key: &str) -> Result<T>
    where
        T: DeserializeOwned + Send + Sync;

    /// Update an existing resource
    async fn update<T>(&self, key: &str, value: &T) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync;

    /// Update a resource with raw JSON value (for GC operations)
    async fn update_raw(&self, key: &str, value: &serde_json::Value) -> Result<()>;

    /// Delete a resource
    async fn delete(&self, key: &str) -> Result<()>;

    /// List resources with a given prefix
    async fn list<T>(&self, prefix: &str) -> Result<Vec<T>>
    where
        T: Serialize + DeserializeOwned + Send + Sync;

    /// Watch for changes to resources with a given prefix
    async fn watch(&self, prefix: &str) -> Result<WatchStream>;

    /// Watch for changes starting from a specific revision
    async fn watch_from_revision(&self, prefix: &str, revision: i64) -> Result<WatchStream>;

    /// Get the current storage revision (etcd mod_revision)
    async fn current_revision(&self) -> Result<i64>;

    /// Check if a revision has been compacted (no longer available)
    async fn is_revision_compacted(&self, revision: i64) -> Result<bool>;
}

/// Event types for watch operations
#[derive(Debug, Clone)]
pub enum WatchEvent {
    Added(String, String),    // key, value
    Modified(String, String), // key, value
    Deleted(String, String),  // key, previous value (for Kubernetes compliance)
}

/// Stream of watch events
pub type WatchStream = futures::stream::BoxStream<'static, Result<WatchEvent>>;

/// Helper function to build resource keys
pub fn build_key(resource_type: &str, namespace: Option<&str>, name: &str) -> String {
    match namespace {
        Some(ns) => format!("/registry/{}/{}/{}", resource_type, ns, name),
        None => format!("/registry/{}/{}", resource_type, name),
    }
}

/// Helper function to build prefix for listing
pub fn build_prefix(resource_type: &str, namespace: Option<&str>) -> String {
    match namespace {
        Some(ns) => format!("/registry/{}/{}/", resource_type, ns),
        None => format!("/registry/{}/", resource_type),
    }
}
