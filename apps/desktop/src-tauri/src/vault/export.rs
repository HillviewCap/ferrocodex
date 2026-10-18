use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::encryption::{DataKey, FileEncryption, KdfParams, derive_key, generate_salt};
use super::VaultInfo;

const EXPORT_FORMAT: &str = "ferrocodex-vault-export-v1";
pub const MIN_PASSPHRASE_LEN: usize = 12;

/// A vault exported for use on another install. The data key never leaves the
/// install that created it, so each secret is re-encrypted under a key
/// derived from a passphrase chosen at export time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultExportFile {
    pub format: String,
    pub kdf: KdfParams,
    pub salt: String,
    pub vault: VaultInfo,
}

/// Re-encrypts the vault's secrets from the data key to a key derived from
/// `passphrase`, and returns the export file as JSON.
pub fn seal_vault_export(vault_info: &VaultInfo, data_key: &DataKey, passphrase: &str) -> Result<String> {
    seal_with_params(vault_info, data_key, passphrase, KdfParams::default())
}

/// Opens an export file and re-encrypts its secrets under the data key, ready
/// for `VaultRepository::import_vault`.
pub fn open_vault_export(json: &str, passphrase: &str, data_key: &DataKey) -> Result<VaultInfo> {
    let file: VaultExportFile = serde_json::from_str(json)
        .map_err(|_| anyhow::anyhow!("Not a passphrase-protected Ferrocodex vault export"))?;
    if file.format != EXPORT_FORMAT {
        return Err(anyhow::anyhow!("Unsupported vault export format: {}", file.format));
    }

    let salt = general_purpose::STANDARD.decode(&file.salt)
        .map_err(|_| anyhow::anyhow!("Corrupted vault export"))?;
    let export_key = derive_key(passphrase.as_bytes(), &salt, &file.kdf)?;

    let mut vault = file.vault;
    reencrypt_secrets(&mut vault, &FileEncryption::new(&export_key), &FileEncryption::new(data_key))
        .map_err(|_| anyhow::anyhow!("Incorrect passphrase or corrupted vault export"))?;
    Ok(vault)
}

fn seal_with_params(vault_info: &VaultInfo, data_key: &DataKey, passphrase: &str, params: KdfParams) -> Result<String> {
    if passphrase.chars().count() < MIN_PASSPHRASE_LEN {
        return Err(anyhow::anyhow!("Export passphrase must be at least {} characters", MIN_PASSPHRASE_LEN));
    }

    let salt = generate_salt();
    let export_key = derive_key(passphrase.as_bytes(), &salt, &params)?;

    let mut vault = vault_info.clone();
    reencrypt_secrets(&mut vault, &FileEncryption::new(data_key), &FileEncryption::new(&export_key))?;

    let file = VaultExportFile {
        format: EXPORT_FORMAT.to_string(),
        kdf: params,
        salt: general_purpose::STANDARD.encode(salt),
        vault,
    };
    Ok(serde_json::to_string_pretty(&file)?)
}

fn reencrypt_secrets(vault: &mut VaultInfo, from: &FileEncryption, to: &FileEncryption) -> Result<()> {
    for secret in &mut vault.secrets {
        let mut plaintext = from.decrypt_from_base64(&secret.encrypted_value)?;
        let reencrypted = to.encrypt_to_base64(&plaintext);
        plaintext.zeroize();
        secret.encrypted_value = reencrypted?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::{IdentityVault, SecretType, VaultSecret};

    const PASSPHRASE: &str = "substation-7 recovery";

    fn vault_with_secret(data_key: &DataKey, value: &str) -> VaultInfo {
        let encrypted_value = FileEncryption::new(data_key).encrypt_to_base64(value.as_bytes()).unwrap();
        VaultInfo {
            vault: IdentityVault {
                id: 1,
                asset_id: 1,
                name: "PLC-01".to_string(),
                description: String::new(),
                created_by: 1,
                created_at: String::new(),
                updated_at: String::new(),
            },
            secrets: vec![VaultSecret {
                id: 1,
                vault_id: 1,
                secret_type: SecretType::Password,
                label: "admin".to_string(),
                encrypted_value,
                created_at: String::new(),
                updated_at: String::new(),
                strength_score: None,
                last_changed: None,
                generation_method: None,
                policy_version: None,
                last_rotated: None,
                rotation_interval_days: None,
                next_rotation_due: None,
                rotation_policy_id: None,
                key_version: 1,
            }],
            secret_count: 1,
        }
    }

    fn seal(vault: &VaultInfo, data_key: &DataKey, passphrase: &str) -> Result<String> {
        seal_with_params(vault, data_key, passphrase, KdfParams::fast_for_tests())
    }

    fn decrypt_first(vault: &VaultInfo, data_key: &DataKey) -> String {
        let plaintext = FileEncryption::new(data_key).decrypt_from_base64(&vault.secrets[0].encrypted_value).unwrap();
        String::from_utf8(plaintext).unwrap()
    }

    #[test]
    fn test_export_moves_secrets_between_installs() {
        let source_key = DataKey::generate();
        let target_key = DataKey::generate();
        let vault = vault_with_secret(&source_key, "Pl4nt-Admin!");

        let json = seal(&vault, &source_key, PASSPHRASE).unwrap();
        let imported = open_vault_export(&json, PASSPHRASE, &target_key).unwrap();

        assert_eq!(decrypt_first(&imported, &target_key), "Pl4nt-Admin!");
        assert_eq!(imported.secrets[0].label, "admin");
    }

    #[test]
    fn test_export_is_not_readable_with_the_data_key() {
        let data_key = DataKey::generate();
        let vault = vault_with_secret(&data_key, "Pl4nt-Admin!");

        let json = seal(&vault, &data_key, PASSPHRASE).unwrap();
        assert!(!json.contains("Pl4nt-Admin!"));

        let file: VaultExportFile = serde_json::from_str(&json).unwrap();
        assert!(FileEncryption::new(&data_key).decrypt_from_base64(&file.vault.secrets[0].encrypted_value).is_err());
    }

    #[test]
    fn test_wrong_passphrase_is_rejected() {
        let data_key = DataKey::generate();
        let json = seal(&vault_with_secret(&data_key, "secret"), &data_key, PASSPHRASE).unwrap();

        let err = open_vault_export(&json, "not the passphrase", &data_key).unwrap_err();
        assert!(err.to_string().contains("Incorrect passphrase"));
    }

    #[test]
    fn test_short_passphrase_is_rejected() {
        let data_key = DataKey::generate();
        assert!(seal(&vault_with_secret(&data_key, "secret"), &data_key, "short").is_err());
    }

    #[test]
    fn test_unprotected_vault_json_is_rejected() {
        let data_key = DataKey::generate();
        let plain_json = serde_json::to_string(&vault_with_secret(&data_key, "secret")).unwrap();
        assert!(open_vault_export(&plain_json, PASSPHRASE, &data_key).is_err());
    }
}
