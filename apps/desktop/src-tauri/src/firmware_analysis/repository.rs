use anyhow::Result;
use rusqlite::{Connection, Row};
use crate::firmware_analysis::models::{FirmwareAnalysisResult, AnalysisStatus};

pub trait FirmwareAnalysisRepository {
    fn initialize_schema(&self) -> Result<()>;
    fn create_analysis(&self, firmware_version_id: i64) -> Result<i64>;
    fn update_analysis_status(&self, id: i64, status: AnalysisStatus, error_message: Option<String>) -> Result<()>;
    fn update_analysis_results(
        &self,
        id: i64,
        file_type: Option<String>,
        detected_versions: Option<Vec<String>>,
        entropy_score: Option<f64>,
        security_findings: Option<String>,
        raw_results: Option<String>,
    ) -> Result<()>;
    fn get_analysis_by_firmware_id(&self, firmware_version_id: i64) -> Result<Option<FirmwareAnalysisResult>>;
    fn get_analysis_by_id(&self, id: i64) -> Result<Option<FirmwareAnalysisResult>>;
}

pub struct SqliteFirmwareAnalysisRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteFirmwareAnalysisRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
    
    fn row_to_analysis_result(row: &Row) -> rusqlite::Result<FirmwareAnalysisResult> {
        Ok(FirmwareAnalysisResult {
            id: row.get("id")?,
            firmware_version_id: row.get("firmware_version_id")?,
            analysis_status: AnalysisStatus::from(row.get::<_, String>("analysis_status")?),
            file_type: row.get("file_type")?,
            detected_versions: row.get::<_, Option<String>>("detected_versions")?
                .and_then(|v| serde_json::from_str(&v).ok()),
            entropy_score: row.get("entropy_score")?,
            security_findings: row.get::<_, Option<String>>("security_findings")?
                .and_then(|v| serde_json::from_str(&v).ok()),
            raw_results: row.get("raw_results")?,
            started_at: row.get("started_at")?,
            completed_at: row.get("completed_at")?,
            error_message: row.get("error_message")?,
            created_at: row.get("created_at")?,
        })
    }
}

