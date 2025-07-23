use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareAnalysisResult {
    pub id: i64,
    pub firmware_version_id: i64,
    pub analysis_status: AnalysisStatus,
    pub file_type: Option<String>,
    pub detected_versions: Option<Vec<String>>,
    pub entropy_score: Option<f64>,
    pub security_findings: Option<Vec<SecurityFinding>>,
    pub raw_results: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AnalysisStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl std::fmt::Display for AnalysisStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisStatus::Pending => write!(f, "pending"),
            AnalysisStatus::InProgress => write!(f, "in_progress"),
            AnalysisStatus::Completed => write!(f, "completed"),
            AnalysisStatus::Failed => write!(f, "failed"),
        }
    }
}

impl From<String> for AnalysisStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "pending" => AnalysisStatus::Pending,
            "in_progress" => AnalysisStatus::InProgress,
            "completed" => AnalysisStatus::Completed,
            "failed" => AnalysisStatus::Failed,
            _ => AnalysisStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityFinding {
    pub severity: SecuritySeverity,
    pub finding_type: String,
    pub description: String,
    pub offset: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecuritySeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisProgress {
    pub firmware_version_id: i64,
    pub status: AnalysisStatus,
    pub progress_percent: Option<u8>,
    pub current_step: Option<String>,
}