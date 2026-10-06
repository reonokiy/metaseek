//! Caching module for Metaseek
//!
//! Provides various caching mechanisms for search results and engine data.

use moka::future::Cache;
use std::time::Duration;

/// Cache for search results
pub struct ResultCache {
    cache: Cache<String, Vec<u8>>,
}

impl ResultCache {
    /// Create a new result cache with specified TTL
    pub fn new(ttl_seconds: u64, max_capacity: u64) -> Self {
        let cache = Cache::builder()
            .time_to_live(Duration::from_secs(ttl_seconds))
            .max_capacity(max_capacity)
            .build();

        Self { cache }
    }

    /// Get a cached result
    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.cache.get(key).await
    }

    /// Store a result in cache
    pub async fn set(&self, key: String, value: Vec<u8>) {
        self.cache.insert(key, value).await;
    }

    /// Remove a cached result
    pub async fn remove(&self, key: &str) {
        self.cache.remove(key).await;
    }

    /// Clear the entire cache
    pub fn clear(&self) {
        self.cache.invalidate_all();
    }

    /// Get cache size
    pub fn size(&self) -> u64 {
        self.cache.entry_count()
    }
}

impl Default for ResultCache {
    fn default() -> Self {
        Self::new(300, 10000) // 5 minutes TTL, 10k max entries
    }
}

/// Cache for engine tokens/state
pub struct EngineCache {
    cache: Cache<String, String>,
}

impl EngineCache {
    /// Create a new engine cache
    pub fn new(ttl_seconds: u64) -> Self {
        let cache = Cache::builder()
            .time_to_live(Duration::from_secs(ttl_seconds))
            .max_capacity(1000)
            .build();

        Self { cache }
    }

    /// Get cached value
    pub async fn get(&self, engine: &str, key: &str) -> Option<String> {
        let cache_key = format!("{}:{}", engine, key);
        self.cache.get(&cache_key).await
    }

    /// Set cached value
    pub async fn set(&self, engine: &str, key: &str, value: String) {
        let cache_key = format!("{}:{}", engine, key);
        self.cache.insert(cache_key, value).await;
    }
}

impl Default for EngineCache {
    fn default() -> Self {
        Self::new(3600) // 1 hour TTL
    }
}

/// Generate a cache key for a search query
pub fn query_cache_key(query: &str, engines: &[String], page: u32, lang: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(query.as_bytes());
    for engine in engines {
        hasher.update(engine.as_bytes());
    }
    hasher.update(page.to_string().as_bytes());
    hasher.update(lang.as_bytes());

    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_result_cache() {
        let cache = ResultCache::new(60, 100);
        cache.set("test".to_string(), vec![1, 2, 3]).await;

        let result = cache.get("test").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![1, 2, 3]);
    }
}
