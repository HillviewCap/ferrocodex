use anyhow::Result;
use rusqlite::{Connection, Row, params};
use serde_json::Value;
use tracing::{info, warn};

use super::{
    AssetMetadataSchema, MetadataFieldTemplate, AssetMetadata, FieldType, FieldCategory,
    CreateMetadataSchemaRequest, UpdateMetadataSchemaRequest, TemplateUsageStats,
};

/// Repository for managing metadata schemas
pub trait MetadataRepository {
    fn create_metadata_schema(&self, request: CreateMetadataSchemaRequest, created_by: i64) -> Result<AssetMetadataSchema>;
    fn get_metadata_schemas(&self, asset_type_filter: Option<String>) -> Result<Vec<AssetMetadataSchema>>;
    fn get_metadata_schema_by_id(&self, schema_id: i64) -> Result<Option<AssetMetadataSchema>>;
    fn update_metadata_schema(&self, schema_id: i64, request: UpdateMetadataSchemaRequest) -> Result<AssetMetadataSchema>;
    fn delete_metadata_schema(&self, schema_id: i64) -> Result<()>;
    fn get_schemas_for_asset_type(&self, asset_type: String) -> Result<Vec<AssetMetadataSchema>>;
}

/// Repository for managing field templates
pub trait FieldTemplateRepository {
    fn get_system_templates(&self) -> Result<Vec<MetadataFieldTemplate>>;
    fn get_templates_by_category(&self, category: FieldCategory) -> Result<Vec<MetadataFieldTemplate>>;
    fn import_field_template(&self, template: MetadataFieldTemplate) -> Result<MetadataFieldTemplate>;
    fn get_template_usage_stats(&self) -> Result<Vec<TemplateUsageStats>>;
    fn increment_template_usage(&self, template_id: i64) -> Result<()>;
}

/// Repository for asset metadata values
pub trait AssetMetadataRepository {
    fn create_asset_metadata(&self, metadata: AssetMetadata) -> Result<AssetMetadata>;
    fn get_asset_metadata(&self, asset_id: i64) -> Result<Vec<AssetMetadata>>;
    fn get_asset_metadata_by_schema(&self, asset_id: i64, schema_id: i64) -> Result<Option<AssetMetadata>>;
    fn update_asset_metadata(&self, metadata: AssetMetadata) -> Result<AssetMetadata>;
    fn delete_asset_metadata(&self, metadata_id: i64) -> Result<()>;
}

