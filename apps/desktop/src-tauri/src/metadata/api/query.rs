use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::metadata::{AssetMetadata, AssetMetadataSchema, SqliteMetadataRepository, MetadataRepository};
use crate::assets::{Asset, SqliteAssetRepository, AssetRepository};
use super::{Pagination, SortDirection};
use rusqlite::Connection;
use std::collections::HashMap;
use std::time::Instant;

/// Complex metadata query with boolean logic and filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataQuery {
    pub conditions: Vec<QueryCondition>,
    pub logic_operator: LogicOperator,
    pub pagination: Option<Pagination>,
    pub include_metadata: bool,
    pub include_schema: bool,
}

/// Individual query condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCondition {
    pub field_path: String,
    pub operator: ComparisonOperator,
    pub value: Value,
    pub case_sensitive: bool,
}

/// Logic operators for combining conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogicOperator {
    And,
    Or,
    Not,
}

/// Comparison operators for field values
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComparisonOperator {
    Equals,
    NotEquals,
    Contains,
    StartsWith,
    EndsWith,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Between,
    In,
    NotIn,
    IsNull,
    IsNotNull,
    Matches, // Regex pattern matching
}

/// Field statistics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldStatsConfig {
    pub field_paths: Vec<String>,
    pub schema_ids: Option<Vec<i64>>,
    pub asset_type_filter: Option<String>,
    pub include_distributions: bool,
    pub include_trends: bool,
    pub time_range: Option<TimeRange>,
}

/// Time range for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: String,
    pub end: String,
}

/// Field statistics result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldStatistics {
    pub field_path: String,
    pub total_values: u64,
    pub unique_values: u64,
    pub null_count: u64,
    pub data_type_distribution: HashMap<String, u64>,
    pub value_distribution: Option<HashMap<String, u64>>,
    pub numeric_stats: Option<NumericStats>,
    pub trends: Option<Vec<TrendPoint>>,
}

/// Numeric field statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumericStats {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
    pub std_deviation: f64,
    pub percentiles: HashMap<String, f64>, // "25", "50", "75", "90", "95", "99"
}

/// Trend data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendPoint {
    pub timestamp: String,
    pub value_count: u64,
    pub average_value: Option<f64>,
}

/// Search configuration for metadata values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub search_term: String,
    pub search_fields: Option<Vec<String>>,
    pub schema_ids: Option<Vec<i64>>,
    pub asset_type_filter: Option<String>,
    pub fuzzy_search: bool,
    pub highlight_matches: bool,
    pub max_results: Option<u32>,
    pub min_score: Option<f32>,
}

/// Search results with ranking and highlighting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub results: Vec<SearchResult>,
    pub total_matches: u64,
    pub search_time_ms: u64,
    pub suggestions: Vec<String>,
}

/// Individual search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub asset_id: i64,
    pub asset_name: String,
    pub schema_id: i64,
    pub schema_name: String,
    pub score: f32,
    pub matched_fields: Vec<MatchedField>,
    pub metadata_snippet: Value,
}

/// Matched field with highlighting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedField {
    pub field_path: String,
    pub field_value: String,
    pub highlighted_value: Option<String>,
    pub match_score: f32,
}

/// Aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationConfig {
    pub group_by_fields: Vec<String>,
    pub aggregations: Vec<AggregationOperation>,
    pub filters: Option<MetadataQuery>,
    pub having_conditions: Option<Vec<HavingCondition>>,
    pub max_groups: Option<u32>,
}

/// Aggregation operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct AggregationOperation {
    pub name: String,
    pub operation: AggregationType,
    pub field_path: String,
}

/// Types of aggregation operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AggregationType {
    Count,
    Sum,
    Average,
    Min,
    Max,
    StdDev,
    Distinct,
}

/// HAVING clause conditions for aggregations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HavingCondition {
    pub aggregation_name: String,
    pub operator: ComparisonOperator,
    pub value: Value,
}

