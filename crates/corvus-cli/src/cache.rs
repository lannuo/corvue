//! Response caching system
//!
//! Provides caching for LLM responses to improve performance and reduce API calls.

use corvus_core::completion::{
    CompletionModel, CompletionRequest, CompletionResponse, StreamingCompletionResponse,
};
use corvus_core::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// A cached response entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// The cached response content
    pub content: String,
    /// When this entry was created
    pub created_at: SystemTime,
    /// When this entry expires
    pub expires_at: Option<SystemTime>,
    /// Hash of the request that generated this response
    pub request_hash: String,
    /// Model used for this response
    pub model: String,
    /// Token usage (if available)
    pub tokens_used: Option<u32>,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(
        content: String,
        request_hash: String,
        model: String,
        ttl: Option<Duration>,
    ) -> Self {
        let created_at = SystemTime::now();
        let expires_at = ttl.map(|ttl| created_at + ttl);

        Self {
            content,
            created_at,
            expires_at,
            request_hash,
            model,
            tokens_used: None,
        }
    }

    /// Check if this entry has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            SystemTime::now() > expires_at
        } else {
            false
        }
    }

    /// Get the age of this entry
    pub fn age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or(Duration::ZERO)
    }
}

/// Configuration for the cache
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in the cache
    pub max_entries: usize,
    /// Default time-to-live for entries
    pub default_ttl: Option<Duration>,
    /// Whether to persist cache to disk
    pub persist: bool,
    /// Path to cache file (if persisting)
    pub cache_path: Option<PathBuf>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            default_ttl: Some(Duration::from_secs(60 * 60 * 24)), // 24 hours
            persist: true,
            cache_path: None,
        }
    }
}

/// Response cache manager
pub struct ResponseCache {
    /// In-memory cache entries
    entries: HashMap<String, CacheEntry>,
    /// Cache configuration
    config: CacheConfig,
    /// Last save time (to throttle saves)
    last_save: Option<std::time::Instant>,
}

impl ResponseCache {
    /// Create a new response cache with default configuration
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Create a new response cache with custom configuration
    pub fn with_config(config: CacheConfig) -> Self {
        let mut cache = Self {
            entries: HashMap::new(),
            config,
            last_save: None,
        };

        // Try to load from disk if persistence is enabled
        if cache.config.persist {
            if let Some(path) = &cache.config.cache_path {
                if let Ok(loaded) = Self::load_from_file(path) {
                    cache.entries = loaded;
                }
            } else if let Ok(path) = Self::default_cache_path() {
                if let Ok(loaded) = Self::load_from_file(&path) {
                    cache.entries = loaded;
                }
            }
        }

        cache
    }

    /// Get the default cache path
    fn default_cache_path() -> anyhow::Result<PathBuf> {
        let dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("corvus");

        std::fs::create_dir_all(&dir)?;
        Ok(dir.join("cache.json"))
    }

