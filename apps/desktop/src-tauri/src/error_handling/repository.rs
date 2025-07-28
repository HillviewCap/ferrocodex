use crate::error_handling::{ErrorContext, ContextCorrelation, EnhancedError};
use chrono;
use anyhow::Result;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Repository for error context storage following existing patterns
pub trait ErrorContextRepository {
    /// Store error context in database
    fn store_error_context(&self, context: &ErrorContext) -> Result<()>;
    
    /// Retrieve error context by ID
    fn get_error_context(&self, context_id: Uuid) -> Result<Option<ErrorContext>>;
    
    /// Find error contexts by request ID
    fn find_contexts_by_request_id(&self, request_id: Uuid) -> Result<Vec<ErrorContext>>;
    
    /// Find error contexts by user ID
    fn find_contexts_by_user_id(&self, user_id: i64) -> Result<Vec<ErrorContext>>;
    
    /// Store error classification data
    fn store_error_classification(&self, error: &EnhancedError) -> Result<()>;
    
    /// Retrieve error classification by error ID
    fn get_error_classification(&self, error_id: Uuid) -> Result<Option<ErrorClassificationRecord>>;
    
    /// Store context correlation data
    fn store_context_correlation(&self, correlation: &ContextCorrelation) -> Result<()>;
    
    /// Get correlation chain for a context
    fn get_correlation_chain(&self, context_id: Uuid) -> Result<Vec<ContextCorrelation>>;
    
    /// Search error contexts with filters
    fn search_error_contexts(&self, filters: &ErrorContextSearchFilters) -> Result<Vec<ErrorContext>>;
}

/// Error classification record for database storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorClassificationRecord {
    pub error_id: Uuid,
    pub severity_level: String,
    pub domain: String,
    pub recovery_strategy: String,
    pub context_id: Option<Uuid>,
    pub message: String,
    pub details: Option<String>,
    pub component: Option<String>,
    pub operation: Option<String>,
    pub correlation_id: Option<Uuid>,
    pub created_at: String,
}

/// Search filters for error contexts
#[derive(Debug, Clone, Default)]
pub struct ErrorContextSearchFilters {
    pub user_id: Option<i64>,
    pub component: Option<String>,
    pub operation: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub request_id: Option<Uuid>,
    pub correlation_id: Option<Uuid>,
    pub limit: Option<i64>,
}

