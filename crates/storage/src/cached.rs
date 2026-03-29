//! Cached Storage Layer
//!
//! Wraps an underlying Storage backend with an in-memory cache that is kept
//! in sync via etcd watches. This implements the Kubernetes informer pattern:
//!
//! - `list()` and `get()` are served from cache (zero etcd round-trips)
//! - `create()`, `update()`, `delete()` go to etcd, then update the cache
//! - A background watch task keeps the cache in sync with etcd
//!
//! This eliminates the polling anti-pattern where 30+ controllers each
//! list all resources from etcd every 1-2 seconds.

use crate::{Storage, WatchEvent, WatchStream};
use async_trait::async_trait;
use rusternetes_common::{Error, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// In-memory cache entry: stores the raw JSON string for each key
#[derive(Clone)]
struct CacheEntry {
    json: String,
}

/// CachedStorage wraps a Storage backend with an in-memory read cache.
/// Reads (get/list) are served from cache. Writes go through to the backend
/// and update the cache from the backend response.
pub struct CachedStorage<S: Storage> {
    backend: Arc<S>,
    /// Cache: key → JSON string. Using RwLock for concurrent reads.
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Set of prefixes we're actively watching and caching
    watched_prefixes: Arc<RwLock<std::collections::HashSet<String>>>,
}

impl<S: Storage + 'static> CachedStorage<S> {
    /// Create a new CachedStorage wrapping the given backend.
    pub fn new(backend: Arc<S>) -> Arc<Self> {
        let cached = Arc::new(Self {
            backend,
            cache: Arc::new(RwLock::new(HashMap::new())),
            watched_prefixes: Arc::new(RwLock::new(std::collections::HashSet::new())),
        });
        cached
    }

    /// Start watching a prefix and populating the cache.
    /// Call this for each resource type the controllers need (e.g., "/registry/pods/").
    /// This does an initial list to populate the cache, then starts a background
    /// watch to keep it in sync.
    pub async fn watch_prefix(self: &Arc<Self>, prefix: &str) -> Result<()> {
        // Check if already watching
        {
            let watched = self.watched_prefixes.read().await;
            if watched.contains(prefix) {
                return Ok(());
            }
        }

        // Mark as watching
        {
            let mut watched = self.watched_prefixes.write().await;
            watched.insert(prefix.to_string());
        }

        // Initial list to populate cache
        let items: Vec<serde_json::Value> = self.backend.list(prefix).await?;
        {
            let mut cache = self.cache.write().await;
            for item in &items {
                if let Some(key) = Self::extract_cache_key(item, prefix) {
                    let json = serde_json::to_string(item)
                        .unwrap_or_default();
                    cache.insert(key, CacheEntry { json });
                }
            }
        }
        info!("CachedStorage: populated {} items for prefix {}", items.len(), prefix);

        // Start background watch
        let this = self.clone();
        let prefix_owned = prefix.to_string();
        tokio::spawn(async move {
            loop {
                match this.backend.watch(&prefix_owned).await {
                    Ok(mut stream) => {
                        use futures::StreamExt;
                        while let Some(event_result) = stream.next().await {
                            match event_result {
                                Ok(WatchEvent::Added(key, value) | WatchEvent::Modified(key, value)) => {
                                    let mut cache = this.cache.write().await;
                                    cache.insert(key, CacheEntry { json: value });
                                }
                                Ok(WatchEvent::Deleted(key, _)) => {
                                    let mut cache = this.cache.write().await;
                                    cache.remove(&key);
                                }
                                Err(e) => {
                                    warn!("CachedStorage watch error for {}: {}", prefix_owned, e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("CachedStorage: failed to start watch for {}: {}", prefix_owned, e);
                    }
                }
                // Reconnect after brief pause
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                // Re-populate cache on reconnect to avoid stale data
                match this.backend.list::<serde_json::Value>(&prefix_owned).await {
                    Ok(items) => {
                        let mut cache = this.cache.write().await;
                        // Remove old entries for this prefix
                        cache.retain(|k, _| !k.starts_with(&prefix_owned));
                        for item in &items {
                            if let Some(key) = Self::extract_cache_key(item, &prefix_owned) {
                                let json = serde_json::to_string(item).unwrap_or_default();
                                cache.insert(key, CacheEntry { json });
                            }
                        }
                    }
                    Err(e) => {
                        warn!("CachedStorage: failed to re-populate cache for {}: {}", prefix_owned, e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Extract the etcd key from a resource's metadata.
    /// Constructs /registry/{type}/{namespace}/{name} or /registry/{type}/{name}
    fn extract_cache_key(value: &serde_json::Value, prefix: &str) -> Option<String> {
        let metadata = value.get("metadata")?;
        let name = metadata.get("name")?.as_str()?;
        let namespace = metadata.get("namespace").and_then(|v| v.as_str());

        // The prefix tells us the resource type path
        // e.g. "/registry/pods/" → we build "/registry/pods/{ns}/{name}"
        match namespace {
            Some(ns) => Some(format!("{}{}/{}", prefix, ns, name)),
            None => Some(format!("{}{}", prefix, name)),
        }
    }

    /// Check if a prefix is being cached
    async fn is_cached_prefix(&self, prefix: &str) -> bool {
        let watched = self.watched_prefixes.read().await;
        watched.iter().any(|p| prefix.starts_with(p))
    }
}

#[async_trait]
impl<S: Storage + 'static> Storage for CachedStorage<S> {
    async fn create<T>(&self, key: &str, value: &T) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
    {
        // Write through to backend
        let result = self.backend.create(key, value).await?;

        // Update cache
        if let Ok(json) = serde_json::to_string(&result) {
            let mut cache = self.cache.write().await;
            cache.insert(key.to_string(), CacheEntry { json });
        }

        Ok(result)
    }

    async fn get<T>(&self, key: &str) -> Result<T>
    where
        T: DeserializeOwned + Send + Sync,
    {
        // Try cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(key) {
                if let Ok(value) = serde_json::from_str(&entry.json) {
                    return Ok(value);
                }
            }
        }

        // Cache miss — fall through to backend.
        // Don't populate cache here since T may not be Serialize.
        // The background watch will populate cache entries for watched prefixes.
        self.backend.get(key).await
    }

    async fn update<T>(&self, key: &str, value: &T) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
    {
        // Write through to backend
        let result = self.backend.update(key, value).await?;

        // Update cache
        if let Ok(json) = serde_json::to_string(&result) {
            let mut cache = self.cache.write().await;
            cache.insert(key.to_string(), CacheEntry { json });
        }

        Ok(result)
    }

    async fn update_raw(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        self.backend.update_raw(key, value).await?;

        // Update cache
        if let Ok(json) = serde_json::to_string(value) {
            let mut cache = self.cache.write().await;
            cache.insert(key.to_string(), CacheEntry { json });
        }

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.backend.delete(key).await?;

        // Remove from cache
        let mut cache = self.cache.write().await;
        cache.remove(key);

        Ok(())
    }

    async fn list<T>(&self, prefix: &str) -> Result<Vec<T>>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
    {
        // If this prefix is being watched, serve from cache
        if self.is_cached_prefix(prefix).await {
            let cache = self.cache.read().await;
            let mut results = Vec::new();
            for (key, entry) in cache.iter() {
                if key.starts_with(prefix) {
                    match serde_json::from_str::<T>(&entry.json) {
                        Ok(value) => results.push(value),
                        Err(e) => {
                            debug!("CachedStorage: failed to deserialize cached entry {}: {}", key, e);
                        }
                    }
                }
            }
            return Ok(results);
        }

        // Not cached — fall through to backend
        self.backend.list(prefix).await
    }

    async fn watch(&self, prefix: &str) -> Result<WatchStream> {
        self.backend.watch(prefix).await
    }

    async fn watch_from_revision(&self, prefix: &str, revision: i64) -> Result<WatchStream> {
        self.backend.watch_from_revision(prefix, revision).await
    }

    async fn current_revision(&self) -> Result<i64> {
        self.backend.current_revision().await
    }

    async fn is_revision_compacted(&self, revision: i64) -> Result<bool> {
        self.backend.is_revision_compacted(revision).await
    }
}
