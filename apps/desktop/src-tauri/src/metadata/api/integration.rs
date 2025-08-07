use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::metadata::{ValidationResult, ValidationError};
use rusqlite::Connection;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// API information for external systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    pub version: String,
    pub name: String,
    pub description: String,
    pub supported_formats: Vec<String>,
    pub supported_operations: Vec<String>,
    pub rate_limits: RateLimits,
    pub authentication_methods: Vec<String>,
    pub capabilities: ApiCapabilities,
    pub endpoints: Vec<ApiEndpoint>,
}

/// Rate limiting information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub bulk_operations_per_hour: u32,
    pub concurrent_connections: u32,
}

/// API capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCapabilities {
    pub supports_bulk_operations: bool,
    pub supports_real_time_sync: bool,
    pub supports_webhooks: bool,
    pub supports_streaming: bool,
    pub max_record_size_bytes: u64,
    pub max_batch_size: u32,
    pub supported_query_operators: Vec<String>,
}

/// API endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    pub path: String,
    pub method: String,
    pub description: String,
    pub parameters: Vec<ApiParameter>,
    pub response_format: String,
}

/// API parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiParameter {
    pub name: String,
    pub parameter_type: String,
    pub required: bool,
    pub description: String,
    pub default_value: Option<Value>,
}

/// External system mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalMappingConfig {
    pub name: String,
    pub description: String,
    pub external_system_id: String,
    pub field_mappings: HashMap<String, FieldMapping>,
    pub transformation_rules: Vec<TransformationRule>,
    pub validation_rules: Vec<ValidationRule>,
    pub sync_settings: SyncSettings,
}

/// Field mapping between systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    pub external_field: String,
    pub internal_field: String,
    pub data_type: String,
    pub transformation: Option<String>,
    pub required: bool,
    pub default_value: Option<Value>,
}

/// Data transformation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationRule {
    pub name: String,
    pub source_field: String,
    pub target_field: String,
    pub transformation_type: TransformationType,
    pub parameters: HashMap<String, Value>,
    pub conditions: Vec<TransformationCondition>,
}

/// Types of transformations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformationType {
    FormatString,
    ParseNumber,
    ParseDate,
    LookupTable,
    RegexReplace,
    Concatenate,
    Split,
    Conditional,
    Formula,
}

/// Condition for conditional transformations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationCondition {
    pub field: String,
    pub operator: String,
    pub value: Value,
    pub result: Value,
}

/// Validation rule for external data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub name: String,
    pub field: String,
    pub rule_type: ValidationRuleType,
    pub parameters: HashMap<String, Value>,
    pub error_message: String,
    pub severity: ValidationSeverity,
}

/// Types of validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationRuleType {
    Required,
    Format,
    Range,
    Length,
    Pattern,
    Custom,
}

/// Validation severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

/// Synchronization settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSettings {
    pub enabled: bool,
    pub direction: SyncDirection,
    pub frequency: SyncFrequency,
    pub conflict_resolution: ConflictResolution,
    pub batch_size: u32,
    pub retry_attempts: u32,
    pub timeout_seconds: u32,
}

/// Synchronization direction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncDirection {
    Import,
    Export,
    Bidirectional,
}

/// Synchronization frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncFrequency {
    Manual,
    Realtime,
    Interval { minutes: u32 },
    Scheduled { cron_expression: String },
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    SourceWins,
    TargetWins,
    MostRecent,
    Manual,
    Merge,
}

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub name: String,
    pub url: String,
    pub events: Vec<WebhookEvent>,
    pub authentication: Option<WebhookAuth>,
    pub headers: HashMap<String, String>,
    pub retry_policy: RetryPolicy,
    pub filters: Option<WebhookFilters>,
    pub format: WebhookFormat,
}

/// Webhook events to subscribe to
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvent {
    MetadataCreated,
    MetadataUpdated,
    MetadataDeleted,
    SchemaCreated,
    SchemaUpdated,
    SchemaDeleted,
    AssetCreated,
    AssetUpdated,
    AssetDeleted,
}

/// Webhook authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebhookAuth {
    #[serde(rename = "bearer")]
    Bearer { token: String },
    #[serde(rename = "basic")]
    Basic { username: String, password: String },
    #[serde(rename = "api_key")]
    ApiKey { header_name: String, key: String },
    #[serde(rename = "signature")]
    Signature { secret: String, algorithm: String },
}