/// SQLite implementation of ErrorContextRepository
pub struct SqliteErrorContextRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteErrorContextRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Initialize database schema for error handling tables
    pub fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            -- Error contexts table for comprehensive error context metadata
            CREATE TABLE IF NOT EXISTS error_contexts (
                context_id TEXT PRIMARY KEY,
                request_id TEXT NOT NULL,
                user_id INTEGER,
                operation TEXT NOT NULL,
                component TEXT NOT NULL,
                metadata TEXT, -- JSON serialized HashMap
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                correlation_id TEXT,
                session_id TEXT,
                username TEXT,
                user_role TEXT,
                session_start DATETIME,
                request_path TEXT,
                user_agent TEXT,
                ip_address TEXT,
                platform TEXT
            );

            -- Error classifications table for error classification data
            CREATE TABLE IF NOT EXISTS error_classifications (
                error_id TEXT PRIMARY KEY,
                severity_level TEXT NOT NULL,
                domain TEXT NOT NULL,
                recovery_strategy TEXT NOT NULL,
                context_id TEXT,
                message TEXT NOT NULL,
                details TEXT,
                component TEXT,
                operation TEXT,
                correlation_id TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (context_id) REFERENCES error_contexts(context_id)
            );

            -- Error correlation table for cross-layer error propagation tracking
            CREATE TABLE IF NOT EXISTS error_correlation (
                correlation_id TEXT PRIMARY KEY,
                layer TEXT NOT NULL,
                component TEXT NOT NULL,
                context_id TEXT NOT NULL,
                parent_correlation_id TEXT,
                metadata TEXT, -- JSON serialized HashMap
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (context_id) REFERENCES error_contexts(context_id),
                FOREIGN KEY (parent_correlation_id) REFERENCES error_correlation(correlation_id)
            );

            -- Indexes for performance optimization
            CREATE INDEX IF NOT EXISTS idx_error_contexts_request_id ON error_contexts(request_id);
            CREATE INDEX IF NOT EXISTS idx_error_contexts_user_id ON error_contexts(user_id);
            CREATE INDEX IF NOT EXISTS idx_error_contexts_component ON error_contexts(component);
            CREATE INDEX IF NOT EXISTS idx_error_contexts_operation ON error_contexts(operation);
            CREATE INDEX IF NOT EXISTS idx_error_contexts_created_at ON error_contexts(created_at);
            CREATE INDEX IF NOT EXISTS idx_error_contexts_correlation_id ON error_contexts(correlation_id);

            CREATE INDEX IF NOT EXISTS idx_error_classifications_severity ON error_classifications(severity_level);
            CREATE INDEX IF NOT EXISTS idx_error_classifications_domain ON error_classifications(domain);
            CREATE INDEX IF NOT EXISTS idx_error_classifications_context_id ON error_classifications(context_id);
            CREATE INDEX IF NOT EXISTS idx_error_classifications_created_at ON error_classifications(created_at);

            CREATE INDEX IF NOT EXISTS idx_error_correlation_layer ON error_correlation(layer);
            CREATE INDEX IF NOT EXISTS idx_error_correlation_component ON error_correlation(component);
            CREATE INDEX IF NOT EXISTS idx_error_correlation_context_id ON error_correlation(context_id);
            CREATE INDEX IF NOT EXISTS idx_error_correlation_parent ON error_correlation(parent_correlation_id);
            "#,
        )?;
        Ok(())
    }

    /// Convert database row to ErrorContext
    fn row_to_error_context(row: &Row) -> rusqlite::Result<ErrorContext> {
        let context_id_str: String = row.get("context_id")?;
        let request_id_str: String = row.get("request_id")?;
        let correlation_id_str: Option<String> = row.get("correlation_id").ok();
        let metadata_json: Option<String> = row.get("metadata").ok();
        
        let context_id = Uuid::parse_str(&context_id_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "context_id".to_string(), rusqlite::types::Type::Text))?;
        let request_id = Uuid::parse_str(&request_id_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(1, "request_id".to_string(), rusqlite::types::Type::Text))?;
        
        let correlation_id = if let Some(corr_id_str) = correlation_id_str {
            Some(Uuid::parse_str(&corr_id_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(2, "correlation_id".to_string(), rusqlite::types::Type::Text))?)
        } else {
            None
        };

        let metadata = if let Some(json) = metadata_json {
            serde_json::from_str(&json).unwrap_or_else(|_| HashMap::new())
        } else {
            HashMap::new()
        };

        let session_info = match (
            row.get::<_, Option<String>>("session_id").ok().flatten(),
            row.get::<_, Option<String>>("username").ok().flatten(),
            row.get::<_, Option<String>>("user_role").ok().flatten(),
            row.get::<_, Option<String>>("session_start").ok().flatten(),
        ) {
            (Some(session_id), Some(username), Some(role), Some(session_start)) => {
                if let Ok(session_start_dt) = chrono::DateTime::parse_from_rfc3339(&session_start) {
                    Some(crate::error_handling::context::SessionInfo {
                        session_id,
                        username,
                        role,
                        session_start: session_start_dt.with_timezone(&chrono::Utc),
                    })
                } else {
                    None
                }
            }
            _ => None,
        };

        let client_info = match (
            row.get::<_, Option<String>>("user_agent").ok().flatten(),
            row.get::<_, Option<String>>("ip_address").ok().flatten(),
            row.get::<_, Option<String>>("platform").ok().flatten(),
        ) {
            (user_agent, ip_address, platform) => {
                if user_agent.is_some() || ip_address.is_some() || platform.is_some() {
                    Some(crate::error_handling::context::ClientInfo {
                        user_agent,
                        ip_address,
                        platform,
                    })
                } else {
                    None
                }
            }
        };

        let created_at_str: String = row.get("created_at")?;
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .or_else(|_| {
                // Try parsing as SQLite datetime format
                chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                    .map(|dt| dt.and_utc().fixed_offset())
            })
            .map_err(|_| rusqlite::Error::InvalidColumnType(3, "created_at".to_string(), rusqlite::types::Type::Text))?
            .with_timezone(&chrono::Utc);

        Ok(ErrorContext {
            context_id,
            request_id,
            user_id: row.get("user_id").ok(),
            operation: row.get("operation")?,
            component: row.get("component")?,
            metadata,
            created_at,
            correlation_id,
            session_info,
            request_path: row.get("request_path").ok(),
            client_info,
        })
    }

    /// Convert database row to ErrorClassificationRecord
    fn row_to_error_classification(row: &Row) -> rusqlite::Result<ErrorClassificationRecord> {
        let error_id_str: String = row.get("error_id")?;
        let context_id_str: Option<String> = row.get("context_id").ok();
        let correlation_id_str: Option<String> = row.get("correlation_id").ok();

        let error_id = Uuid::parse_str(&error_id_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "error_id".to_string(), rusqlite::types::Type::Text))?;

        let context_id = if let Some(ctx_id_str) = context_id_str {
            Some(Uuid::parse_str(&ctx_id_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(1, "context_id".to_string(), rusqlite::types::Type::Text))?)
        } else {
            None
        };

        let correlation_id = if let Some(corr_id_str) = correlation_id_str {
            Some(Uuid::parse_str(&corr_id_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(2, "correlation_id".to_string(), rusqlite::types::Type::Text))?)
        } else {
            None
        };

        Ok(ErrorClassificationRecord {
            error_id,
            severity_level: row.get("severity_level")?,
            domain: row.get("domain")?,
            recovery_strategy: row.get("recovery_strategy")?,
            context_id,
            message: row.get("message")?,
            details: row.get("details").ok(),
            component: row.get("component").ok(),
            operation: row.get("operation").ok(),
            correlation_id,
            created_at: row.get("created_at")?,
        })
    }

    /// Convert database row to ContextCorrelation
    fn row_to_context_correlation(row: &Row) -> rusqlite::Result<ContextCorrelation> {
        let correlation_id_str: String = row.get("correlation_id")?;
        let context_id_str: String = row.get("context_id")?;
        let parent_correlation_id_str: Option<String> = row.get("parent_correlation_id").ok();
        let metadata_json: Option<String> = row.get("metadata").ok();

        let correlation_id = Uuid::parse_str(&correlation_id_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(0, "correlation_id".to_string(), rusqlite::types::Type::Text))?;
        let context_id = Uuid::parse_str(&context_id_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(1, "context_id".to_string(), rusqlite::types::Type::Text))?;

        let parent_correlation_id = if let Some(parent_id_str) = parent_correlation_id_str {
            Some(Uuid::parse_str(&parent_id_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(2, "parent_correlation_id".to_string(), rusqlite::types::Type::Text))?)
        } else {
            None
        };

        let metadata = if let Some(json) = metadata_json {
            serde_json::from_str(&json).unwrap_or_else(|_| HashMap::new())
        } else {
            HashMap::new()
        };

        let created_at_str: String = row.get("created_at")?;
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                    .map(|dt| dt.and_utc().fixed_offset())
            })
            .map_err(|_| rusqlite::Error::InvalidColumnType(3, "created_at".to_string(), rusqlite::types::Type::Text))?
            .with_timezone(&chrono::Utc);

        Ok(ContextCorrelation {
            correlation_id,
            layer: row.get("layer")?,
            component: row.get("component")?,
            context_id,
            parent_correlation_id,
            created_at,
            metadata,
        })
    }
}