/// SQLite implementation of metadata repository
pub struct SqliteMetadataRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteMetadataRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Get the database connection (for internal API use)
    pub fn get_connection(&self) -> &Connection {
        self.conn
    }

    /// Initialize metadata-related database tables
    pub fn initialize_schema(&self) -> Result<()> {
        info!("Initializing metadata schema tables");
        
        self.conn.execute_batch(
            r#"
            -- Asset metadata schemas table
            CREATE TABLE IF NOT EXISTS asset_metadata_schemas (
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

            -- Metadata field templates table
            CREATE TABLE IF NOT EXISTS metadata_field_templates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                field_type TEXT NOT NULL,
                validation_rules TEXT NOT NULL,
                options_json TEXT,
                category TEXT NOT NULL,
                description TEXT NOT NULL,
                is_system BOOLEAN NOT NULL DEFAULT 0,
                usage_count INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            -- Asset metadata values table (basic structure for now)
            CREATE TABLE IF NOT EXISTS asset_metadata (
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

            -- Add additional fields for enhanced API support
            ALTER TABLE asset_metadata_schemas ADD COLUMN is_archived BOOLEAN DEFAULT 0;
            ALTER TABLE asset_metadata_schemas ADD COLUMN archived_at DATETIME;
            ALTER TABLE asset_metadata_schemas ADD COLUMN archive_reason TEXT;

            -- External metadata mappings table
            CREATE TABLE IF NOT EXISTS external_metadata_mappings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                external_system_id TEXT NOT NULL,
                config_json TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            -- Metadata webhooks table
            CREATE TABLE IF NOT EXISTS metadata_webhooks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                url TEXT NOT NULL,
                config_json TEXT NOT NULL,
                enabled BOOLEAN NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            -- Metadata sync jobs table
            CREATE TABLE IF NOT EXISTS metadata_sync_jobs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                config_json TEXT NOT NULL,
                enabled BOOLEAN NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_run DATETIME,
                next_run DATETIME
            );

            -- Sync conflict resolutions table
            CREATE TABLE IF NOT EXISTS sync_conflict_resolutions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                job_id INTEGER NOT NULL,
                conflict_id TEXT NOT NULL,
                resolution_json TEXT NOT NULL,
                resolved_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (job_id) REFERENCES metadata_sync_jobs(id) ON DELETE CASCADE
            );

            -- Transformation templates table
            CREATE TABLE IF NOT EXISTS transformation_templates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                template_json TEXT NOT NULL,
                is_system BOOLEAN NOT NULL DEFAULT 0,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE CASCADE
            );

            -- Saved metadata queries table
            CREATE TABLE IF NOT EXISTS saved_metadata_queries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                query_json TEXT NOT NULL,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_executed DATETIME,
                execution_count INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE CASCADE
            );

            -- Metadata filters table
            CREATE TABLE IF NOT EXISTS metadata_filters (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                query_json TEXT NOT NULL,
                is_system BOOLEAN NOT NULL DEFAULT 0,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_used DATETIME,
                usage_count INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE CASCADE
            );

            -- Indexes for performance
            CREATE INDEX IF NOT EXISTS idx_metadata_schemas_name ON asset_metadata_schemas(name);
            CREATE INDEX IF NOT EXISTS idx_metadata_schemas_asset_type ON asset_metadata_schemas(asset_type_filter);
            CREATE INDEX IF NOT EXISTS idx_metadata_schemas_created_by ON asset_metadata_schemas(created_by);
            CREATE INDEX IF NOT EXISTS idx_metadata_schemas_system ON asset_metadata_schemas(is_system_template);
            CREATE INDEX IF NOT EXISTS idx_metadata_schemas_archived ON asset_metadata_schemas(is_archived);

            CREATE INDEX IF NOT EXISTS idx_field_templates_name ON metadata_field_templates(name);
            CREATE INDEX IF NOT EXISTS idx_field_templates_category ON metadata_field_templates(category);
            CREATE INDEX IF NOT EXISTS idx_field_templates_type ON metadata_field_templates(field_type);
            CREATE INDEX IF NOT EXISTS idx_field_templates_system ON metadata_field_templates(is_system);

            CREATE INDEX IF NOT EXISTS idx_asset_metadata_asset_id ON asset_metadata(asset_id);
            CREATE INDEX IF NOT EXISTS idx_asset_metadata_schema_id ON asset_metadata(schema_id);
            CREATE INDEX IF NOT EXISTS idx_asset_metadata_updated_at ON asset_metadata(updated_at);

            CREATE INDEX IF NOT EXISTS idx_external_mappings_system ON external_metadata_mappings(external_system_id);
            CREATE INDEX IF NOT EXISTS idx_webhooks_enabled ON metadata_webhooks(enabled);
            CREATE INDEX IF NOT EXISTS idx_sync_jobs_enabled ON metadata_sync_jobs(enabled);
            CREATE INDEX IF NOT EXISTS idx_sync_jobs_next_run ON metadata_sync_jobs(next_run);
            CREATE INDEX IF NOT EXISTS idx_transformation_templates_system ON transformation_templates(is_system);
            CREATE INDEX IF NOT EXISTS idx_saved_queries_created_by ON saved_metadata_queries(created_by);
            CREATE INDEX IF NOT EXISTS idx_metadata_filters_created_by ON metadata_filters(created_by);
            "#,
        )?;

        info!("Metadata schema tables initialized successfully");
        Ok(())
    }

    /// Populate system field templates
    pub fn populate_system_templates(&self) -> Result<()> {
        info!("Populating system field templates");
        
        // Check if system templates already exist
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM metadata_field_templates WHERE is_system = 1",
            [],
            |row| row.get(0),
        )?;

        if count > 0 {
            info!("System templates already exist, skipping population");
            return Ok(());
        }

        // Create system templates - will be implemented in templates.rs
        let system_templates = super::get_system_field_templates();
        
        for template in system_templates {
            let _ = self.create_field_template_internal(template)?;
        }

        info!("System field templates populated successfully");
        Ok(())
    }

    fn row_to_metadata_schema(row: &Row) -> rusqlite::Result<AssetMetadataSchema> {
        Ok(AssetMetadataSchema {
            id: Some(row.get("id")?),
            name: row.get("name")?,
            description: row.get("description")?,
            schema_json: row.get("schema_json")?,
            asset_type_filter: row.get("asset_type_filter")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            is_system_template: row.get("is_system_template")?,
            version: row.get("version")?,
        })
    }

    fn row_to_field_template(row: &Row) -> rusqlite::Result<MetadataFieldTemplate> {
        let field_type_str: String = row.get("field_type")?;
        let field_type = FieldType::from_str(&field_type_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "field_type".to_string(), rusqlite::types::Type::Text))?;
        
        let category_str: String = row.get("category")?;
        let category = FieldCategory::from_str(&category_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "category".to_string(), rusqlite::types::Type::Text))?;

        Ok(MetadataFieldTemplate {
            id: Some(row.get("id")?),
            name: row.get("name")?,
            field_type,
            validation_rules: row.get("validation_rules")?,
            options_json: row.get("options_json")?,
            category,
            description: row.get("description")?,
            is_system: row.get("is_system")?,
            usage_count: row.get("usage_count")?,
            created_at: row.get("created_at")?,
        })
    }

    fn row_to_asset_metadata(row: &Row) -> rusqlite::Result<AssetMetadata> {
        Ok(AssetMetadata {
            id: Some(row.get("id")?),
            asset_id: row.get("asset_id")?,
            schema_id: row.get("schema_id")?,
            metadata_values_json: row.get("metadata_values_json")?,
            schema_version: row.get("schema_version")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Internal method to create field template
    fn create_field_template_internal(&self, template: MetadataFieldTemplate) -> Result<MetadataFieldTemplate> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO metadata_field_templates 
             (name, field_type, validation_rules, options_json, category, description, is_system, usage_count) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) 
             RETURNING *"
        )?;

        let created_template = stmt.query_row(
            params![
                &template.name,
                template.field_type.as_str(),
                &template.validation_rules,
                &template.options_json,
                template.category.as_str(),
                &template.description,
                &template.is_system,
                &template.usage_count
            ],
            Self::row_to_field_template,
        )?;

        Ok(created_template)
    }
}

