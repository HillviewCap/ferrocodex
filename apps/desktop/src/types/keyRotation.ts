// Mirrors the serde structs in src-tauri/src/keystore/rotation.rs

export interface UserKeyAccess {
  id: number;
  username: string;
  role: string;
}

export interface RemainingCounts {
  configuration_versions: number;
  vault_secrets: number;
  standalone_credentials: number;
  firmware_versions: number;
  total: number;
}

export interface KeyRotationFailure {
  id: number;
  table_name: string;
  row_id: number;
  from_version: number;
  error: string;
  attempted_at: string;
}

export interface KeyRotationDetails {
  remaining: RemainingCounts;
  failures: KeyRotationFailure[];
  users_without_keypair: UserKeyAccess[];
  users_without_access: UserKeyAccess[];
  active_analysis_jobs: number;
}

export interface KeyRotationStatus {
  in_progress: boolean;
  active_version: number | null;
  retiring_version: number | null;
  // null for non-administrators
  details: KeyRotationDetails | null;
}

export interface BeginKeyRotationResponse {
  recovery_code: string;
  from_version: number;
  to_version: number;
  users_without_access: UserKeyAccess[];
}

export interface KeyRotationProgress {
  from_version: number;
  to_version: number;
  processed: number;
  failed: number;
  remaining: RemainingCounts;
  failure_count: number;
  done: boolean;
  finalized: boolean;
}

export interface KeyRotationFinalized {
  retired_version: number;
  active_version: number;
  discarded: number;
  orphaned_files_removed: number;
}
