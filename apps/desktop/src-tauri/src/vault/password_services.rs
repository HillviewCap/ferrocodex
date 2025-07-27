use anyhow::Result;
use bcrypt::{hash, DEFAULT_COST};
use rand::thread_rng;
use rand::seq::SliceRandom;

use super::{GeneratePasswordRequest, PasswordStrength, PasswordPolicy};

/// Password generation service with secure random generation
pub struct PasswordGenerator;

impl PasswordGenerator {
    /// Generate a secure password based on the provided policy
    pub fn generate(request: &GeneratePasswordRequest) -> Result<String> {
        let mut charset = String::new();
        
        if request.include_lowercase {
            charset.push_str("abcdefghijklmnopqrstuvwxyz");
        }
        
        if request.include_uppercase {
            charset.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        }
        
        if request.include_numbers {
            charset.push_str("0123456789");
        }
        
        if request.include_special {
            if request.exclude_ambiguous {
                // Exclude ambiguous characters like 0, O, l, 1, I
                charset.push_str("!@#$%^&*()_+-=[]{}|;:,.<>?");
            } else {
                charset.push_str("!@#$%^&*()_+-=[]{}|;:,.<>?~`");
            }
        }
        
        if charset.is_empty() {
            return Err(anyhow::anyhow!("At least one character set must be enabled"));
        }
        
        // Remove ambiguous characters if requested
        if request.exclude_ambiguous {
            charset = charset.chars()
                .filter(|&c| !"0OlI1".contains(c))
                .collect();
        }
        
        let charset_chars: Vec<char> = charset.chars().collect();
        let mut rng = thread_rng();
        
        // Ensure minimum length
        if request.length < 8 {
            return Err(anyhow::anyhow!("Password length must be at least 8 characters"));
        }
        
        // Generate password ensuring at least one character from each required set
        let mut password = Vec::new();
        let mut remaining_length = request.length;
        
        // Add required characters first
        if request.include_lowercase {
            let lowercase_chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz".chars().collect();
            password.push(*lowercase_chars.choose(&mut rng).unwrap());
            remaining_length -= 1;
        }
        
        if request.include_uppercase {
            let uppercase_chars: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().collect();
            password.push(*uppercase_chars.choose(&mut rng).unwrap());
            remaining_length -= 1;
        }
        
        if request.include_numbers {
            let number_chars: Vec<char> = "0123456789".chars().collect();
            password.push(*number_chars.choose(&mut rng).unwrap());
            remaining_length -= 1;
        }
        
        if request.include_special {
            let special_chars: Vec<char> = "!@#$%^&*()_+-=[]{}|;:,.<>?".chars().collect();
            password.push(*special_chars.choose(&mut rng).unwrap());
            remaining_length -= 1;
        }
        
        // Fill remaining length with random characters from full charset
        for _ in 0..remaining_length {
            password.push(*charset_chars.choose(&mut rng).unwrap());
        }
        
        // Shuffle the password to avoid predictable patterns
        password.shuffle(&mut rng);
        
        Ok(password.into_iter().collect())
    }
    
    /// Generate a password hash for storage and reuse checking
    pub fn hash_password(password: &str) -> Result<String> {
        hash(password, DEFAULT_COST).map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))
    }
}

/// Password strength analysis service
pub struct PasswordStrengthAnalyzer;

impl PasswordStrengthAnalyzer {
    /// Analyze password strength and return detailed metrics
    pub fn analyze(password: &str) -> PasswordStrength {
        let length = password.len();
        let mut feedback = Vec::new();
        
        // Character set analysis
        let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
        let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
        let has_numbers = password.chars().any(|c| c.is_ascii_digit());
        let has_special = password.chars().any(|c| !c.is_ascii_alphanumeric());
        
        // Calculate entropy
        let mut charset_size = 0;
        if has_lowercase { charset_size += 26; }
        if has_uppercase { charset_size += 26; }
        if has_numbers { charset_size += 10; }
        if has_special { charset_size += 32; } // Approximate special character count
        
        let entropy = if charset_size > 0 {
            (length as f64) * (charset_size as f64).log2()
        } else {
            0.0
        };
        
        // Calculate score (0-100)
        let mut score: i32 = 0;
        
        // Length scoring
        if length >= 12 { score += 25; }
        else if length >= 8 { score += 15; }
        else if length >= 6 { score += 5; }
        else {
            feedback.push("Password should be at least 8 characters long".to_string());
        }
        
        // Character diversity scoring
        let mut diversity_score = 0;
        if has_lowercase { diversity_score += 5; }
        else { feedback.push("Add lowercase letters".to_string()); }
        
        if has_uppercase { diversity_score += 5; }
        else { feedback.push("Add uppercase letters".to_string()); }
        
        if has_numbers { diversity_score += 5; }
        else { feedback.push("Add numbers".to_string()); }
        
        if has_special { diversity_score += 10; }
        else { feedback.push("Add special characters".to_string()); }
        
        score += diversity_score;
        
        // Entropy bonus
        if entropy >= 60.0 { score += 25; }
        else if entropy >= 40.0 { score += 15; }
        else if entropy >= 20.0 { score += 5; }
        else {
            feedback.push("Password predictability is too high".to_string());
        }
        
        // Pattern detection penalties
        if Self::has_common_patterns(password) {
            score = score.saturating_sub(20);
            feedback.push("Avoid common patterns like '123' or 'abc'".to_string());
        }
        
        if Self::has_keyboard_patterns(password) {
            score = score.saturating_sub(15);
            feedback.push("Avoid keyboard patterns like 'qwerty'".to_string());
        }
        
        // Cap score at 100
        score = score.min(100);
        
        // Add positive feedback for strong passwords
        if score >= 80 {
            feedback.clear();
            feedback.push("Excellent password strength!".to_string());
        } else if score >= 60 {
            feedback.insert(0, "Good password strength".to_string());
        } else if score >= 40 {
            feedback.insert(0, "Moderate password strength".to_string());
        } else {
            feedback.insert(0, "Weak password".to_string());
        }
        
        PasswordStrength {
            score: score as i32,
            entropy,
            has_uppercase,
            has_lowercase,
            has_numbers,
            has_special,
            length,
            feedback,
        }
    }
    
