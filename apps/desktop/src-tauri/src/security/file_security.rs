use super::SecurityError;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use tracing::{error, warn, info, debug};
use unicode_normalization::UnicodeNormalization;

/// File security validator for comprehensive file upload security
/// Handles filename sanitization, content validation, and security scanning
pub struct FileSecurityValidator {
    dangerous_extensions: &'static HashSet<String>,
    safe_extensions: &'static HashSet<String>,
    max_filename_length: usize,
    max_file_size: u64,
}

impl FileSecurityValidator {
    pub fn new() -> Self {
        Self {
            dangerous_extensions: Self::get_dangerous_extensions(),
            safe_extensions: Self::get_safe_extensions(),
            max_filename_length: 255,
            max_file_size: 100 * 1024 * 1024, // 100MB
        }
    }

    /// Get list of dangerous file extensions
    fn get_dangerous_extensions() -> &'static HashSet<String> {
        static DANGEROUS_EXTENSIONS: OnceLock<HashSet<String>> = OnceLock::new();
        DANGEROUS_EXTENSIONS.get_or_init(|| {
            let mut extensions = HashSet::new();
            
            // Executable files
            let executables = vec![
                "exe", "com", "scr", "bat", "cmd", "pif", "vbs", "vbe", "js", "jse",
                "wsf", "wsh", "msi", "msp", "hta", "cpl", "scf", "lnk", "inf",
                "reg", "dll", "ocx", "sys", "drv", "bin", "run", "app", "deb",
                "rpm", "dmg", "pkg", "appimage", "snap", "flatpak",
            ];

            // Script files
            let scripts = vec![
                "ps1", "ps2", "psc1", "psc2", "msh", "msh1", "msh2", "mshxml",
                "sh", "bash", "zsh", "fish", "csh", "tcsh", "ksh", "rb", "py",
                "pl", "php", "asp", "aspx", "jsp", "jar", "class", "swift",
            ];

            // Archive files that might contain executables
            let archives = vec![
                "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "z", "lz", "lzma",
                "cab", "ace", "arc", "arj", "lha", "lzh", "zoo", "cpio", "rpm",
                "deb", "dmg", "iso", "img", "bin", "nrg", "mdf", "cue",
            ];

            // Document files with macro capabilities
            let macro_documents = vec![
                "doc", "docx", "docm", "dot", "dotx", "dotm", "xls", "xlsx",
                "xlsm", "xlt", "xltx", "xltm", "ppt", "pptx", "pptm", "pot",
                "potx", "potm", "pps", "ppsx", "ppsm", "rtf",
            ];

            // Add all extensions in lowercase
            for ext in executables.iter()
                .chain(scripts.iter())
                .chain(archives.iter())
                .chain(macro_documents.iter()) {
                extensions.insert(ext.to_lowercase());
                extensions.insert(ext.to_uppercase());
            }

            extensions
        })
    }

    /// Get list of generally safe file extensions for industrial use
    fn get_safe_extensions() -> &'static HashSet<String> {
        static SAFE_EXTENSIONS: OnceLock<HashSet<String>> = OnceLock::new();
        SAFE_EXTENSIONS.get_or_init(|| {
            let mut extensions = HashSet::new();
            
            // Configuration and data files
            let config_files = vec![
                "cfg", "conf", "config", "ini", "properties", "settings",
                "json", "yaml", "yml", "toml", "xml", "csv", "tsv",
            ];

            // Documentation and text files
            let text_files = vec![
                "txt", "md", "rst", "log", "readme", "changelog", "license",
                "pdf", "html", "htm", "css", "svg",
            ];

            // Industrial automation files
            let industrial_files = vec![
                "acd", "mer", "apa", "l5k", "l5x", "rss", "rsp", "fhx",
                "gxw", "wlm", "s7p", "awl", "db", "udt", "fb", "fc",
                "step7", "tia", "zap15", "xti", "zap", "s5d", "s7",
            ];

            // Image files (generally safe)
            let image_files = vec![
                "png", "jpg", "jpeg", "gif", "bmp", "tiff", "tif", "webp",
                "ico", "svg",
            ];

            // Data and backup files
            let data_files = vec![
                "dat", "data", "backup", "bak", "tmp", "temp", "cache",
                "db", "sqlite", "sqlite3", "mdb", "accdb",
            ];

            // Add all extensions in lowercase
            for ext in config_files.iter()
                .chain(text_files.iter())
                .chain(industrial_files.iter())
                .chain(image_files.iter())
                .chain(data_files.iter()) {
                extensions.insert(ext.to_lowercase());
                extensions.insert(ext.to_uppercase());
            }

            extensions
        })
    }

    /// Sanitize filename for security
    pub fn sanitize_filename(&self, filename: &str) -> Result<String, SecurityError> {
        debug!("Sanitizing filename: {}", filename);

        // Unicode normalization
        let normalized: String = filename.nfc().collect();
        
        // Remove or replace dangerous characters
        let dangerous_chars = ['<', '>', ':', '"', '|', '?', '*', '\\', '/', '\0'];
        let mut sanitized = String::new();
        
        for ch in normalized.chars() {
            if dangerous_chars.contains(&ch) {
                sanitized.push('_');
            } else if ch.is_control() && !ch.is_whitespace() {
                // Replace control characters with underscore
                sanitized.push('_');
            } else if ch as u32 > 127 {
                // Replace non-ASCII characters with underscore to avoid encoding issues
                sanitized.push('_');
            } else {
                sanitized.push(ch);
            }
        }

        // Trim whitespace and dots from ends (Windows restriction)
        let trimmed = sanitized.trim_matches(|c: char| c.is_whitespace() || c == '.').to_string();
        
        if trimmed.is_empty() {
            return Err(SecurityError::FileSecurityViolation {
                filename: filename.to_string(),
                violation: "Filename becomes empty after sanitization".to_string(),
            });
        }

        // Check length
        if trimmed.len() > self.max_filename_length {
            let truncated = if let Some(dot_pos) = trimmed.rfind('.') {
                let name_part = &trimmed[..dot_pos];
                let ext_part = &trimmed[dot_pos..];
                let max_name_len = self.max_filename_length - ext_part.len();
                if max_name_len > 0 {
                    format!("{}{}", &name_part[..max_name_len.min(name_part.len())], ext_part)
                } else {
                    trimmed[..self.max_filename_length].to_string()
                }
            } else {
                trimmed[..self.max_filename_length].to_string()
            };
            
            warn!("Filename too long, truncated: {} -> {}", filename, truncated);
            return Ok(truncated);
        }

        // Check for reserved Windows names in the base filename
        let base_name = if let Some(dot_pos) = trimmed.rfind('.') {
            &trimmed[..dot_pos]
        } else {
            &trimmed
        };

        let windows_reserved = vec![
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5",
            "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4",
            "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ];

        if windows_reserved.contains(&base_name.to_uppercase().as_str()) {
            let safe_name = format!("FILE_{}", trimmed);
            warn!("Reserved filename detected, prefixed: {} -> {}", filename, safe_name);
            return Ok(safe_name);
        }

        info!("Filename sanitized successfully: {} -> {}", filename, trimmed);
        Ok(trimmed)
    }

    /// Check file extension for security concerns
    pub fn check_file_extension(&self, filename: &str) -> Result<(), SecurityError> {
        let path = Path::new(filename);
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        if extension.is_empty() {
            warn!("File has no extension: {}", filename);
            return Ok(()); // Allow files without extensions
        }

        // Check if extension is dangerous
        if self.dangerous_extensions.contains(&extension) {
            error!("Dangerous file extension detected: {} ({})", filename, extension);
            return Err(SecurityError::FileSecurityViolation {
                filename: filename.to_string(),
                violation: format!("Dangerous file extension: .{}", extension),
            });
        }

        // Log if extension is not in safe list (warning, not error)
        if !self.safe_extensions.contains(&extension) {
            warn!("Unknown file extension (proceeding with caution): {} ({})", filename, extension);
        }

        Ok(())
    }

    /// Validate file size
    pub fn check_file_size(&self, file_path: &str) -> Result<(), SecurityError> {
        let metadata = fs::metadata(file_path).map_err(|e| SecurityError::FileSecurityViolation {
            filename: file_path.to_string(),
            violation: format!("Cannot read file metadata: {}", e),
        })?;

        let file_size = metadata.len();
        
        if file_size > self.max_file_size {
            error!("File too large: {} ({} bytes, max: {} bytes)", 
                   file_path, file_size, self.max_file_size);
            return Err(SecurityError::FileSecurityViolation {
                filename: file_path.to_string(),
                violation: format!("File too large: {} bytes (max: {} bytes)", 
                                 file_size, self.max_file_size),
            });
        }

        if file_size == 0 {
            warn!("Empty file detected: {}", file_path);
        }

        debug!("File size OK: {} ({} bytes)", file_path, file_size);
        Ok(())
    }

    /// Check file content for magic numbers and basic security
    pub fn check_file_content(&self, file_path: &str) -> Result<Vec<String>, SecurityError> {
        let mut issues = Vec::new();
        
        // Read first few bytes for magic number checking
        let file_content = fs::read(file_path).map_err(|e| SecurityError::FileSecurityViolation {
            filename: file_path.to_string(),
            violation: format!("Cannot read file content: {}", e),
        })?;

        if file_content.is_empty() {
            issues.push("File is empty".to_string());
            return Ok(issues);
        }

        // Check for executable magic numbers
        let magic_checks: Vec<(&[u8], &str)> = vec![
            (&[0x4D, 0x5A], "PE executable"), // MZ header
            (&[0x7F, 0x45, 0x4C, 0x46], "ELF executable"), // ELF header
            (&[0xFE, 0xED, 0xFA, 0xCE], "Mach-O executable"), // Mach-O 32-bit
            (&[0xFE, 0xED, 0xFA, 0xCF], "Mach-O executable"), // Mach-O 64-bit
            (&[0xCA, 0xFE, 0xBA, 0xBE], "Java class file"), // Java class
            (&[0x50, 0x4B, 0x03, 0x04], "ZIP archive"), // ZIP/JAR/Office docs
            (&[0x50, 0x4B, 0x05, 0x06], "ZIP archive"), // Empty ZIP
            (&[0x50, 0x4B, 0x07, 0x08], "ZIP archive"), // Spanned ZIP
            (&[0x52, 0x61, 0x72, 0x21], "RAR archive"), // RAR
            (&[0x37, 0x7A, 0xBC, 0xAF], "7-Zip archive"), // 7z
        ];

        for (magic_bytes, file_type) in magic_checks {
            if file_content.len() >= magic_bytes.len() && file_content.starts_with(magic_bytes) {
                // For ZIP files, we need to be more careful as many safe formats use ZIP
                if file_type == "ZIP archive" {
                    let filename_lower = file_path.to_lowercase();
                    if filename_lower.ends_with(".jar") || filename_lower.ends_with(".war") || 
                       filename_lower.ends_with(".ear") {
                        issues.push(format!("Potentially executable archive: {}", file_type));
                    } else if filename_lower.ends_with(".docx") || filename_lower.ends_with(".xlsx") || 
                             filename_lower.ends_with(".pptx") {
                        // Office documents are ZIP-based but generally safe
                        debug!("Office document detected: {}", file_path);
                    } else {
                        issues.push(format!("Archive file detected: {}", file_type));
                    }
                } else if file_type.contains("executable") || file_type.contains("class") {
                    error!("Executable content detected in {}: {}", file_path, file_type);
                    return Err(SecurityError::MaliciousContentDetected {
                        content_type: file_type.to_string(),
                        details: "Executable magic number detected".to_string(),
                    });
                } else {
                    issues.push(format!("Archive detected: {}", file_type));
                }
            }
        }

        // Check for embedded scripts or suspicious patterns
        let content_str = String::from_utf8_lossy(&file_content[..1024.min(file_content.len())]);
        let suspicious_patterns = vec![
            ("javascript:", "JavaScript execution"),
            ("vbscript:", "VBScript execution"),
            ("<script", "Script tags"),
            ("eval(", "Code evaluation"),
            ("exec(", "Code execution"),
            ("system(", "System command"),
            ("shell(", "Shell command"),
            ("cmd.exe", "Command prompt"),
            ("powershell", "PowerShell"),
            ("/bin/sh", "Shell execution"),
            ("/bin/bash", "Bash execution"),
        ];

        for (pattern, description) in suspicious_patterns {
            if content_str.to_lowercase().contains(pattern) {
                warn!("Suspicious pattern detected in {}: {}", file_path, description);
                issues.push(format!("Suspicious content: {}", description));
            }
        }

        // Check for excessive null bytes (might indicate binary executable)
        let null_count = file_content.iter().filter(|&&b| b == 0).count();
        let null_ratio = null_count as f64 / file_content.len() as f64;
        
        if null_ratio > 0.3 && file_content.len() > 1024 {
            issues.push("High ratio of null bytes (possibly binary executable)".to_string());
        }

        Ok(issues)
    }

    /// Comprehensive file security scan
    pub fn scan_file(&self, file_path: &str) -> Result<Vec<String>, SecurityError> {
        let mut all_issues = Vec::new();

        // Check file size
        self.check_file_size(file_path)?;

        // Check file extension
        self.check_file_extension(file_path)?;

        // Check file content
        let content_issues = self.check_file_content(file_path)?;
        all_issues.extend(content_issues);

        // Additional path validation
        if file_path.contains("..") {
            return Err(SecurityError::PathTraversalAttempt {
                path: file_path.to_string(),
            });
        }

        info!("File security scan completed: {} ({} issues)", file_path, all_issues.len());
        Ok(all_issues)
    }

    /// Generate safe filename from potentially unsafe input  
    pub fn generate_safe_filename(&self, base_name: &str, extension: Option<&str>) -> String {
        let safe_base = self.sanitize_filename(base_name).unwrap_or_else(|_| "file".to_string());
        
        match extension {
            Some(ext) if !ext.is_empty() => {
                let clean_ext = ext.trim_start_matches('.').to_lowercase();
                if self.safe_extensions.contains(&clean_ext) {
                    format!("{}.{}", safe_base, clean_ext)
                } else {
                    format!("{}.txt", safe_base)
                }
            }
            _ => format!("{}.txt", safe_base),
        }
    }

    /// Check if file type is allowed for upload
    pub fn is_file_type_allowed(&self, filename: &str) -> bool {
        !matches!(self.check_file_extension(filename), Err(_))
    }

    /// Get list of safe extensions
    pub fn get_allowed_extensions(&self) -> Vec<String> {
        self.safe_extensions.iter().cloned().collect()
    }

    /// Get maximum file size
    pub fn get_max_file_size(&self) -> u64 {
        self.max_file_size
    }

    /// Set maximum file size (for configuration)
    pub fn set_max_file_size(&mut self, size: u64) {
        self.max_file_size = size;
        info!("Maximum file size updated to: {} bytes", size);
    }
}

