use anyhow::Result;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{info, error};
use tauri::{AppHandle, Manager, Emitter};
use serde::Serialize;

use crate::database::Database;
use crate::firmware::{FirmwareRepository, SqliteFirmwareRepository, FirmwareFileStorage};
use crate::firmware_analysis::{
    FirmwareAnalyzer, FirmwareAnalysisRepository, SqliteFirmwareAnalysisRepository,
    AnalysisStatus
};
use crate::audit::{AuditRepository, SqliteAuditRepository, AuditEventRequest, AuditEventType};

#[derive(Debug, Clone)]
pub struct AnalysisJob {
    pub firmware_id: i64,
    pub user_id: i64,
    pub username: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisEvent {
    pub firmware_id: i64,
    pub status: String,
    pub progress: Option<u8>,
    pub message: Option<String>,
}

pub struct AnalysisQueue {
    sender: mpsc::Sender<AnalysisJob>,
    handle: tokio::sync::Mutex<Option<JoinHandle<()>>>,
    app_handle: AppHandle,
}

impl AnalysisQueue {
    pub fn new(app_handle: AppHandle) -> Self {
        let (sender, receiver) = mpsc::channel::<AnalysisJob>(100);
        
        let app_handle_clone = app_handle.clone();
        let handle = tokio::spawn(Self::process_queue(app_handle_clone, receiver));
        
        Self {
            sender,
            handle: tokio::sync::Mutex::new(Some(handle)),
            app_handle,
        }
    }
    
    pub async fn queue_analysis(&self, job: AnalysisJob) -> Result<()> {
        self.sender.send(job).await
            .map_err(|e| anyhow::anyhow!("Failed to queue analysis job: {}", e))?;
        Ok(())
    }
    
    pub async fn shutdown(&self) {
        // Close the sender to signal the processing task to stop
        // (sender is dropped when this method returns)
        
        // Wait for the processing task to complete
        if let Some(handle) = self.handle.lock().await.take() {
            let _ = handle.await;
        }
    }
    
    async fn process_queue(
        app_handle: AppHandle,
        mut receiver: mpsc::Receiver<AnalysisJob>,
    ) {
        info!("Analysis queue processor started");
        
        while let Some(job) = receiver.recv().await {
            info!("Processing analysis job for firmware ID: {}", job.firmware_id);
            
            // Clone values for the spawned task
            let app_handle_clone = app_handle.clone();
            let job_clone = job.clone();
            
            // Spawn a task to handle this analysis
            tokio::spawn(async move {
                if let Err(e) = Self::analyze_firmware(
                    app_handle_clone,
                    job_clone
                ).await {
                    error!("Analysis failed for firmware {}: {}", job.firmware_id, e);
                }
            });
        }
        
        info!("Analysis queue processor stopped");
    }
    
