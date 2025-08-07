use serde::{Deserialize, Serialize};
use crate::metadata::{AssetMetadataSchema, AssetMetadata};
use serde_json::Value;
use std::collections::HashMap;

// API modules
pub mod crud;
pub mod query;
pub mod bulk;
pub mod export;
pub mod integration;

// Re-export key types
pub use crud::*;
pub use query::*;
pub use bulk::*;
pub use export::*;
pub use integration::*;

/// Validation result for API operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn success() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn with_errors(errors: Vec<ValidationError>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    pub fn with_warnings(warnings: Vec<String>) -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings,
        }
    }
}

/// Validation error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub error_type: String,
}

/// Pagination parameters for API requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub page: u32,
    pub page_size: u32,
    pub sort_field: Option<String>,
    pub sort_direction: Option<SortDirection>,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
            sort_field: None,
            sort_direction: Some(SortDirection::Asc),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    Asc,
    Desc,
}

/// Filters for schema listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaFilters {
    pub name_contains: Option<String>,
    pub asset_type: Option<String>,
    pub created_by: Option<i64>,
    pub is_system_template: Option<bool>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
}

/// Schema list response with pagination info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaList {
    pub schemas: Vec<AssetMetadataSchema>,
    pub total_count: u32,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

/// Options for schema duplication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicationOptions {
    pub new_name: String,
    pub new_description: Option<String>,
    pub copy_usage_stats: bool,
    pub mark_as_template: bool,
}

/// Full asset metadata with history and relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadataFull {
    pub current: Option<AssetMetadata>,
    pub schema: Option<AssetMetadataSchema>,
    pub history: Vec<AssetMetadataHistory>,
    pub validation_status: ValidationResult,
    pub related_assets: Vec<RelatedAsset>,
}

/// Asset metadata history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadataHistory {
    pub id: i64,
    pub asset_id: i64,
    pub schema_id: i64,
    pub metadata_values_json: String,
    pub schema_version: i32,
    pub created_at: String,
    pub created_by: i64,
    pub change_reason: Option<String>,
    pub is_current: bool,
}

/// Related asset information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedAsset {
    pub asset_id: i64,
    pub asset_name: String,
    pub relationship_type: String,
    pub similarity_score: f32,
}

/// Partial metadata update structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialMetadata {
    pub schema_id: Option<i64>,
    pub field_updates: Vec<FieldUpdate>,
    pub preserve_history: bool,
    pub change_reason: Option<String>,
}

/// Individual field update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldUpdate {
    pub field_path: String,
    pub new_value: Value,
    pub operation: UpdateOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateOperation {
    Set,
    Delete,
    Append,
    Merge,
}

/// Options for copying metadata between assets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyOptions {
    pub schema_id: Option<i64>,
    pub include_fields: Option<Vec<String>>,
    pub exclude_fields: Option<Vec<String>>,
    pub overwrite_existing: bool,
    pub preserve_timestamps: bool,
}

/// Metadata validation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataValidationRequest {
    pub asset_id: Option<i64>,
    pub schema_id: i64,
    pub metadata_values: Value,
    pub context: Option<String>,
}

/// Test case for schema validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub name: String,
    pub input_values: Value,
    pub expected_valid: bool,
    pub expected_errors: Option<Vec<String>>,
}

/// Test results for schema validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResults {
    pub test_name: String,
    pub total_tests: u32,
    pub passed_tests: u32,
    pub failed_tests: u32,
    pub results: Vec<TestCaseResult>,
}

/// Individual test case result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseResult {
    pub test_name: String,
    pub passed: bool,
    pub expected: bool,
    pub actual_errors: Vec<ValidationError>,
    pub execution_time_ms: u64,
}

/// Fix results for metadata validation issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixResults {
    pub asset_id: i64,
    pub issues_found: u32,
    pub issues_fixed: u32,
    pub fixes_applied: Vec<AppliedFix>,
    pub remaining_issues: Vec<ValidationError>,
}

/// Applied fix information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedFix {
    pub field_path: String,
    pub issue_type: String,
    pub fix_description: String,
    pub old_value: Option<Value>,
    pub new_value: Option<Value>,
}

/// Metadata relationship information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataRelationship {
    pub source_asset_id: i64,
    pub target_asset_id: i64,
    pub relationship_type: String,
    pub strength: f32,
    pub shared_fields: Vec<String>,
    pub metadata_context: Value,
}

/// Similar asset information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarAsset {
    pub asset_id: i64,
    pub asset_name: String,
    pub similarity_score: f32,
    pub matching_fields: Vec<String>,
    pub shared_schema_id: Option<i64>,
    pub metadata_summary: Value,
}

/// Schema dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDependency {
    pub schema_id: i64,
    pub schema_name: String,
    pub dependency_type: String,
    pub referenced_fields: Vec<String>,
    pub usage_count: i64,
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
    Hour,
    Day,
    Week,
    Month,
}

/// Usage analytics information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageAnalytics {
    pub period: TimePeriod,
    pub total_operations: u64,
    pub schema_usage: Vec<SchemaUsageMetric>,
    pub field_usage: Vec<FieldUsageMetric>,
    pub performance_metrics: PerformanceMetrics,
}

/// Schema usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaUsageMetric {
    pub schema_id: i64,
    pub schema_name: String,
    pub usage_count: u64,
    pub asset_count: u64,
    pub last_used: String,
}

/// Field usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldUsageMetric {
    pub field_name: String,
    pub usage_count: u64,
    pub schemas_count: u64,
    pub average_update_frequency: f32,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub average_response_time_ms: f32,
    pub peak_response_time_ms: u64,
    pub throughput_ops_per_second: f32,
    pub error_rate_percent: f32,
}

/// API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: String,
    pub request_id: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            request_id: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: chrono::Utc::now().to_rfc3339(),
            request_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_pagination_default() {
        let pagination = Pagination::default();
        assert_eq!(pagination.page, 1);
        assert_eq!(pagination.page_size, 20);
    }

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success("test data".to_string());
        assert!(response.success);
        assert_eq!(response.data, Some("test data".to_string()));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let response: ApiResponse<String> = ApiResponse::error("test error".to_string());
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("test error".to_string()));
    }

    #[test] 
    fn test_field_update_serialization() {
        let update = FieldUpdate {
            field_path: "name".to_string(),
            new_value: json!("new value"),
            operation: UpdateOperation::Set,
        };
        
        let serialized = serde_json::to_string(&update).unwrap();
        let deserialized: FieldUpdate = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.field_path, "name");
        assert_eq!(deserialized.new_value, json!("new value"));
    }
}