impl<'a> ErrorContextRepository for SqliteErrorContextRepository<'a> {
    fn store_error_context(&self, context: &ErrorContext) -> Result<()> {
        let metadata_json = serde_json::to_string(&context.metadata)?;
        
        let mut stmt = self.conn.prepare(
            "INSERT INTO error_contexts (
                context_id, request_id, user_id, operation, component, metadata, 
                created_at, correlation_id, session_id, username, user_role, 
                session_start, request_path, user_agent, ip_address, platform
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"
        )?;

        stmt.execute((
            context.context_id.to_string(),
            context.request_id.to_string(),
            context.user_id,
            &context.operation,
            &context.component,
            metadata_json,
            context.created_at.to_rfc3339(),
            context.correlation_id.map(|id| id.to_string()),
            context.session_info.as_ref().map(|s| &s.session_id),
            context.session_info.as_ref().map(|s| &s.username),
            context.session_info.as_ref().map(|s| &s.role),
            context.session_info.as_ref().map(|s| s.session_start.to_rfc3339()),
            context.request_path.as_ref(),
            context.client_info.as_ref().and_then(|c| c.user_agent.as_ref()),
            context.client_info.as_ref().and_then(|c| c.ip_address.as_ref()),
            context.client_info.as_ref().and_then(|c| c.platform.as_ref()),
        ))?;

        Ok(())
    }

