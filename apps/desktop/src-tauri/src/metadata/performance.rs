use anyhow::Result;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{info, warn, debug};

/// Performance metrics for search operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPerformanceMetrics {
    pub operation_type: String,
    pub execution_time_ms: u64,
    pub result_count: usize,
    pub query_complexity_score: f32,
    pub cache_hit: bool,
    pub index_usage: Vec<String>,
    pub timestamp: String,
}

/// Search performance monitoring and optimization service
pub struct SearchPerformanceMonitor<'a> {
    conn: &'a Connection,
}

impl<'a> SearchPerformanceMonitor<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Record search performance metrics
    pub fn record_search_performance(&self, metrics: SearchPerformanceMetrics) -> Result<()> {
        debug!("Recording search performance metrics: {:?}", metrics);

        // Store detailed metrics in a separate table for analysis
        self.conn.execute(
            r#"
            INSERT OR IGNORE INTO search_performance_metrics (
                operation_type, execution_time_ms, result_count, query_complexity_score,
                cache_hit, index_usage, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
            "#,
            params![
                metrics.operation_type,
                metrics.execution_time_ms as i64,
                metrics.result_count as i64,
                metrics.query_complexity_score,
                metrics.cache_hit,
                serde_json::to_string(&metrics.index_usage).unwrap_or_default()
            ],
        )?;

        // Log performance warnings
        if metrics.execution_time_ms > 1000 {
            warn!(
                "Slow search operation: {} took {}ms with {} results",
                metrics.operation_type, metrics.execution_time_ms, metrics.result_count
            );
        }

        Ok(())
    }

    /// Calculate query complexity score based on various factors
    pub fn calculate_query_complexity(
        &self,
        text_query: &Option<String>,
        filter_count: usize,
        has_hierarchy_scope: bool,
        has_sorting: bool,
    ) -> f32 {
        let mut complexity = 1.0;

        // Text query complexity
        if let Some(query) = text_query {
            complexity += query.len() as f32 * 0.1;
            if query.contains('*') || query.contains('?') {
                complexity += 2.0; // Wildcard searches are more expensive
            }
            if query.contains("AND") || query.contains("OR") || query.contains("NOT") {
                complexity += 1.5; // Boolean queries add complexity
            }
        }

        // Filter complexity
        complexity += filter_count as f32 * 0.5;

        // Hierarchy and sorting
        if has_hierarchy_scope {
            complexity += 0.5;
        }
        if has_sorting {
            complexity += 0.3;
        }

        complexity
    }

    /// Get search performance statistics
    pub fn get_performance_statistics(&self, hours_back: u32) -> Result<SearchPerformanceStats> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT 
                operation_type,
                COUNT(*) as operation_count,
                AVG(execution_time_ms) as avg_time_ms,
                MAX(execution_time_ms) as max_time_ms,
                MIN(execution_time_ms) as min_time_ms,
                AVG(result_count) as avg_results,
                AVG(query_complexity_score) as avg_complexity,
                SUM(CASE WHEN cache_hit = 1 THEN 1 ELSE 0 END) * 100.0 / COUNT(*) as cache_hit_rate
            FROM search_performance_metrics 
            WHERE created_at >= datetime('now', '-' || ?1 || ' hours')
            GROUP BY operation_type
            ORDER BY operation_count DESC
            "#,
        )?;

        let mut operations = Vec::new();
        let rows = stmt.query_map([hours_back], |row| {
            Ok(OperationStats {
                operation_type: row.get("operation_type")?,
                operation_count: row.get("operation_count")?,
                avg_time_ms: row.get("avg_time_ms")?,
                max_time_ms: row.get("max_time_ms")?,
                min_time_ms: row.get("min_time_ms")?,
                avg_results: row.get("avg_results")?,
                avg_complexity: row.get("avg_complexity")?,
                cache_hit_rate: row.get("cache_hit_rate")?,
            })
        })?;

        for row in rows {
            operations.push(row?);
        }

        // Get overall statistics
        let mut stmt = self.conn.prepare(
            r#"
            SELECT 
                COUNT(*) as total_searches,
                AVG(execution_time_ms) as overall_avg_time,
                COUNT(DISTINCT substr(created_at, 1, 13)) as active_hours
            FROM search_performance_metrics 
            WHERE created_at >= datetime('now', '-' || ?1 || ' hours')
            "#,
        )?;

        let (total_searches, overall_avg_time, active_hours) = stmt.query_row([hours_back], |row| {
            Ok((
                row.get::<_, i64>("total_searches")?,
                row.get::<_, f64>("overall_avg_time")?,
                row.get::<_, i64>("active_hours")?,
            ))
        })?;

        Ok(SearchPerformanceStats {
            total_searches: total_searches as u64,
            overall_avg_time_ms: overall_avg_time as f32,
            active_hours: active_hours as u32,
            operations,
            period_hours: hours_back,
        })
    }

    /// Optimize search indexes based on usage patterns
    pub fn optimize_search_indexes(&self) -> Result<IndexOptimizationResult> {
        info!("Starting search index optimization");
        let start_time = Instant::now();
        
        let mut optimizations = Vec::new();
        let mut warnings = Vec::new();

        // Analyze FTS5 index statistics
        let fts_stats = self.analyze_fts_performance()?;
        if fts_stats.needs_rebuild {
            self.conn.execute("INSERT INTO asset_metadata_fts(asset_metadata_fts) VALUES('rebuild')", [])?;
            optimizations.push("Rebuilt FTS5 full-text search index".to_string());
        }

        // Check for unused indexes
        let unused_indexes = self.find_unused_indexes()?;
        for index in unused_indexes {
            warnings.push(format!("Index '{}' appears to be unused", index));
        }

        // Update table statistics
        self.conn.execute_batch(
            r#"
            ANALYZE asset_metadata;
            ANALYZE asset_metadata_fts;
            ANALYZE search_analytics;
            ANALYZE metadata_filter_presets;
            "#,
        )?;
        optimizations.push("Updated table statistics for query planner".to_string());

        // Optimize database
        self.conn.execute("PRAGMA optimize", [])?;
        optimizations.push("Ran SQLite query planner optimization".to_string());

        let optimization_time = start_time.elapsed();
        info!("Search index optimization completed in {:?}", optimization_time);

        Ok(IndexOptimizationResult {
            optimizations_applied: optimizations,
            warnings,
            optimization_time_ms: optimization_time.as_millis() as u64,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Check if search indexes need maintenance
    pub fn check_index_health(&self) -> Result<IndexHealthReport> {
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Check FTS5 index size and fragmentation
        let fts_info = self.get_fts_index_info()?;
        if fts_info.fragmentation_percent > 20.0 {
            issues.push(format!(
                "FTS5 index is {:.1}% fragmented", 
                fts_info.fragmentation_percent
            ));
            recommendations.push("Consider rebuilding FTS5 index during maintenance window".to_string());
        }

        // Check for missing statistics
        let stats_age = self.get_statistics_age()?;
        if stats_age > Duration::from_secs(24 * 60 * 60) {
            issues.push("Table statistics are over 24 hours old".to_string());
            recommendations.push("Run ANALYZE to update table statistics".to_string());
        }

        // Check for performance degradation
        let recent_performance = self.get_performance_statistics(24)?;
        if recent_performance.overall_avg_time_ms > 500.0 {
            issues.push(format!(
                "Average search time is {:.0}ms (target: <200ms)", 
                recent_performance.overall_avg_time_ms
            ));
            recommendations.push("Consider optimizing queries or adding indexes".to_string());
        }

        let health_score = if issues.is_empty() {
            100.0
        } else {
            std::cmp::max(0, 100 - (issues.len() * 20)) as f32
        };

        Ok(IndexHealthReport {
            health_score,
            issues,
            recommendations,
            last_check: chrono::Utc::now().to_rfc3339(),
            fts_index_info: fts_info,
        })
    }

    /// Initialize performance monitoring tables
    pub fn initialize_performance_schema(&self) -> Result<()> {
        info!("Initializing search performance monitoring schema");

        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS search_performance_metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                operation_type TEXT NOT NULL,
                execution_time_ms INTEGER NOT NULL,
                result_count INTEGER NOT NULL,
                query_complexity_score REAL NOT NULL,
                cache_hit BOOLEAN NOT NULL DEFAULT 0,
                index_usage TEXT, -- JSON array of used indexes
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_perf_metrics_operation_time 
            ON search_performance_metrics(operation_type, created_at DESC);

            CREATE INDEX IF NOT EXISTS idx_perf_metrics_execution_time 
            ON search_performance_metrics(execution_time_ms DESC);

            -- Cleanup old performance metrics (keep last 30 days)
            DELETE FROM search_performance_metrics 
            WHERE created_at < datetime('now', '-30 days');
            "#,
        )?;

        info!("Search performance monitoring schema initialized");
        Ok(())
    }

    // Private helper methods

    fn analyze_fts_performance(&self) -> Result<FtsIndexStats> {
        // Check FTS5 index statistics
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) as doc_count FROM asset_metadata_fts"
        )?;
        
        let doc_count: i64 = stmt.query_row([], |row| row.get(0))?;

        // Simple heuristic for rebuild necessity
        let needs_rebuild = doc_count > 10000 && doc_count % 5000 == 0;

        Ok(FtsIndexStats {
            document_count: doc_count as u64,
            needs_rebuild,
            fragmentation_percent: 0.0, // Would need more complex analysis
        })
    }

    fn find_unused_indexes(&self) -> Result<Vec<String>> {
        // This is a simplified check - in production, you'd analyze query plans
        let mut unused = Vec::new();
        
        // Check if certain indexes are being used by examining recent query patterns
        let mut stmt = self.conn.prepare(
            r#"
            SELECT name FROM sqlite_master 
            WHERE type = 'index' 
            AND name LIKE 'idx_%'
            AND tbl_name IN ('asset_metadata', 'search_analytics', 'metadata_filter_presets')
            "#
        )?;

        let index_names = stmt.query_map([], |row| {
            Ok(row.get::<_, String>("name")?)
        })?;

        for name in index_names {
            let index_name = name?;
            // Simplified check - in practice, you'd check EXPLAIN QUERY PLAN
            if index_name.contains("_temp_") {
                unused.push(index_name);
            }
        }

        Ok(unused)
    }

    fn get_fts_index_info(&self) -> Result<FtsIndexInfo> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) as total_docs FROM asset_metadata_fts"
        )?;
        
        let total_docs: i64 = stmt.query_row([], |row| row.get(0))?;

        Ok(FtsIndexInfo {
            total_documents: total_docs as u64,
            fragmentation_percent: 0.0, // Simplified
            last_rebuild: None,
            size_mb: 0.0, // Would need PRAGMA page_count * page_size calculation
        })
    }

    fn get_statistics_age(&self) -> Result<Duration> {
        // Check when statistics were last updated (simplified)
        Ok(Duration::from_secs(60 * 60)) // Placeholder - 1 hour
    }
}

