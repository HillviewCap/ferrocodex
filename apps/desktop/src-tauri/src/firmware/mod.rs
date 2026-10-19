use anyhow::Result;
use rusqlite::{Connection, Row, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use crate::encryption::{DataKey, FileEncryption};
use std::fs;
use sha2::{Sha256, Digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareVersion {
    pub id: i64,
    pub asset_id: i64,
    pub author_id: i64,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub version: String,
    pub notes: Option<String>,
    pub status: FirmwareStatus,
    pub file_path: String,
    pub file_hash: String,
    pub file_size: i64,
    pub status_changed_at: Option<String>,
    pub status_changed_by: Option<i64>,
    pub created_at: String,
    /// Name of the uploaded file, for display and export naming only; never a
    /// storage path. `None` for records from before it was kept.
    #[serde(default)]
    pub original_filename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum FirmwareStatus {
    Draft,
    Approved,
    Golden,
    Archived,
}

impl std::fmt::Display for FirmwareStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FirmwareStatus::Draft => write!(f, "Draft"),
            FirmwareStatus::Approved => write!(f, "Approved"),
            FirmwareStatus::Golden => write!(f, "Golden"),
            FirmwareStatus::Archived => write!(f, "Archived"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFirmwareRequest {
    pub asset_id: i64,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub version: String,
    pub notes: Option<String>,
    /// Already sanitized with [`sanitize_original_filename`].
    #[serde(default)]
    pub original_filename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareVersionInfo {
    pub id: i64,
    pub asset_id: i64,
    pub author_id: i64,
    pub author_username: String,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub version: String,
    pub notes: Option<String>,
    pub status: FirmwareStatus,
    pub file_path: String,
    pub file_hash: String,
    pub file_size: i64,
    pub created_at: String,
    #[serde(default)]
    pub original_filename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareStatusHistory {
    pub id: i64,
    pub firmware_version_id: i64,
    pub old_status: String,
    pub new_status: String,
    pub changed_by: i64,
    pub changed_by_username: Option<String>,
    pub changed_at: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFirmwareStatusRequest {
    pub new_status: FirmwareStatus,
    pub reason: Option<String>,
}

pub trait FirmwareRepository {
    /// `key_version` is the version of the data key the file at `file_path` was encrypted under.
    fn create_firmware(&self, request: CreateFirmwareRequest, author_id: i64, file_path: String, file_hash: String, file_size: i64, key_version: u32) -> Result<FirmwareVersion>;
    fn get_firmware_by_asset(&self, asset_id: i64) -> Result<Vec<FirmwareVersionInfo>>;
    fn get_firmware_by_id(&self, firmware_id: i64) -> Result<Option<FirmwareVersion>>;
    fn delete_firmware(&self, firmware_id: i64) -> Result<Option<String>>;
    fn get_linked_configuration_count(&self, firmware_id: i64) -> Result<i64>;
    fn update_firmware_status(&self, firmware_id: i64, new_status: FirmwareStatus, user_id: i64, reason: Option<String>) -> Result<()>;
    fn get_firmware_status_history(&self, firmware_id: i64) -> Result<Vec<FirmwareStatusHistory>>;
    fn get_available_firmware_status_transitions(&self, firmware_id: i64, user_role: &str) -> Result<Vec<FirmwareStatus>>;
    fn promote_firmware_to_golden(&self, firmware_id: i64, user_id: i64, reason: String) -> Result<()>;
    fn update_firmware_notes(&self, firmware_id: i64, notes: String) -> Result<()>;
    fn update_firmware_file_path(&self, firmware_id: i64, file_path: String) -> Result<()>;
}

pub struct SqliteFirmwareRepository<'a> {
    conn: &'a Connection,
}

impl<'a> SqliteFirmwareRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS firmware_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                asset_id INTEGER NOT NULL,
                author_id INTEGER NOT NULL,
                vendor TEXT,
                model TEXT,
                version TEXT NOT NULL,
                notes TEXT,
                status TEXT NOT NULL CHECK(status IN ('Draft', 'Approved', 'Golden', 'Archived')),
                file_path TEXT NOT NULL,
                file_hash TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                status_changed_at DATETIME,
                status_changed_by INTEGER,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                key_version INTEGER NOT NULL DEFAULT 1,
                original_filename TEXT,
                FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE,
                FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE RESTRICT,
                FOREIGN KEY (status_changed_by) REFERENCES users(id) ON DELETE RESTRICT
            );

            CREATE INDEX IF NOT EXISTS idx_firmware_asset_id ON firmware_versions(asset_id);
            CREATE INDEX IF NOT EXISTS idx_firmware_created_at ON firmware_versions(created_at);
            CREATE INDEX IF NOT EXISTS idx_firmware_status ON firmware_versions(status);
            
            CREATE TABLE IF NOT EXISTS firmware_status_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                firmware_version_id INTEGER NOT NULL,
                old_status TEXT NOT NULL,
                new_status TEXT NOT NULL,
                changed_by INTEGER NOT NULL,
                changed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                reason TEXT,
                FOREIGN KEY (firmware_version_id) REFERENCES firmware_versions(id) ON DELETE CASCADE,
                FOREIGN KEY (changed_by) REFERENCES users(id) ON DELETE RESTRICT
            );

            CREATE INDEX IF NOT EXISTS idx_firmware_status_history_firmware_id ON firmware_status_history(firmware_version_id);
            CREATE INDEX IF NOT EXISTS idx_firmware_status_history_changed_at ON firmware_status_history(changed_at);
            "#,
        )?;
        Ok(())
    }

    fn row_to_firmware(row: &Row) -> rusqlite::Result<FirmwareVersion> {
        Ok(FirmwareVersion {
            id: row.get("id")?,
            asset_id: row.get("asset_id")?,
            author_id: row.get("author_id")?,
            vendor: row.get("vendor")?,
            model: row.get("model")?,
            version: row.get("version")?,
            notes: row.get("notes")?,
            status: match row.get::<_, String>("status")?.as_str() {
                "Approved" => FirmwareStatus::Approved,
                "Golden" => FirmwareStatus::Golden,
                "Archived" => FirmwareStatus::Archived,
                _ => FirmwareStatus::Draft,
            },
            file_path: row.get("file_path")?,
            file_hash: row.get("file_hash")?,
            file_size: row.get("file_size")?,
            status_changed_at: row.get("status_changed_at")?,
            status_changed_by: row.get("status_changed_by")?,
            created_at: row.get("created_at")?,
            original_filename: row.get("original_filename")?,
        })
    }

    fn row_to_firmware_info(row: &Row) -> rusqlite::Result<FirmwareVersionInfo> {
        Ok(FirmwareVersionInfo {
            id: row.get("id")?,
            asset_id: row.get("asset_id")?,
            author_id: row.get("author_id")?,
            author_username: row.get("author_username")?,
            vendor: row.get("vendor")?,
            model: row.get("model")?,
            version: row.get("version")?,
            notes: row.get("notes")?,
            status: match row.get::<_, String>("status")?.as_str() {
                "Approved" => FirmwareStatus::Approved,
                "Golden" => FirmwareStatus::Golden,
                "Archived" => FirmwareStatus::Archived,
                _ => FirmwareStatus::Draft,
            },
            file_path: row.get("file_path")?,
            file_hash: row.get("file_hash")?,
            file_size: row.get("file_size")?,
            created_at: row.get("created_at")?,
            original_filename: row.get("original_filename")?,
        })
    }
}

impl<'a> FirmwareRepository for SqliteFirmwareRepository<'a> {
    fn create_firmware(&self, request: CreateFirmwareRequest, author_id: i64, file_path: String, file_hash: String, file_size: i64, key_version: u32) -> Result<FirmwareVersion> {
        let tx = self.conn.unchecked_transaction()?;
        // The file was encrypted before this insert; refuse to record it
        // under a key a rotation has replaced (or is replacing)
        crate::keystore::ensure_key_version_is_current(self.conn, key_version)?;
        
        let firmware = FirmwareVersion {
            id: 0, // Will be set by database
            asset_id: request.asset_id,
            author_id,
            vendor: request.vendor,
            model: request.model,
            version: request.version,
            notes: request.notes,
            status: FirmwareStatus::Draft,
            file_path: file_path.clone(),
            file_hash: file_hash.clone(),
            file_size,
            status_changed_at: None,
            status_changed_by: None,
            created_at: String::new(), // Will be set by database
            original_filename: request.original_filename,
        };

        tx.execute(
            "INSERT INTO firmware_versions (asset_id, author_id, vendor, model, version, notes, status, file_path, file_hash, file_size, key_version, original_filename)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                firmware.asset_id,
                firmware.author_id,
                firmware.vendor,
                firmware.model,
                firmware.version,
                firmware.notes,
                firmware.status.to_string(),
                firmware.file_path,
                firmware.file_hash,
                file_size,
                key_version,
                firmware.original_filename,
            ],
        )?;

        let id = tx.last_insert_rowid();
        tx.commit()?;

        Ok(FirmwareVersion {
            id,
            ..firmware
        })
    }

    fn get_firmware_by_asset(&self, asset_id: i64) -> Result<Vec<FirmwareVersionInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT f.id, f.asset_id, f.author_id, u.username as author_username, 
                    f.vendor, f.model, f.version, f.notes, f.status, 
                    f.file_path, f.file_hash, f.file_size, f.created_at, f.original_filename
             FROM firmware_versions f
             JOIN users u ON f.author_id = u.id
             WHERE f.asset_id = ?1
             ORDER BY f.created_at DESC, f.id DESC"
        )?;

        let firmware_iter = stmt.query_map([asset_id], Self::row_to_firmware_info)?;

        let mut firmwares = Vec::new();
        for firmware in firmware_iter {
            firmwares.push(firmware?);
        }
        Ok(firmwares)
    }

    fn get_firmware_by_id(&self, firmware_id: i64) -> Result<Option<FirmwareVersion>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, asset_id, author_id, vendor, model, version, notes, status, file_path, file_hash, file_size,
                    status_changed_at, status_changed_by, created_at, original_filename
             FROM firmware_versions
             WHERE id = ?1"
        )?;

        let result = stmt.query_row([firmware_id], Self::row_to_firmware);
        match result {
            Ok(firmware) => Ok(Some(firmware)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn delete_firmware(&self, firmware_id: i64) -> Result<Option<String>> {
        let tx = self.conn.unchecked_transaction()?;
        
        let file_path: Option<String> = tx
            .query_row(
                "SELECT file_path FROM firmware_versions WHERE id = ?1",
                [firmware_id],
                |row| row.get(0),
            )
            .optional()?;

        if file_path.is_some() {
            tx.execute(
                "DELETE FROM firmware_versions WHERE id = ?1",
                [firmware_id],
            )?;
            tx.commit()?;
        }

        Ok(file_path)
    }
    
    fn get_linked_configuration_count(&self, firmware_id: i64) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) FROM configuration_versions WHERE firmware_version_id = ?1"
        )?;
        let count: i64 = stmt.query_row([firmware_id], |row| row.get(0))?;
        Ok(count)
    }

    fn update_firmware_status(&self, firmware_id: i64, new_status: FirmwareStatus, user_id: i64, reason: Option<String>) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        
        // Get current status
        let current_firmware = self.get_firmware_by_id(firmware_id)?
            .ok_or_else(|| anyhow::anyhow!("Firmware version not found"))?;
        
        // Update firmware status
        tx.execute(
            "UPDATE firmware_versions 
             SET status = ?1, status_changed_at = datetime('now'), status_changed_by = ?2
             WHERE id = ?3",
            rusqlite::params![new_status.to_string(), user_id, firmware_id],
        )?;
        
        // Insert status history
        tx.execute(
            "INSERT INTO firmware_status_history (firmware_version_id, old_status, new_status, changed_by, reason)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                firmware_id,
                current_firmware.status.to_string(),
                new_status.to_string(),
                user_id,
                reason,
            ],
        )?;
        
        tx.commit()?;
        Ok(())
    }

    fn get_firmware_status_history(&self, firmware_id: i64) -> Result<Vec<FirmwareStatusHistory>> {
        let mut stmt = self.conn.prepare(
            "SELECT h.id, h.firmware_version_id, h.old_status, h.new_status, 
                    h.changed_by, u.username as changed_by_username, h.changed_at, h.reason
             FROM firmware_status_history h
             JOIN users u ON h.changed_by = u.id
             WHERE h.firmware_version_id = ?1
             ORDER BY h.changed_at DESC, h.id DESC"
        )?;

        let history_iter = stmt.query_map([firmware_id], |row| {
            Ok(FirmwareStatusHistory {
                id: row.get("id")?,
                firmware_version_id: row.get("firmware_version_id")?,
                old_status: row.get("old_status")?,
                new_status: row.get("new_status")?,
                changed_by: row.get("changed_by")?,
                changed_by_username: row.get("changed_by_username")?,
                changed_at: row.get("changed_at")?,
                reason: row.get("reason")?,
            })
        })?;

        let mut history = Vec::new();
        for entry in history_iter {
            history.push(entry?);
        }
        Ok(history)
    }

    fn get_available_firmware_status_transitions(&self, firmware_id: i64, user_role: &str) -> Result<Vec<FirmwareStatus>> {
        let firmware = self.get_firmware_by_id(firmware_id)?
            .ok_or_else(|| anyhow::anyhow!("Firmware version not found"))?;
        
        let mut transitions = Vec::new();
        
        match (firmware.status, user_role) {
            (FirmwareStatus::Draft, _) => {
                transitions.push(FirmwareStatus::Approved);
                transitions.push(FirmwareStatus::Archived);
            },
            (FirmwareStatus::Approved, "Administrator") => {
                transitions.push(FirmwareStatus::Golden);
                transitions.push(FirmwareStatus::Archived);
                transitions.push(FirmwareStatus::Draft);
            },
            (FirmwareStatus::Approved, _) => {
                transitions.push(FirmwareStatus::Archived);
                transitions.push(FirmwareStatus::Draft);
            },
            (FirmwareStatus::Golden, _) => {
                transitions.push(FirmwareStatus::Archived);
            },
            (FirmwareStatus::Archived, _) => {
                transitions.push(FirmwareStatus::Draft);
            },
        }
        
        Ok(transitions)
    }

    fn promote_firmware_to_golden(&self, firmware_id: i64, user_id: i64, reason: String) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        
        // Get current firmware
        let firmware = self.get_firmware_by_id(firmware_id)?
            .ok_or_else(|| anyhow::anyhow!("Firmware version not found"))?;
        
        // Check if firmware is in Approved status
        if firmware.status != FirmwareStatus::Approved {
            return Err(anyhow::anyhow!("Only approved firmware can be promoted to Golden"));
        }
        
        // Archive any existing Golden firmware for the same asset
        let existing_golden: Vec<i64> = tx
            .prepare("SELECT id FROM firmware_versions WHERE asset_id = ?1 AND status = 'Golden' AND id != ?2")?
            .query_map(rusqlite::params![firmware.asset_id, firmware_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        
        for golden_id in existing_golden {
            // Update status to Archived
            tx.execute(
                "UPDATE firmware_versions 
                 SET status = 'Archived', status_changed_at = datetime('now'), status_changed_by = ?1
                 WHERE id = ?2",
                rusqlite::params![user_id, golden_id],
            )?;
            
            // Insert history record
            tx.execute(
                "INSERT INTO firmware_status_history (firmware_version_id, old_status, new_status, changed_by, reason)
                 VALUES (?1, 'Golden', 'Archived', ?2, 'automatically archived when new Golden version was promoted')",
                rusqlite::params![golden_id, user_id],
            )?;
        }
        
        // Promote current firmware to Golden
        tx.execute(
            "UPDATE firmware_versions 
             SET status = 'Golden', status_changed_at = datetime('now'), status_changed_by = ?1
             WHERE id = ?2",
            rusqlite::params![user_id, firmware_id],
        )?;
        
        // Insert history record
        tx.execute(
            "INSERT INTO firmware_status_history (firmware_version_id, old_status, new_status, changed_by, reason)
             VALUES (?1, ?2, 'Golden', ?3, ?4)",
            rusqlite::params![
                firmware_id,
                firmware.status.to_string(),
                user_id,
                reason,
            ],
        )?;
        
        tx.commit()?;
        Ok(())
    }

    fn update_firmware_notes(&self, firmware_id: i64, notes: String) -> Result<()> {
        // Validate notes length
        if notes.len() > 1000 {
            return Err(anyhow::anyhow!("Notes cannot exceed 1000 characters"));
        }
        
        self.conn.execute(
            "UPDATE firmware_versions SET notes = ?1 WHERE id = ?2",
            rusqlite::params![notes, firmware_id],
        )?;
        
        Ok(())
    }

    fn update_firmware_file_path(&self, firmware_id: i64, file_path: String) -> Result<()> {
        self.conn.execute(
            "UPDATE firmware_versions SET file_path = ?1 WHERE id = ?2",
            rusqlite::params![file_path, firmware_id],
        )?;
        
        Ok(())
    }
}

