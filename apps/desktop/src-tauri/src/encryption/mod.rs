// The keypair primitives and `reencrypt_in_place` have no callers until the
// keystore and rotation commands adopt them (data-key rotation design, chunks
// 4 and 5). Remove these allows then.
#[allow(dead_code)]
mod keypair;

#[allow(unused_imports)]
pub use keypair::{open_with_secret, seal_to_public_key, PublicKeyWrap, UserKeyPair};

use anyhow::Result;
use std::fmt;
use std::io::{Read, Write};
use aes_gcm::{
    aead::{rand_core::RngCore, Aead, AeadCore, AeadInPlace, KeyInit, OsRng, Payload},
    Aes256Gcm, Nonce, Key, Tag
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

pub const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
#[allow(dead_code)]
const TAG_LEN: usize = 16;
const SALT_LEN: usize = 16;
const RECOVERY_CODE_BYTES: usize = 20;
const KEY_WRAP_AAD: &[u8] = b"ferrocodex-data-key-wrap-v1";

/// Version of the first data key of an install. Rotation (design chunk 5)
/// moves to higher versions; every ciphertext records the version it was
/// written under in its row's `key_version` column.
pub const INITIAL_KEY_VERSION: u32 = 1;

/// A random 256-bit AES key and the data-key version it belongs to. The bytes
/// are zeroed on drop and never printed.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct DataKey {
    bytes: [u8; KEY_LEN],
    version: u32,
}

impl DataKey {
    /// Generates a key at [`INITIAL_KEY_VERSION`].
    pub fn generate() -> Self {
        Self::generate_with_version(INITIAL_KEY_VERSION)
    }

    pub fn generate_with_version(version: u32) -> Self {
        let mut key = Self::zeroed(version);
        OsRng.fill_bytes(&mut key.bytes);
        key
    }

    fn zeroed(version: u32) -> Self {
        Self { bytes: [0u8; KEY_LEN], version }
    }

    /// Builds a key at [`INITIAL_KEY_VERSION`] from raw bytes.
    fn from_slice(bytes: &[u8]) -> Result<Self> {
        Self::from_slice_with_version(bytes, INITIAL_KEY_VERSION)
    }

    fn from_slice_with_version(bytes: &[u8], version: u32) -> Result<Self> {
        let bytes: [u8; KEY_LEN] = bytes.try_into()
            .map_err(|_| anyhow::anyhow!("Invalid key length"))?;
        Ok(Self { bytes, version })
    }

    /// The data-key version, stored as `key_version` next to every
    /// ciphertext written under this key.
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Returns the same key bytes labelled with `version`, e.g. the
    /// `key_version` of the row it was unwrapped from.
    pub fn with_version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }
}

impl fmt::Debug for DataKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DataKey(v{}, <redacted>)", self.version)
    }
}

// AES-256-GCM encryption for configuration files, firmware, and vault secrets
pub struct FileEncryption {
    cipher: Aes256Gcm,
}