// Supporting data structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPerformanceStats {
    pub total_searches: u64,
    pub overall_avg_time_ms: f32,
    pub active_hours: u32,
    pub operations: Vec<OperationStats>,
    pub period_hours: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationStats {
    pub operation_type: String,
    pub operation_count: i64,
    pub avg_time_ms: f64,
    pub max_time_ms: i64,
    pub min_time_ms: i64,
    pub avg_results: f64,
    pub avg_complexity: f64,
    pub cache_hit_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexOptimizationResult {
    pub optimizations_applied: Vec<String>,
    pub warnings: Vec<String>,
    pub optimization_time_ms: u64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexHealthReport {
    pub health_score: f32,
    pub issues: Vec<String>,
    pub recommendations: Vec<String>,
    pub last_check: String,
    pub fts_index_info: FtsIndexInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FtsIndexInfo {
    pub total_documents: u64,
    pub fragmentation_percent: f32,
    pub last_rebuild: Option<String>,
    pub size_mb: f32,
}

#[derive(Debug, Clone)]
struct FtsIndexStats {
    document_count: u64,
    needs_rebuild: bool,
    fragmentation_percent: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use rusqlite::Connection;

    fn setup_test_db() -> (NamedTempFile, Connection) {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();
        
        let monitor = SearchPerformanceMonitor::new(&conn);
        monitor.initialize_performance_schema().unwrap();
        
        (temp_file, conn)
    }

    #[test]
    fn test_performance_metrics_recording() {
        let (_temp_file, conn) = setup_test_db();
        let monitor = SearchPerformanceMonitor::new(&conn);

        let metrics = SearchPerformanceMetrics {
            operation_type: "text_search".to_string(),
            execution_time_ms: 150,
            result_count: 25,
            query_complexity_score: 2.5,
            cache_hit: false,
            index_usage: vec!["idx_asset_metadata_fts".to_string()],
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        assert!(monitor.record_search_performance(metrics).is_ok());
    }

    #[test]
    fn test_query_complexity_calculation() {
        let (_temp_file, conn) = setup_test_db();
        let monitor = SearchPerformanceMonitor::new(&conn);

        // Simple query
        let complexity1 = monitor.calculate_query_complexity(
            &Some("test".to_string()),
            0,
            false,
            false,
        );
        assert!(complexity1 > 1.0 && complexity1 < 2.0);

        // Complex query with filters
        let complexity2 = monitor.calculate_query_complexity(
            &Some("test AND keyword*".to_string()),
            3,
            true,
            true,
        );
        assert!(complexity2 > 4.0);
    }

    #[test]
    fn test_performance_statistics() {
        let (_temp_file, conn) = setup_test_db();
        let monitor = SearchPerformanceMonitor::new(&conn);

        // Record some test metrics
        let metrics = SearchPerformanceMetrics {
            operation_type: "metadata_search".to_string(),
            execution_time_ms: 200,
            result_count: 15,
            query_complexity_score: 1.8,
            cache_hit: true,
            index_usage: vec!["idx_metadata".to_string()],
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        monitor.record_search_performance(metrics).unwrap();

        let stats = monitor.get_performance_statistics(24).unwrap();
        assert_eq!(stats.total_searches, 1);
        assert_eq!(stats.operations.len(), 1);
        assert_eq!(stats.operations[0].operation_type, "metadata_search");
    }
}