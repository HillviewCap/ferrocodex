# Design: data-key rotation with per-user X25519 keypairs

Status: approved (Option A). Chunks 0-5 are implemented on `fix/vault-security-bugs`; chunk 6 (frontend) and chunk 7 (documentation) follow. See "Deviations from this design" at the end.

Paths are relative to `apps/desktop/src-tauri/src` unless they start with `apps/` or `src/types`.

## 0. Things found in the code that affect this plan

1. **Deactivation does not revoke the key.** `deactivate_user` (`commands/user_commands.rs:165-235`) only runs `UPDATE users SET is_active = 0` (`users/mod.rs:210-216`). The `ON DELETE CASCADE` on `data_key_wraps` (`keystore/mod.rs:53`) never fires, because no code path deletes a user row. The deactivated user's wrap stays in the DB and still opens the key.
2. **Deactivation does not end live sessions.** The in-memory session keeps its `Arc<DataKey>` for up to 24h (`auth/mod.rs:60`, `:68-91`).
3. **There is only one password-reset path, and it is safe.** `reset_password_with_recovery_key` calls `grant_user`, which goes through `store_wrap`; that deletes the old wrap and inserts the new one (`keystore/mod.rs:75-98`). There is no admin password reset and no user deletion. A deactivated user can't reach recovery reset, because `find_by_username` filters on `is_active = 1` (`users/mod.rs:168`).
4. **Bug: `run_data_migrations` returns early** (`database/mod.rs:169-173`). It returns `Ok(())` as soon as the first (firmware) migration is marked applied, so any migration appended later never runs on an existing install. Fix this before adding the migrations below.
5. **Password history holds no data-key ciphertext.** `password_history.password_hash` and `password_rotation_history.old_password_hash` are bcrypt hashes. One exception: `vault/rotation.rs:179-180` stores the **raw ciphertext** in `old_password_hash` if bcrypt fails. Remove that fallback, and redact existing rows that don't start with `$2`.
6. **These are the only data-key ciphertexts:**
   - `configuration_versions.file_content` (BLOB): written at `configurations/mod.rs:349/353`, read at `:403`, copied as-is at `:258-283` (via `branches/mod.rs:332`).
   - `vault_secrets.encrypted_value` (base64 TEXT): written at `vault/mod.rs:1278/1284`, `:1346` (update_secret), `:1447` (import, no key passed), `:1529/1534`, `:1616-1634`, and `vault/rotation.rs:203`. Decrypted at `commands/vault_commands.rs:328` and `:1247`, and in `vault/export.rs:69`.
   - `standalone_credentials.encrypted_data`: `vault/mod.rs:1705/1710`, `:1801-1807`.
   - Firmware files `<app_data>/firmware/<asset>/<id>.enc`: whole-file AES-GCM, up to 2 GB, held in memory (`firmware/mod.rs:519-594`). The path is stored in `firmware_versions.file_path`. Only the upload code builds the `{id}.enc` name (`commands/firmware_commands.rs:380`, `firmware/mod.rs:540`); every reader uses the stored `file_path`.
7. **Queued firmware-analysis jobs hold a copy of the key** (`firmware_analysis/queue.rs:25`).
8. **SQLite runs in WAL mode without `secure_delete`** (`database/mod.rs:52-56`). Deleted wraps and old ciphertext pages can stay in free pages or in the WAL.

## 1. Threat model

**What rotation protects against**

- **Old key + current DB.** Once rotation finishes, someone holding the old data key K1 can't read anything in the current DB file or the firmware directory. That applies however they got K1: an old DB copy plus their password, the old recovery code plus an old copy, or process memory. Data that existed before rotation is covered too, because it was re-encrypted, unless they kept their own copy of that ciphertext.
- **Old recovery code.** It no longer opens anything in the current DB.

**What wrap deletion alone protects against (chunk 1, no rotation needed)**