impl<'a> MetadataRepository for SqliteMetadataRepository<'a> {
    fn create_metadata_schema(&self, request: CreateMetadataSchemaRequest, created_by: i64) -> Result<AssetMetadataSchema> {
        // Validate schema name
        if request.name.trim().is_empty() {
            return Err(anyhow::anyhow!("Schema name cannot be empty"));
        }
        if request.name.len() < 2 {
            return Err(anyhow::anyhow!("Schema name must be at least 2 characters long"));
        }
        if request.name.len() > 100 {
            return Err(anyhow::anyhow!("Schema name cannot exceed 100 characters"));
        }

        // Validate JSON schema
        let schema_value: Value = serde_json::from_str(&request.schema_json)
            .map_err(|e| anyhow::anyhow!("Invalid JSON in schema: {}", e))?;
        
        // Test compilation to ensure it's valid JSON Schema
        let _compiled = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&schema_value)
            .map_err(|e| anyhow::anyhow!("Invalid JSON Schema: {}", e))?;

        // Check for duplicate name
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM asset_metadata_schemas WHERE name = ?1",
            [&request.name],
            |row| row.get(0),
        )?;
        
        if count > 0 {
            return Err(anyhow::anyhow!("Schema with this name already exists"));
        }

        let mut stmt = self.conn.prepare(
            "INSERT INTO asset_metadata_schemas 
             (name, description, schema_json, asset_type_filter, created_by) 
             VALUES (?1, ?2, ?3, ?4, ?5) 
             RETURNING *"
        )?;

        let created_schema = stmt.query_row(
            params![
                &request.name,
                &request.description,
                &request.schema_json,
                &request.asset_type_filter,
                &created_by
            ],
            Self::row_to_metadata_schema,
        )?;

        info!("Created metadata schema: {} (id: {})", created_schema.name, created_schema.id.unwrap());
        Ok(created_schema)
    }

    fn get_metadata_schemas(&self, asset_type_filter: Option<String>) -> Result<Vec<AssetMetadataSchema>> {
        let (query, params) = match asset_type_filter {
            Some(filter) => (
                "SELECT id, name, description, schema_json, asset_type_filter, created_by, created_at, is_system_template, version 
                 FROM asset_metadata_schemas 
                 WHERE asset_type_filter IS NULL OR asset_type_filter = ?1 
                 ORDER BY is_system_template DESC, name",
                vec![filter]
            ),
            None => (
                "SELECT id, name, description, schema_json, asset_type_filter, created_by, created_at, is_system_template, version 
                 FROM asset_metadata_schemas 
                 ORDER BY is_system_template DESC, name",
                vec![]
            ),
        };

        let mut stmt = self.conn.prepare(query)?;
        let schema_iter = stmt.query_map(rusqlite::params_from_iter(params.iter()), Self::row_to_metadata_schema)?;
        
        let mut schemas = Vec::new();
        for schema in schema_iter {
            schemas.push(schema?);
        }

        Ok(schemas)
    }

    fn get_metadata_schema_by_id(&self, schema_id: i64) -> Result<Option<AssetMetadataSchema>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, schema_json, asset_type_filter, created_by, created_at, is_system_template, version 
             FROM asset_metadata_schemas WHERE id = ?1"
        )?;

        let result = stmt.query_row([schema_id], Self::row_to_metadata_schema);
        
        match result {
            Ok(schema) => Ok(Some(schema)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn update_metadata_schema(&self, schema_id: i64, request: UpdateMetadataSchemaRequest) -> Result<AssetMetadataSchema> {
        // First check if schema exists
        let existing = self.get_metadata_schema_by_id(schema_id)?
            .ok_or_else(|| anyhow::anyhow!("Schema not found"))?;

        // Build dynamic update query
        let mut updates = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(name) = &request.name {
            if name.trim().is_empty() || name.len() < 2 || name.len() > 100 {
                return Err(anyhow::anyhow!("Invalid schema name"));
            }
            updates.push("name = ?");
            params.push(Box::new(name.clone()));
        }

        if let Some(description) = &request.description {
            updates.push("description = ?");
            params.push(Box::new(description.clone()));
        }

        if let Some(schema_json) = &request.schema_json {
            // Validate JSON schema
            let schema_value: Value = serde_json::from_str(schema_json)
                .map_err(|e| anyhow::anyhow!("Invalid JSON in schema: {}", e))?;
            
            let _compiled = jsonschema::JSONSchema::options()
                .with_draft(jsonschema::Draft::Draft7)
                .compile(&schema_value)
                .map_err(|e| anyhow::anyhow!("Invalid JSON Schema: {}", e))?;

            updates.push("schema_json = ?");
            params.push(Box::new(schema_json.clone()));
            
            // Increment version when schema changes
            updates.push("version = version + 1");
        }

        if let Some(asset_type_filter) = &request.asset_type_filter {
            updates.push("asset_type_filter = ?");
            params.push(Box::new(asset_type_filter.clone()));
        }

        if updates.is_empty() {
            return Ok(existing);
        }

        updates.push("updated_at = CURRENT_TIMESTAMP");
        params.push(Box::new(schema_id));

        let query = format!(
            "UPDATE asset_metadata_schemas SET {} WHERE id = ?",
            updates.join(", ")
        );

        let rows_affected = self.conn.execute(&query, rusqlite::params_from_iter(params.iter()))?;
        
        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Schema not found"));
        }

        // Return updated schema
        self.get_metadata_schema_by_id(schema_id)?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve updated schema"))
    }

    fn delete_metadata_schema(&self, schema_id: i64) -> Result<()> {
        // Check if schema is a system template
        let is_system: bool = self.conn.query_row(
            "SELECT is_system_template FROM asset_metadata_schemas WHERE id = ?1",
            [schema_id],
            |row| row.get(0),
        ).unwrap_or(false);

        if is_system {
            return Err(anyhow::anyhow!("Cannot delete system templates"));
        }

        // Check if schema is in use
        let usage_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM asset_metadata WHERE schema_id = ?1",
            [schema_id],
            |row| row.get(0),
        )?;

        if usage_count > 0 {
            return Err(anyhow::anyhow!("Cannot delete schema that is in use by {} assets", usage_count));
        }

        let rows_affected = self.conn.execute(
            "DELETE FROM asset_metadata_schemas WHERE id = ?1",
            [schema_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Schema not found"));
        }

        info!("Deleted metadata schema with id: {}", schema_id);
        Ok(())
    }

    fn get_schemas_for_asset_type(&self, asset_type: String) -> Result<Vec<AssetMetadataSchema>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, schema_json, asset_type_filter, created_by, created_at, is_system_template, version 
             FROM asset_metadata_schemas 
             WHERE asset_type_filter IS NULL OR asset_type_filter = ?1 
             ORDER BY is_system_template DESC, name"
        )?;

        let schema_iter = stmt.query_map([&asset_type], Self::row_to_metadata_schema)?;
        let mut schemas = Vec::new();

        for schema in schema_iter {
            schemas.push(schema?);
        }

        Ok(schemas)
    }
}

