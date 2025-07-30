use crate::error_handling::{EnhancedError, ErrorDomain, ErrorSeverity, RecoveryStrategy};
use crate::error_handling::service_provider::{ServiceProvider, ServiceHealth, ServicePriority};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Cached data entry with staleness tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedDataEntry<T> {
    /// The cached data
    pub data: T,
    /// Timestamp when data was cached
    pub cached_at: u64, // Unix timestamp in seconds
    /// Time-to-live for the data (seconds)
    pub ttl_seconds: u64,
    /// Data version/etag for consistency
    pub version: String,
    /// Source of the data (e.g., "primary_service", "secondary_service")
    pub source: String,
}

impl<T> CachedDataEntry<T> {
    /// Create a new cached data entry
    pub fn new(data: T, ttl_seconds: u64, source: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            data,
            cached_at: now,
            ttl_seconds,
            version: uuid::Uuid::new_v4().to_string(),
            source,
        }
    }
    
    /// Check if the cached data is still fresh
    pub fn is_fresh(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        (now - self.cached_at) < self.ttl_seconds
    }
    
    /// Get the age of the cached data in seconds
    pub fn age_seconds(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now - self.cached_at
    }
    
    /// Check if data is stale (beyond TTL)
    pub fn is_stale(&self) -> bool {
        !self.is_fresh()
    }
    
    /// Get staleness level (0.0 = fresh, 1.0+ = stale)
    pub fn staleness_factor(&self) -> f64 {
        let age = self.age_seconds() as f64;
        let ttl = self.ttl_seconds as f64;
        age / ttl
    }
}

/// Offline cache manager for storing critical data
pub struct OfflineCacheManager {
    /// In-memory cache storage
    storage: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    /// Cache metadata
    metadata: Arc<Mutex<HashMap<String, CacheMetadata>>>,
    /// Cache configuration
    config: OfflineCacheConfig,
}

/// Cache metadata for tracking entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    /// Key identifier
    pub key: String,
    /// Data size in bytes
    pub size_bytes: usize,
    /// Timestamp when cached
    pub cached_at: u64,
    /// Time-to-live in seconds
    pub ttl_seconds: u64,
    /// Access count
    pub access_count: u64,
    /// Last accessed timestamp
    pub last_accessed: u64,
    /// Data source
    pub source: String,
    /// Data type/category
    pub data_type: String,
}

/// Offline cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineCacheConfig {
    /// Maximum cache size in bytes
    pub max_cache_size_bytes: usize,
    /// Default TTL for cached items (seconds)
    pub default_ttl_seconds: u64,
    /// Maximum number of cached items
    pub max_items: usize,
    /// Enable LRU eviction when cache is full
    pub enable_lru_eviction: bool,
    /// Enable cache compression
    pub enable_compression: bool,
}

impl Default for OfflineCacheConfig {
    fn default() -> Self {
        Self {
            max_cache_size_bytes: 100 * 1024 * 1024, // 100MB
            default_ttl_seconds: 24 * 60 * 60, // 24 hours
            max_items: 10000,
            enable_lru_eviction: true,
            enable_compression: false,
        }
    }
}