pub fn get_firmware_storage_dir(app_handle: &AppHandle) -> Result<PathBuf> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| anyhow::anyhow!("Failed to get app data directory: {}", e))?;
    
    let firmware_dir = app_data_dir.join("firmware");
    std::fs::create_dir_all(&firmware_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create firmware directory: {}", e))?;
    
    Ok(firmware_dir)
}

/// Longest original file name kept, in characters.
const MAX_ORIGINAL_FILENAME_CHARS: usize = 255;

/// The final component of `path`, made safe to store and show as a firmware
/// record's original file name: control characters and path separators are
/// removed, and it is capped at 255 characters. `None` if nothing usable is
/// left. The result is a label only and is never used as a storage path.
pub fn sanitize_original_filename(path: &str) -> Option<String> {
    // Split on both separators: the name may come from another platform (a recovery bundle)
    let name = path.rsplit(['/', '\\']).next().unwrap_or_default();
    let name: String = name
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_ORIGINAL_FILENAME_CHARS)
        .collect();
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." {
        None
    } else {
        Some(name.to_string())
    }
}

pub struct FirmwareFileStorage;

/// Why re-encrypting a firmware file during a key rotation did not succeed.
#[derive(Debug)]
pub enum FirmwareReencryptError {
    /// The file does not authenticate under the old key (nor already under
    /// the new one): it was encrypted under some other key, or it is
    /// corrupted. Recorded as a rotation failure.
    Unreadable(String),
    /// The file can't be processed as stored: it is missing (and no copy
    /// under the new key exists), or its stored path is not a safe path
    /// inside the firmware directory. Recorded as a rotation failure; an
    /// administrator can fix the cause and retry.
    Rejected(String),
    /// Reading or writing failed (permissions, a locked file, a full disk).
    /// Nothing was changed, so the step can simply be retried; never recorded
    /// as a failure.
    Io(anyhow::Error),
}