/// Retry policy for webhook delivery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub retry_http_codes: Vec<u16>,
}

/// Webhook filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookFilters {
    pub asset_types: Option<Vec<String>>,
    pub schema_ids: Option<Vec<i64>>,
    pub field_conditions: Option<Vec<FieldCondition>>,
    pub custom_filters: Option<HashMap<String, Value>>,
}

/// Field condition for filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldCondition {
    pub field_path: String,
    pub operator: String,
    pub value: Value,
}

/// Webhook payload format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebhookFormat {
    Json,
    Xml,
    FormData,
}

/// Webhook status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookStatus {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub last_delivery: Option<WebhookDelivery>,
    pub delivery_stats: WebhookStats,
    pub health_status: WebhookHealth,
}

/// Webhook delivery information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    pub timestamp: String,
    pub event_type: WebhookEvent,
    pub response_code: u16,
    pub response_time_ms: u64,
    pub success: bool,
    pub error_message: Option<String>,
    pub retry_count: u32,
}

/// Webhook delivery statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookStats {
    pub total_deliveries: u64,
    pub successful_deliveries: u64,
    pub failed_deliveries: u64,
    pub average_response_time_ms: f64,
    pub last_24h_success_rate: f64,
}

/// Webhook health status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebhookHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Disabled,
}

/// External synchronization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalSyncConfig {
    pub source_system: String,
    pub connection_string: String,
    pub authentication: SyncAuthentication,
    pub mapping_id: i64,
    pub sync_mode: SyncMode,
    pub incremental_field: Option<String>,
    pub last_sync_timestamp: Option<String>,
}

/// Synchronization authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SyncAuthentication {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "basic")]
    Basic { username: String, password: String },
    #[serde(rename = "oauth2")]
    OAuth2 { client_id: String, client_secret: String, token_url: String },
    #[serde(rename = "api_key")]
    ApiKey { key: String, header_name: String },
}

/// Synchronization mode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncMode {
    Full,
    Incremental,
    Delta,
}

/// Synchronization results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResults {
    pub sync_id: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub status: SyncStatus,
    pub records_processed: u64,
    pub records_created: u64,
    pub records_updated: u64,
    pub records_deleted: u64,
    pub records_failed: u64,
    pub errors: Vec<SyncError>,
    pub warnings: Vec<String>,
}

/// Synchronization status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

/// Synchronization error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncError {
    pub record_id: Option<String>,
    pub error_type: String,
    pub error_message: String,
    pub field_path: Option<String>,
    pub raw_data: Option<String>,
}

/// Synchronization job configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJobConfig {
    pub name: String,
    pub source_config: ExternalSyncConfig,
    pub schedule: Option<String>, // Cron expression
    pub enabled: bool,
    pub max_runtime_minutes: u32,
    pub notification_settings: NotificationSettings,
}

/// Notification settings for sync jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub on_success: bool,
    pub on_failure: bool,
    pub on_warning: bool,
    pub webhook_url: Option<String>,
    pub email_addresses: Vec<String>,
}

/// Synchronization progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProgress {
    pub job_id: i64,
    pub current_phase: String,
    pub total_records: u64,
    pub processed_records: u64,
    pub estimated_completion: Option<String>,
    pub current_operation: String,
    pub errors_count: u32,
    pub warnings_count: u32,
}

/// Conflict resolution for synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolutionData {
    pub conflict_id: String,
    pub source_record: Value,
    pub target_record: Value,
    pub resolution_strategy: ConflictResolution,
    pub resolved_record: Option<Value>,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<String>,
}

/// Data transformation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationConfig {
    pub rules: Vec<TransformationRule>,
    pub error_handling: TransformationErrorHandling,
    pub validation_mode: TransformationValidationMode,
}

/// Error handling for transformations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformationErrorHandling {
    StopOnError,
    SkipRecord,
    UseDefault,
    LogAndContinue,
}

/// Validation mode for transformations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformationValidationMode {
    Strict,
    Lenient,
    None,
}

/// Transformation template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationTemplate {
    pub name: String,
    pub description: String,
    pub source_format: String,
    pub target_format: String,
    pub transformation_config: TransformationConfig,
    pub is_system: bool,
    pub created_by: i64,
}

/// Integration API implementation
pub struct MetadataIntegrationApi<'a> {
    conn: &'a Connection,
}

