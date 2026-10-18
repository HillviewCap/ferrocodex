//! Per-user X25519 keypairs, and sealing the data key to a user's public key.
//!
//! Sealing has the shape of HPKE base mode with DHKEM(X25519), HKDF-SHA256 and
//! AES-256-GCM: an ephemeral X25519 key agreement with the recipient's public
//! key, HKDF over the shared secret, then AES-256-GCM with associated data that
//! binds the wrap to one user and one data-key version.

use std::fmt;

use aes_gcm::aead::OsRng;
use anyhow::Result;
use hkdf::Hkdf;
use sha2::Sha256;
use x25519_dalek::{EphemeralSecret, PublicKey, SharedSecret, StaticSecret};

use super::{DataKey, FileEncryption, KdfParams, WrappedKey};

const USER_PRIVATE_KEY_AAD: &[u8] = b"ferrocodex-user-private-key-v1";
const PUBWRAP_HKDF_INFO: &[u8] = b"ferrocodex-data-key-to-user-v1";
const PUBWRAP_AAD_PREFIX: &[u8] = b"ferrocodex-pubwrap-v1";

/// A user's X25519 keypair. The secret is zeroed on drop and never printed.
pub struct UserKeyPair {
    public: [u8; 32],
    secret: StaticSecret,
}

impl UserKeyPair {
    pub fn generate() -> Self {
        Self::from_secret(StaticSecret::random_from_rng(OsRng))
    }

    fn from_secret(secret: StaticSecret) -> Self {
        let public = PublicKey::from(&secret).to_bytes();
        Self { public, secret }
    }

    pub fn public_key(&self) -> &[u8; 32] {
        &self.public
    }

    /// Wraps the private key under an Argon2id key derived from `password`.
    pub fn seal_secret(&self, password: &str, params: KdfParams) -> Result<WrappedKey> {
        WrappedKey::seal_bytes(self.secret.as_bytes(), password, params, USER_PRIVATE_KEY_AAD)
    }

    /// Unwraps a private key sealed by [`UserKeyPair::seal_secret`] and
    /// recomputes its public key.
    pub fn open_secret(wrapped: &WrappedKey, password: &str) -> Result<Self> {
        let bytes = wrapped.open_bytes(password, USER_PRIVATE_KEY_AAD)?;
        if bytes.len() != 32 {
            return Err(anyhow::anyhow!("Invalid private key length"));
        }
        // x25519-dalek 2 only builds a StaticSecret from an owned [u8; 32]
        // (there is no constructor from a reference or slice), so the bytes
        // have to be passed by value. Copy them into a zeroizing array and
        // move that in: our own copy and the decrypted Vec are wiped on drop,
        // and the StaticSecret zeroizes itself. The by-value argument of
        // `From<[u8; 32]>` may still leave a transient copy in a stack slot
        // the compiler chooses; Rust gives no way to wipe that, so it is
        // accepted here.
        let mut secret = zeroize::Zeroizing::new([0u8; 32]);
        secret.copy_from_slice(&bytes);
        Ok(Self::from_secret(StaticSecret::from(*secret)))
    }
}

impl fmt::Debug for UserKeyPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserKeyPair")
            .field("public", &self.public)
            .field("secret", &"<redacted>")
            .finish()
    }
}

/// The data key sealed to one user's public key. Contains no secret material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKeyWrap {
    pub ephemeral_public: [u8; 32],
    /// Random nonce followed by the AES-256-GCM ciphertext and tag, the
    /// layout `FileEncryption::encrypt_with_aad` produces.
    pub ciphertext: Vec<u8>,
}

/// Encrypts `key` so only the holder of `recipient_public`'s secret can open
/// it, bound to `user_id` and `key_version`.
pub fn seal_to_public_key(
    key: &DataKey,
    recipient_public: &[u8; 32],
    user_id: i64,
    key_version: u32,
) -> Result<PublicKeyWrap> {
    let ephemeral = EphemeralSecret::random_from_rng(OsRng);
    let ephemeral_public = PublicKey::from(&ephemeral).to_bytes();
    let shared = ephemeral.diffie_hellman(&PublicKey::from(*recipient_public));

    let wrapping_key = derive_wrapping_key(&shared, &ephemeral_public, recipient_public)?;
    let ciphertext = FileEncryption::new(&wrapping_key)
        .encrypt_with_aad(&key.bytes, &pubwrap_aad(user_id, key_version))?;

    Ok(PublicKeyWrap { ephemeral_public, ciphertext })
}

