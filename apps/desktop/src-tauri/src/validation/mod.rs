use anyhow::Result;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{warn, error};

#[derive(Debug)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ValidationError {}

pub struct UsernameValidator;

impl UsernameValidator {
    pub fn validate(username: &str) -> Result<(), ValidationError> {
        if username.trim().is_empty() {
            return Err(ValidationError {
                field: "username".to_string(),
                message: "Username cannot be empty".to_string(),
            });
        }

        if username.len() < 3 {
            return Err(ValidationError {
                field: "username".to_string(),
                message: "Username must be at least 3 characters long".to_string(),
            });
        }

        if username.len() > 50 {
            return Err(ValidationError {
                field: "username".to_string(),
                message: "Username cannot exceed 50 characters".to_string(),
            });
        }

        // Check for valid characters (alphanumeric, underscore, hyphen)
        if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(ValidationError {
                field: "username".to_string(),
                message: "Username can only contain letters, numbers, hyphens, and underscores".to_string(),
            });
        }

        // Check for reserved usernames
        let reserved_usernames = vec![
            "admin", "administrator", "root", "system", "guest", "user", "test", "demo",
            "ferrocodex", "ferrocode", "ferro", "codex", "password", "login", "logout",
            "api", "www", "ftp", "mail", "email", "support", "help", "service",
        ];

        if reserved_usernames.contains(&username.to_lowercase().as_str()) {
            return Err(ValidationError {
                field: "username".to_string(),
                message: "Username is reserved and cannot be used".to_string(),
            });
        }

        Ok(())
    }
}

pub struct PasswordValidator;

impl PasswordValidator {
    pub fn validate(password: &str) -> Result<(), ValidationError> {
        if password.is_empty() {
            return Err(ValidationError {
                field: "password".to_string(),
                message: "Password cannot be empty".to_string(),
            });
        }

        if password.len() < 8 {
            return Err(ValidationError {
                field: "password".to_string(),
                message: "Password must be at least 8 characters long".to_string(),
            });
        }

        if password.len() > 128 {
            return Err(ValidationError {
                field: "password".to_string(),
                message: "Password cannot exceed 128 characters".to_string(),
            });
        }

        // Check for at least one uppercase letter
        if !password.chars().any(|c| c.is_uppercase()) {
            return Err(ValidationError {
                field: "password".to_string(),
                message: "Password must contain at least one uppercase letter".to_string(),
            });
        }

        // Check for at least one lowercase letter
        if !password.chars().any(|c| c.is_lowercase()) {
            return Err(ValidationError {
                field: "password".to_string(),
                message: "Password must contain at least one lowercase letter".to_string(),
            });
        }

        // Check for at least one digit
        if !password.chars().any(|c| c.is_ascii_digit()) {
            return Err(ValidationError {
                field: "password".to_string(),
                message: "Password must contain at least one digit".to_string(),
            });
        }

        // Check for at least one special character
        let special_chars = "!@#$%^&*()_+-=[]{}|;:,.<>?";
        if !password.chars().any(|c| special_chars.contains(c)) {
            return Err(ValidationError {
                field: "password".to_string(),
                message: "Password must contain at least one special character".to_string(),
            });
        }

        // Check for common weak passwords
        let weak_passwords = vec![
            "password", "password123", "12345678", "qwerty", "admin123",
            "letmein", "welcome", "monkey", "dragon", "master", "shadow",
            "123456789", "abc123", "password1", "iloveyou", "princess",
        ];

        if weak_passwords.contains(&password.to_lowercase().as_str()) {
            return Err(ValidationError {
                field: "password".to_string(),
                message: "Password is too common and weak".to_string(),
            });
        }

        Ok(())
    }
}

pub struct RateLimiter {
    requests: Mutex<HashMap<String, Vec<Instant>>>,
    max_requests: usize,
    window_duration: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_duration: Duration) -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
            max_requests,
            window_duration,
        }
    }

    pub fn check_rate_limit(&self, key: &str) -> Result<(), String> {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();
        
        // Clean up old requests
        let cutoff_time = now - self.window_duration;
        
        let user_requests = requests.entry(key.to_string()).or_insert_with(Vec::new);
        user_requests.retain(|&request_time| request_time > cutoff_time);
        
        if user_requests.len() >= self.max_requests {
            warn!("Rate limit exceeded for key: {}", key);
            return Err("Rate limit exceeded. Please try again later.".to_string());
        }
        
        user_requests.push(now);
        Ok(())
    }

    pub fn cleanup_old_entries(&self) {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();
        let cutoff_time = now - self.window_duration;
        
        // Remove entries that are completely expired
        requests.retain(|_key, timestamps| {
            timestamps.retain(|&timestamp| timestamp > cutoff_time);
            !timestamps.is_empty()
        });
    }
}

pub struct InputSanitizer;

impl InputSanitizer {
    pub fn sanitize_string(input: &str) -> String {
        // Remove control characters and trim whitespace
        input
            .chars()
            .filter(|c| !c.is_control() || c.is_whitespace())
            .collect::<String>()
            .trim()
            .to_string()
    }

    pub fn sanitize_username(input: &str) -> String {
        // Remove any characters that aren't alphanumeric, underscore, or hyphen
        input
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect::<String>()
            .to_lowercase()
            .trim()
            .to_string()
    }

