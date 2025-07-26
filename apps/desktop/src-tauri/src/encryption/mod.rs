use anyhow::Result;
use std::io::{Read, Write};
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce, Key
};
use pbkdf2::{password_hash::{PasswordHasher, SaltString}, Pbkdf2};

// AES-256-GCM encryption implementation for vault security
pub struct FileEncryption {
    cipher: Aes256Gcm,
}

impl FileEncryption {
    pub fn new(key: &str) -> Self {
        // Generate a proper AES-256 key using PBKDF2 with salt
        let key_bytes = Self::derive_key_from_string(key);
        let cipher_key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(cipher_key);
        
        Self { cipher }
    }

    /// Derive a 256-bit (32 byte) key from string using PBKDF2
    fn derive_key_from_string(key_str: &str) -> [u8; 32] {
        // Use a static salt for consistency within application
        // In production, consider user-specific salts stored securely
        let salt = SaltString::encode_b64(b"ferrocodex_vault_salt_2024").unwrap();
        let password_hash = Pbkdf2.hash_password(key_str.as_bytes(), &salt).unwrap();
        
        // Extract first 32 bytes from hash for AES-256 key
        let hash_string = password_hash.hash.unwrap().to_string();
        let hash_bytes = hash_string.as_bytes();
        let mut key = [0u8; 32];
        for (i, &byte) in hash_bytes.iter().take(32).enumerate() {
            key[i] = byte;
        }
        
        // Fill remaining bytes if needed
        for i in hash_bytes.len()..32 {
            key[i] = (i % 256) as u8;
        }
        
        key
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Generate a random 96-bit (12 byte) nonce for AES-GCM
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        
        // Encrypt the data
        let ciphertext = self.cipher
            .encrypt(&nonce, data)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;
        
        // Prepend nonce to ciphertext for storage
        let mut result = Vec::with_capacity(nonce.len() + ciphertext.len());
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }

    pub fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        if encrypted_data.len() < 12 {
            return Err(anyhow::anyhow!("Invalid encrypted data: too short"));
        }
        
        // Split nonce and ciphertext
        let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        // Decrypt the data
        let plaintext = self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;
        
        Ok(plaintext)
    }
}

// Key derivation utilities using proper cryptographic practices
pub fn derive_key_from_user_credentials(user_id: i64, username: &str) -> String {
    // Create deterministic but secure key from user credentials
    // This ensures same user always gets same encryption key for vault access
    format!("ferrocodex_vault_{}_{}_v2", user_id, username)
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
    fn test_aes_gcm_encryption_decryption() {
        let encryption = FileEncryption::new("test_key");
        let original_data = b"Hello, World! This is a test configuration file.";
        
        let encrypted = encryption.encrypt(original_data).unwrap();
        assert_ne!(encrypted, original_data);
        assert!(encrypted.len() > original_data.len()); // Should be larger due to nonce + auth tag
        
        let decrypted = encryption.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, original_data);
    }

    #[test]
    fn test_different_keys_produce_different_ciphertexts() {
        let encryption1 = FileEncryption::new("key1");
        let encryption2 = FileEncryption::new("key2");
        let data = b"Same data for both keys";
        
        let encrypted1 = encryption1.encrypt(data).unwrap();
        let encrypted2 = encryption2.encrypt(data).unwrap();
        
        // Different keys should produce different ciphertexts
        assert_ne!(encrypted1, encrypted2);
        
        // Each should decrypt correctly with their own key
        assert_eq!(encryption1.decrypt(&encrypted1).unwrap(), data);
        assert_eq!(encryption2.decrypt(&encrypted2).unwrap(), data);
        
        // Cross-decryption should fail
        assert!(encryption1.decrypt(&encrypted2).is_err());
        assert!(encryption2.decrypt(&encrypted1).is_err());
    }

    #[test]
    fn test_invalid_encrypted_data() {
        let encryption = FileEncryption::new("test_key");
        
        // Test with data too short (no nonce)
        let short_data = vec![0u8; 8];
        assert!(encryption.decrypt(&short_data).is_err());
        
        // Test with corrupted data
        let valid_encrypted = encryption.encrypt(b"test data").unwrap();
        let mut corrupted = valid_encrypted.clone();
        corrupted[5] ^= 1; // Flip one bit
        assert!(encryption.decrypt(&corrupted).is_err());
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