    /// Load cache from a file
    fn load_from_file(path: &Path) -> anyhow::Result<HashMap<String, CacheEntry>> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let entries: HashMap<String, CacheEntry> = serde_json::from_str(&content)?;
            // Filter out expired entries
            Ok(entries
                .into_iter()
                .filter(|(_, entry)| !entry.is_expired())
                .collect())
        } else {
            Ok(HashMap::new())
        }
    }

    /// Save cache to disk
    pub fn save(&self) -> anyhow::Result<()> {
        if !self.config.persist {
            return Ok(());
        }

        let path = if let Some(path) = &self.config.cache_path {
            path.clone()
        } else {
            Self::default_cache_path()?
        };

        // Filter out expired entries before saving
        let entries: HashMap<_, _> = self
            .entries
            .iter()
            .filter(|(_, entry)| !entry.is_expired())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let content = serde_json::to_string_pretty(&entries)?;
        std::fs::write(path, content)?;

        Ok(())
    }

    /// Generate a hash for a request
    pub fn hash_request(prompt: &str, model: &str, temperature: f32) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        prompt.hash(&mut hasher);
        model.hash(&mut hasher);
        temperature.to_bits().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Get a cached response
    pub fn get(&self, key: &str) -> Option<&CacheEntry> {
        self.entries.get(key).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(entry)
            }
        })
    }

    /// Get a mutable cached response and clean up expired entries
    pub fn get_mut(&mut self, key: &str) -> Option<&CacheEntry> {
        self.remove_expired_entries();
        self.entries.get(key).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(entry)
            }
        })
    }

    /// Check if a key exists in the cache (and is not expired)
    pub fn contains(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Clean up expired entries (call this periodically)
    pub fn cleanup(&mut self) {
        self.remove_expired_entries();
    }

    /// Put a response into the cache
    pub fn put(&mut self, key: String, entry: CacheEntry) {
        // First, remove any expired entries
        self.remove_expired_entries();

        // If we're still at capacity, remove oldest entries
        if self.entries.len() >= self.config.max_entries {
            // Find and remove the oldest entry
            if let Some(oldest_key) = self.find_oldest_entry() {
                self.entries.remove(&oldest_key);
            }
        }

        self.entries.insert(key, entry);

        // Try to save, but throttle saves to at most once every 5 seconds
        self.throttled_save();
    }

    /// Find the oldest entry key
    fn find_oldest_entry(&self) -> Option<String> {
        self.entries
            .iter()
            .min_by_key(|(_, entry)| entry.created_at)
            .map(|(key, _)| key.clone())
    }

    /// Remove all expired entries
    fn remove_expired_entries(&mut self) {
        let expired_keys: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.is_expired())
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            self.entries.remove(&key);
        }
    }

    /// Save with throttling
    fn throttled_save(&mut self) {
        let now = std::time::Instant::now();
        let should_save = match self.last_save {
            None => true,
            Some(last) => now.duration_since(last) >= Duration::from_secs(5),
        };

        if should_save {
            let _ = self.save();
            self.last_save = Some(now);
        }
    }

    /// Remove an entry from the cache
    pub fn remove(&mut self, key: &str) -> bool {
        let removed = self.entries.remove(key).is_some();
        if removed {
            let _ = self.save();
        }
        removed
    }

    /// Clear all entries from the cache
    pub fn clear(&mut self) {
        self.entries.clear();
        let _ = self.save();
    }

    /// Get statistics about the cache
    pub fn stats(&self) -> CacheStats {
        let total = self.entries.len();
        let expired = self.entries.values().filter(|e| e.is_expired()).count();
        let valid = total - expired;

        CacheStats {
            total_entries: total,
            valid_entries: valid,
            expired_entries: expired,
            oldest_entry: self
                .entries
                .values()
                .min_by_key(|e| e.created_at)
                .map(|e| e.age()),
            newest_entry: self
                .entries
                .values()
                .max_by_key(|e| e.created_at)
                .map(|e| e.age()),
        }
    }
}

impl Default for ResponseCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total number of entries (including expired)
    pub total_entries: usize,
    /// Number of valid (non-expired) entries
    pub valid_entries: usize,
    /// Number of expired entries
    pub expired_entries: usize,
    /// Age of the oldest entry
    pub oldest_entry: Option<Duration>,
    /// Age of the newest entry
    pub newest_entry: Option<Duration>,
}

/// A completion model wrapper that caches responses
pub struct CachedCompletionModel {
    /// The underlying completion model
    inner: Arc<dyn CompletionModel>,
    /// The response cache
    cache: Arc<Mutex<ResponseCache>>,
    /// Whether caching is enabled
    enabled: bool,
}

impl CachedCompletionModel {
    /// Create a new cached completion model
    pub fn new(inner: Arc<dyn CompletionModel>, cache: ResponseCache, enabled: bool) -> Self {
        Self {
            inner,
            cache: Arc::new(Mutex::new(cache)),
            enabled,
        }
    }