/// Aggregation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationResults {
    pub groups: Vec<AggregationGroup>,
    pub total_groups: u64,
    pub execution_time_ms: u64,
}

/// Individual aggregation group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationGroup {
    pub group_values: HashMap<String, Value>,
    pub aggregated_values: HashMap<String, Value>,
    pub count: u64,
}

/// Filter definition for reusable filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterDefinition {
    pub name: String,
    pub description: String,
    pub query: MetadataQuery,
    pub is_system: bool,
    pub created_by: i64,
}

/// Metadata filter instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataFilter {
    pub id: i64,
    pub definition: FilterDefinition,
    pub created_at: String,
    pub last_used: Option<String>,
    pub usage_count: u64,
}

/// Saved query for complex searches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedQuery {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub query: MetadataQuery,
    pub created_by: i64,
    pub created_at: String,
    pub last_executed: Option<String>,
    pub execution_count: u64,
}

/// Query parameters for saved queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParameters {
    pub parameter_values: HashMap<String, Value>,
    pub pagination: Option<Pagination>,
    pub include_metadata: Option<bool>,
}

/// Query results with execution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResults {
    pub assets: Vec<Asset>,
    pub metadata: Option<Vec<AssetMetadata>>,
    pub total_matches: u64,
    pub execution_time_ms: u64,
    pub query_plan: Option<String>,
    pub cache_hit: bool,
}

/// Query API implementation
pub struct MetadataQueryApi<'a> {
    conn: &'a Connection,
    metadata_repo: SqliteMetadataRepository<'a>,
    asset_repo: SqliteAssetRepository<'a>,
}