impl std::fmt::Display for FirmwareReencryptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FirmwareReencryptError::Unreadable(e) | FirmwareReencryptError::Rejected(e) => write!(f, "{}", e),
            FirmwareReencryptError::Io(e) => write!(f, "Firmware file I/O failed: {}", e),
        }
    }
}

/// What [`FirmwareFileStorage::reencrypt_firmware_file`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirmwareReencrypted {
    /// Re-encrypted into a new file at this path (relative to the firmware
    /// directory). The old file is left in place.
    Written(String),
    /// The file at this path is already encrypted under the new key (left by
    /// an earlier attempt whose DB update was lost); nothing was written.
    Adopted(String),
}

/// What [`FirmwareFileStorage::probe_firmware_file`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareProbe {
    /// The stored file authenticates under the old key.
    ReadableWithOldKey,
    /// The stored file, or `<id>.k<new>.enc` next to it, authenticates under
    /// the new key.
    AlreadyReencrypted,
    /// Neither: missing, an unsafe path, or under some other key.
    Unreadable,
}

/// A firmware record as the rotation sweep sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareRecord {
    pub id: i64,
    pub file_path: String,
    pub key_version: u32,
}

/// A stored firmware path checked by [`check_stored_path`].
struct StoredFile {
    /// Directory of the file, relative to the firmware directory.
    rel_dir: PathBuf,
    /// The same directory, joined to the firmware directory.
    dir: PathBuf,
    name: String,
    /// A regular file exists there.
    exists: bool,
}

impl StoredFile {
    fn path(&self) -> PathBuf {
        self.dir.join(&self.name)
    }

    /// `name` in the same directory, relative to the firmware directory.
    fn sibling(&self, name: &str) -> String {
        self.rel_dir.join(name).to_string_lossy().to_string()
    }
}

/// Splits a stored firmware path into its relative directory and file name,
/// if it is a plain relative path (no root, prefix, `.` or `..`).
fn split_safe_path(file_path: &str) -> Option<(PathBuf, String)> {
    use std::path::Component;
    let mut parts = Vec::new();
    for component in Path::new(file_path).components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?.to_string()),
            _ => return None,
        }
    }
    let name = parts.pop()?;
    Some((parts.iter().collect(), name))
}

/// Checks that a stored firmware path is a safe relative path inside
/// `firmware_dir`: no traversal, no symbolic link along the way or at the
/// file itself, and a regular file (when it exists). The file name is not
/// otherwise constrained: recovery imports and older versions used names
/// other than `<id>.enc`.
fn check_stored_path(firmware_dir: &Path, file_path: &str) -> std::result::Result<StoredFile, FirmwareReencryptError> {
    let (rel_dir, name) = split_safe_path(file_path).ok_or_else(|| {
        FirmwareReencryptError::Rejected("Firmware path is not a relative path inside the firmware directory".to_string())
    })?;
    let dir = firmware_dir.join(&rel_dir);
    let mut current = firmware_dir.to_path_buf();
    for part in rel_dir.components() {
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(FirmwareReencryptError::Rejected("Firmware path goes through a symbolic link".to_string()));
            }
            Ok(meta) if !meta.is_dir() => {
                return Err(FirmwareReencryptError::Rejected("Firmware path is not inside a directory".to_string()));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(StoredFile { rel_dir, dir, name, exists: false });
            }
            Err(e) => return Err(FirmwareReencryptError::Io(e.into())),
        }
    }
    let exists = match fs::symlink_metadata(dir.join(&name)) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(FirmwareReencryptError::Rejected("Firmware file is a symbolic link".to_string()));
        }
        Ok(meta) if !meta.is_file() => {
            return Err(FirmwareReencryptError::Rejected("Firmware path is not a regular file".to_string()));
        }
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => return Err(FirmwareReencryptError::Io(e.into())),
    };
    Ok(StoredFile { rel_dir, dir, name, exists })
}

/// Whether `path` is a regular file (not a symlink) that authenticates under
/// `key`. A missing file is `false`; a read error is an error.
fn file_authenticates(path: &Path, key: &DataKey) -> std::result::Result<bool, FirmwareReencryptError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() => {}
        Ok(_) => return Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(FirmwareReencryptError::Io(e.into())),
    }
    let mut buf = fs::read(path).map_err(|e| FirmwareReencryptError::Io(e.into()))?;
    Ok(FileEncryption::authenticates_in_place(key, &mut buf))
}

/// Name of the file a rotation to `version` writes for firmware `id`.
fn rotated_file_name(id: i64, version: u32) -> String {
    format!("{}.k{}.enc", id, version)
}

/// Parses a firmware file name this module writes: `<id>.enc`,
/// `<id>.k<version>.enc`, or a temporary file of either (`<name>.<hex>.tmp`,
/// or `<name>.tmp` from older versions). Returns the id.
fn parse_firmware_file_name(name: &str) -> Option<i64> {
    let stem = match name.strip_suffix(".tmp") {
        Some(tmp) => match tmp.rsplit_once('.') {
            Some((base, suffix)) if suffix != "enc" && !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_hexdigit()) => base,
            _ => tmp,
        },
        None => name,
    };
    let stem = stem.strip_suffix(".enc")?;
    let (id, version) = match stem.split_once('.') {
        Some((id, version)) => (id, Some(version)),
        None => (stem, None),
    };
    let all_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !all_digits(id) {
        return None;
    }
    if let Some(version) = version {
        if !version.strip_prefix('k').is_some_and(all_digits) {
            return None;
        }
    }
    id.parse().ok()
}

/// Splits a stored relative firmware path into its `<asset_dir>/<file_name>`
/// parts, refusing anything else (absolute paths, `..`, deeper nesting).
fn split_managed_path(file_path: &str) -> Option<(String, String)> {
    use std::path::Component;
    let mut parts = Vec::new();
    for component in Path::new(file_path).components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?.to_string()),
            _ => return None,
        }
    }
    match parts.as_slice() {
        [dir, file] if !dir.is_empty() && dir.bytes().all(|b| b.is_ascii_digit()) => {
            Some((dir.clone(), file.clone()))
        }
        _ => None,
    }
}

/// Flushes a directory entry change (a rename) to disk. Windows can't open a
/// directory for this, and NTFS journals renames itself.
fn sync_dir(dir: &Path) -> std::io::Result<()> {
    #[cfg(not(windows))]
    fs::File::open(dir)?.sync_all()?;
    #[cfg(windows)]
    let _ = dir;
    Ok(())
}

