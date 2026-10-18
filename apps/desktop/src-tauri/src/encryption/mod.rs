use anyhow::Result;
use std::fmt;
use std::io::{Read, Write};
use aes_gcm::{
    aead::{rand_core::RngCore, Aead, AeadCore, KeyInit, OsRng, Payload},
    Aes256Gcm, Nonce, Key
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const SALT_LEN: usize = 16;
const RECOVERY_CODE_BYTES: usize = 20;
const KEY_WRAP_AAD: &[u8] = b"ferrocodex-data-key-wrap-v1";

/// A random 256-bit AES key. The bytes are zeroed on drop and never printed.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct DataKey([u8; KEY_LEN]);

impl DataKey {
    pub fn generate() -> Self {
        let mut bytes = [0u8; KEY_LEN];
        OsRng.fill_bytes(&mut bytes);
        Self(bytes)
    }

    fn from_slice(bytes: &[u8]) -> Result<Self> {
        let bytes: [u8; KEY_LEN] = bytes.try_into()
            .map_err(|_| anyhow::anyhow!("Invalid key length"))?;
        Ok(Self(bytes))
    }
}

impl fmt::Debug for DataKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DataKey(<redacted>)")
    }
}

// AES-256-GCM encryption for configuration files, firmware, and vault secrets
pub struct FileEncryption {
    cipher: Aes256Gcm,
}

impl FileEncryption {
    pub fn new(key: &DataKey) -> Self {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key.0));
        Self { cipher }
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        self.encrypt_with_aad(data, b"")
    }

    pub fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        self.decrypt_with_aad(encrypted_data, b"")
    }

    /// Encrypts `data`, authenticating `aad` along with it. The output is the
    /// random 96-bit nonce followed by the ciphertext and tag.
    pub fn encrypt_with_aad(&self, data: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let ciphertext = self.cipher
            .encrypt(&nonce, Payload { msg: data, aad })
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        let mut result = Vec::with_capacity(nonce.len() + ciphertext.len());
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    pub fn decrypt_with_aad(&self, encrypted_data: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
        if encrypted_data.len() < NONCE_LEN {
            return Err(anyhow::anyhow!("Invalid encrypted data: too short"));
        }

        let (nonce_bytes, ciphertext) = encrypted_data.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);

        self.cipher
            .decrypt(nonce, Payload { msg: ciphertext, aad })
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))
    }

    /// Encrypts `data` and base64-encodes it for the TEXT columns that hold
    /// vault secrets and standalone credentials.
    pub fn encrypt_to_base64(&self, data: &[u8]) -> Result<String> {
        Ok(general_purpose::STANDARD.encode(self.encrypt(data)?))
    }

    pub fn decrypt_from_base64(&self, encoded: &str) -> Result<Vec<u8>> {
        let encrypted = general_purpose::STANDARD.decode(encoded)
            .map_err(|e| anyhow::anyhow!("Invalid encrypted value: {}", e))?;
        self.decrypt(&encrypted)
    }
}

/// Argon2id cost parameters. They are stored with each salt so the defaults
/// can be raised later without breaking existing wrapped keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KdfParams {
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl Default for KdfParams {
    // RFC 9106's second recommended option: 64 MiB, 3 passes, 4 lanes
    fn default() -> Self {
        Self { memory_kib: 64 * 1024, iterations: 3, parallelism: 4 }
    }
}

impl KdfParams {
    #[cfg(test)]
    pub fn fast_for_tests() -> Self {
        Self { memory_kib: 8, iterations: 1, parallelism: 1 }
    }
}

/// One data key shared by every test, so data stored by one call can be read by the next.
#[cfg(test)]
pub fn test_key() -> &'static DataKey {
    static KEY: once_cell::sync::Lazy<DataKey> = once_cell::sync::Lazy::new(DataKey::generate);
    &KEY
}

