use super::SecurityError;
use std::collections::HashSet;
use std::sync::OnceLock;
use tracing::{warn, debug};

/// Windows reserved name checker
/// Prevents use of Windows reserved filenames and device names
/// Includes standard reserved names and industrial/OT specific additions
pub struct WindowsReservedNameChecker {
    reserved_names: &'static HashSet<String>,
    reserved_prefixes: &'static Vec<String>,
}

impl WindowsReservedNameChecker {
    pub fn new() -> Self {
        Self {
            reserved_names: Self::get_reserved_names(),
            reserved_prefixes: Self::get_reserved_prefixes(),
        }
    }

    /// Get the compiled list of reserved names
    fn get_reserved_names() -> &'static HashSet<String> {
        static RESERVED_NAMES: OnceLock<HashSet<String>> = OnceLock::new();
        RESERVED_NAMES.get_or_init(|| {
            let mut names = HashSet::new();
            
            // Standard Windows reserved names
            let standard_reserved = vec![
                // DOS device names
                "CON", "PRN", "AUX", "NUL",
                // Serial ports
                "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
                // Parallel ports
                "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
                // Additional Windows reserved names
                "CLOCK$", "CONFIG$", "KEYBD$", "SCREEN$", "$IDLE$",
                // System files that should not be used as asset names
                "BOOT", "SYSTEM", "WINDOWS", "WINNT", "PROGRAM", "PROGRAMS",
                "AUTOEXEC", "CONFIG", "MSDOS", "IO", "COMMAND",
            ];

            // Industrial/OT specific reserved names that could cause confusion
            let industrial_reserved = vec![
                // Common industrial protocols and systems
                "MODBUS", "PROFIBUS", "PROFINET", "ETHERNET", "FIELDBUS", "CANBUS",
                "DEVICENET", "CONTROLNET", "FOUNDATION", "HART", "WIRELESS", "WLAN",
                // Common automation terms that should be avoided for clarity
                "PLC", "HMI", "SCADA", "DCS", "PAC", "RTU", "IED", "MCC", "VFD",
                "MOTOR", "DRIVE", "PUMP", "VALVE", "SENSOR", "ACTUATOR", "RELAY",
                "SWITCH", "ROUTER", "GATEWAY", "FIREWALL", "SERVER", "CLIENT",
                // Safety and security terms
                "SAFETY", "SECURITY", "EMERGENCY", "ALARM", "WARNING", "CRITICAL",
                "FAULT", "ERROR", "FAIL", "STOP", "START", "RESET", "TEST",
                // Database and system terms
                "DATABASE", "TABLE", "INDEX", "COLUMN", "ROW", "RECORD", "FIELD",
                "ADMIN", "ADMINISTRATOR", "ROOT", "SYSTEM", "USER", "GUEST",
                "SERVICE", "DAEMON", "PROCESS", "THREAD", "TASK", "JOB",
                // Network and communication terms
                "NETWORK", "INTERNET", "INTRANET", "WEB", "HTTP", "HTTPS", "FTP",
                "SMTP", "POP", "IMAP", "DNS", "DHCP", "TCP", "UDP", "IP",
                // File system terms
                "FILE", "FOLDER", "DIRECTORY", "PATH", "DRIVE", "DISK", "VOLUME",
                "PARTITION", "BACKUP", "RESTORE", "ARCHIVE", "LOG", "TEMP",
                // Application terms
                "APPLICATION", "PROGRAM", "SOFTWARE", "FIRMWARE", "DRIVER",
                "LIBRARY", "MODULE", "COMPONENT", "INTERFACE", "API", "SDK",
            ];

            // Add all reserved names in uppercase
            for name in standard_reserved.iter().chain(industrial_reserved.iter()) {
                names.insert(name.to_uppercase());
                // Also add with common extensions
                names.insert(format!("{}.TXT", name.to_uppercase()));
                names.insert(format!("{}.DAT", name.to_uppercase()));
                names.insert(format!("{}.CFG", name.to_uppercase()));
                names.insert(format!("{}.INI", name.to_uppercase()));
                names.insert(format!("{}.LOG", name.to_uppercase()));
            }

            names
        })
    }

    /// Get reserved prefixes (COM, LPT with numbers)
    fn get_reserved_prefixes() -> &'static Vec<String> {
        static RESERVED_PREFIXES: OnceLock<Vec<String>> = OnceLock::new();
        RESERVED_PREFIXES.get_or_init(|| {
            vec![
                "COM".to_string(),
                "LPT".to_string(),
                "TEMP".to_string(),
                "TMP".to_string(),
            ]
        })
    }

    /// Check if a name is reserved
    pub fn check_name(&self, name: &str) -> Result<(), SecurityError> {
        let name_upper = name.trim().to_uppercase();
        
        debug!("Checking reserved name: {}", name_upper);

        // Check if name is empty after trim
        if name_upper.is_empty() {
            return Err(SecurityError::ReservedName {
                name: name.to_string(),
                reserved_type: "Empty name".to_string(),
            });
        }

        // Check exact matches
        if self.reserved_names.contains(&name_upper) {
            warn!("Reserved name detected: {}", name_upper);
            return Err(SecurityError::ReservedName {
                name: name.to_string(),
                reserved_type: "Windows reserved name".to_string(),
            });
        }

        // Check name without extension
        let name_without_ext = if let Some(dot_pos) = name_upper.rfind('.') {
            &name_upper[..dot_pos]
        } else {
            &name_upper
        };

        if self.reserved_names.contains(name_without_ext) {
            warn!("Reserved name with extension detected: {}", name_upper);
            return Err(SecurityError::ReservedName {
                name: name.to_string(),
                reserved_type: "Windows reserved name with extension".to_string(),
            });
        }

        // Check reserved prefixes with numbers
        for prefix in self.reserved_prefixes.iter() {
            if name_without_ext.starts_with(prefix) {
                let suffix = &name_without_ext[prefix.len()..];
                
                // Check if suffix is a number (for COM1-COM9, LPT1-LPT9, etc.)
                if suffix.chars().all(|c| c.is_ascii_digit()) && !suffix.is_empty() {
                    warn!("Reserved prefix with number detected: {}", name_upper);
                    return Err(SecurityError::ReservedName {
                        name: name.to_string(),
                        reserved_type: format!("{} device name", prefix),
                    });
                }
                
                // Check for industrial naming conflicts
                if prefix == "TEMP" || prefix == "TMP" {
                    if suffix.is_empty() || suffix.chars().all(|c| c.is_ascii_digit() || c == '_' || c == '-') {
                        warn!("Temporary name pattern detected: {}", name_upper);
                        return Err(SecurityError::ReservedName {
                            name: name.to_string(),
                            reserved_type: "Temporary file pattern".to_string(),
                        });
                    }
                }
            }
        }

        // Check for names that might cause confusion in industrial environments
        if self.is_confusing_industrial_name(&name_upper) {
            warn!("Potentially confusing industrial name: {}", name_upper);
            return Err(SecurityError::ReservedName {
                name: name.to_string(),
                reserved_type: "Potentially confusing in industrial context".to_string(),
            });
        }

        // Check for version-like patterns that might be confused with system versions
        if self.is_version_like_pattern(&name_upper) {
            warn!("Version-like pattern detected: {}", name_upper);
            return Err(SecurityError::ReservedName {
                name: name.to_string(),
                reserved_type: "Version-like pattern that may cause confusion".to_string(),
            });
        }

        Ok(())
    }

    /// Check if name might be confusing in industrial context
    fn is_confusing_industrial_name(&self, name: &str) -> bool {
        // Patterns that might be confused with system components
        let confusing_patterns = vec![
            "MAIN", "PRIMARY", "SECONDARY", "BACKUP", "REDUNDANT",
            "MASTER", "SLAVE", "CLIENT", "SERVER", "HOST",
            "LOCAL", "REMOTE", "GLOBAL", "PUBLIC", "PRIVATE",
            "INPUT", "OUTPUT", "ANALOG", "DIGITAL", "DISCRETE",
            "STATUS", "STATE", "MODE", "LEVEL", "LIMIT",
            "HIGH", "LOW", "MIN", "MAX", "AVG", "AVERAGE",
            "TOTAL", "SUM", "COUNT", "VALUE", "DATA",
            "SIGNAL", "CHANNEL", "PORT", "INTERFACE", "CONNECTION",
        ];

        // Check if the name is exactly one of these confusing terms
        confusing_patterns.iter().any(|&pattern| name == pattern)
    }

    /// Check for version-like patterns
    fn is_version_like_pattern(&self, name: &str) -> bool {
        // Patterns like V1, V2.0, VERSION1, etc.
        if name.starts_with('V') && name.len() > 1 {
            let suffix = &name[1..];
            if suffix.chars().all(|c| c.is_ascii_digit() || c == '.') {
                return true;
            }
        }

        // Patterns like VERSION1, VER1, etc.
        if name.starts_with("VERSION") || name.starts_with("VER") {
            return true;
        }

        // Patterns that are purely numeric (might be confused with IDs)
        if name.chars().all(|c| c.is_ascii_digit()) && name.len() < 10 {
            return true;
        }

        false
    }

    /// Get list of all reserved names for reference
    pub fn get_all_reserved_names(&self) -> Vec<String> {
        self.reserved_names.iter().cloned().collect()
    }

    /// Check if a name is safe to use (opposite of check_name)
    pub fn is_name_safe(&self, name: &str) -> bool {
        self.check_name(name).is_ok()
    }

    /// Suggest alternative names for reserved names
    pub fn suggest_alternatives(&self, name: &str) -> Vec<String> {
        let base_name = name.trim().to_uppercase();
        let mut suggestions = Vec::new();

        // Add prefix/suffix alternatives
        suggestions.push(format!("ASSET_{}", base_name));
        suggestions.push(format!("DEVICE_{}", base_name));
        suggestions.push(format!("EQUIP_{}", base_name));
        suggestions.push(format!("{}_UNIT", base_name));
        suggestions.push(format!("{}_001", base_name));

        // For specific reserved names, provide context-appropriate alternatives
        match base_name.as_str() {
            "CON" => suggestions.extend(vec!["CONTROL_01".to_string(), "CONSOLE_01".to_string()]),
            "PRN" => suggestions.extend(vec!["PRINTER_01".to_string(), "PRINT_DEVICE".to_string()]),
            "AUX" => suggestions.extend(vec!["AUXILIARY_01".to_string(), "AUX_DEVICE".to_string()]),
            "NUL" => suggestions.extend(vec!["NULL_DEVICE".to_string(), "VOID_01".to_string()]),
            "PLC" => suggestions.extend(vec!["PLC_01".to_string(), "CONTROLLER_01".to_string()]),
            "HMI" => suggestions.extend(vec!["HMI_01".to_string(), "INTERFACE_01".to_string()]),
            _ => {
                // Generic alternatives for other reserved names
                if base_name.len() < 47 {
                    suggestions.push(format!("{}_SYS", base_name));
                    suggestions.push(format!("{}_DEV", base_name));
                }
            }
        }

        // Filter out any suggestions that might also be reserved
        suggestions.retain(|suggestion| self.is_name_safe(suggestion));
        
        // Limit to reasonable number of suggestions
        suggestions.into_iter().take(8).collect()
    }
}