    /// Create a new cached completion model with default cache config
    pub fn with_default_cache(inner: Arc<dyn CompletionModel>, enabled: bool) -> Self {
        Self::new(inner, ResponseCache::new(), enabled)
    }

    /// Get a reference to the cache
    pub fn cache(&self) -> Arc<Mutex<ResponseCache>> {
        self.cache.clone()
    }

    /// Check if caching is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable caching
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Generate a cache key from a completion request
    fn cache_key_for_request(request: &CompletionRequest) -> String {
        // Hash the messages, model, temperature, and other relevant parameters
        let messages_str: String = request
            .messages
            .iter()
            .map(|m| {
                let role_str = format!("{:?}", m.role).to_lowercase();
                format!("{}:{}", role_str, m.content)
            })
            .collect::<Vec<_>>()
            .join("|");

        ResponseCache::hash_request(&messages_str, &request.model, request.temperature.unwrap_or(0.7))
    }

    /// Serialize a completion response for caching
    fn serialize_response(response: &CompletionResponse) -> String {
        serde_json::to_string(response).unwrap_or_default()
    }

    /// Deserialize a completion response from cache
    fn deserialize_response(content: &str) -> Option<CompletionResponse> {
        serde_json::from_str(content).ok()
    }
}

#[async_trait::async_trait]
impl CompletionModel for CachedCompletionModel {
    fn model_name(&self) -> &str {
        self.inner.model_name()
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        if !self.enabled {
            return self.inner.complete(request).await;
        }

        let cache_key = Self::cache_key_for_request(&request);

        // Check cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(entry) = cache.get(&cache_key) {
                if let Some(response) = Self::deserialize_response(&entry.content) {
                    tracing::debug!("Cache hit for request");
                    return Ok(response);
                }
            }
        }

        // Cache miss - call the inner model
        tracing::debug!("Cache miss for request");
        let response = self.inner.complete(request).await?;

        // Store in cache
        {
            let mut cache = self.cache.lock().unwrap();
            let entry = CacheEntry::new(
                Self::serialize_response(&response),
                cache_key.clone(),
                self.model_name().to_string(),
                None,
            );
            cache.put(cache_key, entry);
        }

        Ok(response)
    }

    async fn complete_stream(&self, request: CompletionRequest) -> Result<StreamingCompletionResponse> {
        // Don't cache streaming responses for now
        self.inner.complete_stream(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry::new(
            "test".to_string(),
            "hash".to_string(),
            "gpt-4".to_string(),
            Some(Duration::from_millis(10)),
        );

        assert!(!entry.is_expired());

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(20));

        assert!(entry.is_expired());
    }

    #[test]
    fn test_cache_put_and_get() {
        let mut cache = ResponseCache::with_config(CacheConfig {
            persist: false,
            ..CacheConfig::default()
        });

        let entry = CacheEntry::new(
            "test content".to_string(),
            "hash123".to_string(),
            "gpt-4".to_string(),
            None,
        );

        cache.put("test-key".to_string(), entry);

        let retrieved = cache.get("test-key").unwrap();
        assert_eq!(retrieved.content, "test content");
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = ResponseCache::with_config(CacheConfig {
            max_entries: 3,
            persist: false,
            ..CacheConfig::default()
        });

        for i in 0..5 {
            let entry = CacheEntry::new(
                format!("content {}", i),
                format!("hash{}", i),
                "gpt-4".to_string(),
                None,
            );
            cache.put(format!("key{}", i), entry);
        }

        // Should only have 3 entries
        assert_eq!(cache.entries.len(), 3);

        // Oldest entries should be evicted
        assert!(cache.get("key0").is_none());
        assert!(cache.get("key1").is_none());
        assert!(cache.get("key2").is_some());
        assert!(cache.get("key3").is_some());
        assert!(cache.get("key4").is_some());
    }
}