pub fn generate_salt() -> Vec<u8> {
    let mut salt = vec![0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    salt
}

/// Derives a 256-bit key from a password or passphrase with Argon2id.
pub fn derive_key(secret: &[u8], salt: &[u8], params: &KdfParams) -> Result<DataKey> {
    let argon_params = Params::new(params.memory_kib, params.iterations, params.parallelism, Some(KEY_LEN))
        .map_err(|e| anyhow::anyhow!("Invalid key derivation parameters: {}", e))?;
    let mut key = DataKey([0u8; KEY_LEN]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params)
        .hash_password_into(secret, salt, &mut key.0)
        .map_err(|e| anyhow::anyhow!("Key derivation failed: {}", e))?;
    Ok(key)
}

/// A data key encrypted under a key derived from a secret: a user's login
/// password or the recovery code.
#[derive(Debug, Clone)]
pub struct WrappedKey {
    pub salt: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub params: KdfParams,
}

impl WrappedKey {
    pub fn seal(key: &DataKey, secret: &str, params: KdfParams) -> Result<Self> {
        let salt = generate_salt();
        let wrapping_key = derive_key(secret.as_bytes(), &salt, &params)?;
        let ciphertext = FileEncryption::new(&wrapping_key).encrypt_with_aad(&key.0, KEY_WRAP_AAD)?;
        Ok(Self { salt, ciphertext, params })
    }

    pub fn open(&self, secret: &str) -> Result<DataKey> {
        let wrapping_key = derive_key(secret.as_bytes(), &self.salt, &self.params)?;
        let mut bytes = FileEncryption::new(&wrapping_key)
            .decrypt_with_aad(&self.ciphertext, KEY_WRAP_AAD)
            .map_err(|_| anyhow::anyhow!("Incorrect secret or corrupted key"))?;
        let key = DataKey::from_slice(&bytes);
        bytes.zeroize();
        key
    }
}

/// Generates a recovery code: 160 random bits as 40 hex digits in groups of four.
pub fn generate_recovery_code() -> String {
    let mut bytes = [0u8; RECOVERY_CODE_BYTES];
    OsRng.fill_bytes(&mut bytes);
    let hex: String = bytes.iter().map(|b| format!("{:02X}", b)).collect();
    bytes.zeroize();
    hex.as_bytes()
        .chunks(4)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join("-")
}

/// Strips the separators and whitespace a user may type and uppercases the
/// rest, so a recovery code matches however it was copied.
pub fn normalize_recovery_code(input: &str) -> String {
    input.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
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
        let encryption = FileEncryption::new(&DataKey::generate());
        let original_data = b"Hello, World! This is a test configuration file.";

        let encrypted = encryption.encrypt(original_data).unwrap();
        assert_ne!(encrypted, original_data);
        assert!(encrypted.len() > original_data.len()); // Should be larger due to nonce + auth tag

        let decrypted = encryption.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, original_data);
    }

    #[test]
    fn test_different_keys_produce_different_ciphertexts() {
        let encryption1 = FileEncryption::new(&DataKey::generate());
        let encryption2 = FileEncryption::new(&DataKey::generate());
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
        let encryption = FileEncryption::new(&DataKey::generate());

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
    fn test_associated_data_must_match() {
        let encryption = FileEncryption::new(&DataKey::generate());
        let encrypted = encryption.encrypt_with_aad(b"secret", b"context-a").unwrap();

        assert_eq!(encryption.decrypt_with_aad(&encrypted, b"context-a").unwrap(), b"secret");
        assert!(encryption.decrypt_with_aad(&encrypted, b"context-b").is_err());
        assert!(encryption.decrypt(&encrypted).is_err());
    }

    #[test]
    fn test_base64_round_trip() {
        let encryption = FileEncryption::new(&DataKey::generate());
        let encoded = encryption.encrypt_to_base64(b"plc-admin-password").unwrap();

        assert_eq!(encryption.decrypt_from_base64(&encoded).unwrap(), b"plc-admin-password");
        assert!(encryption.decrypt_from_base64("not base64!").is_err());
    }

    #[test]
    fn test_generated_keys_are_unique() {
        assert_ne!(DataKey::generate().0, DataKey::generate().0);
    }

    #[test]
    fn test_debug_does_not_print_key_bytes() {
        assert_eq!(format!("{:?}", DataKey::generate()), "DataKey(<redacted>)");
    }

    #[test]
    fn test_derive_key_depends_on_secret_and_salt() {
        let params = KdfParams::fast_for_tests();
        let salt = generate_salt();

        let key = derive_key(b"password", &salt, &params).unwrap();
        assert_eq!(key.0, derive_key(b"password", &salt, &params).unwrap().0);
        assert_ne!(key.0, derive_key(b"Password", &salt, &params).unwrap().0);
        assert_ne!(key.0, derive_key(b"password", &generate_salt(), &params).unwrap().0);
    }

    #[test]
    fn test_wrapped_key_round_trip() {
        let key = DataKey::generate();
        let wrapped = WrappedKey::seal(&key, "correct horse battery", KdfParams::fast_for_tests()).unwrap();

        assert!(!wrapped.ciphertext.windows(KEY_LEN).any(|w| w == key.0));
        assert_eq!(wrapped.open("correct horse battery").unwrap().0, key.0);
        assert!(wrapped.open("wrong password").is_err());
    }

    #[test]
    fn test_wrapped_key_uses_fresh_salt() {
        let key = DataKey::generate();
        let params = KdfParams::fast_for_tests();
        let first = WrappedKey::seal(&key, "same password", params).unwrap();
        let second = WrappedKey::seal(&key, "same password", params).unwrap();

        assert_ne!(first.salt, second.salt);
        assert_ne!(first.ciphertext, second.ciphertext);
    }

    #[test]
    fn test_recovery_code_format() {
        let code = generate_recovery_code();
        let groups: Vec<&str> = code.split('-').collect();

        assert_eq!(groups.len(), 10);
        assert!(groups.iter().all(|g| g.len() == 4 && g.chars().all(|c| c.is_ascii_hexdigit())));
        assert_ne!(code, generate_recovery_code());
    }

    #[test]
    fn test_normalize_recovery_code() {
        assert_eq!(normalize_recovery_code(" 1a2b-3C4D\n5e6f "), "1A2B3C4D5E6F");
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