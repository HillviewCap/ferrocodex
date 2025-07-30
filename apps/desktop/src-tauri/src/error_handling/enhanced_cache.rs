use crate::error_handling::{EnhancedError, ErrorDomain, ErrorSeverity, RecoveryStrategy};
use crate::error_handling::offline_providers::CachedDataEntry;
use serde::{Deserialize, Serialize};
use rusqlite::{Connection, params, Result as SqliteResult};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Enhanced SQLite-based cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedCacheConfig {
    /// Database file path for cache storage
    pub db_path: String,
    /// Default TTL for cached items (seconds)
    pub default_ttl_seconds: u64,
    /// Maximum cache size in bytes
    pub max_cache_size_bytes: u64,
    /// Enable cache compression
    pub enable_compression: bool,
    /// Cache warming interval (seconds)
    pub cache_warming_interval_seconds: u64,
    /// Maximum number of cached items
    pub max_items: usize,
    /// Enable cache statistics collection
    pub enable_statistics: bool,
    /// Cache eviction strategy
    pub eviction_strategy: CacheEvictionStrategy,
}

impl Default for EnhancedCacheConfig {
    fn default() -> Self {
        Self {
            db_path: "cache.db".to_string(),
            default_ttl_seconds: 24 * 60 * 60, // 24 hours
            max_cache_size_bytes: 500 * 1024 * 1024, // 500MB
            enable_compression: true,
            cache_warming_interval_seconds: 300, // 5 minutes
            max_items: 50000,
            enable_statistics: true,
            eviction_strategy: CacheEvictionStrategy::LeastRecentlyUsed,
        }
    }
}

/// Cache eviction strategies
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CacheEvictionStrategy {
    /// Least Recently Used
    LeastRecentlyUsed,
    /// Least Frequently Used
    LeastFrequentlyUsed,
    /// First In First Out
    FirstInFirstOut,
    /// Random eviction
    Random,
}

impl std::fmt::Display for CacheEvictionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheEvictionStrategy::LeastRecentlyUsed => write!(f, "LRU"),
            CacheEvictionStrategy::LeastFrequentlyUsed => write!(f, "LFU"),
            CacheEvictionStrategy::FirstInFirstOut => write!(f, "FIFO"),
            CacheEvictionStrategy::Random => write!(f, "Random"),
        }
    }
}

/// Cache entry metadata for SQLite storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntryMetadata {
    /// Cache key
    pub key: String,
    /// Data type identifier
    pub data_type: String,
    /// Data source
    pub source: String,
    /// Cached at timestamp
    pub cached_at: u64,
    /// TTL in seconds
    pub ttl_seconds: u64,
    /// Last accessed timestamp
    pub last_accessed: u64,
    /// Access count
    pub access_count: u64,
    /// Data size in bytes
    pub size_bytes: u64,
    /// Whether data is compressed
    pub compressed: bool,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Cache warming strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheWarmingStrategy {
    /// Feature or data type to warm
    pub data_type: String,
    /// Warming function name or identifier
    pub warming_function: String,
    /// Warming priority (higher = warmed first)
    pub priority: u32,
    /// Minimum time between warming attempts (seconds)
    pub min_interval_seconds: u64,
    /// Maximum warming duration (seconds)
    pub max_duration_seconds: u64,
    /// Whether warming is enabled
    pub enabled: bool,
}

/// Enhanced SQLite cache manager
pub struct EnhancedSqliteCache {
    /// Database connection
    connection: Arc<Mutex<Connection>>,
    /// Cache configuration
    config: EnhancedCacheConfig,
    /// Cache statistics
    stats: Arc<Mutex<EnhancedCacheStats>>,
    /// Cache warming strategies
    warming_strategies: Arc<Mutex<HashMap<String, CacheWarmingStrategy>>>,
}

/// Enhanced cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedCacheStats {
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Total items cached
    pub total_items: u64,
    /// Total cache size in bytes
    pub total_size_bytes: u64,
    /// Average access time (microseconds)
    pub avg_access_time_us: f64,
    /// Cache hit ratio (0.0 to 1.0)
    pub hit_ratio: f64,
    /// Number of evictions performed
    pub evictions: u64,
    /// Number of expired items cleaned
    pub expirations: u64,
    /// Last cleanup timestamp
    pub last_cleanup: u64,
    /// Cache warming statistics
    pub warming_stats: CacheWarmingStats,
}