impl<'a> FieldTemplateRepository for SqliteMetadataRepository<'a> {
    fn get_system_templates(&self) -> Result<Vec<MetadataFieldTemplate>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, field_type, validation_rules, options_json, category, description, is_system, usage_count, created_at 
             FROM metadata_field_templates 
             WHERE is_system = 1 
             ORDER BY category, name"
        )?;

        let template_iter = stmt.query_map([], Self::row_to_field_template)?;
        let mut templates = Vec::new();

        for template in template_iter {
            templates.push(template?);
        }

        Ok(templates)
    }

    fn get_templates_by_category(&self, category: FieldCategory) -> Result<Vec<MetadataFieldTemplate>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, field_type, validation_rules, options_json, category, description, is_system, usage_count, created_at 
             FROM metadata_field_templates 
             WHERE category = ?1 
             ORDER BY is_system DESC, usage_count DESC, name"
        )?;

        let template_iter = stmt.query_map([category.as_str()], Self::row_to_field_template)?;
        let mut templates = Vec::new();

        for template in template_iter {
            templates.push(template?);
        }

        Ok(templates)
    }

    fn import_field_template(&self, template: MetadataFieldTemplate) -> Result<MetadataFieldTemplate> {
        // Validate template data
        if template.name.trim().is_empty() {
            return Err(anyhow::anyhow!("Template name cannot be empty"));
        }

        // Check for duplicate name
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM metadata_field_templates WHERE name = ?1",
            [&template.name],
            |row| row.get(0),
        )?;
        
        if count > 0 {
            return Err(anyhow::anyhow!("Template with this name already exists"));
        }

        // Validate JSON in validation_rules
        let _: Value = serde_json::from_str(&template.validation_rules)
            .map_err(|e| anyhow::anyhow!("Invalid validation rules JSON: {}", e))?;

        // Validate options_json if present
        if let Some(ref options) = template.options_json {
            let _: Value = serde_json::from_str(options)
                .map_err(|e| anyhow::anyhow!("Invalid options JSON: {}", e))?;
        }

        self.create_field_template_internal(template)
    }

    fn get_template_usage_stats(&self) -> Result<Vec<TemplateUsageStats>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.name, t.usage_count, t.category,
                    MAX(am.updated_at) as last_used
             FROM metadata_field_templates t
             LEFT JOIN asset_metadata am ON JSON_EXTRACT(am.metadata_values_json, '$') LIKE '%' || t.name || '%'
             GROUP BY t.id, t.name, t.usage_count, t.category
             ORDER BY t.usage_count DESC, t.name"
        )?;

        let stats_iter = stmt.query_map([], |row| {
            let category_str: String = row.get("category")?;
            let category = FieldCategory::from_str(&category_str).unwrap_or(FieldCategory::Device);
            
            Ok(TemplateUsageStats {
                template_id: row.get("id")?,
                template_name: row.get("name")?,
                usage_count: row.get("usage_count")?,
                last_used: row.get("last_used")?,
                category,
            })
        })?;

        let mut stats = Vec::new();
        for stat in stats_iter {
            stats.push(stat?);
        }

        Ok(stats)
    }

    fn increment_template_usage(&self, template_id: i64) -> Result<()> {
        let rows_affected = self.conn.execute(
            "UPDATE metadata_field_templates SET usage_count = usage_count + 1 WHERE id = ?1",
            [template_id],
        )?;

        if rows_affected == 0 {
            warn!("Attempted to increment usage for non-existent template: {}", template_id);
        }

        Ok(())
    }
}