impl OfflineCacheManager {
    /// Create a new offline cache manager
    pub fn new(config: OfflineCacheConfig) -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
            metadata: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }
    
    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(OfflineCacheConfig::default())
    }
    
    /// Store data in cache
    pub fn store<T: Serialize>(&self, key: &str, data: &T, ttl_seconds: Option<u64>, source: &str, data_type: &str) -> Result<(), EnhancedError> {
        // Serialize data
        let serialized = serde_json::to_vec(data)
            .map_err(|e| EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::Data,
                RecoveryStrategy::AutoRecoverable,
                format!("Failed to serialize cache data: {}", e),
            ))?;
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let ttl = ttl_seconds.unwrap_or(self.config.default_ttl_seconds);
        
        // Check cache size limits
        if self.would_exceed_limits(&serialized)? {
            self.evict_if_needed(&serialized)?;
        }
        
        // Store data and metadata
        let metadata = CacheMetadata {
            key: key.to_string(),
            size_bytes: serialized.len(),
            cached_at: now,
            ttl_seconds: ttl,
            access_count: 0,
            last_accessed: now,
            source: source.to_string(),
            data_type: data_type.to_string(),
        };
        
        {
            let mut storage = self.storage.lock().unwrap();
            let mut metadata_map = self.metadata.lock().unwrap();
            
            storage.insert(key.to_string(), serialized);
            metadata_map.insert(key.to_string(), metadata);
        }
        
        Ok(())
    }
    
    /// Retrieve data from cache
    pub fn retrieve<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<CachedDataEntry<T>>, EnhancedError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let (data_bytes, mut metadata) = {
            let storage = self.storage.lock().unwrap();
            let mut metadata_map = self.metadata.lock().unwrap();
            
            match (storage.get(key), metadata_map.get_mut(key)) {
                (Some(data), Some(meta)) => {
                    // Update access tracking
                    meta.access_count += 1;
                    meta.last_accessed = now;
                    (data.clone(), meta.clone())
                }
                _ => return Ok(None),
            }
        };
        
        // Check if data is expired
        if (now - metadata.cached_at) > metadata.ttl_seconds {
            // Data is expired, remove it
            self.remove(key)?;
            return Ok(None);
        }
        
        // Deserialize data
        let data: T = serde_json::from_slice(&data_bytes)
            .map_err(|e| EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::Data,
                RecoveryStrategy::AutoRecoverable,
                format!("Failed to deserialize cache data: {}", e),
            ))?;
        
        Ok(Some(CachedDataEntry {
            data,
            cached_at: metadata.cached_at,
            ttl_seconds: metadata.ttl_seconds,
            version: uuid::Uuid::new_v4().to_string(), // Generate new version for this retrieval
            source: metadata.source,
        }))
    }
    
    /// Remove data from cache
    pub fn remove(&self, key: &str) -> Result<bool, EnhancedError> {
        let mut storage = self.storage.lock().unwrap();
        let mut metadata_map = self.metadata.lock().unwrap();
        
        let removed_storage = storage.remove(key).is_some();
        let removed_metadata = metadata_map.remove(key).is_some();
        
        Ok(removed_storage && removed_metadata)
    }
    
    /// Clear all cache entries
    pub fn clear(&self) -> Result<(), EnhancedError> {
        let mut storage = self.storage.lock().unwrap();
        let mut metadata_map = self.metadata.lock().unwrap();
        
        storage.clear();
        metadata_map.clear();
        
        Ok(())
    }
    
    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let storage = self.storage.lock().unwrap();
        let metadata_map = self.metadata.lock().unwrap();
        
        let total_size_bytes: usize = metadata_map.values()
            .map(|m| m.size_bytes)
            .sum();
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let expired_count = metadata_map.values()
            .filter(|m| (now - m.cached_at) > m.ttl_seconds)
            .count();
        
        let total_accesses: u64 = metadata_map.values()
            .map(|m| m.access_count)
            .sum();
        
        CacheStats {
            total_items: storage.len(),
            total_size_bytes,
            expired_items: expired_count,
            hit_count: total_accesses,
            max_size_bytes: self.config.max_cache_size_bytes,
            utilization_percent: (total_size_bytes as f64 / self.config.max_cache_size_bytes as f64 * 100.0).min(100.0),
        }
    }
    
    /// Clean up expired entries
    pub fn cleanup_expired(&self) -> Result<usize, EnhancedError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut storage = self.storage.lock().unwrap();
        let mut metadata_map = self.metadata.lock().unwrap();
        
        let expired_keys: Vec<String> = metadata_map.iter()
            .filter(|(_, meta)| (now - meta.cached_at) > meta.ttl_seconds)
            .map(|(key, _)| key.clone())
            .collect();
        
        for key in &expired_keys {
            storage.remove(key);
            metadata_map.remove(key);
        }
        
        Ok(expired_keys.len())
    }
    
    /// Check if adding data would exceed limits
    fn would_exceed_limits(&self, new_data: &[u8]) -> Result<bool, EnhancedError> {
        let storage = self.storage.lock().unwrap();
        let metadata_map = self.metadata.lock().unwrap();
        
        let current_size: usize = metadata_map.values()
            .map(|m| m.size_bytes)
            .sum();
        
        let would_exceed_size = (current_size + new_data.len()) > self.config.max_cache_size_bytes;
        let would_exceed_count = storage.len() >= self.config.max_items;
        
        Ok(would_exceed_size || would_exceed_count)
    }
    
    /// Evict items if needed using LRU strategy
    fn evict_if_needed(&self, new_data: &[u8]) -> Result<(), EnhancedError> {
        if !self.config.enable_lru_eviction {
            return Err(EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::Data,
                RecoveryStrategy::UserRecoverable,
                "Cache is full and eviction is disabled".to_string(),
            ));
        }
        
        // Sort by last accessed time (LRU)
        let mut items_by_access: Vec<(String, CacheMetadata)> = {
            let metadata_map = self.metadata.lock().unwrap();
            metadata_map.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        };
        
        items_by_access.sort_by_key(|(_, meta)| meta.last_accessed);
        
        // Evict items until we have enough space
        let target_size = self.config.max_cache_size_bytes - new_data.len();
        let mut current_size = {
            let metadata_map = self.metadata.lock().unwrap();
            metadata_map.values().map(|m| m.size_bytes).sum::<usize>()
        };
        
        for (key, _) in items_by_access {
            if current_size <= target_size {
                break;
            }
            
            // Remove this item
            let removed_size = {
                let mut storage = self.storage.lock().unwrap();
                let mut metadata_map = self.metadata.lock().unwrap();
                
                let size = metadata_map.get(&key).map(|m| m.size_bytes).unwrap_or(0);
                storage.remove(&key);
                metadata_map.remove(&key);
                size
            };
            
            current_size -= removed_size;
        }
        
        Ok(())
    }
}

