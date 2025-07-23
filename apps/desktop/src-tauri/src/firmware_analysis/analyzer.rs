use anyhow::Result;
use binwalk::Binwalk;
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;
use tracing::info;

use crate::firmware_analysis::models::{SecurityFinding, SecuritySeverity};

pub struct FirmwareAnalyzer;

impl FirmwareAnalyzer {
    pub async fn analyze_firmware(firmware_data: &[u8]) -> Result<AnalysisResult> {
        let analysis_timeout = Duration::from_secs(300); // 5 minutes timeout
        
        match timeout(analysis_timeout, Self::perform_analysis(firmware_data)).await {
            Ok(result) => result,
            Err(_) => Err(anyhow::anyhow!("Analysis timed out after 5 minutes")),
        }
    }
    
    async fn perform_analysis(firmware_data: &[u8]) -> Result<AnalysisResult> {
        info!("Starting firmware analysis");
        
        let file_type = Self::detect_file_type(firmware_data)?;
        let entropy_score = Self::calculate_entropy(firmware_data);
        let detected_versions = Self::extract_version_strings(firmware_data);
        let security_findings = Self::perform_security_checks(firmware_data, &file_type)?;
        
        let binwalk_results = Self::run_binwalk_analysis(firmware_data).await?;
        
        Ok(AnalysisResult {
            file_type,
            entropy_score,
            detected_versions,
            security_findings,
            raw_binwalk_output: binwalk_results,
        })
    }
    
    fn detect_file_type(data: &[u8]) -> Result<String> {
        if data.len() < 4 {
            return Ok("Unknown".to_string());
        }
        
        let file_type = match &data[0..4] {
            [0x7F, 0x45, 0x4C, 0x46] => "ELF",
            [0x4D, 0x5A, _, _] => "PE/COFF",
            [0x50, 0x4B, 0x03, 0x04] => "ZIP",
            [0x50, 0x4B, 0x05, 0x06] => "ZIP",
            [0x50, 0x4B, 0x07, 0x08] => "ZIP",
            [0x1F, 0x8B, _, _] => "GZIP",
            _ => {
                if data.starts_with(b"MZ") {
                    "DOS/Windows Executable"
                } else if data.starts_with(b"#!/") {
                    "Shell Script"
                } else if data.starts_with(b"PK") {
                    "Archive"
                } else {
                    "Binary/Unknown"
                }
            }
        };
        
        Ok(file_type.to_string())
    }
    
    fn calculate_entropy(data: &[u8]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }
        
        let mut freq = [0u64; 256];
        for &byte in data {
            freq[byte as usize] += 1;
        }
        
        let len = data.len() as f64;
        let mut entropy = 0.0;
        
        for &count in &freq {
            if count > 0 {
                let probability = count as f64 / len;
                entropy -= probability * probability.log2();
            }
        }
        