/// Opens a [`PublicKeyWrap`] with the recipient's keypair. Fails unless
/// `user_id` and `key_version` match the values it was sealed with.
pub fn open_with_secret(
    wrap: &PublicKeyWrap,
    keypair: &UserKeyPair,
    user_id: i64,
    key_version: u32,
) -> Result<DataKey> {
    let shared = keypair.secret.diffie_hellman(&PublicKey::from(wrap.ephemeral_public));

    let wrapping_key = derive_wrapping_key(&shared, &wrap.ephemeral_public, &keypair.public)?;
    let bytes = FileEncryption::new(&wrapping_key)
        .decrypt_with_aad(&wrap.ciphertext, &pubwrap_aad(user_id, key_version))
        .map(zeroize::Zeroizing::new)
        .map_err(|_| anyhow::anyhow!("Wrong key or corrupted key wrap"))?;
    DataKey::from_slice_with_version(&bytes, key_version)
}

/// HKDF-SHA256(salt = ephemeral_public || recipient_public, ikm = shared,
/// info = PUBWRAP_HKDF_INFO). Rejects a shared secret from a low-order point.
fn derive_wrapping_key(
    shared: &SharedSecret,
    ephemeral_public: &[u8; 32],
    recipient_public: &[u8; 32],
) -> Result<DataKey> {
    if !shared.was_contributory() {
        return Err(anyhow::anyhow!("Invalid public key"));
    }

    let mut salt = [0u8; 64];
    salt[..32].copy_from_slice(ephemeral_public);
    salt[32..].copy_from_slice(recipient_public);

    // DataKey zeroes the output on drop
    let mut okm = DataKey::zeroed(super::INITIAL_KEY_VERSION);
    Hkdf::<Sha256>::new(Some(&salt), shared.as_bytes())
        .expand(PUBWRAP_HKDF_INFO, &mut okm.bytes)
        .map_err(|_| anyhow::anyhow!("Key derivation failed"))?;
    Ok(okm)
}

