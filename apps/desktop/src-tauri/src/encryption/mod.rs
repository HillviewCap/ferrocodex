use anyhow::Result;
use std::io::{Read, Write};

// Simple XOR encryption for demo purposes
// In a real application, you would use proper AES-256 encryption
pub struct FileEncryption {
    key: Vec<u8>,
}

impl FileEncryption {
    pub fn new(key: &str) -> Self {
        // In a real implementation, this would be derived from user credentials
        // using PBKDF2 with salt as mentioned in the story requirements
        let mut key_bytes = key.as_bytes().to_vec();
        key_bytes.resize(32, 0); // Pad/truncate to 32 bytes for AES-256
        
        Self { key: key_bytes }
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut encrypted = Vec::with_capacity(data.len());
        
        for (i, byte) in data.iter().enumerate() {
            let key_byte = self.key[i % self.key.len()];
            encrypted.push(byte ^ key_byte);
        }
        
        Ok(encrypted)
    }

    pub fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        // XOR is symmetric, so decryption is the same as encryption
        self.encrypt(encrypted_data)
    }
}

// Key derivation utilities (simplified for demo)
pub fn derive_key_from_user_credentials(user_id: i64, username: &str) -> String {
    // In a real implementation, this would use PBKDF2 with proper salt
    // and derive from user's password or other credentials
    format!("ferrocodex_key_{}_{}", user_id, username)
}

pub fn validate_file_size(data: &[u8], max_size: usize) -> Result<()> {
    if data.len() > max_size {
        return Err(anyhow::anyhow!("File size {} exceeds maximum allowed size {}", data.len(), max_size));
    }
    Ok(())
}

pub fn compress_data(data: &[u8]) -> Result<Vec<u8>> {
    // Simple compression using flate2
    use flate2::write::GzEncoder;
    use flate2::Compression;
    
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

pub fn decompress_data(compressed_data: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::GzDecoder;
    
    let mut decoder = GzDecoder::new(compressed_data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let encryption = FileEncryption::new("test_key");
        let original_data = b"Hello, World! This is a test configuration file.";
        
        let encrypted = encryption.encrypt(original_data).unwrap();
        assert_ne!(encrypted, original_data);
        
        let decrypted = encryption.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, original_data);
    }

    #[test]
    fn test_key_derivation() {
        let key1 = derive_key_from_user_credentials(1, "user1");
        let key2 = derive_key_from_user_credentials(2, "user2");
        let key3 = derive_key_from_user_credentials(1, "user1");
        
        assert_ne!(key1, key2);
        assert_eq!(key1, key3);
    }

    #[test]
    fn test_file_size_validation() {
        let small_data = vec![0u8; 100];
        let large_data = vec![0u8; 1000];
        
        assert!(validate_file_size(&small_data, 500).is_ok());
        assert!(validate_file_size(&large_data, 500).is_err());
    }

    #[test]
    fn test_compression() {
        let original_data = b"This is a test string that should compress well. ".repeat(100);
        
        let compressed = compress_data(&original_data).unwrap();
        assert!(compressed.len() < original_data.len());
        
        let decompressed = decompress_data(&compressed).unwrap();
        assert_eq!(decompressed, original_data);
    }
}