- **Deactivated user's password + current DB.** Once their wrap is deleted, their password opens nothing in the current DB. This holds only if `secure_delete` is on and the WAL has been checkpointed.

**What neither protects against**

- **Copies taken before rotation.** Any DB copy or backup taken before rotation (with firmware files, if those were copied too) stays readable forever with K1. That includes a pre-deactivation copy plus that user's password, or plus the old recovery code. Rotation cannot reach backward.
- **Plaintext that has left the app.** Exported configs, firmware exports, recovery bundles, and vault exports whose passphrase is known.
- **Columns that were never encrypted.** Asset and vault names, labels, metadata, the audit log, firmware-analysis results. The DB as a whole is not encrypted.
- **Trusted parties turning hostile.** A malicious or compromised active administrator, or malware on the host while a session is unlocked.
- **Leftover blocks on disk.** Ciphertext remnants on SSD or filesystem blocks after files are rewritten or deleted.
- **Tampering by someone who can write to the DB.** Without AAD, ciphertexts can be swapped between rows (see §5).

## 2. Immediate cheap mitigation (chunk 1)

- `KeyStoreRepository::revoke_user(user_id)`: deletes the user's rows in `data_key_wraps`, and (after chunk 4) in `data_key_public_wraps` and `user_keypairs`.
- `deactivate_user`:
  - Revoke in the same transaction as `user_repo.deactivate_user`.
  - Then call the new `SessionManager::invalidate_user_sessions(user_id)` and run `PRAGMA wal_checkpoint(TRUNCATE)`.
- One-off data migration: `DELETE FROM data_key_wraps WHERE user_id IN (SELECT id FROM users WHERE is_active = 0)`.
- `reactivate_user` must re-grant access, but the admin doesn't know the user's password. It takes a `new_password` (temporary), sets the bcrypt hash, and grants a fresh wrap.
- New command `admin_reset_user_password(token, user_id, new_password)`:
  - Admin only, and not on yourself.
  - Same logic as reactivation.
  - This is also how users who lose access after a rotation get it back.

## 3. Options

- **A (chosen): per-user X25519 keypairs.**
  - Each user has a keypair. The private key is wrapped under the user's password; the public key is stored.
  - The data key is wrapped to each user's public key, so an admin can wrap a new key for every active user without knowing their passwords.
- **B: pending re-wrap / escrow handoff.** Rotation deletes every wrap except the admin's.
  - Other users then need an admin password reset or a second, per-user secret handed to them.
  - In practice that means "rotation resets everyone's password".
  - Rejected, except as the fallback for users who don't have a keypair yet.
- **C: envelope encryption** (a data key per object, wrapped by a key-encryption key).
  - Rotation only re-wraps the small per-object keys, which is fast with large firmware.
  - It doesn't protect against an old DB copy, which still holds the old per-object wraps.
  - So it's a performance option, not a security one. Revisit with a chunked v2 firmware format.

## 4. Implementation plan for Option A

### Cryptographic choices

- **Crates:** add `x25519-dalek = { version = "2", features = ["static_secrets", "zeroize"] }` and `hkdf = "0.12"`. `sha2 0.10`, `aes-gcm 0.10`, `argon2` and `zeroize` are already present.
  - **RNG:** x25519-dalek 2 uses `rand_core 0.6`, which matches `aes_gcm::aead::OsRng`. Use that, not `rand 0.10`.
  - **Why not `crypto_box`:** sealed boxes take no AAD, and it would add a second AEAD.
  - The construction below has the same shape as HPKE base mode with DHKEM(X25519) + HKDF-SHA256 + AES-256-GCM.

**Sealing the data key to a user's public key**

1. Generate an ephemeral keypair: `EphemeralSecret::random_from_rng(OsRng)`.
2. Compute the shared secret `ss = eph.diffie_hellman(recipient_pub)`. Reject it if `!ss.was_contributory()`.
3. Derive a 32-byte key `okm = HKDF-SHA256(salt = eph_pub || recipient_pub, ikm = ss, info = "ferrocodex-data-key-to-user-v1")`.
4. Encrypt the data key with AES-256-GCM under `okm`, using `aad = "ferrocodex-pubwrap-v1" || user_id (i64 BE) || key_version (u32 BE)`.

