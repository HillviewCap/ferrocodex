use anyhow::Result;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{info, debug};
use std::collections::HashMap;

use super::FieldType;

/// Search result with metadata context and relevance scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSearchResult {
    pub asset_id: i64,
    pub asset_name: String,
    pub asset_type: String,
    pub hierarchy_path: Vec<String>,
    pub metadata_matches: Vec<MetadataMatch>,
    pub relevance_score: f32,
    pub last_updated: String,
}

/// Individual metadata field match with highlighting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataMatch {
    pub field_name: String,
    pub field_value: String,
    pub highlighted_value: String,
    pub schema_name: String,
    pub match_type: MatchType,
}

/// Type of match found in search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchType {
    ExactMatch,
    PartialMatch,
    FuzzyMatch,
    FieldName,
}

/// Filter operator for metadata field filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    Equals,
    NotEquals,
    Contains,
    StartsWith,
    EndsWith,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    IsNull,
    IsNotNull,
    InRange,
    Regex,
}

/// Logic operator for combining filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogicOperator {
    And,
    Or,
    Not,
}

/// Individual metadata filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataFilter {
    pub field_name: String,
    pub field_type: FieldType,
    pub operator: FilterOperator,
    pub value: Value,
    pub logic_operator: LogicOperator,
}

/// Sort options for search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortField {
    Relevance,
    AssetName,
    LastUpdated,
    CreatedDate,
    AssetType,
}

/// Complete search query structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text_query: Option<String>,
    pub filters: Vec<MetadataFilter>,
    pub hierarchy_scope: Option<i64>,
    pub sort_by: Option<SortField>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            text_query: None,
            filters: Vec::new(),
            hierarchy_scope: None,
            sort_by: Some(SortField::Relevance),
            limit: Some(50),
            offset: Some(0),
        }
    }
}

/// Search suggestion for auto-complete
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestion {
    pub text: String,
    pub suggestion_type: SuggestionType,
    pub field_name: Option<String>,
    pub description: String,
    pub usage_count: i64,
}

/// Type of search suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    FieldName,
    FieldValue,
    AssetName,
    SchemaName,
    RecentSearch,
}

/// Filter preset for commonly used searches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterPreset {
    pub id: Option<i64>,
    pub name: String,
    pub description: String,
    pub filters: Vec<MetadataFilter>,
    pub created_by: i64,
    pub usage_count: i64,
    pub created_at: String,
}

/// Search analytics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAnalytics {
    pub total_searches: i64,
    pub average_response_time_ms: f32,
    pub most_common_searches: Vec<String>,
    pub performance_metrics: HashMap<String, f32>,
    pub period_start: String,
    pub period_end: String,
}

/// Repository trait for metadata search operations
pub trait MetadataSearchRepository {
    fn search_assets_by_metadata(&self, query: SearchQuery) -> Result<Vec<AssetSearchResult>>;
    fn get_metadata_search_suggestions(&self, partial_query: String, limit: Option<u32>) -> Result<Vec<SearchSuggestion>>;
    fn create_metadata_filter_preset(&self, preset: FilterPreset) -> Result<FilterPreset>;
    fn get_filter_presets(&self, user_id: i64) -> Result<Vec<FilterPreset>>;
    fn get_search_analytics(&self, start_date: String, end_date: String) -> Result<SearchAnalytics>;
    fn find_similar_assets(&self, asset_id: i64, similarity_threshold: f32) -> Result<Vec<AssetSearchResult>>;
    fn search_assets_in_hierarchy(&self, parent_id: Option<i64>, query: SearchQuery) -> Result<Vec<AssetSearchResult>>;
    fn get_filterable_metadata_fields(&self) -> Result<Vec<FilterableField>>;
}

/// Information about fields that can be filtered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterableField {
    pub field_name: String,
    pub field_type: FieldType,
    pub schema_name: String,
    pub usage_count: i64,
    pub sample_values: Vec<String>,
}

