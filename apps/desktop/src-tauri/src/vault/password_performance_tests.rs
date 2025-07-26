#[cfg(test)]
mod performance_tests {
    use crate::vault::password_services::*;
    use crate::vault::{GeneratePasswordRequest, PasswordPolicy};
    use std::time::{Duration, Instant};

    #[test]
    fn test_password_generation_performance() {
        let request = GeneratePasswordRequest {
            length: 16,
            include_uppercase: true,
            include_lowercase: true,
            include_numbers: true,
            include_special: true,
            exclude_ambiguous: true,
        };

        // Test that password generation completes within 1 second
        let start = Instant::now();
        let password = PasswordGenerator::generate(&request).unwrap();
        let duration = start.elapsed();
        
        assert!(duration < Duration::from_secs(1), "Password generation took {:?}, expected < 1s", duration);
        assert_eq!(password.len(), 16);
        println!("Password generation time: {:?}", duration);
    }

    #[test]
    fn test_password_strength_analysis_performance() {
        let password = "Complex!P@ssw0rd123$%^";

        // Test that strength analysis completes within 0.5 seconds
        let start = Instant::now();
        let strength = PasswordStrengthAnalyzer::analyze(password);
        let duration = start.elapsed();
        
        assert!(duration < Duration::from_millis(500), "Password strength analysis took {:?}, expected < 0.5s", duration);
        assert!(strength.score > 0);
        println!("Password strength analysis time: {:?}", duration);
    }

    #[test]
    fn test_password_hashing_performance() {
        let password = "TestPassword123!";

        // Test that password hashing completes within reasonable time
        let start = Instant::now();
        let hash = PasswordGenerator::hash_password(password).unwrap();
        let duration = start.elapsed();
        
        assert!(duration < Duration::from_secs(2), "Password hashing took {:?}, expected < 2s", duration);
        assert!(!hash.is_empty());
        println!("Password hashing time: {:?}", duration);
    }

    #[test]
    fn test_batch_password_generation_performance() {
        let request = GeneratePasswordRequest {
            length: 16,
            include_uppercase: true,
            include_lowercase: true,
            include_numbers: true,
            include_special: true,
            exclude_ambiguous: true,
        };

        // Generate 10 passwords and ensure average time is reasonable
        let start = Instant::now();
        let passwords: Vec<String> = (0..10)
            .map(|_| PasswordGenerator::generate(&request).unwrap())
            .collect();
        let duration = start.elapsed();
        
        let avg_duration = duration / 10;
        assert!(avg_duration < Duration::from_millis(100), "Average password generation took {:?}, expected < 100ms", avg_duration);
        assert_eq!(passwords.len(), 10);
        
        // Ensure all passwords are unique
        let unique_count = passwords.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(unique_count, 10, "All generated passwords should be unique");
        
        println!("Batch password generation (10 passwords): {:?}, avg: {:?}", duration, avg_duration);
    }

    #[test]
    fn test_password_policy_compliance_performance() {
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

        let passwords = vec![
            "WeakPassword",
            "StrongP@ssw0rd123",
            "AnotherStr0ng!Pass",
            "short",
            "VeryLongAndComplexP@ssw0rd123!@#",
        ];

        // Test that policy checking for multiple passwords completes quickly
        let start = Instant::now();
        let results: Vec<(bool, Vec<String>)> = passwords
            .iter()
            .map(|&password| PasswordStrengthAnalyzer::meets_policy(password, &policy))
            .collect();
        let duration = start.elapsed();
        
        let avg_duration = duration / passwords.len() as u32;
        assert!(avg_duration < Duration::from_millis(50), "Average policy check took {:?}, expected < 50ms", avg_duration);
        assert_eq!(results.len(), passwords.len());
        
        println!("Password policy compliance check (5 passwords): {:?}, avg: {:?}", duration, avg_duration);
    }
}