**Wrapping the private key**

- Use the existing Argon2id `WrappedKey` (`encryption/mod.rs:154-178`), generalized to `seal_bytes` / `open_bytes` with an explicit AAD `"ferrocodex-user-private-key-v1"`.
- Keep the old `seal` / `open` with `KEY_WRAP_AAD` for legacy rows.

### Session gate

- **Why:** re-encryption can take minutes (several GB of firmware), and reads must never see a mix of key versions.
- **Gate behavior:** while rotation runs, `SessionToken::data_key()` returns `Err("Encryption key rotation in progress; an administrator must finish it")`.
- **Mechanism:** a shared `Arc<AtomicBool>` held by `SessionManager` and cloned into each token (`#[serde(skip)]`).
- **Exception:** the rotation commands read the key through a new `rotation_key()` instead.
- **Set and cleared:** set by `begin`, and at login if a `retiring` key exists. Cleared by `finalize`.
- **Benefit:** none of the 23 `session.data_key()?` call sites need key-version dispatch, and nothing writes under the old key during rotation.

### Chunks (each builds and passes tests on its own)

**Chunk 0: prerequisites**

- `database/mod.rs`:
  - Restructure `run_data_migrations` so each migration is an independent `if get_metadata(key).is_none() { ...; set_metadata }` block, removing the early return.
  - Add `PRAGMA secure_delete = ON;`.
  - Add `Database::checkpoint_wal()` (`PRAGMA wal_checkpoint(TRUNCATE)`).
- `vault/rotation.rs:179-180`: propagate the bcrypt error instead of falling back to the ciphertext. Migration `redact_rotation_history_20261019` sets `old_password_hash = 'redacted'` where it is `NOT LIKE '$2%'`.
- `auth/mod.rs`: add `invalidate_user_sessions(user_id)`, `invalidate_all_except(token)`, and `replace_session_key(token, DataKey)`.
- Tests:
  - On an old-style DB where the firmware migration is already applied, a newer migration still runs.
  - Unit tests for the new session methods.

**Chunk 1: revoke on deactivation (§2)**

- `keystore/mod.rs`: add `revoke_user`.
- `commands/user_commands.rs`:
  - `deactivate_user`: revoke, invalidate sessions, checkpoint the WAL.
  - `reactivate_user(token, user_id, new_password, ...)`: set the password and grant a wrap.
  - New `admin_reset_user_password` command, audited as `UserPasswordChanged`.
- Data migration `revoke_inactive_user_wraps_20261019`.
- Register the new command in `generate_handler!` in `lib.rs`.
- Frontend:
  - `src/store/userManagement.ts`: `reactivateUser(token, userId, newPassword)` and `resetUserPassword`.
  - `UserManagement.tsx`: temporary-password modal for Reactivate and a new "Reset password" row action, plus updated deactivate confirmation text.
- Tests:
  - After deactivation, unlocking with the old password returns `None` and the user's sessions are gone.
  - After reactivation, the user gets the same key.
  - An engineer can't call the admin reset.

**Chunk 2: key versioning (no behavior change)**

- New `data_keys` table:

  ```sql
  CREATE TABLE IF NOT EXISTS data_keys (
    version INTEGER PRIMARY KEY,
    status TEXT NOT NULL CHECK(status IN ('active','retiring','retired')),
    wrapped_by_active BLOB,
    created_by INTEGER REFERENCES users(id),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    retired_at DATETIME);
  CREATE UNIQUE INDEX IF NOT EXISTS idx_data_keys_active ON data_keys(status) WHERE status = 'active';
  ```