/// SQLite implementation of metadata search repository
pub struct SqliteMetadataSearchRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteMetadataSearchRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Initialize search-related database structures including FTS5 indexes
    pub fn initialize_search_schema(&self) -> Result<()> {
        info!("Initializing metadata search schema with FTS5 support");
        
        self.conn.execute_batch(
            r#"
            -- Enable FTS5 extension
            PRAGMA foreign_keys=OFF;
            
            -- Create FTS5 virtual table for full-text search on metadata
            CREATE VIRTUAL TABLE IF NOT EXISTS asset_metadata_fts USING fts5(
                asset_id UNINDEXED,
                schema_id UNINDEXED,
                schema_name UNINDEXED,
                field_names,
                field_values,
                combined_content,
                content='',
                contentless_delete=1
            );

            -- Create search analytics table
            CREATE TABLE IF NOT EXISTS search_analytics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                query_text TEXT,
                query_filters TEXT,
                result_count INTEGER,
                response_time_ms INTEGER,
                user_id INTEGER,
                executed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );

            -- Create filter presets table
            CREATE TABLE IF NOT EXISTS metadata_filter_presets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                filters_json TEXT NOT NULL,
                created_by INTEGER NOT NULL,
                usage_count INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE CASCADE
            );

            -- Create indexes for search performance
            CREATE INDEX IF NOT EXISTS idx_search_analytics_user_id ON search_analytics(user_id);
            CREATE INDEX IF NOT EXISTS idx_search_analytics_executed_at ON search_analytics(executed_at);
            CREATE INDEX IF NOT EXISTS idx_filter_presets_created_by ON metadata_filter_presets(created_by);
            CREATE INDEX IF NOT EXISTS idx_filter_presets_usage ON metadata_filter_presets(usage_count DESC);

            -- Create triggers for automatic FTS5 index updates
            CREATE TRIGGER IF NOT EXISTS metadata_fts_insert AFTER INSERT ON asset_metadata BEGIN
                INSERT INTO asset_metadata_fts(asset_id, schema_id, schema_name, field_names, field_values, combined_content)
                SELECT 
                    NEW.asset_id,
                    NEW.schema_id,
                    COALESCE(s.name, 'Unknown Schema'),
                    (SELECT GROUP_CONCAT(key) FROM json_each(NEW.metadata_values_json)),
                    (SELECT GROUP_CONCAT(value) FROM json_each(NEW.metadata_values_json) WHERE type != 'object'),
                    COALESCE(s.name, '') || ' ' || a.name || ' ' || 
                    COALESCE((SELECT GROUP_CONCAT(key || ' ' || value) FROM json_each(NEW.metadata_values_json)), '')
                FROM asset_metadata_schemas s
                LEFT JOIN assets a ON a.id = NEW.asset_id
                WHERE s.id = NEW.schema_id;
            END;

            CREATE TRIGGER IF NOT EXISTS metadata_fts_update AFTER UPDATE ON asset_metadata BEGIN
                DELETE FROM asset_metadata_fts WHERE asset_id = OLD.asset_id AND schema_id = OLD.schema_id;
                INSERT INTO asset_metadata_fts(asset_id, schema_id, schema_name, field_names, field_values, combined_content)
                SELECT 
                    NEW.asset_id,
                    NEW.schema_id,
                    COALESCE(s.name, 'Unknown Schema'),
                    (SELECT GROUP_CONCAT(key) FROM json_each(NEW.metadata_values_json)),
                    (SELECT GROUP_CONCAT(value) FROM json_each(NEW.metadata_values_json) WHERE type != 'object'),
                    COALESCE(s.name, '') || ' ' || a.name || ' ' || 
                    COALESCE((SELECT GROUP_CONCAT(key || ' ' || value) FROM json_each(NEW.metadata_values_json)), '')
                FROM asset_metadata_schemas s
                LEFT JOIN assets a ON a.id = NEW.asset_id
                WHERE s.id = NEW.schema_id;
            END;

            CREATE TRIGGER IF NOT EXISTS metadata_fts_delete AFTER DELETE ON asset_metadata BEGIN
                DELETE FROM asset_metadata_fts WHERE asset_id = OLD.asset_id AND schema_id = OLD.schema_id;
            END;

            PRAGMA foreign_keys=ON;
            "#,
        )?;

        // Populate existing metadata into FTS index
        self.populate_fts_index()?;

        info!("Metadata search schema with FTS5 initialized successfully");
        Ok(())
    }

    /// Populate FTS5 index with existing metadata
    fn populate_fts_index(&self) -> Result<()> {
        info!("Populating FTS5 index with existing metadata");
        
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM asset_metadata_fts",
            [],
            |row| row.get(0),
        )?;

        if count > 0 {
            info!("FTS index already populated, skipping");
            return Ok(());
        }

        self.conn.execute(
            r#"
            INSERT INTO asset_metadata_fts(asset_id, schema_id, schema_name, field_names, field_values, combined_content)
            SELECT 
                am.asset_id,
                am.schema_id,
                COALESCE(s.name, 'Unknown Schema'),
                (SELECT GROUP_CONCAT(key) FROM json_each(am.metadata_values_json)),
                (SELECT GROUP_CONCAT(value) FROM json_each(am.metadata_values_json) WHERE type != 'object'),
                COALESCE(s.name, '') || ' ' || a.name || ' ' || 
                COALESCE((SELECT GROUP_CONCAT(key || ' ' || value) FROM json_each(am.metadata_values_json)), '')
            FROM asset_metadata am
            LEFT JOIN asset_metadata_schemas s ON s.id = am.schema_id
            LEFT JOIN assets a ON a.id = am.asset_id
            "#,
            [],
        )?;

        let populated_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM asset_metadata_fts",
            [],
            |row| row.get(0),
        )?;

        info!("Populated FTS5 index with {} metadata entries", populated_count);
        Ok(())
    }

    /// Build WHERE clause for metadata filters
    fn build_filter_clause(&self, filters: &[MetadataFilter]) -> Result<(String, Vec<Box<dyn rusqlite::ToSql>>)> {
        if filters.is_empty() {
            return Ok((String::new(), Vec::new()));
        }

        let mut conditions = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut param_index = 1;

        for (i, filter) in filters.iter().enumerate() {
            let logic_op = if i == 0 { "" } else {
                match filter.logic_operator {
                    LogicOperator::And => " AND ",
                    LogicOperator::Or => " OR ",
                    LogicOperator::Not => " AND NOT ",
                }
            };

            let condition = match filter.operator {
                FilterOperator::Equals => {
                    params.push(Box::new(filter.field_name.clone()));
                    params.push(Box::new(filter.value.to_string()));
                    format!("{}JSON_EXTRACT(am.metadata_values_json, '$.\"{}\"') = ?{}", 
                           logic_op, filter.field_name, param_index + 1)
                }
                FilterOperator::Contains => {
                    params.push(Box::new(format!("%{}%", filter.value.as_str().unwrap_or(""))));
                    format!("{}JSON_EXTRACT(am.metadata_values_json, '$.\"{}\"') LIKE ?{}", 
                           logic_op, filter.field_name, param_index)
                }
                FilterOperator::GreaterThan => {
                    params.push(Box::new(filter.value.to_string()));
                    format!("{}CAST(JSON_EXTRACT(am.metadata_values_json, '$.\"{}\"') AS REAL) > ?{}", 
                           logic_op, filter.field_name, param_index)
                }
                FilterOperator::LessThan => {
                    params.push(Box::new(filter.value.to_string()));
                    format!("{}CAST(JSON_EXTRACT(am.metadata_values_json, '$.\"{}\"') AS REAL) < ?{}", 
                           logic_op, filter.field_name, param_index)
                }
                FilterOperator::IsNull => {
                    format!("{}JSON_EXTRACT(am.metadata_values_json, '$.\"{}\"') IS NULL", 
                           logic_op, filter.field_name)
                }
                FilterOperator::IsNotNull => {
                    format!("{}JSON_EXTRACT(am.metadata_values_json, '$.\"{}\"') IS NOT NULL", 
                           logic_op, filter.field_name)
                }
                _ => {
                    // Handle other operators as needed
                    params.push(Box::new(filter.value.to_string()));
                    format!("{}JSON_EXTRACT(am.metadata_values_json, '$.\"{}\"') = ?{}", 
                           logic_op, filter.field_name, param_index)
                }
            };

            conditions.push(condition);
            param_index += params.len();
        }

        Ok((conditions.join(""), params))
    }

    /// Calculate relevance score based on match quality and metadata importance
    fn calculate_relevance_score(&self, matches: &[MetadataMatch], text_query: &Option<String>) -> f32 {
        let mut score = 0.0;
        
        for metadata_match in matches {
            let base_score = match metadata_match.match_type {
                MatchType::ExactMatch => 10.0,
                MatchType::PartialMatch => 5.0,
                MatchType::FuzzyMatch => 2.0,
                MatchType::FieldName => 3.0,
            };
            
            // Boost score for important field types
            let field_boost = match metadata_match.field_name.to_lowercase().as_str() {
                field if field.contains("name") => 2.0,
                field if field.contains("ip") || field.contains("address") => 1.5,
                field if field.contains("model") || field.contains("type") => 1.3,
                _ => 1.0,
            };
            
            score += base_score * field_boost;
        }
        
        // Normalize score based on query complexity
        if let Some(query) = text_query {
            let query_length = query.len() as f32;
            score = score * (1.0 + (query_length / 100.0).min(0.5));
        }
        
        score
    }

    /// Get asset hierarchy path as string vector
    fn get_asset_hierarchy_path(&self, asset_id: i64) -> Result<Vec<String>> {
        let mut path = Vec::new();
        let mut current_id = Some(asset_id);
        
        while let Some(id) = current_id {
            let mut stmt = self.conn.prepare(
                "SELECT name, parent_id FROM assets WHERE id = ?1"
            )?;
            
            match stmt.query_row([id], |row| {
                Ok((row.get::<_, String>("name")?, row.get::<_, Option<i64>>("parent_id")?))
            }) {
                Ok((name, parent_id)) => {
                    path.push(name);
                    current_id = parent_id;
                }
                Err(_) => break,
            }
        }
        
        path.reverse(); // Root to asset order
        Ok(path)
    }

    /// Record search analytics
    fn record_search_analytics(&self, query: &SearchQuery, result_count: usize, response_time_ms: i64, user_id: Option<i64>) -> Result<()> {
        let query_text = query.text_query.as_deref().unwrap_or("");
        let query_filters = serde_json::to_string(&query.filters)?;
        
        self.conn.execute(
            "INSERT INTO search_analytics (query_text, query_filters, result_count, response_time_ms, user_id) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                query_text,
                query_filters,
                result_count as i64,
                response_time_ms,
                user_id
            ],
        )?;
        
        Ok(())
    }
}