/// Cache usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total number of cached items
    pub total_items: usize,
    /// Total cache size in bytes
    pub total_size_bytes: usize,
    /// Number of expired items
    pub expired_items: usize,
    /// Total cache hits
    pub hit_count: u64,
    /// Maximum cache size
    pub max_size_bytes: usize,
    /// Cache utilization percentage
    pub utilization_percent: f64,
}

/// Offline asset management service provider
pub struct OfflineAssetProvider {
    /// Cache manager for asset data
    cache: Arc<OfflineCacheManager>,
    /// Service name
    name: String,
}

impl OfflineAssetProvider {
    /// Create a new offline asset provider
    pub fn new(cache: Arc<OfflineCacheManager>) -> Self {
        Self {
            cache,
            name: "offline_assets".to_string(),
        }
    }
    
    /// Store asset data in cache
    pub fn cache_asset_data<T: Serialize>(&self, asset_id: &str, data: &T) -> Result<(), EnhancedError> {
        let key = format!("asset:{}", asset_id);
        self.cache.store(&key, data, Some(7 * 24 * 60 * 60), "offline_provider", "asset")?; // 7 days TTL
        Ok(())
    }
    
    /// Retrieve asset data from cache
    pub fn get_cached_asset_data<T: for<'de> Deserialize<'de>>(&self, asset_id: &str) -> Result<Option<CachedDataEntry<T>>, EnhancedError> {
        let key = format!("asset:{}", asset_id);
        self.cache.retrieve(&key)
    }
}

#[async_trait::async_trait]
impl ServiceProvider for OfflineAssetProvider {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn priority(&self) -> ServicePriority {
        ServicePriority::Offline
    }
    
    async fn is_available(&self) -> bool {
        // Offline service is always available
        true
    }
    
    async fn health_check(&self) -> ServiceHealth {
        // Check cache health
        let stats = self.cache.get_stats();
        
        if stats.utilization_percent > 95.0 {
            ServiceHealth::Degraded
        } else {
            ServiceHealth::Healthy
        }
    }
    
    async fn warm_up(&self) -> Result<(), EnhancedError> {
        // Clean up expired entries
        self.cache.cleanup_expired()?;
        Ok(())
    }
    