    /// Check if password meets policy requirements
    pub fn meets_policy(password: &str, policy: &PasswordPolicy) -> (bool, Vec<String>) {
        let strength = Self::analyze(password);
        let mut violations = Vec::new();
        
        if (strength.length as i32) < policy.min_length {
            violations.push(format!("Password must be at least {} characters long", policy.min_length));
        }
        
        if policy.require_uppercase && !strength.has_uppercase {
            violations.push("Password must contain uppercase letters".to_string());
        }
        
        if policy.require_lowercase && !strength.has_lowercase {
            violations.push("Password must contain lowercase letters".to_string());
        }
        
        if policy.require_numbers && !strength.has_numbers {
            violations.push("Password must contain numbers".to_string());
        }
        
        if policy.require_special && !strength.has_special {
            violations.push("Password must contain special characters".to_string());
        }
        
        (violations.is_empty(), violations)
    }
    
    fn has_common_patterns(password: &str) -> bool {
        let common_patterns = [
            "123", "234", "345", "456", "567", "678", "789", "890",
            "abc", "bcd", "cde", "def", "efg", "fgh", "ghi", "hij",
            "password", "admin", "user", "test", "demo", "guest",
        ];
        
        let password_lower = password.to_lowercase();
        common_patterns.iter().any(|&pattern| password_lower.contains(pattern))
    }
    
    fn has_keyboard_patterns(password: &str) -> bool {
        let keyboard_patterns = [
            "qwerty", "asdf", "zxcv", "qwertyuiop", "asdfghjkl", "zxcvbnm",
            "1234567890", "!@#$%^&*()", "qwe", "asd", "zxc", "wer", "sdf",
        ];
        
        let password_lower = password.to_lowercase();
        keyboard_patterns.iter().any(|&pattern| password_lower.contains(pattern))
    }
}

/// Password reuse checking service
pub struct PasswordReuseChecker;

impl PasswordReuseChecker {
    /// Check if a password has been used before
    pub fn is_password_reused(password_hash: &str, existing_hashes: &[String]) -> bool {
        existing_hashes.contains(&password_hash.to_string())
    }
    
    /// Verify password against hash (for checking existing passwords)
    pub fn verify_password(password: &str, hash: &str) -> bool {
        bcrypt::verify(password, hash).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_generation() {
        let request = GeneratePasswordRequest {
            length: 16,
            include_uppercase: true,
            include_lowercase: true,
            include_numbers: true,
            include_special: true,
            exclude_ambiguous: true,
        };

        let password = PasswordGenerator::generate(&request).unwrap();
        assert_eq!(password.len(), 16);
        
        // Verify it contains required character types
        assert!(password.chars().any(|c| c.is_ascii_lowercase()));
        assert!(password.chars().any(|c| c.is_ascii_uppercase()));
        assert!(password.chars().any(|c| c.is_ascii_digit()));
        assert!(password.chars().any(|c| !c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_password_strength_analysis() {
        let weak_password = "123";
        let strong_password = "Str0ng!P@ssw0rd#2024";

        let weak_strength = PasswordStrengthAnalyzer::analyze(weak_password);
        let strong_strength = PasswordStrengthAnalyzer::analyze(strong_password);

        assert!(weak_strength.score < 40);
        assert!(strong_strength.score > 80);
        assert!(strong_strength.entropy > weak_strength.entropy);
    }

    #[test]
    fn test_password_policy_compliance() {
        let policy = PasswordPolicy {
            id: 1,
            min_length: 12,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_special: true,
            max_age_days: None,
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
        };

        let compliant_password = "StrongP@ssw0rd123";
        let non_compliant_password = "weak";

        let (compliant, _) = PasswordStrengthAnalyzer::meets_policy(compliant_password, &policy);
        let (non_compliant, violations) = PasswordStrengthAnalyzer::meets_policy(non_compliant_password, &policy);

        assert!(compliant);
        assert!(!non_compliant);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_password_hashing() {
        let password = "TestPassword123!";
        let hash = PasswordGenerator::hash_password(password).unwrap();
        
        assert_ne!(hash, password);
        assert!(PasswordReuseChecker::verify_password(password, &hash));
        assert!(!PasswordReuseChecker::verify_password("WrongPassword", &hash));
    }
}