- Add `key_version INTEGER NOT NULL DEFAULT 1` to `data_key_wraps`, `configuration_versions`, `vault_secrets`, `standalone_credentials`, and `firmware_versions`:
  - In each `CREATE TABLE`, for new installs.
  - Through an `ALTER TABLE` migration `key_version_columns_20261019`, guarded by `pragma_table_info`, for existing installs.
  - The migration seeds `data_keys(1, 'active')` when wraps exist.
- `DataKey` carries `version: u32`. Unwrapping sets it from the row's `key_version`.
- Every writer listed in §0.6 sets `key_version = key.version()`:
  - `duplicate_configuration_version` copies the source row's version.
  - `import_vault` and `update_secret` gain a `key_version` parameter.

**Chunk 3: cryptographic primitives (`encryption/mod.rs`)**

- New types:
  - `UserKeyPair { public: [u8; 32], secret: StaticSecret }` with `generate()`, `seal_secret(password, params) -> WrappedKey`, and `open_secret(&WrappedKey, password)`.
  - `PublicKeyWrap { ephemeral_public: [u8; 32], ciphertext: Vec<u8> }`.
- New functions:
  - `seal_to_public_key(&DataKey, &[u8; 32], user_id) -> PublicKeyWrap`.
  - `open_with_secret(&PublicKeyWrap, &UserKeyPair, user_id, version) -> DataKey`.
  - `FileEncryption::reencrypt_in_place(from, to, &mut Vec<u8>)`, using `AeadInPlace`.
- Tests:
  - Round trips.
  - A wrong password fails.
  - A mismatched user_id or version in the AAD fails.
  - An all-zero public key is rejected.
  - The secret never appears in the wrapped private key.

**Chunk 4: keypairs and lazy migration at login**

- New tables:

  ```sql
  CREATE TABLE IF NOT EXISTS user_keypairs (
    user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    public_key BLOB NOT NULL CHECK(length(public_key) = 32),
    salt BLOB NOT NULL, wrapped_private_key BLOB NOT NULL,
    kdf_memory_kib INTEGER NOT NULL, kdf_iterations INTEGER NOT NULL, kdf_parallelism INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP);
  CREATE TABLE IF NOT EXISTS data_key_public_wraps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_version INTEGER NOT NULL REFERENCES data_keys(version),
    recipient_public_key BLOB NOT NULL, ephemeral_public_key BLOB NOT NULL, ciphertext BLOB NOT NULL,
    created_by INTEGER, created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, key_version));
  ```

- `data_key_wraps` keeps the `recovery` row, plus legacy `user` rows until those users migrate.
- `set_user_keys(user_id, password, &DataKey)`: generates a keypair, stores the wrapped private key and a public wrap for the active version, and deletes the user's legacy wrap. `grant_user` becomes this.
- `unlock_for_user`:
  1. If the user has a keypair, open the private key, then open the public wrap for the active version.
  2. Otherwise, if a legacy wrap exists, open it and call `set_user_keys` in one transaction.
  3. Otherwise return `None`.
- `revoke_user` also deletes the user's keypair and public wraps.
- Callers that switch to `set_user_keys`:
  - recovery reset
  - `create_engineer_user`
  - the chunk-1 reactivate and reset commands
- New audit event `UserEncryptionKeysUpgraded` (`SEC_003`).
- Tests:
  - A legacy-wrap user who logs in gets a keypair and the same key, and the legacy row is gone.
  - Logging in again doesn't re-migrate.
  - Recovery reset replaces the keypair.

**Chunk 5: rotation engine and commands (`keystore/rotation.rs`)**

New table:

```sql
CREATE TABLE IF NOT EXISTS key_rotation_failures (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  table_name TEXT NOT NULL, row_id INTEGER NOT NULL,
  from_version INTEGER NOT NULL, error TEXT NOT NULL,
  attempted_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(table_name, row_id, from_version));
```

`begin_data_key_rotation(token, password, acknowledge_users_without_keys)`:

- Admin only. The admin re-authenticates: bcrypt check, plus opening their own private key.
- Preflight refuses if:
  - a rotation is already in progress,
  - firmware-analysis jobs are pending or running,
  - some active users have no keypair and `acknowledge_users_without_keys` isn't set.
- In one transaction:
  1. Generate K2 at version v+1 and insert `data_keys(v+1, 'active')`. Mark row v `retiring`, with `wrapped_by_active = AES-GCM(K2, K1, aad = "ferrocodex-retiring-key-v1" || v || v+1)`.
  2. Seal K2 to every **active** user that has a keypair.
  3. Delete the version-v public wraps, the legacy user wraps, and all wraps belonging to inactive users.
  4. `replace_recovery_code(&K2)`.
  5. Write the audit event `DataKeyRotationStarted` (`SEC_004`).
- After commit:
  - Set the gate.
  - `replace_session_key(admin_token, K2)`.
  - `invalidate_all_except(admin_token)`.
  - Return `{recovery_code, to_version, users_without_access}`.
- **Crash recovery:** because K1 is stored wrapped under K2 (`wrapped_by_active`), rotation can resume after a crash. Any admin logs in, opens K2, and from it gets K1.

`continue_data_key_rotation(token)` processes one bounded batch and returns `KeyRotationProgress`:

- **DB rows:**
  - Select up to n rows `WHERE key_version = v AND id NOT IN failures`.
  - Decrypt with K1, encrypt with K2, and `UPDATE ... SET <col> = ?, key_version = v+1 WHERE id = ? AND key_version = v`.
  - One transaction per batch, so reruns are idempotent.
  - Rows that fail to decrypt go into `key_rotation_failures`.
- **Firmware (one file per call):**
  1. Read the row under the DB lock, then release it.
  2. In `spawn_blocking`: re-encrypt, write `<id>.k<v+1>.enc.tmp`, fsync, rename to `<id>.k<v+1>.enc`, fsync the directory (skip on Windows).
  3. Re-lock and run a conditional `UPDATE firmware_versions SET file_path = new, key_version = v+1 WHERE id = ? AND key_version = v AND file_path = old`.
  4. Delete the old file.
  - A crash before step 3 leaves the DB pointing at the old file, which is still valid.
  - A crash after step 3 leaves an orphaned old file, which the finalize sweep removes.
  - Refactor `FirmwareFileStorage` to use `&Path` internals so it can be tested without an `AppHandle`.

`finalize_data_key_rotation(token, discard_unreadable)`:

- Runs automatically when nothing remains and there are no failures. Otherwise it needs `discard_unreadable = true`.
- In one transaction:
  - Set row v to `retired` and clear `wrapped_by_active`.
  - Delete the version-v failure rows.
  - Write the audit event `DataKeyRotationCompleted` (`SEC_005`).
- Then:
  - Checkpoint the WAL.
  - Clear the gate.
  - Run the firmware sweep: delete files under known `<asset_id>/` directories that don't match the row's `file_path`.

`get_key_rotation_status(token)`:

- Returns versions, in-progress state, remaining counts per table, failures, users without a keypair or wrap, and the number of active analysis jobs.
- Callable by any user; the details are admin-only.

Other changes in this chunk:

- `LoginResponse` gains `key_rotation_in_progress: bool`.
- Register all four commands in `generate_handler!`.

**Chunk 6: frontend**

- Types in `src/types/keyRotation.ts`: `KeyRotationStatus`, `KeyRotationProgress`, `BeginKeyRotationResponse`, `KeyRotationFailure`.
- `LoginResponse` in `src/store/auth.ts` gains `key_rotation_in_progress`.
- New store `src/store/keyRotation.ts`: `fetchStatus`, `begin`, a `runUntilDone` loop, and `finalize`.
- `KeyRotationModal.tsx`, opened from a "Rotate Encryption Key" button next to Replace Recovery Key:
  - Preflight summary listing the users who will lose access.
  - Admin password field and an acknowledgement checkbox.
  - Shows the new recovery code through `RecoveryCodeModal`.
  - Progress bar, failure list, and "Finalize and discard unreadable items".
  - Reopening while a rotation is in progress resumes it.