    fn get_configuration(&self) -> HashMap<String, String> {
        let stats = self.cache.get_stats();
        let mut config = HashMap::new();
        config.insert("type".to_string(), "offline_cache".to_string());
        config.insert("total_items".to_string(), stats.total_items.to_string());
        config.insert("utilization_percent".to_string(), format!("{:.1}", stats.utilization_percent));
        config
    }
}

/// Offline configuration service provider
pub struct OfflineConfigProvider {
    /// Cache manager for configuration data
    cache: Arc<OfflineCacheManager>,
    /// Service name
    name: String,
}

impl OfflineConfigProvider {
    /// Create a new offline configuration provider
    pub fn new(cache: Arc<OfflineCacheManager>) -> Self {
        Self {
            cache,
            name: "offline_configs".to_string(),
        }
    }
    
    /// Store configuration data in cache
    pub fn cache_config_data<T: Serialize>(&self, config_id: &str, data: &T) -> Result<(), EnhancedError> {
        let key = format!("config:{}", config_id);
        self.cache.store(&key, data, Some(30 * 24 * 60 * 60), "offline_provider", "configuration")?; // 30 days TTL
        Ok(())
    }
    
    /// Retrieve configuration data from cache
    pub fn get_cached_config_data<T: for<'de> Deserialize<'de>>(&self, config_id: &str) -> Result<Option<CachedDataEntry<T>>, EnhancedError> {
        let key = format!("config:{}", config_id);
        self.cache.retrieve(&key)
    }
}

#[async_trait::async_trait]
impl ServiceProvider for OfflineConfigProvider {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn priority(&self) -> ServicePriority {
        ServicePriority::Offline
    }
    
    async fn is_available(&self) -> bool {
        true
    }
    
    async fn health_check(&self) -> ServiceHealth {
        let stats = self.cache.get_stats();
        
        if stats.utilization_percent > 90.0 {
            ServiceHealth::Degraded
        } else {
            ServiceHealth::Healthy
        }
    }
    
    async fn warm_up(&self) -> Result<(), EnhancedError> {
        self.cache.cleanup_expired()?;
        Ok(())
    }
}

/// Offline audit service provider (read-only for safety)
pub struct OfflineAuditProvider {
    /// Cache manager for audit data
    cache: Arc<OfflineCacheManager>,
    /// Service name
    name: String,
}

impl OfflineAuditProvider {
    /// Create a new offline audit provider
    pub fn new(cache: Arc<OfflineCacheManager>) -> Self {
        Self {
            cache,
            name: "offline_audit".to_string(),
        }
    }
    
    /// Store audit data in cache (for read-only access)
    pub fn cache_audit_data<T: Serialize>(&self, audit_id: &str, data: &T) -> Result<(), EnhancedError> {
        let key = format!("audit:{}", audit_id);
        self.cache.store(&key, data, Some(90 * 24 * 60 * 60), "offline_provider", "audit")?; // 90 days TTL
        Ok(())
    }
    
    /// Retrieve audit data from cache
    pub fn get_cached_audit_data<T: for<'de> Deserialize<'de>>(&self, audit_id: &str) -> Result<Option<CachedDataEntry<T>>, EnhancedError> {
        let key = format!("audit:{}", audit_id);
        self.cache.retrieve(&key)
    }
}

#[async_trait::async_trait]
impl ServiceProvider for OfflineAuditProvider {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn priority(&self) -> ServicePriority {
        ServicePriority::Offline
    }
    
    async fn is_available(&self) -> bool {
        true
    }
    
    async fn health_check(&self) -> ServiceHealth {
        // Audit service is read-only in offline mode, always healthy if cache is working
        ServiceHealth::Healthy
    }
    
    async fn warm_up(&self) -> Result<(), EnhancedError> {
        self.cache.cleanup_expired()?;
        Ok(())
    }
    
    fn get_configuration(&self) -> HashMap<String, String> {
        let mut config = HashMap::new();
        config.insert("type".to_string(), "offline_audit".to_string());
        config.insert("mode".to_string(), "read_only".to_string());
        config.insert("safety_level".to_string(), "high".to_string());
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        id: u32,
        name: String,
        value: f64,
    }
    