impl FileEncryption {
    pub fn new(key: &DataKey) -> Self {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key.bytes));
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

    /// Re-encrypts a blob produced by [`FileEncryption::encrypt`] under `from`
    /// so that it is encrypted under `to`, without allocating a second copy.
    ///
    /// The output has the same layout as `encrypt` (fresh random nonce, then
    /// ciphertext and tag) and the same length as the input, so `decrypt` with
    /// `to` reads it. If the blob does not authenticate under `from`, `buf` is
    /// left unchanged.
    #[allow(dead_code)]
    pub fn reencrypt_in_place(from: &DataKey, to: &DataKey, buf: &mut [u8]) -> Result<()> {
        if buf.len() < NONCE_LEN + TAG_LEN {
            return Err(anyhow::anyhow!("Invalid encrypted data: too short"));
        }

        let (nonce_bytes, rest) = buf.split_at_mut(NONCE_LEN);
        let (body, tag_bytes) = rest.split_at_mut(rest.len() - TAG_LEN);

        // aes-gcm verifies the tag before applying the keystream, so a failed
        // decryption leaves `body` untouched
        FileEncryption::new(from).cipher
            .decrypt_in_place_detached(
                Nonce::from_slice(nonce_bytes),
                b"",
                body,
                Tag::from_slice(tag_bytes),
            )
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

        let new_nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        match FileEncryption::new(to).cipher.encrypt_in_place_detached(&new_nonce, b"", body) {
            Ok(tag) => {
                nonce_bytes.copy_from_slice(&new_nonce);
                tag_bytes.copy_from_slice(&tag);
                Ok(())
            }
            Err(e) => {
                // Never hand back a buffer that holds plaintext
                body.zeroize();
                Err(anyhow::anyhow!("Encryption failed: {}", e))
            }
        }
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
    let mut key = DataKey::zeroed(INITIAL_KEY_VERSION);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params)
        .hash_password_into(secret, salt, &mut key.bytes)
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
    /// Wraps a data key under `KEY_WRAP_AAD`, the format of every existing
    /// `data_key_wraps` row. The key's version is not part of the wrap: it is
    /// stored in the row's `key_version` column.
    pub fn seal(key: &DataKey, secret: &str, params: KdfParams) -> Result<Self> {
        Self::seal_bytes(&key.bytes, secret, params, KEY_WRAP_AAD)
    }

    /// Opens a wrap made by [`WrappedKey::seal`]. The key comes back at
    /// [`INITIAL_KEY_VERSION`]; callers relabel it with the row's version.
    pub fn open(&self, secret: &str) -> Result<DataKey> {
        let bytes = self.open_bytes(secret, KEY_WRAP_AAD)?;
        DataKey::from_slice(&bytes)
    }

    /// Encrypts arbitrary secret bytes under an Argon2id key derived from
    /// `secret`. `aad` names what is wrapped, so a wrap of one kind of secret
    /// can't be opened as another.
    pub fn seal_bytes(plaintext: &[u8], secret: &str, params: KdfParams, aad: &[u8]) -> Result<Self> {
        let salt = generate_salt();
        let wrapping_key = derive_key(secret.as_bytes(), &salt, &params)?;
        let ciphertext = FileEncryption::new(&wrapping_key).encrypt_with_aad(plaintext, aad)?;
        Ok(Self { salt, ciphertext, params })
    }

    /// Opens a wrap made by [`WrappedKey::seal_bytes`] with the same `aad`.
    /// The returned bytes are zeroed on drop.
    pub fn open_bytes(&self, secret: &str, aad: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
        let wrapping_key = derive_key(secret.as_bytes(), &self.salt, &self.params)?;
        FileEncryption::new(&wrapping_key)
            .decrypt_with_aad(&self.ciphertext, aad)
            .map(Zeroizing::new)
            .map_err(|_| anyhow::anyhow!("Incorrect secret or corrupted key"))
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
        assert_ne!(DataKey::generate().bytes, DataKey::generate().bytes);
    }

    #[test]
    fn test_key_version_defaults_and_relabelling() {
        let key = DataKey::generate();
        assert_eq!(key.version(), INITIAL_KEY_VERSION);
        assert_eq!(DataKey::from_slice(&[7u8; KEY_LEN]).unwrap().version(), INITIAL_KEY_VERSION);
        assert_eq!(DataKey::generate_with_version(5).version(), 5);

        let bytes = key.bytes;
        let relabelled = key.with_version(9);
        assert_eq!(relabelled.version(), 9);
        assert_eq!(relabelled.bytes, bytes);

        // The version does not change the AES key
        let ciphertext = FileEncryption::new(&relabelled).encrypt(b"probe").unwrap();
        assert_eq!(FileEncryption::new(&relabelled.clone().with_version(1)).decrypt(&ciphertext).unwrap(), b"probe");
    }

    #[test]
    fn test_key_zeroize_clears_bytes() {
        let mut key = DataKey::generate_with_version(4);
        key.zeroize();
        assert_eq!(key.bytes, [0u8; KEY_LEN]);
        assert_eq!(key.version(), 0);
    }

    #[test]
    fn test_wrapped_key_opens_at_initial_version() {
        let key = DataKey::generate_with_version(6);
        let wrapped = WrappedKey::seal(&key, "pw", KdfParams::fast_for_tests()).unwrap();
        let opened = wrapped.open("pw").unwrap();
        assert_eq!(opened.bytes, key.bytes);
        assert_eq!(opened.version(), INITIAL_KEY_VERSION);
    }

    #[test]
    fn test_debug_does_not_print_key_bytes() {
        assert_eq!(format!("{:?}", DataKey::generate()), "DataKey(v1, <redacted>)");
    }

    #[test]
    fn test_derive_key_depends_on_secret_and_salt() {
        let params = KdfParams::fast_for_tests();
        let salt = generate_salt();

        let key = derive_key(b"password", &salt, &params).unwrap();
        assert_eq!(key.bytes, derive_key(b"password", &salt, &params).unwrap().bytes);
        assert_ne!(key.bytes, derive_key(b"Password", &salt, &params).unwrap().bytes);
        assert_ne!(key.bytes, derive_key(b"password", &generate_salt(), &params).unwrap().bytes);
    }

    #[test]
    fn test_wrapped_key_round_trip() {
        let key = DataKey::generate();
        let wrapped = WrappedKey::seal(&key, "correct horse battery", KdfParams::fast_for_tests()).unwrap();

        assert!(!wrapped.ciphertext.windows(KEY_LEN).any(|w| w == key.bytes));
        assert_eq!(wrapped.open("correct horse battery").unwrap().bytes, key.bytes);
        assert!(wrapped.open("wrong password").is_err());
    }

    fn from_hex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    /// Produced by `WrappedKey::seal` before it was generalized to `seal_bytes`,
    /// with key bytes 0..32, password "legacy-password", and fast test params.
    fn legacy_wrapped_key() -> WrappedKey {
        WrappedKey {
            salt: from_hex("0ed5f97029ecc640f86a7966b8dc6e79"),
            ciphertext: from_hex(
                "fb9d3bcc5c740d6b3f086d3599a0934005c61ab1a8f2a3a479556da71d4eb30f\
                 f43dd79e4cbc807eaa86c0ece86ce64183274318c9fefea27e12210c",
            ),
            params: KdfParams::fast_for_tests(),
        }
    }

    #[test]
    fn test_legacy_wrapped_key_vector_still_opens() {
        let expected: Vec<u8> = (0u8..32).collect();
        let wrapped = legacy_wrapped_key();

        assert_eq!(wrapped.open("legacy-password").unwrap().bytes.to_vec(), expected);
        assert_eq!(*wrapped.open_bytes("legacy-password", KEY_WRAP_AAD).unwrap(), expected);
        assert!(wrapped.open("wrong password").is_err());
        assert!(wrapped.open_bytes("legacy-password", b"some-other-aad").is_err());
    }

    #[test]
    fn test_seal_matches_seal_bytes_with_legacy_aad() {
        let key = DataKey::generate();
        let params = KdfParams::fast_for_tests();

        let via_bytes = WrappedKey::seal_bytes(&key.bytes, "pw", params, KEY_WRAP_AAD).unwrap();
        assert_eq!(via_bytes.open("pw").unwrap().bytes, key.bytes);

        let via_seal = WrappedKey::seal(&key, "pw", params).unwrap();
        assert_eq!(*via_seal.open_bytes("pw", KEY_WRAP_AAD).unwrap(), key.bytes.to_vec());
    }

    #[test]
    fn test_seal_bytes_binds_aad() {
        let wrapped = WrappedKey::seal_bytes(b"payload", "pw", KdfParams::fast_for_tests(), b"aad-a").unwrap();

        assert_eq!(*wrapped.open_bytes("pw", b"aad-a").unwrap(), b"payload".to_vec());
        assert!(wrapped.open_bytes("pw", b"aad-b").is_err());
        assert!(wrapped.open("pw").is_err());
    }

    #[test]
    fn test_reencrypt_in_place_round_trip() {
        let old_key = DataKey::generate();
        let new_key = DataKey::generate();
        let plaintext = b"firmware image bytes ".repeat(500);

        let mut buf = FileEncryption::new(&old_key).encrypt(&plaintext).unwrap();
        let original = buf.clone();
        let capacity = buf.capacity();

        FileEncryption::reencrypt_in_place(&old_key, &new_key, &mut buf).unwrap();

        assert_eq!(buf.len(), original.len());
        assert_eq!(buf.capacity(), capacity);
        assert_ne!(buf[..NONCE_LEN], original[..NONCE_LEN]);
        assert_eq!(FileEncryption::new(&new_key).decrypt(&buf).unwrap(), plaintext);
        assert!(FileEncryption::new(&old_key).decrypt(&buf).is_err());
    }

    #[test]
    fn test_reencrypt_in_place_empty_plaintext() {
        let old_key = DataKey::generate();
        let new_key = DataKey::generate();
        let mut buf = FileEncryption::new(&old_key).encrypt(b"").unwrap();

        FileEncryption::reencrypt_in_place(&old_key, &new_key, &mut buf).unwrap();
        assert_eq!(FileEncryption::new(&new_key).decrypt(&buf).unwrap(), b"");
    }

    #[test]
    fn test_reencrypt_in_place_wrong_key_leaves_buffer_unchanged() {
        let key = DataKey::generate();
        let mut buf = FileEncryption::new(&key).encrypt(b"config contents").unwrap();
        let original = buf.clone();

        assert!(FileEncryption::reencrypt_in_place(&DataKey::generate(), &DataKey::generate(), &mut buf).is_err());
        assert_eq!(buf, original);

        let mut short = vec![0u8; NONCE_LEN + TAG_LEN - 1];
        assert!(FileEncryption::reencrypt_in_place(&key, &key, &mut short).is_err());
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