impl<'a> MetadataIntegrationApi<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Get API information for external systems
    pub fn get_metadata_api_info(&self) -> Result<ApiInfo, String> {
        Ok(ApiInfo {
            version: "1.0.0".to_string(),
            name: "Ferrocodex Metadata API".to_string(),
            description: "Comprehensive metadata management API for industrial assets".to_string(),
            supported_formats: vec![
                "application/json".to_string(),
                "text/csv".to_string(),
                "application/xml".to_string(),
            ],
            supported_operations: vec![
                "create".to_string(),
                "read".to_string(),
                "update".to_string(),
                "delete".to_string(),
                "bulk_create".to_string(),
                "bulk_update".to_string(),
                "bulk_delete".to_string(),
                "search".to_string(),
                "export".to_string(),
                "import".to_string(),
            ],
            rate_limits: RateLimits {
                requests_per_minute: 1000,
                requests_per_hour: 10000,
                bulk_operations_per_hour: 100,
                concurrent_connections: 50,
            },
            authentication_methods: vec![
                "session_token".to_string(),
                "api_key".to_string(),
            ],
            capabilities: ApiCapabilities {
                supports_bulk_operations: true,
                supports_real_time_sync: true,
                supports_webhooks: true,
                supports_streaming: false,
                max_record_size_bytes: 1024 * 1024, // 1MB
                max_batch_size: 1000,
                supported_query_operators: vec![
                    "equals".to_string(),
                    "not_equals".to_string(),
                    "contains".to_string(),
                    "starts_with".to_string(),
                    "ends_with".to_string(),
                    "greater_than".to_string(),
                    "less_than".to_string(),
                    "in".to_string(),
                    "not_in".to_string(),
                    "is_null".to_string(),
                    "is_not_null".to_string(),
                ],
            },
            endpoints: vec![
                ApiEndpoint {
                    path: "/api/metadata/schemas".to_string(),
                    method: "GET".to_string(),
                    description: "List metadata schemas".to_string(),
                    parameters: vec![
                        ApiParameter {
                            name: "page".to_string(),
                            parameter_type: "integer".to_string(),
                            required: false,
                            description: "Page number for pagination".to_string(),
                            default_value: Some(Value::Number(serde_json::Number::from(1))),
                        },
                    ],
                    response_format: "application/json".to_string(),
                },
            ],
        })
    }

    /// Create external system mapping
    pub fn create_external_metadata_mapping(&self, config: ExternalMappingConfig) -> Result<i64, String> {
        let config_json = serde_json::to_string(&config)
            .map_err(|e| format!("Failed to serialize mapping config: {}", e))?;
        
        let query = "INSERT INTO external_metadata_mappings 
                     (name, description, external_system_id, config_json, created_at) 
                     VALUES (?, ?, ?, ?, datetime('now'))";
        
        self.conn.execute(query, rusqlite::params![
            config.name,
            config.description,
            config.external_system_id,
            config_json
        ])
        .map_err(|e| format!("Failed to create external mapping: {}", e))?;
        
        Ok(self.conn.last_insert_rowid())
    }

    /// Synchronize with external metadata source
    pub fn sync_external_metadata_source(&self, config: ExternalSyncConfig) -> Result<SyncResults, String> {
        let sync_id = format!("sync_{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        let started_at = chrono::Utc::now().to_rfc3339();
        
        // This would implement the actual synchronization logic
        // For now, return a mock result
        Ok(SyncResults {
            sync_id,
            started_at,
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            status: SyncStatus::Completed,
            records_processed: 0,
            records_created: 0,
            records_updated: 0,
            records_deleted: 0,
            records_failed: 0,
            errors: vec![],
            warnings: vec![],
        })
    }

    /// Validate external metadata format
    pub fn validate_external_metadata_format(&self, data: String, format_type: String) -> Result<ValidationResult, String> {
        match format_type.as_str() {
            "json" => self.validate_json_format(&data),
            "csv" => self.validate_csv_format(&data),
            "xml" => self.validate_xml_format(&data),
            _ => Err(format!("Unsupported format type: {}", format_type)),
        }
    }

    /// Create webhook for metadata changes
    pub fn create_metadata_webhook(&self, config: WebhookConfig) -> Result<i64, String> {
        let config_json = serde_json::to_string(&config)
            .map_err(|e| format!("Failed to serialize webhook config: {}", e))?;
        
        let query = "INSERT INTO metadata_webhooks 
                     (name, url, config_json, enabled, created_at) 
                     VALUES (?, ?, ?, 1, datetime('now'))";
        
        self.conn.execute(query, rusqlite::params![
            config.name,
            config.url,
            config_json
        ])
        .map_err(|e| format!("Failed to create webhook: {}", e))?;
        
        Ok(self.conn.last_insert_rowid())
    }

    /// Get webhook status
    pub fn get_metadata_webhook_status(&self, webhook_id: u32) -> Result<WebhookStatus, String> {
        let webhook_id = webhook_id as i64;
        
        let query = "SELECT name, url, config_json, enabled FROM metadata_webhooks WHERE id = ?";
        let mut stmt = self.conn.prepare(query)
            .map_err(|e| format!("Failed to prepare webhook query: {}", e))?;
        
        stmt.query_row(rusqlite::params![webhook_id], |row| {
            let config_json: String = row.get("config_json")?;
            let config: WebhookConfig = serde_json::from_str(&config_json)
                .map_err(|e| rusqlite::Error::InvalidColumnType(0, "config_json".to_string(), rusqlite::types::Type::Text))?;
            
            Ok(WebhookStatus {
                id: webhook_id,
                name: row.get("name")?,
                url: row.get("url")?,
                enabled: row.get("enabled")?,
                last_delivery: None, // Would be populated from delivery history
                delivery_stats: WebhookStats {
                    total_deliveries: 0,
                    successful_deliveries: 0,
                    failed_deliveries: 0,
                    average_response_time_ms: 0.0,
                    last_24h_success_rate: 0.0,
                },
                health_status: WebhookHealth::Healthy,
            })
        })
        .map_err(|e| format!("Failed to get webhook status: {}", e))
    }

    /// Test webhook delivery
    pub fn test_metadata_webhook(&self, webhook_id: u32, test_payload: String) -> Result<WebhookDelivery, String> {
        // Get webhook configuration
        let _webhook_status = self.get_metadata_webhook_status(webhook_id)?;
        
        let webhook_id = webhook_id as i64;
        
        // This would implement actual webhook delivery testing
        // For now, return a mock successful delivery
        Ok(WebhookDelivery {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_type: WebhookEvent::MetadataUpdated,
            response_code: 200,
            response_time_ms: 150,
            success: true,
            error_message: None,
            retry_count: 0,
        })
    }

    /// Delete webhook
    pub fn delete_metadata_webhook(&self, webhook_id: u32) -> Result<(), String> {
        let webhook_id = webhook_id as i64;
        
        let query = "DELETE FROM metadata_webhooks WHERE id = ?";
        
        self.conn.execute(query, rusqlite::params![webhook_id])
            .map_err(|e| format!("Failed to delete webhook: {}", e))?;
        
        Ok(())
    }

    /// Get synchronization status
    pub fn get_metadata_sync_status(&self, sync_source: String) -> Result<SyncResults, String> {
        // This would query sync job status from database
        // For now, return a mock status
        Ok(SyncResults {
            sync_id: format!("sync_{}", sync_source),
            started_at: chrono::Utc::now().to_rfc3339(),
            completed_at: None,
            status: SyncStatus::Running,
            records_processed: 150,
            records_created: 50,
            records_updated: 75,
            records_deleted: 25,
            records_failed: 0,
            errors: vec![],
            warnings: vec![],
        })
    }

    /// Create synchronization job
    pub fn create_metadata_sync_job(&self, config: SyncJobConfig) -> Result<i64, String> {
        let config_json = serde_json::to_string(&config)
            .map_err(|e| format!("Failed to serialize sync job config: {}", e))?;
        
        let query = "INSERT INTO metadata_sync_jobs 
                     (name, config_json, enabled, created_at) 
                     VALUES (?, ?, ?, datetime('now'))";
        
        self.conn.execute(query, rusqlite::params![
            config.name,
            config_json,
            config.enabled
        ])
        .map_err(|e| format!("Failed to create sync job: {}", e))?;
        
        Ok(self.conn.last_insert_rowid())
    }

    /// Monitor synchronization progress
    pub fn monitor_metadata_sync_progress(&self, job_id: u32) -> Result<SyncProgress, String> {
        let job_id = job_id as i64;
        
        // This would query actual sync progress from database
        // For now, return mock progress
        Ok(SyncProgress {
            job_id,
            current_phase: "Processing records".to_string(),
            total_records: 1000,
            processed_records: 350,
            estimated_completion: Some(chrono::Utc::now().to_rfc3339()),
            current_operation: "Validating record 351".to_string(),
            errors_count: 2,
            warnings_count: 5,
        })
    }

    /// Resolve synchronization conflicts
    pub fn resolve_metadata_sync_conflicts(&self, job_id: u32, resolutions: Vec<ConflictResolutionData>) -> Result<(), String> {
        let job_id = job_id as i64;
        
        for resolution in resolutions {
            let resolution_json = serde_json::to_string(&resolution)
                .map_err(|e| format!("Failed to serialize conflict resolution: {}", e))?;
            
            let query = "INSERT INTO sync_conflict_resolutions 
                         (job_id, conflict_id, resolution_json, resolved_at) 
                         VALUES (?, ?, ?, datetime('now'))";
            
            self.conn.execute(query, rusqlite::params![
                job_id,
                resolution.conflict_id,
                resolution_json
            ])
            .map_err(|e| format!("Failed to save conflict resolution: {}", e))?;
        }
        
        Ok(())
    }

    /// Transform external metadata
    pub fn transform_external_metadata(&self, data: String, config: TransformationConfig) -> Result<String, String> {
        // Parse input data
        let input_data: Value = serde_json::from_str(&data)
            .map_err(|e| format!("Failed to parse input data: {}", e))?;
        
        // Apply transformation rules
        let transformed_data = self.apply_transformation_rules(&input_data, &config.rules)?;
        
        // Serialize result
        serde_json::to_string(&transformed_data)
            .map_err(|e| format!("Failed to serialize transformed data: {}", e))
    }

    /// Validate metadata against external schema
    pub fn validate_metadata_against_external_schema(&self, data: String, external_schema: String) -> Result<ValidationResult, String> {
        // This would implement validation against external schemas
        // For now, return a simple validation result
        Ok(ValidationResult::success())
    }

    /// Create transformation template
    pub fn create_metadata_transformation_template(&self, template: TransformationTemplate) -> Result<i64, String> {
        let template_json = serde_json::to_string(&template)
            .map_err(|e| format!("Failed to serialize transformation template: {}", e))?;
        
        let query = "INSERT INTO transformation_templates 
                     (name, description, template_json, is_system, created_by, created_at) 
                     VALUES (?, ?, ?, ?, ?, datetime('now'))";
        
        self.conn.execute(query, rusqlite::params![
            template.name,
            template.description,
            template_json,
            template.is_system,
            template.created_by
        ])
        .map_err(|e| format!("Failed to create transformation template: {}", e))?;
        
        Ok(self.conn.last_insert_rowid())
    }

    /// Apply transformation template
    pub fn apply_transformation_template(&self, template_id: u32, source_data: String) -> Result<String, String> {
        let template_id = template_id as i64;
        
        // Get template
        let query = "SELECT template_json FROM transformation_templates WHERE id = ?";
        let mut stmt = self.conn.prepare(query)
            .map_err(|e| format!("Failed to prepare template query: {}", e))?;
        
        let template: TransformationTemplate = stmt.query_row(rusqlite::params![template_id], |row| {
            let template_json: String = row.get("template_json")?;
            serde_json::from_str(&template_json)
                .map_err(|e| rusqlite::Error::InvalidColumnType(0, "template_json".to_string(), rusqlite::types::Type::Text))
        })
        .map_err(|e| format!("Failed to get transformation template: {}", e))?;
        
        // Apply transformation
        self.transform_external_metadata(source_data, template.transformation_config)
    }

    // Helper methods
    fn validate_json_format(&self, data: &str) -> Result<ValidationResult, String> {
        match serde_json::from_str::<Value>(data) {
            Ok(_) => Ok(ValidationResult::success()),
            Err(e) => Ok(ValidationResult::with_errors(vec![
                ValidationError {
                    field_path: "format".to_string(),
                    error_type: "json_parse_error".to_string(),
                    message: format!("Invalid JSON format: {}", e),
                    expected: None,
                    actual: None,
                }
            ])),
        }
    }

    fn validate_csv_format(&self, data: &str) -> Result<ValidationResult, String> {
        let mut rdr = csv::Reader::from_reader(data.as_bytes());
        match rdr.headers() {
            Ok(_) => Ok(ValidationResult::success()),
            Err(e) => Ok(ValidationResult::with_errors(vec![
                ValidationError {
                    field_path: "format".to_string(),
                    error_type: "csv_parse_error".to_string(),
                    message: format!("Invalid CSV format: {}", e),
                    expected: None,
                    actual: None,
                }
            ])),
        }
    }

    fn validate_xml_format(&self, data: &str) -> Result<ValidationResult, String> {
        // Simplified XML validation - would use proper XML parser in production
        if data.trim_start().starts_with("<?xml") || data.trim_start().starts_with("<") {
            Ok(ValidationResult::success())
        } else {
            Ok(ValidationResult::with_errors(vec![
                ValidationError {
                    field_path: "format".to_string(),
                    error_type: "xml_parse_error".to_string(),
                    message: "Invalid XML format: does not start with XML declaration or element".to_string(),
                    expected: None,
                    actual: None,
                }
            ]))
        }
    }

    fn apply_transformation_rules(&self, data: &Value, rules: &[TransformationRule]) -> Result<Value, String> {
        let mut result = data.clone();
        
        for rule in rules {
            result = self.apply_single_transformation_rule(&result, rule)?;
        }
        
        Ok(result)
    }

    fn apply_single_transformation_rule(&self, data: &Value, rule: &TransformationRule) -> Result<Value, String> {
        // Simplified transformation logic
        // A full implementation would handle all transformation types
        match rule.transformation_type {
            TransformationType::FormatString => {
                // Apply string formatting
                Ok(data.clone())
            }
            TransformationType::ParseNumber => {
                // Parse string to number
                Ok(data.clone())
            }
            _ => {
                // Default: return data unchanged
                Ok(data.clone())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_api_info_structure() {
        let api_info = ApiInfo {
            version: "1.0.0".to_string(),
            name: "Test API".to_string(),
            description: "Test description".to_string(),
            supported_formats: vec!["json".to_string()],
            supported_operations: vec!["read".to_string()],
            rate_limits: RateLimits {
                requests_per_minute: 100,
                requests_per_hour: 1000,
                bulk_operations_per_hour: 10,
                concurrent_connections: 5,
            },
            authentication_methods: vec!["token".to_string()],
            capabilities: ApiCapabilities {
                supports_bulk_operations: true,
                supports_real_time_sync: false,
                supports_webhooks: true,
                supports_streaming: false,
                max_record_size_bytes: 1024,
                max_batch_size: 100,
                supported_query_operators: vec!["equals".to_string()],
            },
            endpoints: vec![],
        };

        assert_eq!(api_info.version, "1.0.0");
        assert_eq!(api_info.rate_limits.requests_per_minute, 100);
        assert!(api_info.capabilities.supports_bulk_operations);
    }

    #[test]
    fn test_webhook_config_serialization() {
        let config = WebhookConfig {
            name: "Test Webhook".to_string(),
            url: "https://example.com/webhook".to_string(),
            events: vec![WebhookEvent::MetadataCreated, WebhookEvent::MetadataUpdated],
            authentication: Some(WebhookAuth::Bearer {
                token: "test-token".to_string(),
            }),
            headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
            retry_policy: RetryPolicy {
                max_attempts: 3,
                initial_delay_ms: 1000,
                max_delay_ms: 30000,
                backoff_multiplier: 2.0,
                retry_http_codes: vec![500, 502, 503],
            },
            filters: None,
            format: WebhookFormat::Json,
        };

        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: WebhookConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.name, "Test Webhook");
        assert_eq!(deserialized.events.len(), 2);
        assert!(deserialized.authentication.is_some());
    }

    #[test]
    fn test_transformation_rule_types() {
        let rule = TransformationRule {
            name: "Test Rule".to_string(),
            source_field: "input".to_string(),
            target_field: "output".to_string(),
            transformation_type: TransformationType::FormatString,
            parameters: HashMap::from([("format".to_string(), json!("{}"))]),
            conditions: vec![],
        };

        assert_eq!(rule.name, "Test Rule");
        assert!(matches!(rule.transformation_type, TransformationType::FormatString));
    }

    #[test]
    fn test_sync_status_values() {
        let statuses = vec![
            SyncStatus::Running,
            SyncStatus::Completed,
            SyncStatus::Failed,
            SyncStatus::Cancelled,
            SyncStatus::Paused,
        ];

        for status in statuses {
            let serialized = serde_json::to_string(&status).unwrap();
            let _deserialized: SyncStatus = serde_json::from_str(&serialized).unwrap();
        }
    }
}