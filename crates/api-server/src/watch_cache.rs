//! Watch Cache / Multiplexer
//!
//! Maintains a single etcd watch per resource prefix and fans out events
//! to all subscribed client watches. This avoids creating N etcd watches
//! for N clients, which overwhelms etcd and exhausts HTTP/2 stream limits.

use rusternetes_storage::{WatchEvent, WatchStream, Storage};
use rusternetes_storage::etcd::EtcdStorage;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info};

/// A cached watch event with metadata
#[derive(Debug, Clone)]
pub struct CachedWatchEvent {
    pub event: WatchEventData,
    pub revision: i64,
}

/// The event data (simplified from WatchEvent)
#[derive(Debug, Clone)]
pub enum WatchEventData {
    Added(String, String),    // key, value JSON
    Modified(String, String), // key, value JSON
    Deleted(String, String),  // key, previous value JSON
}

/// WatchCache manages shared watch streams for resource prefixes.
/// Instead of one etcd watch per client, we have one per prefix.
pub struct WatchCache {
    /// Map of resource prefix → broadcast sender
    /// Each prefix has one etcd watch that broadcasts to all subscribers
    watchers: RwLock<HashMap<String, broadcast::Sender<CachedWatchEvent>>>,
    storage: Arc<EtcdStorage>,
    /// Current revision counter (approximation based on timestamp)
    revision: RwLock<i64>,
}

impl WatchCache {
    pub fn new(storage: Arc<EtcdStorage>) -> Self {
        let rev = chrono::Utc::now().timestamp();
        Self {
            watchers: RwLock::new(HashMap::new()),
            storage,
            revision: RwLock::new(rev),
        }
    }

    /// Subscribe to watch events for a resource prefix.
    /// Returns a broadcast receiver that will receive all events for this prefix.
    /// If no etcd watch exists for this prefix, one is started.
    pub async fn subscribe(&self, prefix: &str) -> broadcast::Receiver<CachedWatchEvent> {
        // Check if we already have a watcher for this prefix
        {
            let watchers = self.watchers.read().await;
            if let Some(tx) = watchers.get(prefix) {
                return tx.subscribe();
            }
        }

        // Create a new watcher
        let (tx, rx) = broadcast::channel(1024); // Buffer 1024 events
        {
            let mut watchers = self.watchers.write().await;
            // Double-check after acquiring write lock
            if let Some(existing_tx) = watchers.get(prefix) {
                return existing_tx.subscribe();
            }
            watchers.insert(prefix.to_string(), tx.clone());
        }

        // Start the etcd watch in a background task
        let storage = self.storage.clone();
        let prefix_owned = prefix.to_string();
        let tx_clone = tx.clone();

        tokio::spawn(async move {
            info!("WatchCache: starting shared watch for prefix {}", prefix_owned);
            loop {
                match storage.watch(&prefix_owned).await {
                    Ok(mut stream) => {
                        use futures::StreamExt;
                        while let Some(event_result) = stream.next().await {
                            match event_result {
                                Ok(WatchEvent::Added(key, value)) => {
                                    let cached = CachedWatchEvent {
                                        event: WatchEventData::Added(key, value),
                                        revision: chrono::Utc::now().timestamp(),
                                    };
                                    // If no receivers, the send returns Err but that's OK
                                    let _ = tx_clone.send(cached);
                                }
                                Ok(WatchEvent::Modified(key, value)) => {
                                    let cached = CachedWatchEvent {
                                        event: WatchEventData::Modified(key, value),
                                        revision: chrono::Utc::now().timestamp(),
                                    };
                                    let _ = tx_clone.send(cached);
                                }
                                Ok(WatchEvent::Deleted(key, prev_value)) => {
                                    let cached = CachedWatchEvent {
                                        event: WatchEventData::Deleted(key, prev_value),
                                        revision: chrono::Utc::now().timestamp(),
                                    };
                                    let _ = tx_clone.send(cached);
                                }
                                Err(_) => {
                                    // Transient error, continue
                                    continue;
                                }
                            }
                        }
                        // Stream ended, reconnect
                        debug!("WatchCache: stream ended for {}, reconnecting", prefix_owned);
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                    Err(e) => {
                        error!("WatchCache: failed to create watch for {}: {}", prefix_owned, e);
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
        });

        rx
    }

    /// Get the current approximate revision
    pub async fn current_revision(&self) -> i64 {
        *self.revision.read().await
    }
}

/// Convert a broadcast receiver into a WatchStream compatible with existing handlers.
pub fn broadcast_to_stream(
    mut rx: broadcast::Receiver<CachedWatchEvent>,
) -> WatchStream {
    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(cached) => {
                    let event = match cached.event {
                        WatchEventData::Added(key, value) => WatchEvent::Added(key, value),
                        WatchEventData::Modified(key, value) => WatchEvent::Modified(key, value),
                        WatchEventData::Deleted(key, prev) => WatchEvent::Deleted(key, prev),
                    };
                    yield Ok(event);
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    debug!("Watch stream lagged by {} events, continuing", n);
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };
    Box::pin(stream)
}