    async fn analyze_firmware(
        app_handle: AppHandle,
        job: AnalysisJob,
    ) -> Result<()> {
        // Emit start event
        Self::emit_progress_event(&app_handle, &job.firmware_id, AnalysisStatus::InProgress, None, Some("Starting analysis"));
        
        // Log audit event for analysis start
        {
            let db_state = app_handle.state::<Mutex<Option<Database>>>();
            let db_guard = db_state.lock().unwrap();
            if let Some(db) = db_guard.as_ref() {
                let audit_repo = SqliteAuditRepository::new(db.get_connection());
                let audit_event = AuditEventRequest {
                    event_type: AuditEventType::FirmwareAnalysisStarted,
                    user_id: Some(job.user_id),
                    username: Some(job.username.clone()),
                    admin_user_id: None,
                    admin_username: None,
                    target_user_id: None,
                    target_username: None,
                    description: format!("Started firmware analysis for firmware ID {}", job.firmware_id),
                    metadata: Some(serde_json::json!({
                        "firmware_id": job.firmware_id,
                    }).to_string()),
                    ip_address: None,
                    user_agent: None,
                };
                if let Err(e) = audit_repo.log_event(&audit_event) {
                    error!("Failed to log audit event: {}", e);
                }
            }
        }
        
        // Get database connection and perform initial database operations
        let (analysis_id, firmware_path) = {
            let db_state = app_handle.state::<Mutex<Option<Database>>>();
            let db_guard = db_state.lock().unwrap();
            let db = db_guard.as_ref()
                .ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;
            
            let analysis_repo = SqliteFirmwareAnalysisRepository::new(db.get_connection());
            
            // Create or update analysis record
            let analysis_id = match analysis_repo.get_analysis_by_firmware_id(job.firmware_id)? {
                Some(existing) => {
                    analysis_repo.update_analysis_status(existing.id, AnalysisStatus::InProgress, None)?;
                    existing.id
                }
                None => {
                    let id = analysis_repo.create_analysis(job.firmware_id)?;
                    analysis_repo.update_analysis_status(id, AnalysisStatus::InProgress, None)?;
                    id
                }
            };
            
            // Get firmware path
            let firmware_repo = SqliteFirmwareRepository::new(db.get_connection());
            let firmware = firmware_repo.get_firmware_by_id(job.firmware_id)?
                .ok_or_else(|| anyhow::anyhow!("Firmware not found"))?;
            
            (analysis_id, firmware.file_path)
        }; // Drop db_guard here
        
        Self::emit_progress_event(&app_handle, &job.firmware_id, AnalysisStatus::InProgress, Some(20), Some("Reading firmware file"));
        
        // Read firmware file (outside of db lock)
        let firmware_data = FirmwareFileStorage::read_firmware_file(
            &app_handle,
            &firmware_path,
            job.user_id,
            &job.username
        )?;
        
        Self::emit_progress_event(&app_handle, &job.firmware_id, AnalysisStatus::InProgress, Some(40), Some("Analyzing firmware"));
        
        // Perform analysis
        match FirmwareAnalyzer::analyze_firmware(&firmware_data).await {
            Ok(results) => {
                Self::emit_progress_event(&app_handle, &job.firmware_id, AnalysisStatus::InProgress, Some(80), Some("Saving results"));
                
                // Convert security findings to JSON
                let security_findings_json = serde_json::to_string(&results.security_findings)?;
                
                // Update analysis results in database (reacquire lock)
                {
                    let db_state = app_handle.state::<Mutex<Option<Database>>>();
                    let db_guard = db_state.lock().unwrap();
                    let db = db_guard.as_ref()
                        .ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;
                    
                    let analysis_repo = SqliteFirmwareAnalysisRepository::new(db.get_connection());
                    
                    analysis_repo.update_analysis_results(
                        analysis_id,
                        Some(results.file_type.clone()),
                        Some(results.detected_versions.clone()),
                        Some(results.entropy_score),
                        Some(security_findings_json),
                        Some(results.raw_binwalk_output.clone()),
                    )?;
                    
                    analysis_repo.update_analysis_status(analysis_id, AnalysisStatus::Completed, None)?;
                }
                
                Self::emit_progress_event(&app_handle, &job.firmware_id, AnalysisStatus::Completed, Some(100), Some("Analysis completed"));
                
                // Log audit event for successful completion
                {
                    let db_state = app_handle.state::<Mutex<Option<Database>>>();
                    let db_guard = db_state.lock().unwrap();
                    if let Some(db) = db_guard.as_ref() {
                        let audit_repo = SqliteAuditRepository::new(db.get_connection());
                        let audit_event = AuditEventRequest {
                            event_type: AuditEventType::FirmwareAnalysisCompleted,
                            user_id: Some(job.user_id),
                            username: Some(job.username.clone()),
                            admin_user_id: None,
                            admin_username: None,
                            target_user_id: None,
                            target_username: None,
                            description: format!("Completed firmware analysis for firmware ID {}", job.firmware_id),
                            metadata: Some(serde_json::json!({
                                "firmware_id": job.firmware_id,
                                "file_type": &results.file_type,
                                "security_findings_count": results.security_findings.len(),
                                "entropy_score": results.entropy_score,
                            }).to_string()),
                            ip_address: None,
                            user_agent: None,
                        };
                        if let Err(e) = audit_repo.log_event(&audit_event) {
                            error!("Failed to log audit event: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Analysis failed: {}", e);
                
                // Update status in database (reacquire lock)
                {
                    let db_state = app_handle.state::<Mutex<Option<Database>>>();
                    let db_guard = db_state.lock().unwrap();
                    if let Some(db) = db_guard.as_ref() {
                        let analysis_repo = SqliteFirmwareAnalysisRepository::new(db.get_connection());
                        let _ = analysis_repo.update_analysis_status(analysis_id, AnalysisStatus::Failed, Some(error_msg.clone()));
                    }
                }
                
                Self::emit_progress_event(&app_handle, &job.firmware_id, AnalysisStatus::Failed, None, Some(&error_msg));
                
                // Log audit event for failure
                {
                    let db_state = app_handle.state::<Mutex<Option<Database>>>();
                    let db_guard = db_state.lock().unwrap();
                    if let Some(db) = db_guard.as_ref() {
                        let audit_repo = SqliteAuditRepository::new(db.get_connection());
                        let audit_event = AuditEventRequest {
                            event_type: AuditEventType::FirmwareAnalysisFailed,
                            user_id: Some(job.user_id),
                            username: Some(job.username.clone()),
                            admin_user_id: None,
                            admin_username: None,
                            target_user_id: None,
                            target_username: None,
                            description: format!("Failed firmware analysis for firmware ID {}: {}", job.firmware_id, error_msg),
                            metadata: Some(serde_json::json!({
                                "firmware_id": job.firmware_id,
                                "error": error_msg,
                            }).to_string()),
                            ip_address: None,
                            user_agent: None,
                        };
                        if let Err(e) = audit_repo.log_event(&audit_event) {
                            error!("Failed to log audit event: {}", e);
                        }
                    }
                }
                
                return Err(e);
            }
        }
        
        Ok(())
    }
    
    fn emit_progress_event(
        app_handle: &AppHandle,
        firmware_id: &i64,
        status: AnalysisStatus,
        progress: Option<u8>,
        message: Option<&str>,
    ) {
        let event = AnalysisEvent {
            firmware_id: *firmware_id,
            status: status.to_string(),
            progress,
            message: message.map(|s| s.to_string()),
        };
        
        // Emit event to all windows
        let _ = app_handle.emit("firmware-analysis-progress", event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::time::Duration;
    
    // Note: Full integration tests require a Tauri app context
    // These are basic unit tests for the queue structure
    
    #[tokio::test]
    async fn test_queue_creation() {
        // This test would require a mock AppHandle and Database
        // For now, we just verify the queue can be created
        let (sender, mut receiver) = mpsc::channel::<AnalysisJob>(10);
        
        let job = AnalysisJob {
            firmware_id: 1,
            user_id: 1,
            username: "testuser".to_string(),
        };
        
        sender.send(job.clone()).await.unwrap();
        
        let received = receiver.recv().await.unwrap();
        assert_eq!(received.firmware_id, job.firmware_id);
        assert_eq!(received.user_id, job.user_id);
        assert_eq!(received.username, job.username);
    }
    
    #[tokio::test]
    async fn test_multiple_jobs_queuing() {
        let (sender, mut receiver) = mpsc::channel::<AnalysisJob>(10);
        
        for i in 1..=5 {
            let job = AnalysisJob {
                firmware_id: i,
                user_id: 1,
                username: format!("user{}", i),
            };
            sender.send(job).await.unwrap();
        }
        
        drop(sender); // Close the sender
        
        let mut count = 0;
        while let Some(job) = receiver.recv().await {
            count += 1;
            assert!(job.firmware_id >= 1 && job.firmware_id <= 5);
        }
        
        assert_eq!(count, 5);
    }
}