impl<'a> MetadataSearchRepository for SqliteMetadataSearchRepository<'a> {
    fn search_assets_by_metadata(&self, query: SearchQuery) -> Result<Vec<AssetSearchResult>> {
        let start_time = std::time::Instant::now();
        debug!("Starting metadata search with query: {:?}", query);
        
        let mut search_results = Vec::new();
        
        // Build the search query
        let mut query_parts = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        
        // Add text search using FTS5 if provided
        if let Some(text_query) = &query.text_query {
            if !text_query.trim().is_empty() {
                query_parts.push(
                    "asset_id IN (
                        SELECT DISTINCT asset_id FROM asset_metadata_fts 
                        WHERE asset_metadata_fts MATCH ?1
                    )".to_string()
                );
                params.push(Box::new(text_query.clone()));
            }
        }
        
        // Add metadata filters
        let (filter_clause, mut filter_params) = self.build_filter_clause(&query.filters)?;
        if !filter_clause.is_empty() {
            query_parts.push(format!("({})", filter_clause));
            params.append(&mut filter_params);
        }
        
        // Add hierarchy scope if specified
        if let Some(parent_id) = query.hierarchy_scope {
            query_parts.push("a.parent_id = ?".to_string());
            params.push(Box::new(parent_id));
        }
        