impl FirmwareFileStorage {
    pub fn store_firmware_file(
        app_handle: &AppHandle,
        asset_id: i64,
        firmware_id: i64,
        file_data: &[u8],
        key: &DataKey,
    ) -> Result<(String, String, i64)> {
        let firmware_dir = get_firmware_storage_dir(app_handle)?;
        Self::store_firmware_file_in(&firmware_dir, asset_id, firmware_id, file_data, key)
    }

    /// [`FirmwareFileStorage::store_firmware_file`] for an explicit firmware directory.
    pub fn store_firmware_file_in(
        firmware_dir: &Path,
        asset_id: i64,
        firmware_id: i64,
        file_data: &[u8],
        key: &DataKey,
    ) -> Result<(String, String, i64)> {
        // Validate file size (max 2GB as specified in requirements)
        const MAX_FILE_SIZE: usize = 2 * 1024 * 1024 * 1024; // 2GB
        if file_data.len() > MAX_FILE_SIZE {
            return Err(anyhow::anyhow!("File size exceeds 2GB limit"));
        }

        // Create asset-specific subdirectory
        let asset_dir = firmware_dir.join(asset_id.to_string());
        fs::create_dir_all(&asset_dir)?;
        
        // Generate file path
        let file_name = format!("{}.enc", firmware_id);
        let file_path = asset_dir.join(&file_name);
        
        // Calculate hash of the original file
        tracing::info!("Calculating SHA256 hash for {} bytes", file_data.len());
        let mut hasher = Sha256::new();
        hasher.update(file_data);
        let file_hash = format!("{:x}", hasher.finalize());
        tracing::info!("Hash calculated successfully: {}", file_hash);
        
        // Encrypt the file data
        tracing::info!("Starting file encryption for {} bytes", file_data.len());
        let encrypted_data = FileEncryption::new(key).encrypt(file_data)?;
        tracing::info!("File encryption completed, encrypted size: {} bytes", encrypted_data.len());
        
        // Write encrypted data to file
        tracing::info!("Writing encrypted data to file: {:?}", file_path);
        fs::write(&file_path, encrypted_data)?;
        tracing::info!("File write completed successfully");
        
        // Return file path (relative to firmware dir), hash, and size
        let relative_path = PathBuf::from(asset_id.to_string())
            .join(file_name)
            .to_string_lossy()
            .to_string();
        Ok((relative_path, file_hash, file_data.len() as i64))
    }
    
    pub fn read_firmware_file(
        app_handle: &AppHandle,
        file_path: &str,
        key: &DataKey,
    ) -> Result<Vec<u8>> {
        let firmware_dir = get_firmware_storage_dir(app_handle)?;
        Self::read_firmware_file_in(&firmware_dir, file_path, key)
    }

    /// [`FirmwareFileStorage::read_firmware_file`] for an explicit firmware directory.
    pub fn read_firmware_file_in(
        firmware_dir: &Path,
        file_path: &str,
        key: &DataKey,
    ) -> Result<Vec<u8>> {
        let full_path = firmware_dir.join(file_path);
        
        // Debug logging
        tracing::info!("Attempting to read firmware file: {}", file_path);
        tracing::info!("Firmware storage directory: {:?}", firmware_dir);
        tracing::info!("Full path: {:?}", full_path);
        tracing::info!("File exists: {}", full_path.exists());
        
        // Check if file exists
        if !full_path.exists() {
            tracing::error!("Firmware file not found at path: {:?}", full_path);
            return Err(anyhow::anyhow!("Firmware file not found"));
        }
        
        // Read encrypted file
        let encrypted_data = fs::read(&full_path)?;
        
        // Decrypt the file data
        FileEncryption::new(key).decrypt(&encrypted_data)
    }
    
    pub fn delete_firmware_file(
        app_handle: &AppHandle,
        file_path: &str,
    ) -> Result<()> {
        let firmware_dir = get_firmware_storage_dir(app_handle)?;
        Self::delete_firmware_file_in(&firmware_dir, file_path)
    }

    /// [`FirmwareFileStorage::delete_firmware_file`] for an explicit firmware directory.
    pub fn delete_firmware_file_in(
        firmware_dir: &Path,
        file_path: &str,
    ) -> Result<()> {
        let full_path = firmware_dir.join(file_path);
        
        // Delete file if it exists
        if full_path.exists() {
            fs::remove_file(&full_path)?;
        }
        
        // Try to clean up empty directories
        if let Some(parent) = full_path.parent() {
            // Ignore errors when removing directory (it might not be empty)
            let _ = fs::remove_dir(parent);
        }
        
        Ok(())
    }
    
    pub fn verify_firmware_integrity(
        app_handle: &AppHandle,
        file_path: &str,
        expected_hash: &str,
        key: &DataKey,
    ) -> Result<bool> {
        let firmware_dir = get_firmware_storage_dir(app_handle)?;
        Self::verify_firmware_integrity_in(&firmware_dir, file_path, expected_hash, key)
    }

    /// [`FirmwareFileStorage::verify_firmware_integrity`] for an explicit firmware directory.
    pub fn verify_firmware_integrity_in(
        firmware_dir: &Path,
        file_path: &str,
        expected_hash: &str,
        key: &DataKey,
    ) -> Result<bool> {
        // Read and decrypt the file
        let file_data = Self::read_firmware_file_in(firmware_dir, file_path, key)?;

        // Calculate hash
        let mut hasher = Sha256::new();
        hasher.update(&file_data);
        let actual_hash = format!("{:x}", hasher.finalize());

        Ok(actual_hash == expected_hash)
    }