fn pubwrap_aad(user_id: i64, key_version: u32) -> Vec<u8> {
    let mut aad = Vec::with_capacity(PUBWRAP_AAD_PREFIX.len() + 12);
    aad.extend_from_slice(PUBWRAP_AAD_PREFIX);
    aad.extend_from_slice(&user_id.to_be_bytes());
    aad.extend_from_slice(&key_version.to_be_bytes());
    aad
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encryption::KEY_LEN;

    #[test]
    fn test_keypair_seal_open_round_trip() {
        let keypair = UserKeyPair::generate();
        let wrapped = keypair.seal_secret("correct horse battery", KdfParams::fast_for_tests()).unwrap();

        let opened = UserKeyPair::open_secret(&wrapped, "correct horse battery").unwrap();
        assert_eq!(opened.public, keypair.public);
        assert_eq!(opened.secret.as_bytes(), keypair.secret.as_bytes());
    }

    #[test]
    fn test_keypair_open_wrong_password_fails() {
        let keypair = UserKeyPair::generate();
        let wrapped = keypair.seal_secret("right password", KdfParams::fast_for_tests()).unwrap();

        assert!(UserKeyPair::open_secret(&wrapped, "wrong password").is_err());
    }

    #[test]
    fn test_wrapped_private_key_does_not_contain_secret() {
        let keypair = UserKeyPair::generate();
        let wrapped = keypair.seal_secret("pw", KdfParams::fast_for_tests()).unwrap();

        let secret = keypair.secret.as_bytes();
        assert!(!wrapped.ciphertext.windows(KEY_LEN).any(|w| w == secret));
        assert!(!wrapped.salt.windows(KEY_LEN).any(|w| w == secret));
    }

    #[test]
    fn test_private_key_wrap_and_data_key_wrap_are_not_interchangeable() {
        let params = KdfParams::fast_for_tests();
        let keypair = UserKeyPair::generate();
        let private_wrap = keypair.seal_secret("pw", params).unwrap();
        assert!(private_wrap.open("pw").is_err());

        let data_wrap = WrappedKey::seal(&DataKey::generate(), "pw", params).unwrap();
        assert!(UserKeyPair::open_secret(&data_wrap, "pw").is_err());
    }

    #[test]
    fn test_generated_keypairs_are_unique() {
        assert_ne!(UserKeyPair::generate().public, UserKeyPair::generate().public);
    }

    #[test]
    fn test_keypair_debug_redacts_secret() {
        let keypair = UserKeyPair::generate();
        let debug = format!("{:?}", keypair);

        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains(&format!("{:?}", keypair.secret.as_bytes())));
    }

    #[test]
    fn test_public_key_wrap_round_trip() {
        let key = DataKey::generate();
        let keypair = UserKeyPair::generate();

        let wrap = seal_to_public_key(&key, keypair.public_key(), 7, 3).unwrap();
        assert!(!wrap.ciphertext.windows(KEY_LEN).any(|w| w == key.bytes));
        let opened = open_with_secret(&wrap, &keypair, 7, 3).unwrap();
        assert_eq!(opened.bytes, key.bytes);
        assert_eq!(opened.version(), 3);
    }

    #[test]
    fn test_public_key_wrap_uses_fresh_ephemeral_key() {
        let key = DataKey::generate();
        let keypair = UserKeyPair::generate();

        let first = seal_to_public_key(&key, keypair.public_key(), 1, 1).unwrap();
        let second = seal_to_public_key(&key, keypair.public_key(), 1, 1).unwrap();
        assert_ne!(first.ephemeral_public, second.ephemeral_public);
        assert_ne!(first.ciphertext, second.ciphertext);
    }

    #[test]
    fn test_public_key_wrap_binds_user_id_and_version() {
        let key = DataKey::generate();
        let keypair = UserKeyPair::generate();
        let wrap = seal_to_public_key(&key, keypair.public_key(), 7, 3).unwrap();

        assert!(open_with_secret(&wrap, &keypair, 8, 3).is_err());
        assert!(open_with_secret(&wrap, &keypair, -7, 3).is_err());
        assert!(open_with_secret(&wrap, &keypair, 7, 2).is_err());
        assert!(open_with_secret(&wrap, &keypair, 7, 4).is_err());
    }

    #[test]
    fn test_public_key_wrap_wrong_recipient_fails() {
        let key = DataKey::generate();
        let recipient = UserKeyPair::generate();
        let wrap = seal_to_public_key(&key, recipient.public_key(), 1, 1).unwrap();

        assert!(open_with_secret(&wrap, &UserKeyPair::generate(), 1, 1).is_err());
    }

    #[test]
    fn test_public_key_wrap_tampering_fails() {
        let key = DataKey::generate();
        let keypair = UserKeyPair::generate();
        let wrap = seal_to_public_key(&key, keypair.public_key(), 1, 1).unwrap();

        let mut bad_ct = wrap.clone();
        bad_ct.ciphertext[20] ^= 1;
        assert!(open_with_secret(&bad_ct, &keypair, 1, 1).is_err());

        let mut bad_eph = wrap.clone();
        bad_eph.ephemeral_public[0] ^= 1;
        assert!(open_with_secret(&bad_eph, &keypair, 1, 1).is_err());
    }

    /// Small-order points on Curve25519 (in Montgomery u-coordinates), whose
    /// shared secret with any scalar is all zeros.
    fn low_order_points() -> Vec<[u8; 32]> {
        let mut one = [0u8; 32];
        one[0] = 1;
        let order_8 = [
            0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1, 0x9f, 0xc4, 0x6a,
            0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16, 0x5f, 0x49, 0xb8, 0x00,
        ];
        // p - 1, which has order 2
        let mut p_minus_one = [0xffu8; 32];
        p_minus_one[0] = 0xec;
        p_minus_one[31] = 0x7f;
        vec![[0u8; 32], one, order_8, p_minus_one]
    }

    #[test]
    fn test_seal_rejects_low_order_public_keys() {
        let key = DataKey::generate();
        for point in low_order_points() {
            assert!(seal_to_public_key(&key, &point, 1, 1).is_err(), "accepted {:02x?}", point);
        }
    }

    #[test]
    fn test_open_rejects_low_order_ephemeral_keys() {
        let keypair = UserKeyPair::generate();
        for point in low_order_points() {
            let wrap = PublicKeyWrap { ephemeral_public: point, ciphertext: vec![0u8; 60] };
            let err = open_with_secret(&wrap, &keypair, 1, 1).unwrap_err();
            assert_eq!(err.to_string(), "Invalid public key");
        }
    }
}