        // Build the complete query
        let where_clause = if query_parts.is_empty() {
            "1=1".to_string()
        } else {
            query_parts.join(" AND ")
        };
        
        let sort_clause = match query.sort_by.as_ref().unwrap_or(&SortField::Relevance) {
            SortField::Relevance => "1", // Will be sorted by calculated relevance
            SortField::AssetName => "a.name",
            SortField::LastUpdated => "am.updated_at DESC",
            SortField::CreatedDate => "a.created_at DESC",
            SortField::AssetType => "a.asset_type, a.name",
        };
        
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);
        
        let sql = format!(
            r#"
            SELECT DISTINCT 
                a.id, a.name, a.asset_type, a.updated_at,
                am.id as metadata_id, am.schema_id, am.metadata_values_json,
                s.name as schema_name
            FROM assets a
            LEFT JOIN asset_metadata am ON a.id = am.asset_id
            LEFT JOIN asset_metadata_schemas s ON am.schema_id = s.id
            WHERE {}
            ORDER BY {}
            LIMIT {} OFFSET {}
            "#,
            where_clause, sort_clause, limit, offset
        );
        
        debug!("Executing search SQL: {}", sql);
        
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok((
                row.get::<_, i64>("id")?,
                row.get::<_, String>("name")?,
                row.get::<_, String>("asset_type")?,
                row.get::<_, String>("updated_at")?,
                row.get::<_, Option<i64>>("metadata_id")?,
                row.get::<_, Option<i64>>("schema_id")?,
                row.get::<_, Option<String>>("metadata_values_json")?,
                row.get::<_, Option<String>>("schema_name")?,
            ))
        })?;
        
        // Group results by asset and build metadata matches
        let mut asset_matches: HashMap<i64, AssetSearchResult> = HashMap::new();
        
        for row in rows {
            let (asset_id, asset_name, asset_type, updated_at, metadata_id, schema_id, metadata_json, schema_name) = row?;
            
            // Get or create asset search result
            let asset_result = asset_matches.entry(asset_id).or_insert_with(|| AssetSearchResult {
                asset_id,
                asset_name: asset_name.clone(),
                asset_type: asset_type.clone(),
                hierarchy_path: self.get_asset_hierarchy_path(asset_id).unwrap_or_default(),
                metadata_matches: Vec::new(),
                relevance_score: 0.0,
                last_updated: updated_at.clone(),
            });
            
            // Add metadata matches if we have metadata
            if let (Some(_), Some(_), Some(json_str), Some(schema)) = (metadata_id, schema_id, metadata_json, schema_name) {
                if let Ok(metadata_values) = serde_json::from_str::<Value>(&json_str) {
                    if let Some(obj) = metadata_values.as_object() {
                        for (field_name, field_value) in obj {
                            // Check if this field matches our search criteria
                            if let Some(text_query) = &query.text_query {
                                let field_str = field_value.to_string();
                                if field_str.to_lowercase().contains(&text_query.to_lowercase()) {
                                    asset_result.metadata_matches.push(MetadataMatch {
                                        field_name: field_name.clone(),
                                        field_value: field_str.clone(),
                                        highlighted_value: self.highlight_matches(&field_str, text_query),
                                        schema_name: schema.clone(),
                                        match_type: if field_str.to_lowercase() == text_query.to_lowercase() {
                                            MatchType::ExactMatch
                                        } else {
                                            MatchType::PartialMatch
                                        },
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Calculate relevance scores and sort results
        for asset_result in asset_matches.values_mut() {
            asset_result.relevance_score = self.calculate_relevance_score(&asset_result.metadata_matches, &query.text_query);
        }
        
        search_results = asset_matches.into_values().collect();
        search_results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));
        
        let elapsed = start_time.elapsed();
        debug!("Search completed in {:?}, found {} results", elapsed, search_results.len());
        
        // Record analytics
        let _ = self.record_search_analytics(&query, search_results.len(), elapsed.as_millis() as i64, None);
        
        Ok(search_results)
    }

    fn get_metadata_search_suggestions(&self, partial_query: String, limit: Option<u32>) -> Result<Vec<SearchSuggestion>> {
        let mut suggestions = Vec::new();
        let limit = limit.unwrap_or(10) as i64;
        let query_lower = partial_query.to_lowercase();
        
        // Get field name suggestions
        let mut stmt = self.conn.prepare(
            r#"
            SELECT DISTINCT key as field_name, COUNT(*) as usage_count
            FROM asset_metadata am, json_each(am.metadata_values_json)
            WHERE LOWER(key) LIKE ?1
            GROUP BY key
            ORDER BY usage_count DESC
            LIMIT ?2
            "#
        )?;
        
        let field_suggestions = stmt.query_map(params![format!("{}%", query_lower), limit], |row| {
            Ok(SearchSuggestion {
                text: row.get("field_name")?,
                suggestion_type: SuggestionType::FieldName,
                field_name: Some(row.get("field_name")?),
                description: format!("Search in {} field", row.get::<_, String>("field_name")?),
                usage_count: row.get("usage_count")?,
            })
        })?;
        
        for suggestion in field_suggestions {
            suggestions.push(suggestion?);
        }
        
        // Get field value suggestions
        let mut stmt = self.conn.prepare(
            r#"
            SELECT DISTINCT value as field_value, key as field_name, COUNT(*) as usage_count
            FROM asset_metadata am, json_each(am.metadata_values_json)
            WHERE LOWER(value) LIKE ?1 AND json_valid(value) = 0
            GROUP BY value, key
            ORDER BY usage_count DESC
            LIMIT ?2
            "#
        )?;
        
        let value_suggestions = stmt.query_map(params![format!("%{}%", query_lower), limit], |row| {
            Ok(SearchSuggestion {
                text: row.get("field_value")?,
                suggestion_type: SuggestionType::FieldValue,
                field_name: Some(row.get("field_name")?),
                description: format!("Value in {} field", row.get::<_, String>("field_name")?),
                usage_count: row.get("usage_count")?,
            })
        })?;
        
        for suggestion in value_suggestions {
            suggestions.push(suggestion?);
        }
        
        // Sort by usage count and limit
        suggestions.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));
        suggestions.truncate(limit as usize);
        
        Ok(suggestions)
    }

    fn create_metadata_filter_preset(&self, preset: FilterPreset) -> Result<FilterPreset> {
        let filters_json = serde_json::to_string(&preset.filters)?;
        
        let mut stmt = self.conn.prepare(
            "INSERT INTO metadata_filter_presets (name, description, filters_json, created_by)
             VALUES (?1, ?2, ?3, ?4) RETURNING *"
        )?;
        
        let created_preset = stmt.query_row(
            params![&preset.name, &preset.description, &filters_json, &preset.created_by],
            |row| {
                Ok(FilterPreset {
                    id: Some(row.get("id")?),
                    name: row.get("name")?,
                    description: row.get("description")?,
                    filters: serde_json::from_str(&row.get::<_, String>("filters_json")?).unwrap_or_default(),
                    created_by: row.get("created_by")?,
                    usage_count: row.get("usage_count")?,
                    created_at: row.get("created_at")?,
                })
            }
        )?;
        
        info!("Created filter preset: {}", created_preset.name);
        Ok(created_preset)
    }

    fn get_filter_presets(&self, user_id: i64) -> Result<Vec<FilterPreset>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, filters_json, created_by, usage_count, created_at
             FROM metadata_filter_presets 
             WHERE created_by = ?1 
             ORDER BY usage_count DESC, created_at DESC"
        )?;
        
        let preset_iter = stmt.query_map([user_id], |row| {
            Ok(FilterPreset {
                id: Some(row.get("id")?),
                name: row.get("name")?,
                description: row.get("description")?,
                filters: serde_json::from_str(&row.get::<_, String>("filters_json")?).unwrap_or_default(),
                created_by: row.get("created_by")?,
                usage_count: row.get("usage_count")?,
                created_at: row.get("created_at")?,
            })
        })?;
        
        let mut presets = Vec::new();
        for preset in preset_iter {
            presets.push(preset?);
        }
        
        Ok(presets)
    }

    fn get_search_analytics(&self, start_date: String, end_date: String) -> Result<SearchAnalytics> {
        // Get basic statistics
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) as total_searches, AVG(response_time_ms) as avg_response_time
             FROM search_analytics 
             WHERE executed_at BETWEEN ?1 AND ?2"
        )?;
    
        let (total_searches, avg_response_time) = stmt.query_row([&start_date, &end_date], |row| {
            Ok((
                row.get::<_, i64>("total_searches")?,
                row.get::<_, f64>("avg_response_time")? as f32
            ))
        })?;
    
        // Get most common searches
        let mut stmt = self.conn.prepare(
            "SELECT query_text, COUNT(*) as search_count
             FROM search_analytics 
             WHERE executed_at BETWEEN ?1 AND ?2 AND query_text != ''
             GROUP BY query_text
             ORDER BY search_count DESC
             LIMIT 10"
        )?;
    
        let search_iter = stmt.query_map([&start_date, &end_date], |row| {
            Ok(row.get::<_, String>("query_text")?)
        })?;
    
        let mut most_common_searches = Vec::new();
        for search in search_iter {
            most_common_searches.push(search?);
        }
    
        // Basic performance metrics
        let mut performance_metrics = HashMap::new();
        performance_metrics.insert("average_response_time_ms".to_string(), avg_response_time);
        performance_metrics.insert("total_searches".to_string(), total_searches as f32);
    
        Ok(SearchAnalytics {
            total_searches,
            average_response_time_ms: avg_response_time,
            most_common_searches,
            performance_metrics,
            period_start: start_date,
            period_end: end_date,
        })
    }

    fn find_similar_assets(&self, asset_id: i64, similarity_threshold: f32) -> Result<Vec<AssetSearchResult>> {
        // Get the metadata for the reference asset
        let mut stmt = self.conn.prepare(
            "SELECT am.metadata_values_json, s.name as schema_name
             FROM asset_metadata am
             LEFT JOIN asset_metadata_schemas s ON am.schema_id = s.id
             WHERE am.asset_id = ?1"
        )?;
        
        let reference_metadata: Vec<(String, String)> = stmt.query_map([asset_id], |row| {
            Ok((
                row.get::<_, String>("metadata_values_json")?,
                row.get::<_, String>("schema_name")?
            ))
        })?.collect::<Result<Vec<_>, _>>()?;
        
        if reference_metadata.is_empty() {
            return Ok(Vec::new());
        }
        
        // For now, implement a simple similarity search based on shared metadata fields
        // This could be enhanced with more sophisticated similarity algorithms
        let mut similar_assets = Vec::new();
        
        for (ref_json, _) in reference_metadata {
            if let Ok(ref_values) = serde_json::from_str::<Value>(&ref_json) {
                if let Some(ref_obj) = ref_values.as_object() {
                    // Find assets with similar metadata structure and values
                    let mut stmt = self.conn.prepare(
                        r#"
                        SELECT DISTINCT a.id, a.name, a.asset_type, a.updated_at,
                               am.metadata_values_json, s.name as schema_name
                        FROM assets a
                        JOIN asset_metadata am ON a.id = am.asset_id
                        LEFT JOIN asset_metadata_schemas s ON am.schema_id = s.id
                        WHERE a.id != ?1
                        "#
                    )?;
                    
                    let candidates = stmt.query_map([asset_id], |row| {
                        Ok((
                            row.get::<_, i64>("id")?,
                            row.get::<_, String>("name")?,
                            row.get::<_, String>("asset_type")?,
                            row.get::<_, String>("updated_at")?,
                            row.get::<_, String>("metadata_values_json")?,
                            row.get::<_, String>("schema_name")?,
                        ))
                    })?;
                    
                    for candidate in candidates {
                        let (cand_id, cand_name, cand_type, cand_updated, cand_json, _cand_schema) = candidate?;
                        
                        if let Ok(cand_values) = serde_json::from_str::<Value>(&cand_json) {
                            if let Some(cand_obj) = cand_values.as_object() {
                                let similarity = self.calculate_metadata_similarity(ref_obj, cand_obj);
                                
                                if similarity >= similarity_threshold {
                                    similar_assets.push(AssetSearchResult {
                                        asset_id: cand_id,
                                        asset_name: cand_name,
                                        asset_type: cand_type,
                                        hierarchy_path: self.get_asset_hierarchy_path(cand_id)?,
                                        metadata_matches: Vec::new(), // Could populate with matching fields
                                        relevance_score: similarity,
                                        last_updated: cand_updated,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by similarity score
        similar_assets.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(similar_assets)
    }

    fn search_assets_in_hierarchy(&self, parent_id: Option<i64>, query: SearchQuery) -> Result<Vec<AssetSearchResult>> {
        // Create a modified query that includes hierarchy filtering
        let mut hierarchy_query = query;
        
        // Get all descendant asset IDs if parent_id is specified
        let _hierarchy_filter = if let Some(parent) = parent_id {
            // Get all descendants of the parent asset
            let descendants = self.get_descendant_asset_ids(parent)?;
            descendants
        } else {
            Vec::new()
        };
        
        // Modify the search to include hierarchy scope
        hierarchy_query.hierarchy_scope = parent_id;
        
        self.search_assets_by_metadata(hierarchy_query)
    }

    fn get_filterable_metadata_fields(&self) -> Result<Vec<FilterableField>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT 
                key as field_name,
                s.name as schema_name,
                COUNT(*) as usage_count,
                GROUP_CONCAT(DISTINCT SUBSTR(value, 1, 50)) as sample_values
            FROM asset_metadata am
            JOIN asset_metadata_schemas s ON am.schema_id = s.id,
            json_each(am.metadata_values_json)
            WHERE json_valid(value) = 0
            GROUP BY key, s.name
            ORDER BY usage_count DESC
            LIMIT 100
            "#
        )?;
        
        let field_iter = stmt.query_map([], |row| {
            let sample_values_str: String = row.get("sample_values")?;
            let sample_values: Vec<String> = sample_values_str
                .split(',')
                .take(5)
                .map(|s| s.trim().to_string())
                .collect();
                
            Ok(FilterableField {
                field_name: row.get("field_name")?,
                field_type: FieldType::Text, // Default, could be inferred from values
                schema_name: row.get("schema_name")?,
                usage_count: row.get("usage_count")?,
                sample_values,
            })
        })?;
        
        let mut fields = Vec::new();
        for field in field_iter {
            fields.push(field?);
        }
        
        Ok(fields)
    }
}

// Helper methods for SqliteMetadataSearchRepository
impl<'a> SqliteMetadataSearchRepository<'a> {
    /// Highlight search matches in text
    fn highlight_matches(&self, text: &str, query: &str) -> String {
        // Simple highlighting - replace with more sophisticated highlighting if needed
        let query_lower = query.to_lowercase();
        let text_lower = text.to_lowercase();
        
        if let Some(start) = text_lower.find(&query_lower) {
            let end = start + query.len();
            format!("{}**{}**{}", 
                   &text[..start], 
                   &text[start..end], 
                   &text[end..])
        } else {
            text.to_string()
        }
    }
    
    /// Calculate similarity between two metadata objects
    fn calculate_metadata_similarity(&self, obj1: &serde_json::Map<String, Value>, obj2: &serde_json::Map<String, Value>) -> f32 {
        let keys1: std::collections::HashSet<&String> = obj1.keys().collect();
        let keys2: std::collections::HashSet<&String> = obj2.keys().collect();
        
        let common_keys = keys1.intersection(&keys2).count();
        let total_keys = keys1.union(&keys2).count();
        
        if total_keys == 0 {
            return 0.0;
        }
        
        let structural_similarity = common_keys as f32 / total_keys as f32;
        
        // Calculate value similarity for common keys
        let mut value_similarity = 0.0;
        let mut common_key_count = 0;
        
        for key in keys1.intersection(&keys2) {
            common_key_count += 1;
            if obj1[*key] == obj2[*key] {
                value_similarity += 1.0;
            } else if let (Some(str1), Some(str2)) = (obj1[*key].as_str(), obj2[*key].as_str()) {
                // Simple string similarity - could use more sophisticated algorithms
                let similarity = self.string_similarity(str1, str2);
                value_similarity += similarity;
            }
        }
        
        if common_key_count > 0 {
            value_similarity /= common_key_count as f32;
        }
        
        // Combine structural and value similarity
        (structural_similarity + value_similarity) / 2.0
    }
    
    /// Simple string similarity calculation
    fn string_similarity(&self, s1: &str, s2: &str) -> f32 {
        if s1 == s2 {
            return 1.0;
        }
        
        let s1_lower = s1.to_lowercase();
        let s2_lower = s2.to_lowercase();
        
        if s1_lower.contains(&s2_lower) || s2_lower.contains(&s1_lower) {
            return 0.7;
        }
        
        // Could implement Levenshtein distance or other algorithms here
        0.0
    }
    
    /// Get all descendant asset IDs recursively
    fn get_descendant_asset_ids(&self, parent_id: i64) -> Result<Vec<i64>> {
        let mut descendants = Vec::new();
        let mut to_process = vec![parent_id];
        
        while let Some(current_parent) = to_process.pop() {
            let mut stmt = self.conn.prepare(
                "SELECT id FROM assets WHERE parent_id = ?1"
            )?;
            
            let children = stmt.query_map([current_parent], |row| {
                Ok(row.get::<_, i64>("id")?)
            })?;
            
            for child in children {
                let child_id = child?;
                descendants.push(child_id);
                to_process.push(child_id);
            }
        }
        
        Ok(descendants)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use rusqlite::Connection;
    use serde_json::json;

    fn setup_test_db() -> (NamedTempFile, Connection) {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();
        
        // Create required tables
        conn.execute_batch(
            r#"
            CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                username TEXT NOT NULL,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                is_active BOOLEAN DEFAULT 1
            );
            
            CREATE TABLE assets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                asset_type TEXT NOT NULL DEFAULT 'device',
                parent_id INTEGER,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (created_by) REFERENCES users(id),
                FOREIGN KEY (parent_id) REFERENCES assets(id) ON DELETE CASCADE
            );

            CREATE TABLE asset_metadata_schemas (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT NOT NULL,
                schema_json TEXT NOT NULL,
                asset_type_filter TEXT,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                is_system_template BOOLEAN NOT NULL DEFAULT 0,
                version INTEGER NOT NULL DEFAULT 1,
                FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE CASCADE
            );

            CREATE TABLE asset_metadata (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                asset_id INTEGER NOT NULL,
                schema_id INTEGER NOT NULL,
                metadata_values_json TEXT NOT NULL,
                schema_version INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE,
                FOREIGN KEY (schema_id) REFERENCES asset_metadata_schemas(id) ON DELETE CASCADE,
                UNIQUE(asset_id, schema_id)
            );
            
            INSERT INTO users (id, username, password_hash, role) VALUES (1, 'testuser', 'hash', 'Engineer');
            INSERT INTO assets (id, name, description, asset_type, created_by) VALUES 
                (1, 'PLC-001', 'Primary PLC', 'device', 1),
                (2, 'HMI-001', 'Main HMI', 'device', 1);
            "#,
        ).unwrap();
        
        let search_repo = SqliteMetadataSearchRepository::new(&conn);
        search_repo.initialize_search_schema().unwrap();
        
        (temp_file, conn)
    }

    #[test]
    fn test_search_schema_initialization() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteMetadataSearchRepository::new(&conn);
        
        // Test that FTS table was created
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='asset_metadata_fts'",
            [],
            |row| row.get(0),
        ).unwrap();
        
        assert_eq!(count, 1);
    }

    #[test]
    fn test_basic_metadata_search() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteMetadataSearchRepository::new(&conn);
        
        // Insert test schema and metadata
        conn.execute(
            "INSERT INTO asset_metadata_schemas (id, name, description, schema_json, created_by) 
             VALUES (1, 'PLC Schema', 'Test schema', '{}', 1)",
            [],
        ).unwrap();
        
        conn.execute(
            "INSERT INTO asset_metadata (asset_id, schema_id, metadata_values_json, schema_version) 
             VALUES (1, 1, ?, 1)",
            [&json!({"ip_address": "192.168.1.100", "model": "Siemens S7-1200"}).to_string()],
        ).unwrap();
        
        // Test text search
        let query = SearchQuery {
            text_query: Some("192.168.1.100".to_string()),
            ..Default::default()
        };
        
        let results = repo.search_assets_by_metadata(query).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].asset_name, "PLC-001");
    }

    #[test]
    fn test_metadata_filter() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteMetadataSearchRepository::new(&conn);
        
        // Insert test data
        conn.execute(
            "INSERT INTO asset_metadata_schemas (id, name, description, schema_json, created_by) 
             VALUES (1, 'PLC Schema', 'Test schema', '{}', 1)",
            [],
        ).unwrap();
        
        conn.execute(
            "INSERT INTO asset_metadata (asset_id, schema_id, metadata_values_json, schema_version) 
             VALUES (1, 1, ?, 1)",
            [&json!({"model": "Siemens S7-1200", "version": "4.2"}).to_string()],
        ).unwrap();
        
        // Test filter search
        let filter = MetadataFilter {
            field_name: "model".to_string(),
            field_type: FieldType::Text,
            operator: FilterOperator::Contains,
            value: json!("Siemens"),
            logic_operator: LogicOperator::And,
        };
        
        let query = SearchQuery {
            filters: vec![filter],
            ..Default::default()
        };
        
        let results = repo.search_assets_by_metadata(query).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_suggestions() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteMetadataSearchRepository::new(&conn);
        
        // Insert test data
        conn.execute(
            "INSERT INTO asset_metadata_schemas (id, name, description, schema_json, created_by) 
             VALUES (1, 'PLC Schema', 'Test schema', '{}', 1)",
            [],
        ).unwrap();
        
        conn.execute(
            "INSERT INTO asset_metadata (asset_id, schema_id, metadata_values_json, schema_version) 
             VALUES (1, 1, ?, 1)",
            [&json!({"ip_address": "192.168.1.100", "model": "Siemens"}).to_string()],
        ).unwrap();
        
        let suggestions = repo.get_metadata_search_suggestions("ip".to_string(), Some(5)).unwrap();
        assert!(!suggestions.is_empty());
        
        // Should find "ip_address" field
        let has_ip_field = suggestions.iter().any(|s| s.text.contains("ip_address"));
        assert!(has_ip_field);
    }

    #[test]
    fn test_filter_preset_creation() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteMetadataSearchRepository::new(&conn);
        
        let filter = MetadataFilter {
            field_name: "model".to_string(),
            field_type: FieldType::Text,
            operator: FilterOperator::Contains,
            value: json!("Siemens"),
            logic_operator: LogicOperator::And,
        };
        
        let preset = FilterPreset {
            id: None,
            name: "Siemens PLCs".to_string(),
            description: "Find all Siemens PLC assets".to_string(),
            filters: vec![filter],
            created_by: 1,
            usage_count: 0,
            created_at: String::new(),
        };
        
        let created_preset = repo.create_metadata_filter_preset(preset).unwrap();
        assert!(created_preset.id.is_some());
        assert_eq!(created_preset.name, "Siemens PLCs");
        
        let presets = repo.get_filter_presets(1).unwrap();
        assert_eq!(presets.len(), 1);
    }
}