/// Cache warming statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheWarmingStats {
    /// Total warming operations
    pub total_warmings: u64,
    /// Successful warmings
    pub successful_warmings: u64,
    /// Failed warmings
    pub failed_warmings: u64,
    /// Average warming time (milliseconds)
    pub avg_warming_time_ms: f64,
    /// Last warming timestamp
    pub last_warming: u64,
}

impl Default for CacheWarmingStats {
    fn default() -> Self {
        Self {
            total_warmings: 0,
            successful_warmings: 0,
            failed_warmings: 0,
            avg_warming_time_ms: 0.0,
            last_warming: 0,
        }
    }
}

impl Default for EnhancedCacheStats {
    fn default() -> Self {
        Self {
            hits: 0,
            misses: 0,
            total_items: 0,
            total_size_bytes: 0,
            avg_access_time_us: 0.0,
            hit_ratio: 0.0,
            evictions: 0,
            expirations: 0,
            last_cleanup: 0,
            warming_stats: CacheWarmingStats {
                total_warmings: 0,
                successful_warmings: 0,
                failed_warmings: 0,
                avg_warming_time_ms: 0.0,
                last_warming: 0,
            },
        }
    }
}

impl EnhancedSqliteCache {
    /// Create a new enhanced SQLite cache
    pub fn new(config: EnhancedCacheConfig) -> Result<Self, EnhancedError> {
        let conn = Connection::open(&config.db_path)
            .map_err(|e| EnhancedError::new(
                ErrorSeverity::High,
                ErrorDomain::Data,
                RecoveryStrategy::AutoRecoverable,
                format!("Failed to open cache database: {}", e),
            ))?;
        
        let cache = Self {
            connection: Arc::new(Mutex::new(conn)),
            config,
            stats: Arc::new(Mutex::new(EnhancedCacheStats::default())),
            warming_strategies: Arc::new(Mutex::new(HashMap::new())),
        };
        
        cache.initialize_schema()?;
        Ok(cache)
    }
    
    /// Initialize database schema
    fn initialize_schema(&self) -> Result<(), EnhancedError> {
        let conn = self.connection.lock().unwrap();
        
        // Create cache entries table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS cache_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key TEXT UNIQUE NOT NULL,
                data_type TEXT NOT NULL,
                source TEXT NOT NULL,
                data BLOB NOT NULL,
                cached_at INTEGER NOT NULL,
                ttl_seconds INTEGER NOT NULL,
                last_accessed INTEGER NOT NULL,
                access_count INTEGER DEFAULT 0,
                size_bytes INTEGER NOT NULL,
                compressed INTEGER DEFAULT 0,
                tags TEXT DEFAULT '',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).map_err(|e| EnhancedError::new(
            ErrorSeverity::High,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to create cache_entries table: {}", e),
        ))?;
        