impl Default for WindowsReservedNameChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_reserved_names() {
        let checker = WindowsReservedNameChecker::new();

        // Test standard Windows reserved names
        let reserved_names = vec![
            "CON", "PRN", "AUX", "NUL",
            "COM1", "COM2", "COM9",
            "LPT1", "LPT2", "LPT9",
        ];

        for name in reserved_names {
            assert!(checker.check_name(name).is_err(), "Name '{}' should be reserved", name);
        }
    }

    #[test]
    fn test_reserved_names_with_extensions() {
        let checker = WindowsReservedNameChecker::new();

        // Test reserved names with extensions
        let reserved_with_ext = vec![
            "CON.txt", "PRN.dat", "AUX.cfg", "COM1.log",
        ];

        for name in reserved_with_ext {
            assert!(checker.check_name(name).is_err(), "Name '{}' should be reserved", name);
        }
    }

    #[test]
    fn test_case_insensitive_checking() {
        let checker = WindowsReservedNameChecker::new();

        // Test case insensitive checking
        let case_variations = vec![
            "con", "Con", "CON", "cOn",
            "prn", "Prn", "PRN", "pRn",
        ];

        for name in case_variations {
            assert!(checker.check_name(name).is_err(), "Name '{}' should be reserved", name);
        }
    }

    #[test]
    fn test_industrial_reserved_names() {
        let checker = WindowsReservedNameChecker::new();

        // Test industrial reserved names
        let industrial_reserved = vec![
            "MODBUS", "PROFIBUS", "SCADA", "PLC", "HMI", "DCS",
        ];

        for name in industrial_reserved {
            assert!(checker.check_name(name).is_err(), "Name '{}' should be reserved", name);
        }
    }

    #[test]
    fn test_safe_names() {
        let checker = WindowsReservedNameChecker::new();

        // Test names that should be safe
        let safe_names = vec![
            "MOTOR_001", "PUMP_A1", "SENSOR_TEMP", "VALVE_CTRL",
            "DEVICE_123", "EQUIP_ABC", "ASSET_XYZ", "UNIT_001",
        ];

        for name in safe_names {
            assert!(checker.check_name(name).is_ok(), "Name '{}' should be safe", name);
        }
    }

    #[test]
    fn test_confusing_industrial_names() {
        let checker = WindowsReservedNameChecker::new();

        // Test potentially confusing names
        let confusing_names = vec![
            "MAIN", "PRIMARY", "MASTER", "INPUT", "OUTPUT",
        ];

        for name in confusing_names {
            assert!(checker.check_name(name).is_err(), "Name '{}' should be confusing", name);
        }
    }

    #[test]
    fn test_version_like_patterns() {
        let checker = WindowsReservedNameChecker::new();

        // Test version-like patterns
        let version_patterns = vec![
            "V1", "V2.0", "VERSION1", "VER1", "123",
        ];

        for name in version_patterns {
            assert!(checker.check_name(name).is_err(), "Name '{}' should be version-like", name);
        }
    }

    #[test]
    fn test_suggestion_generation() {
        let checker = WindowsReservedNameChecker::new();

        let suggestions = checker.suggest_alternatives("CON");
        assert!(!suggestions.is_empty());
        assert!(suggestions.contains(&"CONTROL_01".to_string()));

        let suggestions = checker.suggest_alternatives("PLC");
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains("CONTROLLER")));
    }

    #[test]
    fn test_empty_and_whitespace_names() {
        let checker = WindowsReservedNameChecker::new();

        assert!(checker.check_name("").is_err());
        assert!(checker.check_name("   ").is_err());
        assert!(checker.check_name("\t\n").is_err());
    }

    #[test]
    fn test_prefix_patterns() {
        let checker = WindowsReservedNameChecker::new();

        // Test COM/LPT with numbers
        assert!(checker.check_name("COM10").is_err());
        assert!(checker.check_name("LPT10").is_err());
        
        // Test TEMP patterns
        assert!(checker.check_name("TEMP123").is_err());
        assert!(checker.check_name("TMP_001").is_err());
    }

    #[test]
    fn test_get_all_reserved_names() {
        let checker = WindowsReservedNameChecker::new();
        let all_reserved = checker.get_all_reserved_names();
        
        assert!(!all_reserved.is_empty());
        assert!(all_reserved.contains(&"CON".to_string()));
        assert!(all_reserved.contains(&"MODBUS".to_string()));
    }
}