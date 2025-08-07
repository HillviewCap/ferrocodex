// This file contains the missing type definitions for the metadata API
// These will be added to mod.rs

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Fix results for metadata validation issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixResults {
    pub fixes_applied: Vec<AppliedFix>,
    pub fixes_failed: Vec<FailedFix>,
    pub validation_result: ValidationResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedFix {
    pub field: String,
    pub fix_type: String,
    pub old_value: Option<Value>,
    pub new_value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedFix {
    pub field: String,
    pub fix_type: String,
    pub error: String,
}

/// Metadata relationship between assets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataRelationship {
    pub from_asset_id: i64,
    pub to_asset_id: i64,
    pub relationship_type: String,
    pub shared_fields: Vec<String>,
    pub similarity_score: f32,
}

/// Similar asset based on metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarAsset {
    pub asset_id: i64,
    pub asset_name: String,
    pub similarity_score: f32,
    pub matching_fields: Vec<String>,
    pub asset_type: String,
}

/// Schema dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDependency {
    pub schema_id: i64,
    pub depends_on_schema_id: i64,
    pub dependency_type: String,
    pub field_mappings: Vec<FieldMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    pub source_field: String,
    pub target_field: String,
    pub mapping_type: String,
}

/// Time period for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimePeriod {
    pub start_date: String,
    pub end_date: String,
    pub granularity: TimeGranularity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeGranularity {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// Usage analytics for metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageAnalytics {
    pub period: TimePeriod,
    pub total_schemas: u32,
    pub active_schemas: u32,
    pub total_metadata_records: u32,
    pub field_usage_stats: HashMap<String, FieldUsageStats>,
    pub schema_usage_stats: Vec<SchemaUsageStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldUsageStats {
    pub field_name: String,
    pub usage_count: u32,
    pub unique_values: u32,
    pub validation_errors: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaUsageStats {
    pub schema_id: i64,
    pub schema_name: String,
    pub usage_count: u32,
    pub assets_using: u32,
    pub validation_success_rate: f32,
}