    pub fn is_potentially_malicious(input: &str) -> bool {
        let suspicious_patterns = vec![
            "script", "javascript", "vbscript", "onload", "onerror", "onclick",
            "eval", "exec", "system", "shell", "cmd", "powershell", "bash",
            "drop", "delete", "insert", "update", "select", "union", "where",
            "../", "..\\", "/etc/", "c:\\", "windows\\", "system32\\",
            "passwd", "shadow", "hosts", "boot.ini", "autoexec",
        ];

        let input_lower = input.to_lowercase();
        
        for pattern in suspicious_patterns {
            if input_lower.contains(pattern) {
                error!("Potentially malicious input detected: {}", input);
                return true;
            }
        }

        false
    }

    /// Validates file paths for security concerns without flagging legitimate OS paths
    pub fn validate_file_path(path: &str) -> Result<(), String> {
        // Check for directory traversal attempts
        if path.contains("..") || path.contains("~") {
            return Err("Path traversal attempt detected".to_string());
        }

        // Check for null bytes
        if path.contains('\0') {
            return Err("Invalid path: contains null bytes".to_string());
        }

        // Check for extremely long paths (potential buffer overflow)
        if path.len() > 4096 {
            return Err("Path too long".to_string());
        }

        // Allow legitimate Windows paths (C:\, D:\, etc.) and Unix paths
        // but still check for suspicious patterns in the path content
        let path_lower = path.to_lowercase();
        
        // Check for suspicious executable or system file patterns
        let dangerous_patterns = vec![
            "autoexec.bat", "boot.ini", "win.ini", "system.ini",
            "/etc/passwd", "/etc/shadow", ".ssh/id_rsa",
        ];
        
        for pattern in dangerous_patterns {
            if path_lower.contains(pattern) {
                return Err(format!("Access to {} is not allowed", pattern));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_username_validation() {
        // Valid usernames
        assert!(UsernameValidator::validate("validuser").is_ok());
        assert!(UsernameValidator::validate("user_123").is_ok());
        assert!(UsernameValidator::validate("test-user").is_ok());

        // Invalid usernames
        assert!(UsernameValidator::validate("").is_err());
        assert!(UsernameValidator::validate("ab").is_err());
        assert!(UsernameValidator::validate("a".repeat(51).as_str()).is_err());
        assert!(UsernameValidator::validate("user@domain").is_err());
        assert!(UsernameValidator::validate("admin").is_err());
    }

    #[test]
    fn test_password_validation() {
        // Valid passwords
        assert!(PasswordValidator::validate("StrongPass123!").is_ok());
        assert!(PasswordValidator::validate("MySecur3@Pass").is_ok());

        // Invalid passwords
        assert!(PasswordValidator::validate("").is_err());
        assert!(PasswordValidator::validate("short").is_err());
        assert!(PasswordValidator::validate("nouppercase123!").is_err());
        assert!(PasswordValidator::validate("NOLOWERCASE123!").is_err());
        assert!(PasswordValidator::validate("NoDigits!").is_err());
        assert!(PasswordValidator::validate("NoSpecialChars123").is_err());
        assert!(PasswordValidator::validate("password123").is_err());
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(2, Duration::from_secs(1));
        
        // First request should succeed
        assert!(limiter.check_rate_limit("user1").is_ok());
        
        // Second request should succeed
        assert!(limiter.check_rate_limit("user1").is_ok());
        
        // Third request should fail
        assert!(limiter.check_rate_limit("user1").is_err());
        
        // Different user should succeed
        assert!(limiter.check_rate_limit("user2").is_ok());
    }

    #[test]
    fn test_input_sanitizer() {
        assert_eq!(InputSanitizer::sanitize_string("  hello world  "), "hello world");
        assert_eq!(InputSanitizer::sanitize_username("User@123"), "user123");
        
        assert!(InputSanitizer::is_potentially_malicious("javascript:alert('xss')"));
        assert!(InputSanitizer::is_potentially_malicious("DROP TABLE users"));
        assert!(InputSanitizer::is_potentially_malicious("../../../etc/passwd"));
        assert!(!InputSanitizer::is_potentially_malicious("normal user input"));
    }

    #[test]
    fn test_file_path_validation() {
        // Valid paths
        assert!(InputSanitizer::validate_file_path("C:\\Users\\user\\Documents\\file.txt").is_ok());
        assert!(InputSanitizer::validate_file_path("C:/Users/user/Documents/file.txt").is_ok());
        assert!(InputSanitizer::validate_file_path("/home/user/documents/file.txt").is_ok());
        assert!(InputSanitizer::validate_file_path("D:\\Projects\\config.json").is_ok());
        assert!(InputSanitizer::validate_file_path("file.txt").is_ok());
        
        // Invalid paths - directory traversal
        assert!(InputSanitizer::validate_file_path("../../../etc/passwd").is_err());
        assert!(InputSanitizer::validate_file_path("..\\..\\windows\\system32").is_err());
        assert!(InputSanitizer::validate_file_path("~/../../etc/passwd").is_err());
        
        // Invalid paths - dangerous files
        assert!(InputSanitizer::validate_file_path("C:\\Windows\\System32\\boot.ini").is_err());
        assert!(InputSanitizer::validate_file_path("/etc/passwd").is_err());
        assert!(InputSanitizer::validate_file_path("/etc/shadow").is_err());
        
        // Invalid paths - null bytes
        assert!(InputSanitizer::validate_file_path("file\0.txt").is_err());
    }
}