- Dashboard banner while a rotation is in progress.
- "Encryption access" column in user management.
- Vitest coverage for the modal, the store, and the reset/reactivate password modal.

**Chunk 7: documentation**

- CLAUDE.md "Encryption" paragraph.
- `docs/security.rst`: the threat model from §1, including the warning about pre-rotation backups.
- `docs/troubleshooting.rst`.

### Rotation test matrix (chunk 5)

- **Full rotation:**
  - Seed configs, secrets, credentials and firmware.
  - Begin, process with batch size 1, finalize.
  - Everything decrypts with K2, nothing with K1, and the orphaned files are gone.
- **Crash simulation:**
  - Drop between batches, then resume from a fresh login.
  - Inject an error after the firmware file is written but before the DB update; resume still succeeds.
- **Corrupt row:** produces a failure entry, and finalize is refused without `discard_unreadable`.
- **User access:**
  - An engineer's `data_key()` errors during rotation.
  - After finalize, the engineer logs in again and gets K2.
  - A user without a keypair gets `None`; an admin reset restores their access.
- **Old copy** (documents the threat model):
  - The old password opens K1 from a pre-rotation copy, and K1 decrypts no current row.
  - The old recovery code fails on the current DB.
- **Refusals:** an engineer can't begin rotation, and begin refuses while analysis jobs are active.

## 5. AAD and SQLCipher (later)

**AAD**

- Rotation rewrites every ciphertext, so it is the natural moment to change the format. Add an `enc_format` column; format 2 is `0x02 || key_version || nonce || ct+tag`.
- `AAD = "ferrocodex/v2/" || table || "/" || column || "/" || blob_id || header`, where `blob_id` is a per-row random UUID. Don't bind to the AUTOINCREMENT id, which is unknown at insert time and changes when rows are duplicated.
- Firmware v2 could switch to chunked STREAM AEAD at the same time, removing the 2 GB in-memory limit.
- Ship it as an optional chunk 8 that hooks into the same rotation batch processing.

**SQLCipher**

- This would protect the plaintext metadata and the audit log. Its page key could be derived from the data key with HKDF.
- The blocker: the DB has to be opened before login, but users and wraps live in it. The fix is a small unencrypted `keystore.db` alongside an encrypted main DB.
- This is a separate project.

## 6. Deviations from this design

What shipped in chunks 0-5 differs from the plan above in these ways:

- **Analysis-job check.** `begin` takes a count of firmware-analysis jobs from an in-memory counter (`AnalysisQueue::active_jobs`), not from a database query. The count resets when the app restarts.
- **Auto-finalize happens inside `continue`.** The `continue_data_key_rotation` call that finds nothing left and no failures also finalizes the rotation and reports `finalized: true`. `finalize_data_key_rotation` is only needed to discard unreadable items.
- **Discard leaves unreadable rows in place.** Finalizing with `discard_unreadable` does not delete the failed rows from their tables. They stay at the old key version, unreadable for good. Only the `key_rotation_failures` entries are removed.
- **New version is `MAX(version) + 1`.** `begin` computes the new version from the highest row in `data_keys`, not from the active version.
- **Begin also deletes inactive users' keypairs.** Besides the old public wraps and the legacy user wraps, `begin` deletes `user_keypairs` rows of deactivated users.
- **Extra migration.** `key_version_indexes_20261020` exists (indexes on the `key_version` columns, used by the rotation queries) in addition to the migrations named in this document.
- **Blocked actions while rotating.** Create engineer, reactivate, and admin password reset call `session.data_key()`, which errors during rotation, so they are blocked. Password reset with the recovery code still works, as do deactivation and Replace Recovery Key (which uses `rotation_key()`).
- **`seal_to_public_key` takes an explicit `key_version`.** Its signature is `(&DataKey, &[u8; 32], user_id, key_version)`. The version is bound into the wrap's AAD and is stated by the caller instead of being read from the key.
- **`grant_user` was removed** in favour of `KeyStoreRepository::set_user_keys`.
- **`VaultSecret.key_version` is `#[serde(skip)]`.** The value is not serialized to the frontend and defaults to the initial version when deserialized, so the TypeScript type is unchanged.