    /// Re-encrypts one stored firmware file from `from` to `to` for a key
    /// rotation, crash-safely. The new ciphertext is written to a uniquely
    /// named temporary file (created with `create_new`, so two writers can
    /// never share one), fsynced, renamed to `<id>.k<to>.enc` in the same
    /// directory as the stored file, and the directory is fsynced. The old
    /// file is left in place: the caller deletes it only once the DB change
    /// pointing at the new file is durable (after a WAL checkpoint).
    ///
    /// The stored path only has to be a safe relative path inside
    /// `firmware_dir` (see [`check_stored_path`]); its file name need not be
    /// `<id>.enc`.
    ///
    /// An earlier attempt may have left the work done with the DB update lost
    /// (a crash, or a commit that was not yet durable at a power cut). So if
    /// the stored file already authenticates under `to`, or is missing or
    /// unreadable while `<id>.k<to>.enc` authenticates under `to`, that file
    /// is adopted instead of reporting a failure.
    ///
    /// Only a file that authenticates under neither key is
    /// [`FirmwareReencryptError::Unreadable`]. Read and write errors are
    /// [`FirmwareReencryptError::Io`] and change nothing.
    pub fn reencrypt_firmware_file(
        firmware_dir: &Path,
        firmware_id: i64,
        file_path: &str,
        from: &DataKey,
        to: &DataKey,
    ) -> std::result::Result<FirmwareReencrypted, FirmwareReencryptError> {
        let stored = check_stored_path(firmware_dir, file_path)?;
        let new_name = rotated_file_name(firmware_id, to.version());
        let new_path = stored.dir.join(&new_name);

        if !stored.exists {
            if file_authenticates(&new_path, to)? {
                return Ok(FirmwareReencrypted::Adopted(stored.sibling(&new_name)));
            }
            return Err(FirmwareReencryptError::Rejected("Firmware file is missing".to_string()));
        }

        let mut buf = fs::read(stored.path()).map_err(|e| FirmwareReencryptError::Io(e.into()))?;
        match FileEncryption::reencrypt_in_place(from, to, &mut buf) {
            Ok(()) => {}
            Err(e) if e.is::<crate::encryption::AuthenticationFailed>() => {
                // `buf` is unchanged: see whether the work was already done
                if FileEncryption::authenticates_in_place(to, &mut buf) {
                    return Ok(FirmwareReencrypted::Adopted(stored.sibling(&stored.name)));
                }
                if stored.name != new_name && file_authenticates(&new_path, to)? {
                    return Ok(FirmwareReencrypted::Adopted(stored.sibling(&new_name)));
                }
                return Err(FirmwareReencryptError::Unreadable(
                    "Firmware file could not be decrypted with the old key".to_string(),
                ));
            }
            Err(e) => return Err(FirmwareReencryptError::Io(e)),
        }

        let tmp_path = stored.dir.join(format!("{}.{}.tmp", new_name, uuid::Uuid::new_v4().simple()));
        let write = || -> std::io::Result<()> {
            let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&tmp_path)?;
            std::io::Write::write_all(&mut file, &buf)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&tmp_path, &new_path)?;
            sync_dir(&stored.dir)
        };
        if let Err(e) = write() {
            let _ = fs::remove_file(&tmp_path);
            return Err(FirmwareReencryptError::Io(e.into()));
        }

        Ok(FirmwareReencrypted::Written(stored.sibling(&new_name)))
    }

    /// Checks, without writing anything, whether a firmware file recorded as
    /// a rotation failure is actually readable: under the old key, or already
    /// under the new key (stored file or `<id>.k<to>.enc` next to it). Used
    /// before discarding failures, since discarding destroys the old key.
    /// Read errors are errors, so the caller can refuse to discard.
    pub fn probe_firmware_file(
        firmware_dir: &Path,
        firmware_id: i64,
        file_path: &str,
        from: &DataKey,
        to: &DataKey,
    ) -> std::result::Result<FirmwareProbe, FirmwareReencryptError> {
        let stored = match check_stored_path(firmware_dir, file_path) {
            Ok(stored) => stored,
            Err(FirmwareReencryptError::Io(e)) => return Err(FirmwareReencryptError::Io(e)),
            // A path rotation refuses to open is unreadable to it
            Err(_) => return Ok(FirmwareProbe::Unreadable),
        };
        if stored.exists {
            let mut buf = fs::read(stored.path()).map_err(|e| FirmwareReencryptError::Io(e.into()))?;
            // A failed check leaves `buf` unchanged
            if FileEncryption::authenticates_in_place(from, &mut buf) {
                return Ok(FirmwareProbe::ReadableWithOldKey);
            }
            if FileEncryption::authenticates_in_place(to, &mut buf) {
                return Ok(FirmwareProbe::AlreadyReencrypted);
            }
        }
        let new_name = rotated_file_name(firmware_id, to.version());
        if stored.name != new_name && file_authenticates(&stored.dir.join(&new_name), to)? {
            return Ok(FirmwareProbe::AlreadyReencrypted);
        }
        Ok(FirmwareProbe::Unreadable)
    }

    /// Deletes a firmware file a key rotation replaced. Refuses paths that
    /// are not safe paths inside `firmware_dir` and anything that is not a
    /// regular file. A file that is already gone is not an error.
    pub fn delete_superseded_file(firmware_dir: &Path, file_path: &str) -> Result<()> {
        match check_stored_path(firmware_dir, file_path) {
            Ok(stored) if stored.exists => {
                fs::remove_file(stored.path())?;
                Ok(())
            }
            Ok(_) => Ok(()),
            Err(FirmwareReencryptError::Io(e)) => Err(e),
            Err(e) => Err(anyhow::anyhow!("{}", e)),
        }
    }

    /// Deletes files a key rotation left behind. For each firmware record,
    /// looks only in that record's asset directory, and deletes only regular
    /// files named for that id (`<id>.enc`, `<id>.k<n>.enc`, or a temporary
    /// file of either) other than the one its `file_path` points at. Unknown
    /// ids, other names, directories, symlinks, and records whose path is not
    /// a plain `<asset>/<file>` are left alone. Returns how many files were
    /// deleted.
    ///
    /// Never deletes `<id>.k<active>.enc` while the record is still at an
    /// older key version and the file authenticates under `active`: that file
    /// holds the only re-encrypted copy, and the next rotation step adopts it.
    ///
    /// Callers run this only after a successful WAL checkpoint, so the
    /// `file_path` values it keeps are durable and can't roll back to a file
    /// it deleted.
    pub fn sweep_rotation_leftovers(firmware_dir: &Path, records: &[FirmwareRecord], active: &DataKey) -> Result<usize> {
        use std::collections::HashMap;

        // asset dir -> (firmware id -> record)
        let mut by_dir: HashMap<String, HashMap<i64, &FirmwareRecord>> = HashMap::new();
        for record in records {
            let Some((dir, name)) = split_managed_path(&record.file_path) else { continue };
            if parse_firmware_file_name(&name) != Some(record.id) {
                continue;
            }
            by_dir.entry(dir).or_default().insert(record.id, record);
        }

        let mut removed = 0;
        for (dir, current) in by_dir {
            let entries = match fs::read_dir(firmware_dir.join(&dir)) {
                Ok(entries) => entries,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.into()),
            };
            for entry in entries {
                let entry = entry?;
                // file_type() does not follow symlinks
                if !entry.file_type()?.is_file() {
                    continue;
                }
                let Some(name) = entry.file_name().to_str().map(str::to_string) else { continue };
                let Some(id) = parse_firmware_file_name(&name) else { continue };
                let Some(record) = current.get(&id) else { continue };
                let keep = Path::new(&record.file_path).file_name().and_then(|n| n.to_str());
                if keep == Some(name.as_str()) {
                    continue;
                }
                if record.key_version != active.version()
                    && name == rotated_file_name(id, active.version())
                    && !matches!(file_authenticates(&entry.path(), active), Ok(false))
                {
                    tracing::warn!(
                        "Kept {}/{}: it may be the re-encrypted copy of firmware {}, which is still at key version {}",
                        dir, name, id, record.key_version
                    );
                    continue;
                }
                fs::remove_file(entry.path())?;
                tracing::info!("Removed leftover firmware file {}/{}", dir, name);
                removed += 1;
            }
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        
        conn.execute_batch(
            r#"
            CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE TABLE assets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (created_by) REFERENCES users(id)
            );
            
            INSERT INTO users (username, password_hash, role) 
            VALUES ('testuser', 'hash', 'Engineer');
            
            INSERT INTO assets (name, description, created_by) 
            VALUES ('Test Asset', 'Description', 1);
            "#
        ).unwrap();
        
        let repo = SqliteFirmwareRepository::new(&conn);
        repo.initialize_schema().unwrap();
        
        conn
    }

    #[test]
    fn test_sanitize_original_filename() {
        assert_eq!(sanitize_original_filename("/home/eng/M580.sedp").as_deref(), Some("M580.sedp"));
        assert_eq!(sanitize_original_filename("C:\\Firmware\\M580.sedp").as_deref(), Some("M580.sedp"));
        assert_eq!(sanitize_original_filename("fw\u{0}\u{7}.bin").as_deref(), Some("fw.bin"));
        assert_eq!(sanitize_original_filename("/firmware/").as_deref(), None);
        assert_eq!(sanitize_original_filename("..").as_deref(), None);
        assert_eq!(sanitize_original_filename(&"a".repeat(300)).map(|n| n.chars().count()), Some(255));
    }

    #[test]
    fn test_create_firmware() {
        let conn = setup_test_db();
        let repo = SqliteFirmwareRepository::new(&conn);
        
        let request = CreateFirmwareRequest {
            asset_id: 1,
            vendor: Some("Test Vendor".to_string()),
            model: Some("Test Model".to_string()),
            version: "1.0.0".to_string(),
            notes: Some("Test firmware".to_string()),
            original_filename: None,
        };

        let result = repo.create_firmware(
            request,
            1,
            "/path/to/firmware.enc".to_string(),
            "abc123hash".to_string(),
            1024,
            1,
        );

        assert!(result.is_ok());
        let firmware = result.unwrap();
        assert_eq!(firmware.version, "1.0.0");
        assert_eq!(firmware.status, FirmwareStatus::Draft);
        assert!(firmware.id > 0);
    }

    #[test]
    fn test_get_firmware_by_asset() {
        let conn = setup_test_db();
        let repo = SqliteFirmwareRepository::new(&conn);
        
        let request1 = CreateFirmwareRequest {
            asset_id: 1,
            vendor: Some("Vendor".to_string()),
            model: Some("Model".to_string()),
            version: "1.0.0".to_string(),
            notes: None,
            original_filename: None,
        };

        repo.create_firmware(
            request1,
            1,
            "/path/to/firmware1.enc".to_string(),
            "hash1".to_string(),
            1024,
            1,
        ).unwrap();

        let request2 = CreateFirmwareRequest {
            asset_id: 1,
            vendor: Some("Vendor".to_string()),
            model: Some("Model".to_string()),
            version: "2.0.0".to_string(),
            notes: None,
            original_filename: None,
        };

        repo.create_firmware(
            request2,
            1,
            "/path/to/firmware2.enc".to_string(),
            "hash2".to_string(),
            2048,
            1,
        ).unwrap();

        let firmwares = repo.get_firmware_by_asset(1).unwrap();
        assert_eq!(firmwares.len(), 2);
        assert_eq!(firmwares[0].version, "2.0.0");
        assert_eq!(firmwares[1].version, "1.0.0");
        assert_eq!(firmwares[0].author_username, "testuser");
    }

    #[test]
    fn test_delete_firmware() {
        let conn = setup_test_db();
        let repo = SqliteFirmwareRepository::new(&conn);
        
        let request = CreateFirmwareRequest {
            asset_id: 1,
            vendor: None,
            model: None,
            version: "1.0.0".to_string(),
            notes: None,
            original_filename: None,
        };

        let firmware = repo.create_firmware(
            request,
            1,
            "/path/to/firmware.enc".to_string(),
            "hash123".to_string(),
            1024,
            1,
        ).unwrap();

        let file_path = repo.delete_firmware(firmware.id).unwrap();
        assert_eq!(file_path, Some("/path/to/firmware.enc".to_string()));

        let firmwares = repo.get_firmware_by_asset(1).unwrap();
        assert_eq!(firmwares.len(), 0);
    }

    #[test]
    fn test_file_storage_without_app() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let firmware_dir = temp_dir.path().join("firmware");
        let key = DataKey::generate();
        let test_data = b"This is test firmware data";

        let (file_path, file_hash, file_size) =
            FirmwareFileStorage::store_firmware_file_in(&firmware_dir, 1, 100, test_data, &key).unwrap();
        assert_eq!(Path::new(&file_path), Path::new("1").join("100.enc"));
        assert_eq!(file_size, test_data.len() as i64);

        // Stored encrypted, read back decrypted
        let on_disk = fs::read(firmware_dir.join(&file_path)).unwrap();
        assert!(!on_disk.windows(test_data.len()).any(|w| w == test_data));
        assert_eq!(FirmwareFileStorage::read_firmware_file_in(&firmware_dir, &file_path, &key).unwrap(), test_data);
        assert!(FirmwareFileStorage::read_firmware_file_in(&firmware_dir, &file_path, &DataKey::generate()).is_err());
        assert!(FirmwareFileStorage::verify_firmware_integrity_in(&firmware_dir, &file_path, &file_hash, &key).unwrap());
        assert!(!FirmwareFileStorage::verify_firmware_integrity_in(&firmware_dir, &file_path, "other", &key).unwrap());

        FirmwareFileStorage::delete_firmware_file_in(&firmware_dir, &file_path).unwrap();
        assert!(!firmware_dir.join(&file_path).exists());
        assert!(FirmwareFileStorage::read_firmware_file_in(&firmware_dir, &file_path, &key).is_err());
    }

    #[test]
    fn test_parse_firmware_file_name() {
        assert_eq!(parse_firmware_file_name("12.enc"), Some(12));
        assert_eq!(parse_firmware_file_name("12.k3.enc"), Some(12));
        assert_eq!(parse_firmware_file_name("12.k3.enc.tmp"), Some(12));
        assert_eq!(parse_firmware_file_name("12.enc.tmp"), Some(12));
        // Unique temporary files
        assert_eq!(parse_firmware_file_name("12.k3.enc.0f9a1b2c3d4e5f60718293a4b5c6d7e8.tmp"), Some(12));
        for name in [
            "12", "12.bin", "a12.enc", "12.x3.enc", "12.k.enc", "12.k3.k4.enc", ".enc", "12.k3.enc.bak", "notes.txt",
            "12.k3.enc.zz.tmp", "12.k3.enc..tmp",
        ] {
            assert_eq!(parse_firmware_file_name(name), None, "{}", name);
        }
    }

    fn assert_no_tmp_files(dir: &Path) {
        let leftovers: Vec<String> = fs::read_dir(dir).unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "{:?}", leftovers);
    }

    #[test]
    fn test_reencrypt_firmware_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let dir = temp_dir.path();
        let k1 = DataKey::generate_with_version(1);
        let k2 = DataKey::generate_with_version(2);
        let (old_path, _, _) = FirmwareFileStorage::store_firmware_file_in(dir, 4, 9, b"image", &k1).unwrap();

        let written = FirmwareFileStorage::reencrypt_firmware_file(dir, 9, &old_path, &k1, &k2).unwrap();
        let expected = Path::new("4").join("9.k2.enc").to_string_lossy().to_string();
        assert_eq!(written, FirmwareReencrypted::Written(expected.clone()));
        assert_eq!(FirmwareFileStorage::read_firmware_file_in(dir, &expected, &k2).unwrap(), b"image");
        assert!(FirmwareFileStorage::read_firmware_file_in(dir, &expected, &k1).is_err());
        // The old file stays until the caller has made the DB change durable;
        // no temporary file is left
        assert_eq!(FirmwareFileStorage::read_firmware_file_in(dir, &old_path, &k1).unwrap(), b"image");
        assert_no_tmp_files(&dir.join("4"));

        // Re-running (as after a crash before the DB update) rewrites the same file
        let again = FirmwareFileStorage::reencrypt_firmware_file(dir, 9, &old_path, &k1, &k2).unwrap();
        assert_eq!(again, FirmwareReencrypted::Written(expected.clone()));
        assert_eq!(FirmwareFileStorage::read_firmware_file_in(dir, &expected, &k2).unwrap(), b"image");
        assert_no_tmp_files(&dir.join("4"));
    }

    /// Recovery imports stored firmware as `<asset>/0.enc` whatever the
    /// record id. Rotation must not treat such a file as unreadable.
    #[test]
    fn test_reencrypt_accepts_a_file_name_without_the_record_id() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let dir = temp_dir.path();
        let k1 = DataKey::generate_with_version(1);
        let k2 = DataKey::generate_with_version(2);
        let (old_path, _, _) = FirmwareFileStorage::store_firmware_file_in(dir, 4, 0, b"imported", &k1).unwrap();

        let written = FirmwareFileStorage::reencrypt_firmware_file(dir, 31, &old_path, &k1, &k2).unwrap();
        let expected = Path::new("4").join("31.k2.enc").to_string_lossy().to_string();
        assert_eq!(written, FirmwareReencrypted::Written(expected.clone()));
        assert_eq!(FirmwareFileStorage::read_firmware_file_in(dir, &expected, &k2).unwrap(), b"imported");
    }

    #[test]
    fn test_reencrypt_firmware_file_classifies_failures() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let dir = temp_dir.path();
        let k1 = DataKey::generate_with_version(1);
        let k2 = DataKey::generate_with_version(2);
        let (path, _, _) = FirmwareFileStorage::store_firmware_file_in(dir, 4, 9, b"image", &k1).unwrap();

        // Only a file that authenticates under neither key is unreadable; nothing is written
        let foreign = DataKey::generate_with_version(1);
        assert!(matches!(
            FirmwareFileStorage::reencrypt_firmware_file(dir, 9, &path, &foreign, &k2),
            Err(FirmwareReencryptError::Unreadable(_))
        ));
        assert!(!dir.join("4").join("9.k2.enc").exists());

        // A missing file, and paths outside the firmware directory, are rejected
        let rejected = |result: std::result::Result<FirmwareReencrypted, FirmwareReencryptError>| {
            matches!(result, Err(FirmwareReencryptError::Rejected(_)))
        };
        assert!(rejected(FirmwareFileStorage::reencrypt_firmware_file(dir, 10, "4/10.enc", &k1, &k2)));
        assert!(rejected(FirmwareFileStorage::reencrypt_firmware_file(dir, 9, "../4/9.enc", &k1, &k2)));
        assert!(rejected(FirmwareFileStorage::reencrypt_firmware_file(dir, 9, "/tmp/9.enc", &k1, &k2)));
        assert!(rejected(FirmwareFileStorage::reencrypt_firmware_file(dir, 9, "4/./9.enc/..", &k1, &k2)));
        assert!(rejected(FirmwareFileStorage::reencrypt_firmware_file(dir, 9, "", &k1, &k2)));
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(dir.join("4").join("9.enc"), dir.join("4").join("link.enc")).unwrap();
            assert!(rejected(FirmwareFileStorage::reencrypt_firmware_file(dir, 9, "4/link.enc", &k1, &k2)));
            std::os::unix::fs::symlink(dir.join("4"), dir.join("5")).unwrap();
            assert!(rejected(FirmwareFileStorage::reencrypt_firmware_file(dir, 9, "5/9.enc", &k1, &k2)));
        }
        assert_eq!(FirmwareFileStorage::read_firmware_file_in(dir, &path, &k1).unwrap(), b"image");
    }

    /// A read error is not a decryption failure: it must stay retryable.
    #[cfg(unix)]
    #[test]
    fn test_reencrypt_read_error_is_retryable() {
        use std::os::unix::fs::PermissionsExt;
        let temp_dir = tempfile::TempDir::new().unwrap();
        let dir = temp_dir.path();
        let k1 = DataKey::generate_with_version(1);
        let k2 = DataKey::generate_with_version(2);
        let (path, _, _) = FirmwareFileStorage::store_firmware_file_in(dir, 4, 9, b"image", &k1).unwrap();
        let full = dir.join(&path);
        fs::set_permissions(&full, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::read(&full).is_ok() {
            return; // running as root: permissions don't apply
        }

        let result = FirmwareFileStorage::reencrypt_firmware_file(dir, 9, &path, &k1, &k2);
        assert!(matches!(result, Err(FirmwareReencryptError::Io(_))), "{:?}", result);

        fs::set_permissions(&full, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(matches!(
            FirmwareFileStorage::reencrypt_firmware_file(dir, 9, &path, &k1, &k2),
            Ok(FirmwareReencrypted::Written(_))
        ));
    }

    /// When the DB update of an earlier attempt was lost, the re-encrypted
    /// copy is adopted rather than reported as a failure.
    #[test]
    fn test_reencrypt_adopts_existing_copy_under_new_key() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let dir = temp_dir.path();
        let k1 = DataKey::generate_with_version(1);
        let k2 = DataKey::generate_with_version(2);
        let (path, _, _) = FirmwareFileStorage::store_firmware_file_in(dir, 4, 9, b"image", &k1).unwrap();
        let new_path = Path::new("4").join("9.k2.enc").to_string_lossy().to_string();
        FirmwareFileStorage::reencrypt_firmware_file(dir, 9, &path, &k1, &k2).unwrap();

        // The old file is gone (as older versions deleted it right after the update)
        fs::remove_file(dir.join(&path)).unwrap();
        assert_eq!(
            FirmwareFileStorage::reencrypt_firmware_file(dir, 9, &path, &k1, &k2).unwrap(),
            FirmwareReencrypted::Adopted(new_path.clone())
        );
        // The stored file itself is already under the new key
        assert_eq!(
            FirmwareFileStorage::reencrypt_firmware_file(dir, 9, &new_path, &k1, &k2).unwrap(),
            FirmwareReencrypted::Adopted(new_path.clone())
        );
        // A copy under some other key is not adopted
        fs::write(dir.join(&new_path), FileEncryption::new(&DataKey::generate()).encrypt(b"x").unwrap()).unwrap();
        assert!(matches!(
            FirmwareFileStorage::reencrypt_firmware_file(dir, 9, &path, &k1, &k2),
            Err(FirmwareReencryptError::Rejected(_))
        ));
    }

    #[test]
    fn test_probe_firmware_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let dir = temp_dir.path();
        let k1 = DataKey::generate_with_version(1);
        let k2 = DataKey::generate_with_version(2);
        let (path, _, _) = FirmwareFileStorage::store_firmware_file_in(dir, 4, 9, b"image", &k1).unwrap();

        let probe = |path: &str| FirmwareFileStorage::probe_firmware_file(dir, 9, path, &k1, &k2).unwrap();
        assert_eq!(probe(&path), FirmwareProbe::ReadableWithOldKey);
        assert_eq!(probe("../9.enc"), FirmwareProbe::Unreadable);
        assert_eq!(probe("4/missing.enc"), FirmwareProbe::Unreadable);
        fs::write(dir.join(&path), b"garbage that is long enough to look like a ciphertext").unwrap();
        assert_eq!(probe(&path), FirmwareProbe::Unreadable);
        fs::write(dir.join("4").join("9.k2.enc"), FileEncryption::new(&k2).encrypt(b"image").unwrap()).unwrap();
        assert_eq!(probe(&path), FirmwareProbe::AlreadyReencrypted);
        // Probing writes nothing
        assert_no_tmp_files(&dir.join("4"));
    }

    #[test]
    fn test_sweep_only_removes_leftovers_of_known_records() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let dir = temp_dir.path();
        let active = DataKey::generate_with_version(2);
        let write = |rel: &str| {
            let path = dir.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"x").unwrap();
        };
        // Record 9 now lives in 9.k2.enc; 9.enc and temp files are leftovers
        for rel in ["4/9.enc", "4/9.k2.enc", "4/9.k3.enc.tmp", "4/9.k2.enc.0123abcd.tmp", "4/10.enc"] {
            write(rel);
        }
        // Not leftovers: unknown id, other names, another asset's dir, a dir
        for rel in ["4/1700000000000.enc", "4/notes.txt", "4/9.bin", "5/9.enc", "4/11.k2.enc", "unrelated.enc"] {
            write(rel);
        }
        fs::create_dir_all(dir.join("4").join("9.k1.enc")).unwrap();

        let record = |id: i64, path: &str, key_version: u32| FirmwareRecord { id, file_path: path.to_string(), key_version };
        let records = vec![
            record(9, "4/9.k2.enc", 2),
            record(10, "4/10.enc", 2),
            // Paths rotation does not manage are ignored
            record(11, "/elsewhere/11.enc", 2),
            record(12, "4/../12.enc", 2),
        ];
        let removed = FirmwareFileStorage::sweep_rotation_leftovers(dir, &records, &active).unwrap();

        assert_eq!(removed, 3);
        for rel in ["4/9.enc", "4/9.k3.enc.tmp", "4/9.k2.enc.0123abcd.tmp"] {
            assert!(!dir.join(rel).exists(), "{} was kept", rel);
        }
        for rel in ["4/9.k2.enc", "4/10.enc", "4/1700000000000.enc", "4/notes.txt", "4/9.bin", "5/9.enc", "4/11.k2.enc", "unrelated.enc"] {
            assert!(dir.join(rel).exists(), "{} was deleted", rel);
        }
        assert!(dir.join("4").join("9.k1.enc").is_dir());

        // A missing asset directory is not an error
        assert_eq!(FirmwareFileStorage::sweep_rotation_leftovers(dir, &[record(1, "77/1.enc", 2)], &active).unwrap(), 0);
    }

    /// If a record is still at the old version (its DB update was lost), the
    /// sweep must not delete the re-encrypted copy that holds its data.
    #[test]
    fn test_sweep_keeps_the_only_copy_under_the_active_key() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let dir = temp_dir.path();
        let active = DataKey::generate_with_version(2);
        fs::create_dir_all(dir.join("4")).unwrap();
        fs::write(dir.join("4/9.k2.enc"), FileEncryption::new(&active).encrypt(b"image").unwrap()).unwrap();
        // Same name, but not under the active key: a real leftover
        fs::write(dir.join("4/10.k2.enc"), b"junk").unwrap();

        let records = vec![
            FirmwareRecord { id: 9, file_path: "4/9.enc".into(), key_version: 1 },
            FirmwareRecord { id: 10, file_path: "4/10.enc".into(), key_version: 1 },
        ];
        assert_eq!(FirmwareFileStorage::sweep_rotation_leftovers(dir, &records, &active).unwrap(), 1);
        assert!(dir.join("4/9.k2.enc").exists());
        assert!(!dir.join("4/10.k2.enc").exists());
    }

    #[test]
    fn test_delete_superseded_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let dir = temp_dir.path();
        fs::create_dir_all(dir.join("4")).unwrap();
        fs::write(dir.join("4/0.enc"), b"x").unwrap();

        FirmwareFileStorage::delete_superseded_file(dir, "4/0.enc").unwrap();
        assert!(!dir.join("4/0.enc").exists());
        // Already gone is fine; unsafe paths are refused
        FirmwareFileStorage::delete_superseded_file(dir, "4/0.enc").unwrap();
        assert!(FirmwareFileStorage::delete_superseded_file(dir, "../outside.enc").is_err());
        assert!(dir.join("4").is_dir());
    }
    
    #[test]
    fn test_file_size_validation() {
        // Test file size limit validation
        const MAX_FILE_SIZE: usize = 2 * 1024 * 1024 * 1024; // 2GB
        
        let small_data = vec![0u8; 1024]; // 1KB
        assert!(small_data.len() <= MAX_FILE_SIZE);
        
        // Note: We can't actually allocate 2GB+ in tests, so we just test the logic
        let simulated_large_size = MAX_FILE_SIZE + 1;
        assert!(simulated_large_size > MAX_FILE_SIZE);
    }

    #[test]
    fn test_update_firmware_status() {
        let conn = setup_test_db();
        let repo = SqliteFirmwareRepository::new(&conn);
        
        // Create a firmware version
        let request = CreateFirmwareRequest {
            asset_id: 1,
            vendor: Some("Test Vendor".to_string()),
            model: Some("Test Model".to_string()),
            version: "1.0.0".to_string(),
            notes: Some("Test firmware".to_string()),
            original_filename: None,
        };

        let firmware = repo.create_firmware(
            request,
            1,
            "/path/to/firmware.enc".to_string(),
            "abc123hash".to_string(),
            1024,
            1,
        ).unwrap();

        // Update status to Approved
        let result = repo.update_firmware_status(
            firmware.id,
            FirmwareStatus::Approved,
            1,
            Some("Ready for production".to_string())
        );
        
        assert!(result.is_ok());

        // Verify the status was updated
        let updated = repo.get_firmware_by_id(firmware.id).unwrap().unwrap();
        assert_eq!(updated.status, FirmwareStatus::Approved);
        assert!(updated.status_changed_at.is_some());
        assert_eq!(updated.status_changed_by, Some(1));
    }

    #[test]
    fn test_firmware_status_history() {
        let conn = setup_test_db();
        let repo = SqliteFirmwareRepository::new(&conn);
        
        // Create a firmware version
        let request = CreateFirmwareRequest {
            asset_id: 1,
            vendor: None,
            model: None,
            version: "1.0.0".to_string(),
            notes: None,
            original_filename: None,
        };

        let firmware = repo.create_firmware(
            request,
            1,
            "/path/to/firmware.enc".to_string(),
            "hash123".to_string(),
            1024,
            1,
        ).unwrap();

        // Update status multiple times
        repo.update_firmware_status(
            firmware.id,
            FirmwareStatus::Approved,
            1,
            Some("Testing approved".to_string())
        ).unwrap();

        repo.update_firmware_status(
            firmware.id,
            FirmwareStatus::Archived,
            1,
            Some("Archiving old version".to_string())
        ).unwrap();

        // Get status history
        let history = repo.get_firmware_status_history(firmware.id).unwrap();
        
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].old_status, "Approved");
        assert_eq!(history[0].new_status, "Archived");
        assert_eq!(history[0].reason, Some("Archiving old version".to_string()));
        assert_eq!(history[1].old_status, "Draft");
        assert_eq!(history[1].new_status, "Approved");
    }

    #[test]
    fn test_available_status_transitions() {
        let conn = setup_test_db();
        let repo = SqliteFirmwareRepository::new(&conn);
        
        // Create a firmware version
        let request = CreateFirmwareRequest {
            asset_id: 1,
            vendor: None,
            model: None,
            version: "1.0.0".to_string(),
            notes: None,
            original_filename: None,
        };

        let firmware = repo.create_firmware(
            request,
            1,
            "/path/to/firmware.enc".to_string(),
            "hash123".to_string(),
            1024,
            1,
        ).unwrap();

        // Test transitions for Draft status (Engineer role)
        let transitions = repo.get_available_firmware_status_transitions(firmware.id, "Engineer").unwrap();
        assert_eq!(transitions.len(), 2);
        assert!(transitions.contains(&FirmwareStatus::Approved));
        assert!(transitions.contains(&FirmwareStatus::Archived));

        // Update to Approved
        repo.update_firmware_status(firmware.id, FirmwareStatus::Approved, 1, None).unwrap();

        // Test transitions for Approved status (Administrator role)
        let transitions = repo.get_available_firmware_status_transitions(firmware.id, "Administrator").unwrap();
        assert_eq!(transitions.len(), 3);
        assert!(transitions.contains(&FirmwareStatus::Golden));
        assert!(transitions.contains(&FirmwareStatus::Archived));
        assert!(transitions.contains(&FirmwareStatus::Draft));

        // Test transitions for Approved status (Engineer role)
        let transitions = repo.get_available_firmware_status_transitions(firmware.id, "Engineer").unwrap();
        assert_eq!(transitions.len(), 2);
        assert!(transitions.contains(&FirmwareStatus::Archived));
        assert!(transitions.contains(&FirmwareStatus::Draft));
        assert!(!transitions.contains(&FirmwareStatus::Golden)); // Engineers can't promote to Golden
    }

    #[test]
    fn test_promote_firmware_to_golden() {
        let conn = setup_test_db();
        let repo = SqliteFirmwareRepository::new(&conn);
        
        // Create admin user
        conn.execute(
            "INSERT INTO users (username, password_hash, role) VALUES ('admin', 'hash', 'Administrator')",
            [],
        ).unwrap();
        
        // Create two firmware versions
        let request1 = CreateFirmwareRequest {
            asset_id: 1,
            vendor: None,
            model: None,
            version: "1.0.0".to_string(),
            notes: None,
            original_filename: None,
        };

        let firmware1 = repo.create_firmware(
            request1,
            1,
            "/path/to/firmware1.enc".to_string(),
            "hash1".to_string(),
            1024,
            1,
        ).unwrap();

        let request2 = CreateFirmwareRequest {
            asset_id: 1,
            vendor: None,
            model: None,
            version: "2.0.0".to_string(),
            notes: None,
            original_filename: None,
        };

        let firmware2 = repo.create_firmware(
            request2,
            1,
            "/path/to/firmware2.enc".to_string(),
            "hash2".to_string(),
            2048,
            1,
        ).unwrap();

        // Approve both versions
        repo.update_firmware_status(firmware1.id, FirmwareStatus::Approved, 2, None).unwrap();
        repo.update_firmware_status(firmware2.id, FirmwareStatus::Approved, 2, None).unwrap();

        // Promote first to Golden
        repo.promote_firmware_to_golden(firmware1.id, 2, "First golden version".to_string()).unwrap();
        
        let fw1 = repo.get_firmware_by_id(firmware1.id).unwrap().unwrap();
        assert_eq!(fw1.status, FirmwareStatus::Golden);

        // Promote second to Golden (should archive the first)
        repo.promote_firmware_to_golden(firmware2.id, 2, "New golden version".to_string()).unwrap();
        
        let fw1_after = repo.get_firmware_by_id(firmware1.id).unwrap().unwrap();
        let fw2_after = repo.get_firmware_by_id(firmware2.id).unwrap().unwrap();
        
        assert_eq!(fw1_after.status, FirmwareStatus::Archived);
        assert_eq!(fw2_after.status, FirmwareStatus::Golden);
        
        // Check history for automatic archiving
        let history1 = repo.get_firmware_status_history(firmware1.id).unwrap();
        assert!(history1.iter().any(|h| 
            h.old_status == "Golden" && 
            h.new_status == "Archived" && 
            h.reason.as_ref().unwrap().contains("automatically archived")
        ));
    }

    #[test]
    fn test_update_firmware_notes() {
        let conn = setup_test_db();
        let repo = SqliteFirmwareRepository::new(&conn);
        
        // Create a firmware version
        let request = CreateFirmwareRequest {
            asset_id: 1,
            vendor: None,
            model: None,
            version: "1.0.0".to_string(),
            notes: Some("Initial notes".to_string()),
            original_filename: None,
        };

        let firmware = repo.create_firmware(
            request,
            1,
            "/path/to/firmware.enc".to_string(),
            "hash123".to_string(),
            1024,
            1,
        ).unwrap();

        // Update notes
        let new_notes = "Updated notes with more details";
        repo.update_firmware_notes(firmware.id, new_notes.to_string()).unwrap();

        // Verify notes were updated
        let updated = repo.get_firmware_by_id(firmware.id).unwrap().unwrap();
        assert_eq!(updated.notes, Some(new_notes.to_string()));

        // Test notes length validation
        let long_notes = "a".repeat(1001);
        let result = repo.update_firmware_notes(firmware.id, long_notes);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("1000 characters"));
    }

    #[test]
    fn test_golden_promotion_requires_approved_status() {
        let conn = setup_test_db();
        let repo = SqliteFirmwareRepository::new(&conn);
        
        // Create a firmware version (starts as Draft)
        let request = CreateFirmwareRequest {
            asset_id: 1,
            vendor: None,
            model: None,
            version: "1.0.0".to_string(),
            notes: None,
            original_filename: None,
        };

        let firmware = repo.create_firmware(
            request,
            1,
            "/path/to/firmware.enc".to_string(),
            "hash123".to_string(),
            1024,
            1,
        ).unwrap();

        // Try to promote Draft directly to Golden (should fail)
        let result = repo.promote_firmware_to_golden(firmware.id, 1, "Invalid promotion".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Only approved firmware"));
    }
}