    #[test]
    fn test_cached_data_entry_freshness() {
        let data = TestData { id: 1, name: "test".to_string(), value: 42.0 };
        let entry = CachedDataEntry::new(data, 60, "test_source".to_string()); // 60 seconds TTL
        
        assert!(entry.is_fresh());
        assert!(!entry.is_stale());
        assert!(entry.staleness_factor() < 1.0);
        assert_eq!(entry.source, "test_source");
    }
    
    #[test]
    fn test_offline_cache_manager_store_retrieve() {
        let cache = OfflineCacheManager::default();
        let data = TestData { id: 1, name: "test".to_string(), value: 42.0 };
        
        // Store data
        let result = cache.store("test_key", &data, Some(3600), "test_source", "test_data");
        assert!(result.is_ok());
        
        // Retrieve data
        let retrieved: Result<Option<CachedDataEntry<TestData>>, _> = cache.retrieve("test_key");
        assert!(retrieved.is_ok());
        
        let cached_data = retrieved.unwrap();
        assert!(cached_data.is_some());
        
        let entry = cached_data.unwrap();
        assert_eq!(entry.data, data);
        assert_eq!(entry.source, "test_source");
        assert!(entry.is_fresh());
    }
    
    #[test]
    fn test_offline_cache_manager_expiration() {
        let cache = OfflineCacheManager::default();
        let data = TestData { id: 1, name: "test".to_string(), value: 42.0 };
        
        // Store data with very short TTL
        let result = cache.store("test_key", &data, Some(0), "test_source", "test_data"); // Immediate expiration
        assert!(result.is_ok());
        
        // Should not retrieve expired data
        std::thread::sleep(std::time::Duration::from_millis(10));
        let retrieved: Result<Option<CachedDataEntry<TestData>>, _> = cache.retrieve("test_key");
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_none());
    }
    
    #[test]
    fn test_cache_stats() {
        let cache = OfflineCacheManager::default();
        let data = TestData { id: 1, name: "test".to_string(), value: 42.0 };
        
        // Initially empty
        let stats = cache.get_stats();
        assert_eq!(stats.total_items, 0);
        assert_eq!(stats.total_size_bytes, 0);
        
        // Store some data
        let _ = cache.store("test1", &data, Some(3600), "source1", "test");
        let _ = cache.store("test2", &data, Some(3600), "source2", "test");
        
        let stats = cache.get_stats();
        assert_eq!(stats.total_items, 2);
        assert!(stats.total_size_bytes > 0);
    }
    
    #[tokio::test]
    async fn test_offline_asset_provider() {
        let cache = Arc::new(OfflineCacheManager::default());
        let provider = OfflineAssetProvider::new(cache);
        
        // Service should always be available
        assert!(provider.is_available().await);
        assert_eq!(provider.priority(), ServicePriority::Offline);
        assert_eq!(provider.name(), "offline_assets");
        
        // Health check should pass
        let health = provider.health_check().await;
        assert!(health == ServiceHealth::Healthy || health == ServiceHealth::Degraded);
        
        // Warm up should succeed
        let warmup_result = provider.warm_up().await;
        assert!(warmup_result.is_ok());
    }
    
    #[test]
    fn test_cache_data_operations() {
        let cache = Arc::new(OfflineCacheManager::default());
        let provider = OfflineAssetProvider::new(cache);
        
        let asset_data = TestData { id: 123, name: "test_asset".to_string(), value: 99.9 };
        
        // Cache asset data
        let cache_result = provider.cache_asset_data("asset_123", &asset_data);
        assert!(cache_result.is_ok());
        
        // Retrieve cached data
        let retrieve_result: Result<Option<CachedDataEntry<TestData>>, _> = 
            provider.get_cached_asset_data("asset_123");
        assert!(retrieve_result.is_ok());
        
        let cached = retrieve_result.unwrap();
        assert!(cached.is_some());
        
        let entry = cached.unwrap();
        assert_eq!(entry.data, asset_data);
        assert_eq!(entry.source, "offline_provider");
    }
}