Changes made after a security review of chunks 0-5:

- **What counts as a failure.** Only a firmware file or row that authenticates under neither the old nor the new key is recorded as unreadable ("could not be decrypted with the old key"). A missing firmware file and a stored path that is not a safe relative path inside the firmware directory (absolute, `..`, or through a symlink) are also recorded, as rejected. Read and write errors (permissions, a locked file, a full disk) are never recorded: `continue` returns an error and can be called again.
- **Firmware file names.** The stored file name no longer has to be `<id>.enc` (recovery imports used `0.enc`). The new file is always `<id>.k<v+1>.enc`, in the same directory as the old one, written through a uniquely named temporary file opened with `create_new`. Recovery imports now store `<asset>/<id>.enc` and record that path.
- **Retry.** New admin command `retry_key_rotation_failures(token) -> usize` deletes the failure rows of the rotation in progress, so the next `continue` calls try those items again. It returns how many were cleared.
- **Discard re-checks first.** `finalize_data_key_rotation` with `discard_unreadable` re-reads every failed item and refuses while any of them decrypts under the old key or is already under the new one ("Retry the failed items instead"), or can't be read for the check.
- **Serialized commands.** A process-wide `tokio::sync::Mutex` (`KeyRotationLockState`) is held for the whole of `begin`, `continue` (including the automatic finalize), `finalize`, and `retry`. A second `finalize` after the rotation finished returns `already_finalized: true` instead of an error, and retries the file cleanup.
- **Old firmware files are deleted at finalize, not at commit.** `PRAGMA synchronous = NORMAL` in WAL mode means a commit can roll back at a power cut. The conditional update records the old file in `key_rotation_superseded_files` in the same transaction. Finalize deletes those files, and runs the sweep, only after `checkpoint_wal()` completed; `checkpoint_wal()` now errors when SQLite reports the checkpoint as busy, and finalize then keeps every file and returns a warning. If a row's recorded file is missing, `continue` adopts an existing `<id>.k<v+1>.enc` that authenticates under the new key. The sweep never deletes `<id>.k<active>.enc` while its row is still at an older version and the file authenticates under the active key. The crash points are listed at the top of `commands/key_rotation_commands.rs`.
- **Write-time gate.** The session gate is checked when a command fetches the key, which can be long before it writes (a firmware upload reads up to 2 GB first). Every ciphertext writer now calls `keystore::ensure_key_is_current` under the DB lock: it refuses unless the key is the active version and no `retiring` key exists. Triggers on the four ciphertext tables also abort any insert, or update of the ciphertext or `key_version`, that records a version other than the active one. A firmware upload deletes the file it encrypted if the check fails; analysis jobs check their key before using it and fail closed. This also closes the gap left by the analysis-job preflight, which is only an in-memory count read before `begin` (a job queued just after it fails closed). Creating the analysis queue is serialized, so racing uploads can't create an unmanaged queue whose jobs are not counted.
- **Lock order.** `login`, `begin`, and `finalize` never hold the DB lock while taking the session-manager lock. Login reads the rotation state under the DB lock, then applies it under the session lock only if no `begin` or `finalize` changed the gate in between (a rotation epoch in `SessionManager`); otherwise it repeats the database part.
- **Begin always returns the recovery code** once its transaction has committed. Problems switching the administrator's session to the new key are returned in `BeginKeyRotationResponse.warnings` instead of an error. `KeyRotationFinalized` also gained `already_finalized` and `warnings`.
- **The `revoke_inactive_user_wraps` migration checkpoints the WAL** after deleting the wraps.