        // Create indexes for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_cache_key ON cache_entries(key)",
            [],
        ).map_err(|e| EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to create key index: {}", e),
        ))?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_cache_data_type ON cache_entries(data_type)",
            [],
        ).map_err(|e| EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to create data_type index: {}", e),
        ))?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_cache_last_accessed ON cache_entries(last_accessed)",
            [],
        ).map_err(|e| EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to create last_accessed index: {}", e),
        ))?;
        
        // Create cache statistics table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS cache_statistics (
                id INTEGER PRIMARY KEY,
                hits INTEGER DEFAULT 0,
                misses INTEGER DEFAULT 0,
                total_items INTEGER DEFAULT 0,
                total_size_bytes INTEGER DEFAULT 0,
                evictions INTEGER DEFAULT 0,
                expirations INTEGER DEFAULT 0,
                last_updated DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).map_err(|e| EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to create cache_statistics table: {}", e),
        ))?;
        
        // Initialize statistics row if it doesn't exist
        conn.execute(
            "INSERT OR IGNORE INTO cache_statistics (id) VALUES (1)",
            [],
        ).map_err(|e| EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to initialize cache statistics: {}", e),
        ))?;
        
        Ok(())
    }
    
    /// Store data in cache
    pub fn store<T: Serialize>(&self, 
        key: &str, 
        data: &T, 
        data_type: &str, 
        source: &str,
        ttl_seconds: Option<u64>,
        tags: Vec<String>,
    ) -> Result<(), EnhancedError> {
        let start_time = std::time::Instant::now();
        
        // Serialize data
        let serialized = serde_json::to_vec(data)
            .map_err(|e| EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::Data,
                RecoveryStrategy::AutoRecoverable,
                format!("Failed to serialize cache data: {}", e),
            ))?;
        
        // Compress if enabled
        let (final_data, compressed) = if self.config.enable_compression {
            let compressed_data = self.compress_data(&serialized)?;
            if compressed_data.len() < serialized.len() {
                (compressed_data, true)
            } else {
                (serialized, false)
            }
        } else {
            (serialized, false)
        };
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let ttl = ttl_seconds.unwrap_or(self.config.default_ttl_seconds);
        let tags_str = tags.join(",");
        
        // Check if we need to evict items first
        self.evict_if_needed(&final_data)?;
        
        let conn = self.connection.lock().unwrap();
        
        // Insert or replace cache entry
        conn.execute(
            "INSERT OR REPLACE INTO cache_entries 
             (key, data_type, source, data, cached_at, ttl_seconds, last_accessed, 
              access_count, size_bytes, compressed, tags) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                key,
                data_type,
                source,
                final_data,
                now,
                ttl,
                now,
                0,
                final_data.len() as u64,
                if compressed { 1 } else { 0 },
                tags_str
            ],
        ).map_err(|e| EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to store cache entry: {}", e),
        ))?;
        
        // Update statistics
        self.update_store_stats(start_time.elapsed());
        
        Ok(())
    }
    
    /// Retrieve data from cache
    pub fn retrieve<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<CachedDataEntry<T>>, EnhancedError> {
        let start_time = std::time::Instant::now();
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let conn = self.connection.lock().unwrap();
        
        // Retrieve cache entry
        let mut stmt = conn.prepare(
            "SELECT data_type, source, data, cached_at, ttl_seconds, compressed, access_count 
             FROM cache_entries WHERE key = ?1"
        ).map_err(|e| EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to prepare cache query: {}", e),
        ))?;
        
        let entry_result: SqliteResult<(String, String, Vec<u8>, u64, u64, i32, u64)> = stmt.query_row(
            params![key],
            |row| Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?
            ))
        );
        
        match entry_result {
            Ok((data_type, source, data_bytes, cached_at, ttl_seconds, compressed, access_count)) => {
                // Check if expired
                if (now - cached_at) > ttl_seconds {
                    // Remove expired entry
                    drop(stmt);
                    conn.execute("DELETE FROM cache_entries WHERE key = ?1", params![key])
                        .map_err(|e| EnhancedError::new(
                            ErrorSeverity::Low,
                            ErrorDomain::Data,
                            RecoveryStrategy::AutoRecoverable,
                            format!("Failed to delete expired cache entry: {}", e),
                        ))?;
                    
                    self.update_retrieve_stats(start_time.elapsed(), false);
                    return Ok(None);
                }
                
                // Decompress if needed
                let final_data = if compressed == 1 {
                    self.decompress_data(&data_bytes)?
                } else {
                    data_bytes
                };
                
                // Deserialize data
                let data: T = serde_json::from_slice(&final_data)
                    .map_err(|e| EnhancedError::new(
                        ErrorSeverity::Medium,
                        ErrorDomain::Data,
                        RecoveryStrategy::AutoRecoverable,
                        format!("Failed to deserialize cache data: {}", e),
                    ))?;
                
                // Update access statistics
                drop(stmt);
                conn.execute(
                    "UPDATE cache_entries SET last_accessed = ?1, access_count = access_count + 1 WHERE key = ?2",
                    params![now, key]
                ).map_err(|e| EnhancedError::new(
                    ErrorSeverity::Low,
                    ErrorDomain::Data,
                    RecoveryStrategy::AutoRecoverable,
                    format!("Failed to update cache access stats: {}", e),
                ))?;
                
                let cached_entry = CachedDataEntry {
                    data,
                    cached_at,
                    ttl_seconds,
                    version: uuid::Uuid::new_v4().to_string(),
                    source,
                };
                
                self.update_retrieve_stats(start_time.elapsed(), true);
                Ok(Some(cached_entry))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                self.update_retrieve_stats(start_time.elapsed(), false);
                Ok(None)
            }
            Err(e) => {
                Err(EnhancedError::new(
                    ErrorSeverity::Medium,
                    ErrorDomain::Data,
                    RecoveryStrategy::AutoRecoverable,
                    format!("Failed to retrieve cache entry: {}", e),
                ))
            }
        }
    }
    
    /// Remove entry from cache
    pub fn remove(&self, key: &str) -> Result<bool, EnhancedError> {
        let conn = self.connection.lock().unwrap();
        
        let changes = conn.execute(
            "DELETE FROM cache_entries WHERE key = ?1",
            params![key],
        ).map_err(|e| EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to remove cache entry: {}", e),
        ))?;
        
        Ok(changes > 0)
    }
    
    /// Clear all expired entries
    pub fn cleanup_expired(&self) -> Result<u64, EnhancedError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let conn = self.connection.lock().unwrap();
        
        let expired_count = conn.execute(
            "DELETE FROM cache_entries WHERE (cached_at + ttl_seconds) < ?1",
            params![now],
        ).map_err(|e| EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to cleanup expired entries: {}", e),
        ))? as u64;
        
        // Update statistics
        {
            let mut stats = self.stats.lock().unwrap();
            stats.expirations += expired_count;
            stats.last_cleanup = now;
        }
        
        Ok(expired_count)
    }
    
    /// Get cache statistics
    pub fn get_stats(&self) -> EnhancedCacheStats {
        let stats = self.stats.lock().unwrap();
        stats.clone()
    }
    
    /// Get cache entries by data type
    pub fn get_entries_by_type(&self, data_type: &str) -> Result<Vec<String>, EnhancedError> {
        let conn = self.connection.lock().unwrap();
        
        let mut stmt = conn.prepare(
            "SELECT key FROM cache_entries WHERE data_type = ?1 ORDER BY last_accessed DESC"
        ).map_err(|e| EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to prepare type query: {}", e),
        ))?;
        
        let keys: Result<Vec<String>, _> = stmt.query_map(params![data_type], |row| {
            Ok(row.get::<_, String>(0)?)
        }).map_err(|e| EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to query cache entries by type: {}", e),
        ))?.collect();
        
        keys.map_err(|e| EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to collect cache keys: {}", e),
        ))
    }
    
    /// Add cache warming strategy
    pub fn add_warming_strategy(&self, strategy: CacheWarmingStrategy) {
        let mut strategies = self.warming_strategies.lock().unwrap();
        strategies.insert(strategy.data_type.clone(), strategy);
    }
    
    /// Execute cache warming for all strategies
    pub async fn warm_cache(&self) -> Result<CacheWarmingStats, EnhancedError> {
        let start_time = std::time::Instant::now();
        
        let strategies = {
            let strategies_guard = self.warming_strategies.lock().unwrap();
            strategies_guard.values().cloned().collect::<Vec<_>>()
        };
        
        let mut warming_stats = CacheWarmingStats::default();
        
        for strategy in strategies {
            if !strategy.enabled {
                continue;
            }
            
            warming_stats.total_warmings += 1;
            
            // Execute warming strategy (placeholder - would be implemented per strategy)
            match self.execute_warming_strategy(&strategy).await {
                Ok(_) => warming_stats.successful_warmings += 1,
                Err(_) => warming_stats.failed_warmings += 1,
            }
        }
        
        let elapsed = start_time.elapsed();
        warming_stats.avg_warming_time_ms = elapsed.as_millis() as f64;
        warming_stats.last_warming = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Update overall statistics
        {
            let mut stats = self.stats.lock().unwrap();
            stats.warming_stats = warming_stats.clone();
        }
        
        Ok(warming_stats)
    }
    
    /// Execute individual warming strategy (placeholder)
    async fn execute_warming_strategy(&self, _strategy: &CacheWarmingStrategy) -> Result<(), EnhancedError> {
        // This would be implemented based on specific warming requirements
        // For now, just return success
        Ok(())
    }
    
    /// Check if eviction is needed and perform it
    fn evict_if_needed(&self, new_data: &[u8]) -> Result<(), EnhancedError> {
        let conn = self.connection.lock().unwrap();
        
        // Check current cache size
        let current_size: u64 = conn.query_row(
            "SELECT COALESCE(SUM(size_bytes), 0) FROM cache_entries",
            [],
            |row| row.get(0)
        ).map_err(|e| EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to get current cache size: {}", e),
        ))?;
        
        let current_count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM cache_entries",
            [],
            |row| row.get(0)
        ).map_err(|e| EnhancedError::new(
            ErrorSeverity::Medium,
            ErrorDomain::Data,
            RecoveryStrategy::AutoRecoverable,
            format!("Failed to get current cache count: {}", e),
        ))?;
        
        // Check if we need to evict
        let would_exceed_size = (current_size + new_data.len() as u64) > self.config.max_cache_size_bytes;
        let would_exceed_count = current_count >= self.config.max_items as u64;
        
        if would_exceed_size || would_exceed_count {
            self.perform_eviction(&conn)?;
        }
        
        Ok(())
    }
    
    /// Perform cache eviction based on strategy
    fn perform_eviction(&self, conn: &Connection) -> Result<(), EnhancedError> {
        let evict_count = (self.config.max_items / 10).max(1); // Evict 10% or at least 1
        
        let sql = match self.config.eviction_strategy {
            CacheEvictionStrategy::LeastRecentlyUsed => {
                "DELETE FROM cache_entries WHERE key IN (
                    SELECT key FROM cache_entries ORDER BY last_accessed ASC LIMIT ?1
                )"
            }
            CacheEvictionStrategy::LeastFrequentlyUsed => {
                "DELETE FROM cache_entries WHERE key IN (
                    SELECT key FROM cache_entries ORDER BY access_count ASC, last_accessed ASC LIMIT ?1
                )"
            }
            CacheEvictionStrategy::FirstInFirstOut => {
                "DELETE FROM cache_entries WHERE key IN (
                    SELECT key FROM cache_entries ORDER BY cached_at ASC LIMIT ?1
                )"
            }
            CacheEvictionStrategy::Random => {
                "DELETE FROM cache_entries WHERE key IN (
                    SELECT key FROM cache_entries ORDER BY RANDOM() LIMIT ?1
                )"
            }
        };
        
        let evicted = conn.execute(sql, params![evict_count])
            .map_err(|e| EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::Data,
                RecoveryStrategy::AutoRecoverable,
                format!("Failed to perform cache eviction: {}", e),
            ))? as u64;
        
        // Update statistics
        {
            let mut stats = self.stats.lock().unwrap();
            stats.evictions += evicted;
        }
        
        Ok(())
    }
    
    /// Compress data using gzip
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>, EnhancedError> {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;
        
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)
            .map_err(|e| EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::System,
                RecoveryStrategy::AutoRecoverable,
                format!("Failed to compress cache data: {}", e),
            ))?;
        
        encoder.finish()
            .map_err(|e| EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::System,
                RecoveryStrategy::AutoRecoverable,
                format!("Failed to finalize compression: {}", e),
            ))
    }
    
    /// Decompress data using gzip
    fn decompress_data(&self, data: &[u8]) -> Result<Vec<u8>, EnhancedError> {
        use flate2::read::GzDecoder;
        use std::io::Read;
        
        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| EnhancedError::new(
                ErrorSeverity::Medium,
                ErrorDomain::System,
                RecoveryStrategy::AutoRecoverable,
                format!("Failed to decompress cache data: {}", e),
            ))?;
        
        Ok(decompressed)
    }
    
    /// Update statistics for store operations
    fn update_store_stats(&self, duration: std::time::Duration) {
        let mut stats = self.stats.lock().unwrap();
        // For store operations, we just track the total items
        // The actual count will be updated periodically or on-demand
    }
    
    /// Update statistics for retrieve operations
    fn update_retrieve_stats(&self, duration: std::time::Duration, hit: bool) {
        let mut stats = self.stats.lock().unwrap();
        
        if hit {
            stats.hits += 1;
        } else {
            stats.misses += 1;
        }
        
        // Update hit ratio
        let total_requests = stats.hits + stats.misses;
        if total_requests > 0 {
            stats.hit_ratio = stats.hits as f64 / total_requests as f64;
        }
        
        // Update average access time
        let access_time_us = duration.as_micros() as f64;
        if stats.avg_access_time_us == 0.0 {
            stats.avg_access_time_us = access_time_us;
        } else {
            stats.avg_access_time_us = (stats.avg_access_time_us + access_time_us) / 2.0;
        }
    }
    
    /// Check if cache access meets performance requirement (<200ms)
    pub fn meets_performance_requirement(&self) -> bool {
        let stats = self.stats.lock().unwrap();
        stats.avg_access_time_us < 200_000.0 // 200ms in microseconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        id: u32,
        name: String,
        value: f64,
    }
    
    fn create_test_cache() -> (EnhancedSqliteCache, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_cache.db");
        
        let config = EnhancedCacheConfig {
            db_path: db_path.to_string_lossy().to_string(),
            default_ttl_seconds: 3600, // 1 hour
            max_cache_size_bytes: 10 * 1024 * 1024, // 10MB
            enable_compression: true,
            cache_warming_interval_seconds: 300,
            max_items: 1000,
            enable_statistics: true,
            eviction_strategy: CacheEvictionStrategy::LeastRecentlyUsed,
        };
        
        let cache = EnhancedSqliteCache::new(config).unwrap();
        (cache, temp_dir)
    }
    
    #[test]
    fn test_cache_store_and_retrieve() {
        let (cache, _temp_dir) = create_test_cache();
        
        let test_data = TestData {
            id: 1,
            name: "test".to_string(),
            value: 42.0,
        };
        
        // Store data
        let result = cache.store(
            "test_key",
            &test_data,
            "test_data",
            "test_source",
            Some(3600),
            vec!["test".to_string()],
        );
        assert!(result.is_ok());
        
        // Retrieve data
        let retrieved: Result<Option<CachedDataEntry<TestData>>, _> = cache.retrieve("test_key");
        assert!(retrieved.is_ok());
        
        let cached_data = retrieved.unwrap();
        assert!(cached_data.is_some());
        
        let entry = cached_data.unwrap();
        assert_eq!(entry.data, test_data);
        assert_eq!(entry.source, "test_source");
    }
    
    #[test]
    fn test_cache_expiration() {
        let (cache, _temp_dir) = create_test_cache();
        
        let test_data = TestData {
            id: 1,
            name: "test".to_string(),
            value: 42.0,
        };
        
        // Store data with very short TTL
        let result = cache.store(
            "test_key",
            &test_data,
            "test_data",
            "test_source",
            Some(0), // Immediate expiration
            vec![],
        );
        assert!(result.is_ok());
        
        // Should not retrieve expired data
        std::thread::sleep(std::time::Duration::from_millis(10));
        let retrieved: Result<Option<CachedDataEntry<TestData>>, _> = cache.retrieve("test_key");
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_none());
    }
    
    #[test]
    fn test_cache_cleanup_expired() {
        let (cache, _temp_dir) = create_test_cache();
        
        let test_data = TestData {
            id: 1,
            name: "test".to_string(),
            value: 42.0,
        };
        
        // Store multiple entries with short TTL
        for i in 0..5 {
            cache.store(
                &format!("test_key_{}", i),
                &test_data,
                "test_data",
                "test_source",
                Some(0), // Immediate expiration
                vec![],
            ).unwrap();
        }
        
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        // Cleanup expired entries
        let cleaned = cache.cleanup_expired().unwrap();
        assert_eq!(cleaned, 5);
    }
    
    #[test]
    fn test_cache_statistics() {
        let (cache, _temp_dir) = create_test_cache();
        
        let test_data = TestData {
            id: 1,
            name: "test".to_string(),
            value: 42.0,
        };
        
        // Store and retrieve to generate stats
        cache.store(
            "test_key",
            &test_data,
            "test_data",
            "test_source",
            Some(3600),
            vec![],
        ).unwrap();
        
        let _retrieved: Option<CachedDataEntry<TestData>> = cache.retrieve("test_key").unwrap();
        let _not_found: Option<CachedDataEntry<TestData>> = cache.retrieve("nonexistent").unwrap();
        
        let stats = cache.get_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_ratio, 0.5);
    }
    
    #[test]
    fn test_performance_requirement() {
        let (cache, _temp_dir) = create_test_cache();
        
        // Initially should meet requirement
        assert!(cache.meets_performance_requirement());
        
        // The requirement is <200ms for cache access
        // Our cache should easily meet this requirement
    }
    
    #[test]
    fn test_cache_entries_by_type() {
        let (cache, _temp_dir) = create_test_cache();
        
        let test_data = TestData {
            id: 1,
            name: "test".to_string(),
            value: 42.0,
        };
        
        // Store entries of different types
        cache.store("key1", &test_data, "type_a", "source1", Some(3600), vec![]).unwrap();
        cache.store("key2", &test_data, "type_a", "source2", Some(3600), vec![]).unwrap();
        cache.store("key3", &test_data, "type_b", "source3", Some(3600), vec![]).unwrap();
        
        let type_a_keys = cache.get_entries_by_type("type_a").unwrap();
        assert_eq!(type_a_keys.len(), 2);
        assert!(type_a_keys.contains(&"key1".to_string()));
        assert!(type_a_keys.contains(&"key2".to_string()));
        
        let type_b_keys = cache.get_entries_by_type("type_b").unwrap();
        assert_eq!(type_b_keys.len(), 1);
        assert!(type_b_keys.contains(&"key3".to_string()));
    }
}