impl<'a> FirmwareAnalysisRepository for SqliteFirmwareAnalysisRepository<'a> {
    fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS firmware_analysis_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                firmware_version_id INTEGER NOT NULL UNIQUE,
                analysis_status TEXT NOT NULL CHECK(analysis_status IN ('pending', 'in_progress', 'completed', 'failed')),
                file_type TEXT,
                detected_versions TEXT,
                entropy_score REAL,
                security_findings TEXT,
                raw_results TEXT,
                started_at DATETIME,
                completed_at DATETIME,
                error_message TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (firmware_version_id) REFERENCES firmware_versions(id) ON DELETE CASCADE
            );
            
            CREATE INDEX IF NOT EXISTS idx_analysis_firmware_id ON firmware_analysis_results(firmware_version_id);
            CREATE INDEX IF NOT EXISTS idx_analysis_status ON firmware_analysis_results(analysis_status);
            "#
        )?;
        Ok(())
    }
    
    fn create_analysis(&self, firmware_version_id: i64) -> Result<i64> {
        let tx = self.conn.unchecked_transaction()?;
        
        tx.execute(
            "INSERT INTO firmware_analysis_results (firmware_version_id, analysis_status, started_at)
             VALUES (?1, ?2, CURRENT_TIMESTAMP)",
            rusqlite::params![firmware_version_id, AnalysisStatus::Pending.to_string()],
        )?;
        
        let id = tx.last_insert_rowid();
        tx.commit()?;
        
        Ok(id)
    }
    
    fn update_analysis_status(&self, id: i64, status: AnalysisStatus, error_message: Option<String>) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        
        match status {
            AnalysisStatus::InProgress => {
                tx.execute(
                    "UPDATE firmware_analysis_results 
                     SET analysis_status = ?1, started_at = CURRENT_TIMESTAMP 
                     WHERE id = ?2",
                    rusqlite::params![status.to_string(), id],
                )?;
            }
            AnalysisStatus::Completed => {
                tx.execute(
                    "UPDATE firmware_analysis_results 
                     SET analysis_status = ?1, completed_at = CURRENT_TIMESTAMP 
                     WHERE id = ?2",
                    rusqlite::params![status.to_string(), id],
                )?;
            }
            AnalysisStatus::Failed => {
                tx.execute(
                    "UPDATE firmware_analysis_results 
                     SET analysis_status = ?1, completed_at = CURRENT_TIMESTAMP, error_message = ?2 
                     WHERE id = ?3",
                    rusqlite::params![status.to_string(), error_message, id],
                )?;
            }
            _ => {
                tx.execute(
                    "UPDATE firmware_analysis_results 
                     SET analysis_status = ?1 
                     WHERE id = ?2",
                    rusqlite::params![status.to_string(), id],
                )?;
            }
        }
        
        tx.commit()?;
        Ok(())
    }
    
    fn update_analysis_results(
        &self,
        id: i64,
        file_type: Option<String>,
        detected_versions: Option<Vec<String>>,
        entropy_score: Option<f64>,
        security_findings: Option<String>,
        raw_results: Option<String>,
    ) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        
        let detected_versions_json = detected_versions
            .map(|v| serde_json::to_string(&v))
            .transpose()?;
        
        tx.execute(
            "UPDATE firmware_analysis_results 
             SET file_type = ?1, detected_versions = ?2, entropy_score = ?3, 
                 security_findings = ?4, raw_results = ?5 
             WHERE id = ?6",
            rusqlite::params![
                file_type,
                detected_versions_json,
                entropy_score,
                security_findings,
                raw_results,
                id
            ],
        )?;
        
        tx.commit()?;
        Ok(())
    }
    
    fn get_analysis_by_firmware_id(&self, firmware_version_id: i64) -> Result<Option<FirmwareAnalysisResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, firmware_version_id, analysis_status, file_type, detected_versions, 
                    entropy_score, security_findings, raw_results, started_at, completed_at, 
                    error_message, created_at 
             FROM firmware_analysis_results 
             WHERE firmware_version_id = ?1"
        )?;
        
        let result = stmt.query_row([firmware_version_id], Self::row_to_analysis_result);
        match result {
            Ok(analysis) => Ok(Some(analysis)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    
    fn get_analysis_by_id(&self, id: i64) -> Result<Option<FirmwareAnalysisResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, firmware_version_id, analysis_status, file_type, detected_versions, 
                    entropy_score, security_findings, raw_results, started_at, completed_at, 
                    error_message, created_at 
             FROM firmware_analysis_results 
             WHERE id = ?1"
        )?;
        
        let result = stmt.query_row([id], Self::row_to_analysis_result);
        match result {
            Ok(analysis) => Ok(Some(analysis)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    
    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        
        // Create firmware_versions table for foreign key
        conn.execute_batch(
            r#"
            CREATE TABLE firmware_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                asset_id INTEGER NOT NULL,
                author_id INTEGER NOT NULL,
                version TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            
            INSERT INTO firmware_versions (asset_id, author_id, version) 
            VALUES (1, 1, '1.0.0');
            "#
        ).unwrap();
        
        let repo = SqliteFirmwareAnalysisRepository::new(&conn);
        repo.initialize_schema().unwrap();
        
        conn
    }
    
    #[test]
    fn test_create_analysis() {
        let conn = setup_test_db();
        let repo = SqliteFirmwareAnalysisRepository::new(&conn);
        
        let id = repo.create_analysis(1).unwrap();
        assert!(id > 0);
        
        let analysis = repo.get_analysis_by_id(id).unwrap().unwrap();
        assert_eq!(analysis.firmware_version_id, 1);
        assert_eq!(analysis.analysis_status, AnalysisStatus::Pending);
    }
    
    #[test]
    fn test_update_analysis_status() {
        let conn = setup_test_db();
        let repo = SqliteFirmwareAnalysisRepository::new(&conn);
        
        let id = repo.create_analysis(1).unwrap();
        
        // Update to in progress
        repo.update_analysis_status(id, AnalysisStatus::InProgress, None).unwrap();
        let analysis = repo.get_analysis_by_id(id).unwrap().unwrap();
        assert_eq!(analysis.analysis_status, AnalysisStatus::InProgress);
        assert!(analysis.started_at.is_some());
        
        // Update to failed
        repo.update_analysis_status(id, AnalysisStatus::Failed, Some("Test error".to_string())).unwrap();
        let analysis = repo.get_analysis_by_id(id).unwrap().unwrap();
        assert_eq!(analysis.analysis_status, AnalysisStatus::Failed);
        assert_eq!(analysis.error_message, Some("Test error".to_string()));
        assert!(analysis.completed_at.is_some());
    }
    
    #[test]
    fn test_update_analysis_results() {
        let conn = setup_test_db();
        let repo = SqliteFirmwareAnalysisRepository::new(&conn);
        
        let id = repo.create_analysis(1).unwrap();
        
        repo.update_analysis_results(
            id,
            Some("ELF".to_string()),
            Some(vec!["1.2.3".to_string(), "4.5.6".to_string()]),
            Some(7.5),
            Some(r#"[{"severity":"info","findingType":"test","description":"test finding"}]"#.to_string()),
            Some(r#"{"test": "results"}"#.to_string()),
        ).unwrap();
        
        let analysis = repo.get_analysis_by_id(id).unwrap().unwrap();
        assert_eq!(analysis.file_type, Some("ELF".to_string()));
        assert_eq!(analysis.detected_versions, Some(vec!["1.2.3".to_string(), "4.5.6".to_string()]));
        assert_eq!(analysis.entropy_score, Some(7.5));
        assert!(analysis.security_findings.is_some());
        assert!(analysis.raw_results.is_some());
    }
    
    #[test]
    fn test_get_analysis_by_firmware_id() {
        let conn = setup_test_db();
        let repo = SqliteFirmwareAnalysisRepository::new(&conn);
        
        // No analysis yet
        let analysis = repo.get_analysis_by_firmware_id(1).unwrap();
        assert!(analysis.is_none());
        
        // Create analysis
        repo.create_analysis(1).unwrap();
        
        // Should find it now
        let analysis = repo.get_analysis_by_firmware_id(1).unwrap();
        assert!(analysis.is_some());
        assert_eq!(analysis.unwrap().firmware_version_id, 1);
    }
}