        entropy
    }
    
    fn extract_version_strings(data: &[u8]) -> Vec<String> {
        let mut versions = Vec::new();
        let text = String::from_utf8_lossy(data);
        
        // Common version patterns
        let patterns = [
            r"[Vv]ersion[:\s]+(\d+\.\d+(?:\.\d+)?)",
            r"[Vv]er[:\s]+(\d+\.\d+(?:\.\d+)?)",
            r"[Ff]irmware[:\s]+(\d+\.\d+(?:\.\d+)?)",
            r"[Bb]uild[:\s]+(\d+\.\d+(?:\.\d+)?)",
            r"(\d+\.\d+\.\d+\.\d+)",
            r"(\d+\.\d+\.\d+)",
        ];
        
        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for cap in re.captures_iter(&text) {
                    if let Some(version) = cap.get(1) {
                        let version_str = version.as_str().to_string();
                        if !versions.contains(&version_str) {
                            versions.push(version_str);
                        }
                    }
                }
            }
        }
        
        versions.sort();
        versions.dedup();
        versions.truncate(10); // Limit to 10 most relevant versions
        
        versions
    }
    
    fn perform_security_checks(data: &[u8], file_type: &str) -> Result<Vec<SecurityFinding>> {
        let mut findings = Vec::new();
        
        // Check entropy for potential encryption/packing
        let entropy = Self::calculate_entropy(data);
        if entropy > 7.5 {
            findings.push(SecurityFinding {
                severity: SecuritySeverity::Info,
                finding_type: "High Entropy".to_string(),
                description: format!("File has high entropy ({:.2}), may be encrypted or packed", entropy),
                offset: None,
            });
        }
        
        // Check for known vulnerable signatures
        let vulnerable_patterns: Vec<(&[u8], &str, SecuritySeverity)> = vec![
            (b"admin\0", "Hardcoded Credentials", SecuritySeverity::High),
            (b"password\0", "Hardcoded Credentials", SecuritySeverity::High),
            (b"root\0", "Hardcoded Credentials", SecuritySeverity::High),
            (b"telnetd", "Telnet Service", SecuritySeverity::Medium),
            (b"dropbear", "SSH Service", SecuritySeverity::Low),
        ];
        
        for (pattern, finding_type, severity) in &vulnerable_patterns {
            if let Some(pos) = data.windows(pattern.len()).position(|window| window == *pattern) {
                findings.push(SecurityFinding {
                    severity: severity.clone(),
                    finding_type: finding_type.to_string(),
                    description: format!("Found {} at offset {:#x}", finding_type, pos),
                    offset: Some(pos as u64),
                });
            }
        }
        
        // Check for executable sections in unexpected file types
        if !matches!(file_type, "ELF" | "PE/COFF" | "DOS/Windows Executable") {
            if data.windows(4).any(|w| w == b"\x7FELF" || w == b"MZ\x90\0") {
                findings.push(SecurityFinding {
                    severity: SecuritySeverity::Medium,
                    finding_type: "Hidden Executable".to_string(),
                    description: "Found executable code in non-executable file type".to_string(),
                    offset: None,
                });
            }
        }
        
        Ok(findings)
    }
    
    async fn run_binwalk_analysis(data: &[u8]) -> Result<String> {
        // Initialize binwalk
        let binwalk = Binwalk::new();
        
        // Run analysis directly on data
        let results = binwalk.scan(data);
        
        // Convert results to JSON
        let json_results: Vec<_> = results.into_iter().map(|r| {
            json!({
                "offset": r.offset,
                "description": r.description,
                "size": r.size,
            })
        }).collect();
        
        Ok(json!({
            "signatures": json_results,
            "total_signatures": json_results.len(),
        }).to_string())
    }
    
}

pub struct AnalysisResult {
    pub file_type: String,
    pub entropy_score: f64,
    pub detected_versions: Vec<String>,
    pub security_findings: Vec<SecurityFinding>,
    pub raw_binwalk_output: String,
}

// Add regex dependency for version extraction
use once_cell::sync::Lazy;
static _REGEX_DEPENDENCY: Lazy<()> = Lazy::new(|| {
    // This ensures regex is available
});

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_file_type() {
        let elf_header = vec![0x7F, 0x45, 0x4C, 0x46];
        assert_eq!(FirmwareAnalyzer::detect_file_type(&elf_header).unwrap(), "ELF");
        
        let zip_header = vec![0x50, 0x4B, 0x03, 0x04];
        assert_eq!(FirmwareAnalyzer::detect_file_type(&zip_header).unwrap(), "ZIP");
        
        let unknown = vec![0x00, 0x01, 0x02, 0x03];
        assert_eq!(FirmwareAnalyzer::detect_file_type(&unknown).unwrap(), "Binary/Unknown");
    }
    
    #[test]
    fn test_calculate_entropy() {
        let zeros = vec![0u8; 100];
        assert_eq!(FirmwareAnalyzer::calculate_entropy(&zeros), 0.0);
        
        let random: Vec<u8> = (0..256).map(|i| i as u8).collect();
        let entropy = FirmwareAnalyzer::calculate_entropy(&random);
        assert!(entropy > 7.0 && entropy <= 8.0);
    }
    
    #[test]
    fn test_extract_version_strings() {
        let data = b"Firmware Version: 1.2.3\nBuild: 4.5.6\nver 7.8.9";
        let versions = FirmwareAnalyzer::extract_version_strings(data);
        assert!(versions.contains(&"1.2.3".to_string()));
        assert!(versions.contains(&"4.5.6".to_string()));
        assert!(versions.contains(&"7.8.9".to_string()));
    }
    
    #[test]
    fn test_security_checks() {
        let data = b"This firmware contains admin\0password\0data";
        let findings = FirmwareAnalyzer::perform_security_checks(data, "Binary").unwrap();
        assert!(findings.iter().any(|f| f.finding_type == "Hardcoded Credentials"));
    }
}