impl<'a> AssetMetadataRepository for SqliteMetadataRepository<'a> {
    fn create_asset_metadata(&self, metadata: AssetMetadata) -> Result<AssetMetadata> {
        // Validate JSON
        let _: Value = serde_json::from_str(&metadata.metadata_values_json)
            .map_err(|e| anyhow::anyhow!("Invalid metadata values JSON: {}", e))?;

        // Verify asset exists
        let asset_exists: bool = self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM assets WHERE id = ?1",
            [metadata.asset_id],
            |row| row.get(0),
        )?;

        if !asset_exists {
            return Err(anyhow::anyhow!("Asset does not exist"));
        }

        // Verify schema exists
        let schema_exists: bool = self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM asset_metadata_schemas WHERE id = ?1",
            [metadata.schema_id],
            |row| row.get(0),
        )?;

        if !schema_exists {
            return Err(anyhow::anyhow!("Schema does not exist"));
        }

        let mut stmt = self.conn.prepare(
            "INSERT INTO asset_metadata 
             (asset_id, schema_id, metadata_values_json, schema_version) 
             VALUES (?1, ?2, ?3, ?4) 
             RETURNING *"
        )?;

        let created_metadata = stmt.query_row(
            params![
                metadata.asset_id,
                metadata.schema_id,
                &metadata.metadata_values_json,
                metadata.schema_version
            ],
            Self::row_to_asset_metadata,
        )?;

        Ok(created_metadata)
    }

    fn get_asset_metadata(&self, asset_id: i64) -> Result<Vec<AssetMetadata>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, asset_id, schema_id, metadata_values_json, schema_version, created_at, updated_at 
             FROM asset_metadata 
             WHERE asset_id = ?1 
             ORDER BY created_at DESC"
        )?;

        let metadata_iter = stmt.query_map([asset_id], Self::row_to_asset_metadata)?;
        let mut metadata_list = Vec::new();

        for metadata in metadata_iter {
            metadata_list.push(metadata?);
        }

        Ok(metadata_list)
    }

    fn get_asset_metadata_by_schema(&self, asset_id: i64, schema_id: i64) -> Result<Option<AssetMetadata>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, asset_id, schema_id, metadata_values_json, schema_version, created_at, updated_at 
             FROM asset_metadata 
             WHERE asset_id = ?1 AND schema_id = ?2"
        )?;

        let result = stmt.query_row([asset_id, schema_id], Self::row_to_asset_metadata);
        
        match result {
            Ok(metadata) => Ok(Some(metadata)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn update_asset_metadata(&self, metadata: AssetMetadata) -> Result<AssetMetadata> {
        let metadata_id = metadata.id.ok_or_else(|| anyhow::anyhow!("Metadata ID is required for update"))?;

        // Validate JSON
        let _: Value = serde_json::from_str(&metadata.metadata_values_json)
            .map_err(|e| anyhow::anyhow!("Invalid metadata values JSON: {}", e))?;

        let rows_affected = self.conn.execute(
            "UPDATE asset_metadata 
             SET metadata_values_json = ?1, schema_version = ?2, updated_at = CURRENT_TIMESTAMP 
             WHERE id = ?3",
            params![&metadata.metadata_values_json, metadata.schema_version, metadata_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Metadata not found"));
        }

        // Return updated metadata
        let mut stmt = self.conn.prepare(
            "SELECT id, asset_id, schema_id, metadata_values_json, schema_version, created_at, updated_at 
             FROM asset_metadata WHERE id = ?1"
        )?;

        let updated_metadata = stmt.query_row([metadata_id], Self::row_to_asset_metadata)?;
        Ok(updated_metadata)
    }

    fn delete_asset_metadata(&self, metadata_id: i64) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM asset_metadata WHERE id = ?1",
            [metadata_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Metadata not found"));
        }

        info!("Deleted asset metadata with id: {}", metadata_id);
        Ok(())
    }
}

// Additional methods for enhanced API support
impl<'a> SqliteMetadataRepository<'a> {
    /// Get single asset metadata record (for API compatibility)
    pub fn get_asset_metadata(&self, asset_id: i64) -> Result<Option<AssetMetadata>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, asset_id, schema_id, metadata_values_json, schema_version, created_at, updated_at 
             FROM asset_metadata WHERE asset_id = ?1 ORDER BY updated_at DESC LIMIT 1"
        )?;

        let result = stmt.query_row([asset_id], Self::row_to_asset_metadata);
        
        match result {
            Ok(metadata) => Ok(Some(metadata)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Update asset metadata by asset_id and schema_id
    pub fn update_asset_metadata(&self, asset_id: i64, schema_id: i64, metadata_values_json: String) -> Result<AssetMetadata> {
        let rows_affected = self.conn.execute(
            "UPDATE asset_metadata 
             SET metadata_values_json = ?1, updated_at = CURRENT_TIMESTAMP 
             WHERE asset_id = ?2 AND schema_id = ?3",
            params![metadata_values_json, asset_id, schema_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Asset metadata not found"));
        }

        // Return updated metadata
        let mut stmt = self.conn.prepare(
            "SELECT id, asset_id, schema_id, metadata_values_json, schema_version, created_at, updated_at 
             FROM asset_metadata WHERE asset_id = ?1 AND schema_id = ?2"
        )?;

        let metadata = stmt.query_row([asset_id, schema_id], Self::row_to_asset_metadata)?;
        Ok(metadata)
    }

    /// Delete asset metadata by asset_id
    pub fn delete_asset_metadata(&self, asset_id: i64) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM asset_metadata WHERE asset_id = ?1",
            [asset_id],
        )?;

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Asset metadata not found"));
        }

        info!("Deleted asset metadata for asset: {}", asset_id);
        Ok(())
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
            
            INSERT INTO users (id, username, password_hash, role) VALUES (1, 'testuser', 'hash', 'Engineer');
            INSERT INTO assets (id, name, description, asset_type, created_by) VALUES (1, 'Test Asset', 'Test', 'device', 1);
            "#,
        ).unwrap();
        
        let repo = SqliteMetadataRepository::new(&conn);
        repo.initialize_schema().unwrap();
        
        (temp_file, conn)
    }

    #[test]
    fn test_create_metadata_schema() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteMetadataRepository::new(&conn);

        let request = CreateMetadataSchemaRequest {
            name: "PLC Configuration Schema".to_string(),
            description: "Schema for PLC metadata".to_string(),
            schema_json: json!({
                "type": "object",
                "properties": {
                    "ip_address": {
                        "type": "string",
                        "pattern": "^\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}$"
                    },
                    "model": {
                        "type": "string",
                        "minLength": 1
                    }
                },
                "required": ["ip_address"]
            }).to_string(),
            asset_type_filter: Some("device".to_string()),
        };

        let schema = repo.create_metadata_schema(request, 1).unwrap();
        assert_eq!(schema.name, "PLC Configuration Schema");
        assert_eq!(schema.version, 1);
        assert!(!schema.is_system_template);
        assert!(schema.id.is_some());
    }

    #[test]
    fn test_invalid_schema_json() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteMetadataRepository::new(&conn);

        let request = CreateMetadataSchemaRequest {
            name: "Invalid Schema".to_string(),
            description: "Test".to_string(),
            schema_json: "{ invalid json }".to_string(),
            asset_type_filter: None,
        };

        let result = repo.create_metadata_schema(request, 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid JSON"));
    }

    #[test]
    fn test_get_metadata_schemas() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteMetadataRepository::new(&conn);

        // Create test schema
        let request = CreateMetadataSchemaRequest {
            name: "Test Schema".to_string(),
            description: "Test".to_string(),
            schema_json: json!({"type": "object"}).to_string(),
            asset_type_filter: Some("device".to_string()),
        };
        repo.create_metadata_schema(request, 1).unwrap();

        let schemas = repo.get_metadata_schemas(Some("device".to_string())).unwrap();
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0].name, "Test Schema");

        let all_schemas = repo.get_metadata_schemas(None).unwrap();
        assert_eq!(all_schemas.len(), 1);
    }

    #[test]
    fn test_update_metadata_schema() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteMetadataRepository::new(&conn);

        // Create test schema
        let request = CreateMetadataSchemaRequest {
            name: "Original Name".to_string(),
            description: "Original description".to_string(),
            schema_json: json!({"type": "object"}).to_string(),
            asset_type_filter: None,
        };
        let schema = repo.create_metadata_schema(request, 1).unwrap();
        let schema_id = schema.id.unwrap();

        // Update schema
        let update_request = UpdateMetadataSchemaRequest {
            name: Some("Updated Name".to_string()),
            description: Some("Updated description".to_string()),
            schema_json: Some(json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"}
                }
            }).to_string()),
            asset_type_filter: Some("device".to_string()),
        };

        let updated_schema = repo.update_metadata_schema(schema_id, update_request).unwrap();
        assert_eq!(updated_schema.name, "Updated Name");
        assert_eq!(updated_schema.description, "Updated description");
        assert_eq!(updated_schema.version, 2); // Version should increment
        assert_eq!(updated_schema.asset_type_filter, Some("device".to_string()));
    }

    #[test]
    fn test_field_template_operations() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteMetadataRepository::new(&conn);

        // Create test template
        let template = MetadataFieldTemplate::new(
            "Test IP Field".to_string(),
            FieldType::Text,
            json!({"pattern": "^\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}$"}).to_string(),
            None,
            FieldCategory::Network,
            "IPv4 address field".to_string(),
            false,
        );

        let created_template = repo.import_field_template(template).unwrap();
        assert_eq!(created_template.name, "Test IP Field");
        assert_eq!(created_template.field_type, FieldType::Text);
        assert_eq!(created_template.category, FieldCategory::Network);

        // Test getting templates by category
        let network_templates = repo.get_templates_by_category(FieldCategory::Network).unwrap();
        assert_eq!(network_templates.len(), 1);
        assert_eq!(network_templates[0].name, "Test IP Field");

        // Test usage increment
        let template_id = created_template.id.unwrap();
        repo.increment_template_usage(template_id).unwrap();

        let updated_templates = repo.get_templates_by_category(FieldCategory::Network).unwrap();
        assert_eq!(updated_templates[0].usage_count, 1);
    }

    #[test]
    fn test_asset_metadata_operations() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteMetadataRepository::new(&conn);

        // Create test schema first
        let schema_request = CreateMetadataSchemaRequest {
            name: "Test Schema".to_string(),
            description: "Test".to_string(),
            schema_json: json!({"type": "object"}).to_string(),
            asset_type_filter: None,
        };
        let schema = repo.create_metadata_schema(schema_request, 1).unwrap();
        let schema_id = schema.id.unwrap();

        // Create asset metadata
        let metadata = AssetMetadata::new(
            1, // asset_id
            schema_id,
            json!({"ip_address": "192.168.1.100"}).to_string(),
            1,
        );

        let created_metadata = repo.create_asset_metadata(metadata).unwrap();
        assert_eq!(created_metadata.asset_id, 1);
        assert_eq!(created_metadata.schema_id, schema_id);

        // Test getting asset metadata
        let asset_metadata = repo.get_asset_metadata(1).unwrap();
        assert_eq!(asset_metadata.len(), 1);

        // Test getting specific metadata by schema
        let specific_metadata = repo.get_asset_metadata_by_schema(1, schema_id).unwrap();
        assert!(specific_metadata.is_some());

        // Test updating metadata
        let mut updated_metadata = created_metadata.clone();
        updated_metadata.metadata_values_json = json!({"ip_address": "192.168.1.101"}).to_string();
        
        let result = repo.update_asset_metadata(updated_metadata).unwrap();
        assert!(result.metadata_values_json.contains("192.168.1.101"));
    }
}