impl<'a> MetadataQueryApi<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self {
            conn,
            metadata_repo: SqliteMetadataRepository::new(conn),
            asset_repo: SqliteAssetRepository::new(conn),
        }
    }

    /// Execute complex metadata query
    pub fn query_assets_by_metadata(&self, query: MetadataQuery) -> Result<Vec<Asset>, String> {
        let start_time = Instant::now();
        
        // Build SQL query from metadata query
        let (sql_query, params) = self.build_metadata_query_sql(&query)?;
        
        // Execute query
        let mut stmt = self.conn.prepare(&sql_query)
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let asset_iter = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            let asset_type_str: String = row.get("asset_type")?;
            let asset_type = crate::assets::AssetType::from_str(&asset_type_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(0, "asset_type".to_string(), rusqlite::types::Type::Text))?;
            
            Ok(crate::assets::Asset {
                id: row.get("id")?,
                name: row.get("name")?,
                asset_type,
                parent_id: row.get("parent_id")?,
                description: row.get("description")?,
                sort_order: row.get("sort_order")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
                created_by: row.get("created_by")?,
            })
        })
        .map_err(|e| format!("Failed to execute query: {}", e))?;

        let mut assets = Vec::new();
        for asset_result in asset_iter {
            assets.push(asset_result.map_err(|e| format!("Failed to parse asset: {}", e))?);
        }

        // Apply pagination if specified
        if let Some(pagination) = &query.pagination {
            let offset = (pagination.page.saturating_sub(1) * pagination.page_size) as usize;
            let limit = pagination.page_size as usize;
            
            if offset < assets.len() {
                let end = std::cmp::min(offset + limit, assets.len());
                assets = assets[offset..end].to_vec();
            } else {
                assets.clear();
            }
        }

        Ok(assets)
    }

    /// Get field statistics
    pub fn get_metadata_field_statistics(&self, config: FieldStatsConfig) -> Result<Vec<FieldStatistics>, String> {
        let mut statistics = Vec::new();
        
        for field_path in &config.field_paths {
            let stats = self.calculate_field_statistics(field_path, &config)?;
            statistics.push(stats);
        }
        
        Ok(statistics)
    }

    /// Search metadata values with full-text search
    pub fn search_metadata_values(&self, config: SearchConfig) -> Result<SearchResults, String> {
        let start_time = Instant::now();
        
        // Build search query
        let (sql_query, params) = self.build_search_query_sql(&config)?;
        
        // Execute search
        let mut stmt = self.conn.prepare(&sql_query)
            .map_err(|e| format!("Failed to prepare search query: {}", e))?;

        let search_iter = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok(SearchResult {
                asset_id: row.get("asset_id")?,
                asset_name: row.get("asset_name")?,
                schema_id: row.get("schema_id")?,
                schema_name: row.get("schema_name")?,
                score: row.get::<_, f64>("score")? as f32,
                matched_fields: Vec::new(), // Would be populated with actual matches
                metadata_snippet: serde_json::from_str(&row.get::<_, String>("metadata_snippet")?)
                    .unwrap_or(Value::Null),
            })
        })
        .map_err(|e| format!("Failed to execute search: {}", e))?;

        let mut results = Vec::new();
        for result in search_iter {
            results.push(result.map_err(|e| format!("Failed to parse search result: {}", e))?);
        }

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(SearchResults {
            total_matches: results.len() as u64,
            results,
            search_time_ms: execution_time,
            suggestions: self.generate_search_suggestions(&config.search_term)?,
        })
    }

    /// Aggregate metadata data
    pub fn aggregate_metadata_data(&self, config: AggregationConfig) -> Result<AggregationResults, String> {
        let start_time = Instant::now();
        
        // Build aggregation query
        let (sql_query, params) = self.build_aggregation_query_sql(&config)?;
        
        // Execute aggregation
        let mut stmt = self.conn.prepare(&sql_query)
            .map_err(|e| format!("Failed to prepare aggregation query: {}", e))?;

        let group_iter = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            let mut group_values = HashMap::new();
            let mut aggregated_values = HashMap::new();
            
            // Extract group by values
            for (i, field) in config.group_by_fields.iter().enumerate() {
                let column_name = format!("group_{}", i);
                let value: Value = serde_json::from_str(&row.get::<_, String>(column_name.as_str())?)
                    .unwrap_or(Value::Null);
                group_values.insert(field.clone(), value);
            }
            
            // Extract aggregated values
            for agg in &config.aggregations {
                let value: Value = match agg.operation {
                    AggregationType::Count => Value::Number(serde_json::Number::from(row.get::<_, i64>(agg.name.as_str())?)),
                    AggregationType::Sum | AggregationType::Average | AggregationType::Min | AggregationType::Max => {
                        Value::Number(serde_json::Number::from_f64(row.get::<_, f64>(agg.name.as_str())?)
                            .unwrap_or_else(|| serde_json::Number::from(0)))
                    }
                    _ => Value::Null,
                };
                aggregated_values.insert(agg.name.clone(), value);
            }
            
            Ok(AggregationGroup {
                group_values,
                aggregated_values,
                count: row.get::<_, i64>("group_count")? as u64,
            })
        })
        .map_err(|e| format!("Failed to execute aggregation: {}", e))?;

        let mut groups = Vec::new();
        for group_result in group_iter {
            groups.push(group_result.map_err(|e| format!("Failed to parse aggregation group: {}", e))?);
        }

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(AggregationResults {
            total_groups: groups.len() as u64,
            groups,
            execution_time_ms: execution_time,
        })
    }

    /// Create metadata filter
    pub fn create_metadata_filter(&self, definition: FilterDefinition) -> Result<MetadataFilter, String> {
        let query = "INSERT INTO metadata_filters (name, description, query_json, is_system, created_by, created_at) 
                     VALUES (?, ?, ?, ?, ?, datetime('now'))";
        
        let query_json = serde_json::to_string(&definition.query)
            .map_err(|e| format!("Failed to serialize query: {}", e))?;
        
        let filter_id = self.conn.execute(query, rusqlite::params![
            definition.name,
            definition.description,
            query_json,
            definition.is_system,
            definition.created_by
        ])
        .map_err(|e| format!("Failed to create filter: {}", e))?;

        Ok(MetadataFilter {
            id: self.conn.last_insert_rowid(),
            definition,
            created_at: chrono::Utc::now().to_rfc3339(),
            last_used: None,
            usage_count: 0,
        })
    }

    /// Apply metadata filter to assets
    pub fn apply_metadata_filter(&self, filter_id: u32, assets: Vec<u32>) -> Result<Vec<Asset>, String> {
        let filter_id = filter_id as i64;
        
        // Get filter definition
        let query = "SELECT name, description, query_json, is_system, created_by FROM metadata_filters WHERE id = ?";
        let mut stmt = self.conn.prepare(query)
            .map_err(|e| format!("Failed to prepare filter query: {}", e))?;

        let filter_row = stmt.query_row(rusqlite::params![filter_id], |row| {
            let query_json: String = row.get("query_json")?;
            let query: MetadataQuery = serde_json::from_str(&query_json)
                .map_err(|e| rusqlite::Error::InvalidColumnType(0, "query_json".to_string(), rusqlite::types::Type::Text))?;
            
            Ok(FilterDefinition {
                name: row.get("name")?,
                description: row.get("description")?,
                query,
                is_system: row.get("is_system")?,
                created_by: row.get("created_by")?,
            })
        })
        .map_err(|e| format!("Failed to get filter: {}", e))?;

        // Apply filter to specified assets
        let asset_ids: Vec<i64> = assets.iter().map(|&id| id as i64).collect();
        let filtered_assets = self.filter_assets_by_query(&filter_row.query, &asset_ids)?;

        // Update usage count
        let update_query = "UPDATE metadata_filters SET usage_count = usage_count + 1, last_used = datetime('now') WHERE id = ?";
        self.conn.execute(update_query, rusqlite::params![filter_id])
            .map_err(|e| format!("Failed to update filter usage: {}", e))?;

        Ok(filtered_assets)
    }

    /// Save a metadata query for reuse
    pub fn save_metadata_query(&self, name: String, query: MetadataQuery, created_by: i64) -> Result<SavedQuery, String> {
        let query_json = serde_json::to_string(&query)
            .map_err(|e| format!("Failed to serialize query: {}", e))?;
        
        let insert_query = "INSERT INTO saved_metadata_queries (name, description, query_json, created_by, created_at) 
                           VALUES (?, ?, ?, ?, datetime('now'))";
        
        self.conn.execute(insert_query, rusqlite::params![
            name.clone(),
            format!("Saved query: {}", name),
            query_json,
            created_by
        ])
        .map_err(|e| format!("Failed to save query: {}", e))?;

        Ok(SavedQuery {
            id: self.conn.last_insert_rowid(),
            name,
            description: format!("Saved query"),
            query,
            created_by,
            created_at: chrono::Utc::now().to_rfc3339(),
            last_executed: None,
            execution_count: 0,
        })
    }

    /// Execute saved query
    pub fn execute_saved_query(&self, query_id: u32, parameters: QueryParameters) -> Result<QueryResults, String> {
        let query_id = query_id as i64;
        let start_time = Instant::now();
        
        // Get saved query
        let get_query = "SELECT name, description, query_json, created_by FROM saved_metadata_queries WHERE id = ?";
        let mut stmt = self.conn.prepare(get_query)
            .map_err(|e| format!("Failed to prepare saved query retrieval: {}", e))?;

        let mut saved_query: MetadataQuery = stmt.query_row(rusqlite::params![query_id], |row| {
            let query_json: String = row.get("query_json")?;
            serde_json::from_str(&query_json)
                .map_err(|e| rusqlite::Error::InvalidColumnType(0, "query_json".to_string(), rusqlite::types::Type::Text))
        })
        .map_err(|e| format!("Failed to get saved query: {}", e))?;

        // Apply parameters if provided
        if let Some(pagination) = parameters.pagination {
            saved_query.pagination = Some(pagination);
        }
        
        if let Some(include_metadata) = parameters.include_metadata {
            saved_query.include_metadata = include_metadata;
        }

        // Execute query
        let assets = self.query_assets_by_metadata(saved_query)?;
        let execution_time = start_time.elapsed().as_millis() as u64;

        // Update execution count
        let update_query = "UPDATE saved_metadata_queries SET execution_count = execution_count + 1, last_executed = datetime('now') WHERE id = ?";
        self.conn.execute(update_query, rusqlite::params![query_id])
            .map_err(|e| format!("Failed to update query execution count: {}", e))?;

        Ok(QueryResults {
            total_matches: assets.len() as u64,
            assets,
            metadata: None, // Would populate if include_metadata is true
            execution_time_ms: execution_time,
            query_plan: None,
            cache_hit: false,
        })
    }

    // Helper methods
    fn build_metadata_query_sql(&self, query: &MetadataQuery) -> Result<(String, Vec<String>), String> {
        let mut conditions = Vec::new();
        let mut params = Vec::new();
        
        for condition in &query.conditions {
            let (condition_sql, condition_params) = self.build_condition_sql(condition)?;
            conditions.push(condition_sql);
            params.extend(condition_params);
        }
        
        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            let operator = match query.logic_operator {
                LogicOperator::And => " AND ",
                LogicOperator::Or => " OR ",
                LogicOperator::Not => " AND NOT ",
            };
            format!("WHERE {}", conditions.join(operator))
        };
        
        let sql = format!(
            "SELECT a.id, a.name, a.asset_type, a.parent_id, a.description, a.status, 
                    a.created_at, a.updated_at, a.created_by, a.path, a.is_folder
             FROM assets a 
             JOIN asset_metadata am ON a.id = am.asset_id 
             {}",
            where_clause
        );
        
        Ok((sql, params))
    }

    fn build_condition_sql(&self, condition: &QueryCondition) -> Result<(String, Vec<String>), String> {
        let field_json_path = format!("json_extract(am.metadata_values_json, '$.{}')", condition.field_path);
        
        match condition.operator {
            ComparisonOperator::Equals => {
                Ok((format!("{} = ?", field_json_path), vec![condition.value.to_string()]))
            }
            ComparisonOperator::NotEquals => {
                Ok((format!("{} != ?", field_json_path), vec![condition.value.to_string()]))
            }
            ComparisonOperator::Contains => {
                let pattern = if condition.case_sensitive {
                    format!("%{}%", condition.value.as_str().unwrap_or(""))
                } else {
                    format!("%{}%", condition.value.as_str().unwrap_or("").to_lowercase())
                };
                let sql = if condition.case_sensitive {
                    format!("{} LIKE ?", field_json_path)
                } else {
                    format!("LOWER({}) LIKE ?", field_json_path)
                };
                Ok((sql, vec![pattern]))
            }
            ComparisonOperator::GreaterThan => {
                Ok((format!("CAST({} AS REAL) > ?", field_json_path), vec![condition.value.to_string()]))
            }
            ComparisonOperator::LessThan => {
                Ok((format!("CAST({} AS REAL) < ?", field_json_path), vec![condition.value.to_string()]))
            }
            ComparisonOperator::IsNull => {
                Ok((format!("{} IS NULL", field_json_path), vec![]))
            }
            ComparisonOperator::IsNotNull => {
                Ok((format!("{} IS NOT NULL", field_json_path), vec![]))
            }
            _ => Err(format!("Unsupported operator: {:?}", condition.operator)),
        }
    }

    fn build_search_query_sql(&self, config: &SearchConfig) -> Result<(String, Vec<String>), String> {
        let search_pattern = if config.fuzzy_search {
            format!("%{}%", config.search_term)
        } else {
            config.search_term.clone()
        };
        
        let sql = "SELECT am.asset_id, a.name as asset_name, am.schema_id, ms.name as schema_name,
                         1.0 as score, am.metadata_values_json as metadata_snippet
                  FROM asset_metadata am
                  JOIN assets a ON am.asset_id = a.id
                  JOIN metadata_schemas ms ON am.schema_id = ms.id
                  WHERE am.metadata_values_json LIKE ?";
        
        Ok((sql.to_string(), vec![search_pattern]))
    }

    fn build_aggregation_query_sql(&self, config: &AggregationConfig) -> Result<(String, Vec<String>), String> {
        // This is a simplified implementation
        // A full implementation would need to handle complex GROUP BY and aggregation functions
        let sql = "SELECT COUNT(*) as group_count FROM asset_metadata am";
        Ok((sql.to_string(), vec![]))
    }

    fn calculate_field_statistics(&self, field_path: &str, config: &FieldStatsConfig) -> Result<FieldStatistics, String> {
        // Simplified implementation
        Ok(FieldStatistics {
            field_path: field_path.to_string(),
            total_values: 0,
            unique_values: 0,
            null_count: 0,
            data_type_distribution: HashMap::new(),
            value_distribution: None,
            numeric_stats: None,
            trends: None,
        })
    }

    fn generate_search_suggestions(&self, search_term: &str) -> Result<Vec<String>, String> {
        // Simplified implementation - would use FTS or similar for real suggestions
        Ok(vec![
            format!("{}*", search_term),
            format!("*{}", search_term),
            format!("*{}*", search_term),
        ])
    }

    fn filter_assets_by_query(&self, query: &MetadataQuery, asset_ids: &[i64]) -> Result<Vec<Asset>, String> {
        let mut filtered_assets = Vec::new();
        
        for &asset_id in asset_ids {
            // Simplified filtering logic
            if let Ok(Some(asset)) = self.asset_repo.get_asset_by_id(asset_id) {
                filtered_assets.push(asset);
            }
        }
        
        Ok(filtered_assets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_metadata_query_serialization() {
        let query = MetadataQuery {
            conditions: vec![
                QueryCondition {
                    field_path: "name".to_string(),
                    operator: ComparisonOperator::Contains,
                    value: json!("test"),
                    case_sensitive: false,
                }
            ],
            logic_operator: LogicOperator::And,
            pagination: Some(Pagination::default()),
            include_metadata: true,
            include_schema: false,
        };

        let serialized = serde_json::to_string(&query).unwrap();
        let deserialized: MetadataQuery = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.conditions.len(), 1);
        assert_eq!(deserialized.conditions[0].field_path, "name");
    }

    #[test]
    fn test_comparison_operators() {
        let operators = vec![
            ComparisonOperator::Equals,
            ComparisonOperator::Contains,
            ComparisonOperator::GreaterThan,
            ComparisonOperator::IsNull,
        ];

        for op in operators {
            let serialized = serde_json::to_string(&op).unwrap();
            let _deserialized: ComparisonOperator = serde_json::from_str(&serialized).unwrap();
        }
    }

    #[test]
    fn test_field_statistics_structure() {
        let stats = FieldStatistics {
            field_path: "test.field".to_string(),
            total_values: 100,
            unique_values: 75,
            null_count: 5,
            data_type_distribution: HashMap::from([
                ("string".to_string(), 80),
                ("number".to_string(), 15),
            ]),
            value_distribution: None,
            numeric_stats: Some(NumericStats {
                min: 0.0,
                max: 100.0,
                mean: 50.0,
                median: 45.0,
                std_deviation: 25.0,
                percentiles: HashMap::from([
                    ("50".to_string(), 45.0),
                    ("95".to_string(), 95.0),
                ]),
            }),
            trends: None,
        };

        assert_eq!(stats.field_path, "test.field");
        assert_eq!(stats.total_values, 100);
        assert!(stats.numeric_stats.is_some());
    }
}