impl Default for FileSecurityValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_filename_sanitization() {
        let validator = FileSecurityValidator::new();

        // Test dangerous characters
        assert_eq!(validator.sanitize_filename("test<>file.txt").unwrap(), "test__file.txt");
        assert_eq!(validator.sanitize_filename("file|name?.txt").unwrap(), "file_name_.txt");
        assert_eq!(validator.sanitize_filename("path\\to\\file.txt").unwrap(), "path_to_file.txt");

        // Test Windows reserved names
        assert_eq!(validator.sanitize_filename("CON.txt").unwrap(), "FILE_CON.txt");
        assert_eq!(validator.sanitize_filename("PRN.dat").unwrap(), "FILE_PRN.dat");

        // Test unicode normalization
        let result = validator.sanitize_filename("café.txt");
        assert!(result.is_ok());

        // Test empty filename after sanitization
        let result = validator.sanitize_filename("...");
        assert!(result.is_err());
    }

    #[test]
    fn test_file_extension_checking() {
        let validator = FileSecurityValidator::new();

        // Test dangerous extensions
        assert!(validator.check_file_extension("malware.exe").is_err());
        assert!(validator.check_file_extension("script.bat").is_err());
        assert!(validator.check_file_extension("virus.scr").is_err());

        // Test safe extensions
        assert!(validator.check_file_extension("config.cfg").is_ok());
        assert!(validator.check_file_extension("data.json").is_ok());
        assert!(validator.check_file_extension("readme.txt").is_ok());

        // Test no extension
        assert!(validator.check_file_extension("filename").is_ok());
    }

    #[test]
    fn test_file_content_checking() {
        let temp_dir = tempdir().unwrap();
        let validator = FileSecurityValidator::new();

        // Test text file
        let text_file = temp_dir.path().join("test.txt");
        fs::write(&text_file, "This is a normal text file.").unwrap();
        let issues = validator.check_file_content(text_file.to_str().unwrap()).unwrap();
        assert!(issues.is_empty());

        // Test executable content
        let exe_file = temp_dir.path().join("test.exe");
        fs::write(&exe_file, b"\x4D\x5A\x90\x00").unwrap(); // MZ header
        let result = validator.check_file_content(exe_file.to_str().unwrap());
        assert!(result.is_err());

        // Test suspicious script content
        let script_file = temp_dir.path().join("script.txt");
        fs::write(&script_file, "This file contains javascript: in it.").unwrap();
        let issues = validator.check_file_content(script_file.to_str().unwrap()).unwrap();
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_file_size_checking() {
        let temp_dir = tempdir().unwrap();
        let validator = FileSecurityValidator::new();

        // Test normal size file
        let normal_file = temp_dir.path().join("normal.txt");
        fs::write(&normal_file, "Normal size content").unwrap();
        assert!(validator.check_file_size(normal_file.to_str().unwrap()).is_ok());

        // Test empty file
        let empty_file = temp_dir.path().join("empty.txt");
        fs::write(&empty_file, "").unwrap();
        assert!(validator.check_file_size(empty_file.to_str().unwrap()).is_ok());
    }

    #[test]
    fn test_safe_filename_generation() {
        let validator = FileSecurityValidator::new();

        let safe_name = validator.generate_safe_filename("test file", Some("cfg"));
        assert_eq!(safe_name, "test file.cfg");

        let safe_name = validator.generate_safe_filename("dangerous<>name", Some("exe"));
        assert_eq!(safe_name, "dangerous__name.txt"); // exe -> txt

        let safe_name = validator.generate_safe_filename("test", None);
        assert_eq!(safe_name, "test.txt");
    }

    #[test]
    fn test_comprehensive_file_scan() {
        let temp_dir = tempdir().unwrap();
        let validator = FileSecurityValidator::new();

        // Test safe file
        let safe_file = temp_dir.path().join("config.json");
        fs::write(&safe_file, r#"{"setting": "value"}"#).unwrap();
        let issues = validator.scan_file(safe_file.to_str().unwrap()).unwrap();
        assert!(issues.is_empty());

        // Test file with issues
        let suspicious_file = temp_dir.path().join("suspicious.txt");
        fs::write(&suspicious_file, "This contains javascript: alert('xss')").unwrap();
        let issues = validator.scan_file(suspicious_file.to_str().unwrap()).unwrap();
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_path_traversal_detection() {
        let validator = FileSecurityValidator::new();

        let result = validator.scan_file("../../../etc/passwd");
        assert!(result.is_err());
        
        if let Err(SecurityError::PathTraversalAttempt { path }) = result {
            assert_eq!(path, "../../../etc/passwd");
        } else {
            panic!("Expected PathTraversalAttempt error");
        }
    }

    #[test]
    fn test_allowed_extensions() {
        let validator = FileSecurityValidator::new();
        
        let allowed = validator.get_allowed_extensions();
        assert!(!allowed.is_empty());
        assert!(allowed.contains(&"cfg".to_string()));
        assert!(allowed.contains(&"json".to_string()));
    }

    #[test]
    fn test_max_file_size_configuration() {
        let mut validator = FileSecurityValidator::new();
        
        let original_size = validator.get_max_file_size();
        assert_eq!(original_size, 100 * 1024 * 1024);
        
        validator.set_max_file_size(50 * 1024 * 1024);
        assert_eq!(validator.get_max_file_size(), 50 * 1024 * 1024);
    }
}