    fn get_error_context(&self, context_id: Uuid) -> Result<Option<ErrorContext>> {
        let mut stmt = self.conn.prepare(
            "SELECT context_id, request_id, user_id, operation, component, metadata, 
             created_at, correlation_id, session_id, username, user_role, session_start, 
             request_path, user_agent, ip_address, platform 
             FROM error_contexts WHERE context_id = ?1"
        )?;

        let result = stmt.query_row([context_id.to_string()], Self::row_to_error_context);
        
        match result {
            Ok(context) => Ok(Some(context)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn find_contexts_by_request_id(&self, request_id: Uuid) -> Result<Vec<ErrorContext>> {
        let mut stmt = self.conn.prepare(
            "SELECT context_id, request_id, user_id, operation, component, metadata, 
             created_at, correlation_id, session_id, username, user_role, session_start, 
             request_path, user_agent, ip_address, platform 
             FROM error_contexts WHERE request_id = ?1 ORDER BY created_at"
        )?;

        let context_iter = stmt.query_map([request_id.to_string()], Self::row_to_error_context)?;
        let mut contexts = Vec::new();

        for context in context_iter {
            contexts.push(context?);
        }

        Ok(contexts)
    }

    fn find_contexts_by_user_id(&self, user_id: i64) -> Result<Vec<ErrorContext>> {
        let mut stmt = self.conn.prepare(
            "SELECT context_id, request_id, user_id, operation, component, metadata, 
             created_at, correlation_id, session_id, username, user_role, session_start, 
             request_path, user_agent, ip_address, platform 
             FROM error_contexts WHERE user_id = ?1 ORDER BY created_at DESC"
        )?;

        let context_iter = stmt.query_map([user_id], Self::row_to_error_context)?;
        let mut contexts = Vec::new();

        for context in context_iter {
            contexts.push(context?);
        }

        Ok(contexts)
    }

    fn store_error_classification(&self, error: &EnhancedError) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO error_classifications (
                error_id, severity_level, domain, recovery_strategy, context_id, 
                message, details, component, operation, correlation_id, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
        )?;

        stmt.execute((
            error.id.to_string(),
            error.severity.to_string(),
            error.domain.to_string(),
            error.recovery_strategy.to_string(),
            error.context_id.map(|id| id.to_string()),
            &error.message,
            error.details.as_ref(),
            error.component.as_ref(),
            error.operation.as_ref(),
            error.correlation_id.map(|id| id.to_string()),
            error.timestamp.to_rfc3339(),
        ))?;

        Ok(())
    }

    fn get_error_classification(&self, error_id: Uuid) -> Result<Option<ErrorClassificationRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT error_id, severity_level, domain, recovery_strategy, context_id, 
             message, details, component, operation, correlation_id, created_at 
             FROM error_classifications WHERE error_id = ?1"
        )?;

        let result = stmt.query_row([error_id.to_string()], Self::row_to_error_classification);
        
        match result {
            Ok(classification) => Ok(Some(classification)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn store_context_correlation(&self, correlation: &ContextCorrelation) -> Result<()> {
        let metadata_json = serde_json::to_string(&correlation.metadata)?;
        
        let mut stmt = self.conn.prepare(
            "INSERT INTO error_correlation (
                correlation_id, layer, component, context_id, parent_correlation_id, 
                metadata, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        )?;

        stmt.execute((
            correlation.correlation_id.to_string(),
            &correlation.layer,
            &correlation.component,
            correlation.context_id.to_string(),
            correlation.parent_correlation_id.map(|id| id.to_string()),
            metadata_json,
            correlation.created_at.to_rfc3339(),
        ))?;

        Ok(())
    }

    fn get_correlation_chain(&self, context_id: Uuid) -> Result<Vec<ContextCorrelation>> {
        let mut stmt = self.conn.prepare(
            "SELECT correlation_id, layer, component, context_id, parent_correlation_id, 
             metadata, created_at 
             FROM error_correlation WHERE context_id = ?1 ORDER BY created_at"
        )?;

        let correlation_iter = stmt.query_map([context_id.to_string()], Self::row_to_context_correlation)?;
        let mut correlations = Vec::new();

        for correlation in correlation_iter {
            correlations.push(correlation?);
        }

        Ok(correlations)
    }

    fn search_error_contexts(&self, filters: &ErrorContextSearchFilters) -> Result<Vec<ErrorContext>> {
        let mut query = "SELECT context_id, request_id, user_id, operation, component, metadata, 
                         created_at, correlation_id, session_id, username, user_role, session_start, 
                         request_path, user_agent, ip_address, platform 
                         FROM error_contexts WHERE 1=1".to_string();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(user_id) = filters.user_id {
            query.push_str(" AND user_id = ?");
            params.push(Box::new(user_id));
        }

        if let Some(ref component) = filters.component {
            query.push_str(" AND component = ?");
            params.push(Box::new(component.clone()));
        }

        if let Some(ref operation) = filters.operation {
            query.push_str(" AND operation = ?");
            params.push(Box::new(operation.clone()));
        }

        if let Some(ref request_id) = filters.request_id {
            query.push_str(" AND request_id = ?");
            params.push(Box::new(request_id.to_string()));
        }

        if let Some(ref correlation_id) = filters.correlation_id {
            query.push_str(" AND correlation_id = ?");
            params.push(Box::new(correlation_id.to_string()));
        }

        if let Some(ref start_time) = filters.start_time {
            query.push_str(" AND created_at >= ?");
            params.push(Box::new(start_time.clone()));
        }

        if let Some(ref end_time) = filters.end_time {
            query.push_str(" AND created_at <= ?");
            params.push(Box::new(end_time.clone()));
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = filters.limit {
            query.push_str(" LIMIT ?");
            params.push(Box::new(limit));
        }

        let mut stmt = self.conn.prepare(&query)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let context_iter = stmt.query_map(param_refs.as_slice(), Self::row_to_error_context)?;
        
        let mut contexts = Vec::new();
        for context in context_iter {
            contexts.push(context?);
        }

        Ok(contexts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use crate::error_handling::{ErrorSeverity, ErrorDomain, RecoveryStrategy};

    fn setup_test_db() -> (NamedTempFile, Connection) {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();
        
        let repo = SqliteErrorContextRepository::new(&conn);
        repo.initialize_schema().unwrap();
        
        (temp_file, conn)
    }

    #[test]
    fn test_schema_initialization() {
        let (_temp_file, conn) = setup_test_db();
        
        // Verify tables were created
        let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'").unwrap();
        let table_iter = stmt.query_map([], |row| {
            Ok(row.get::<_, String>(0)?)
        }).unwrap();
        
        let tables: Vec<String> = table_iter.map(|t| t.unwrap()).collect();
        assert!(tables.contains(&"error_contexts".to_string()));
        assert!(tables.contains(&"error_classifications".to_string()));
        assert!(tables.contains(&"error_correlation".to_string()));
    }

    #[test]
    fn test_store_and_retrieve_error_context() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteErrorContextRepository::new(&conn);

        let context = ErrorContext::new("test_operation".to_string(), "TestComponent".to_string())
            .with_user_id(123)
            .add_metadata("key1".to_string(), "value1".to_string());

        // Store context
        repo.store_error_context(&context).unwrap();

        // Retrieve context
        let retrieved = repo.get_error_context(context.context_id).unwrap().unwrap();
        
        assert_eq!(retrieved.context_id, context.context_id);
        assert_eq!(retrieved.request_id, context.request_id);
        assert_eq!(retrieved.user_id, Some(123));
        assert_eq!(retrieved.operation, "test_operation");
        assert_eq!(retrieved.component, "TestComponent");
        assert_eq!(retrieved.get_metadata("key1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_store_and_retrieve_error_classification() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteErrorContextRepository::new(&conn);

        let error = EnhancedError::new(
            ErrorSeverity::High,
            ErrorDomain::Auth,
            RecoveryStrategy::UserRecoverable,
            "Authentication failed".to_string(),
        )
        .with_details("Invalid credentials provided".to_string())
        .with_component("auth".to_string())
        .with_operation("login".to_string());

        // Store classification
        repo.store_error_classification(&error).unwrap();

        // Retrieve classification
        let retrieved = repo.get_error_classification(error.id).unwrap().unwrap();
        
        assert_eq!(retrieved.error_id, error.id);
        assert_eq!(retrieved.severity_level, "High");
        assert_eq!(retrieved.domain, "Auth");
        assert_eq!(retrieved.recovery_strategy, "UserRecoverable");
        assert_eq!(retrieved.message, "Authentication failed");
        assert_eq!(retrieved.details, Some("Invalid credentials provided".to_string()));
        assert_eq!(retrieved.component, Some("auth".to_string()));
        assert_eq!(retrieved.operation, Some("login".to_string()));
    }

    #[test]
    fn test_context_search_with_filters() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteErrorContextRepository::new(&conn);

        // Create multiple contexts
        let context1 = ErrorContext::new("operation1".to_string(), "Component1".to_string())
            .with_user_id(123);
        let context2 = ErrorContext::new("operation2".to_string(), "Component2".to_string())
            .with_user_id(456);
        let context3 = ErrorContext::new("operation1".to_string(), "Component1".to_string())
            .with_user_id(123);

        repo.store_error_context(&context1).unwrap();
        repo.store_error_context(&context2).unwrap();
        repo.store_error_context(&context3).unwrap();

        // Search by user_id
        let filters = ErrorContextSearchFilters {
            user_id: Some(123),
            ..Default::default()
        };
        let results = repo.search_error_contexts(&filters).unwrap();
        assert_eq!(results.len(), 2);

        // Search by component
        let filters = ErrorContextSearchFilters {
            component: Some("Component1".to_string()),
            ..Default::default()
        };
        let results = repo.search_error_contexts(&filters).unwrap();
        assert_eq!(results.len(), 2);

        // Search with limit
        let filters = ErrorContextSearchFilters {
            limit: Some(1),
            ..Default::default()
        };
        let results = repo.search_error_contexts(&filters).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_find_contexts_by_request_id() {
        let (_temp_file, conn) = setup_test_db();
        let repo = SqliteErrorContextRepository::new(&conn);

        let request_id = Uuid::new_v4();
        
        // Create contexts with same request ID (simulating child contexts)
        let parent_context = ErrorContext {
            context_id: Uuid::new_v4(),
            request_id,
            user_id: Some(123),
            operation: "parent_operation".to_string(),
            component: "ParentComponent".to_string(),
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            correlation_id: None,
            session_info: None,
            request_path: None,
            client_info: None,
        };

        let child_context = ErrorContext {
            context_id: Uuid::new_v4(),
            request_id, // Same request ID
            user_id: Some(123),
            operation: "child_operation".to_string(),
            component: "ChildComponent".to_string(),
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            correlation_id: None,
            session_info: None,
            request_path: None,
            client_info: None,
        };

        repo.store_error_context(&parent_context).unwrap();
        repo.store_error_context(&child_context).unwrap();

        // Find contexts by request ID
        let contexts = repo.find_contexts_by_request_id(request_id).unwrap();
        assert_eq!(contexts.len(), 2);
        
        // Verify both contexts are returned
        let operations: Vec<&String> = contexts.iter().map(|c| &c.operation).collect();
        assert!(operations.contains(&&"parent_operation".to_string()));
        assert!(operations.contains(&&"child_operation".to_string()));
    }
}