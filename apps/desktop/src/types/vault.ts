export interface IdentityVault {
  id: number;
  asset_id: number;
  name: string;
  description: string;
  created_by: number;
  created_at: string;
  updated_at: string;
}

export interface VaultSecret {
  id: number;
  vault_id: number;
  secret_type: SecretType;
  label: string;
  encrypted_value: string;
  created_at: string;
  updated_at: string;
  // Password management fields
  strength_score?: number;
  last_changed?: string;
  generation_method?: string;
  policy_version?: number;
}

export interface VaultVersion {
  id: number;
  vault_id: number;
  change_type: ChangeType;
  author: number;
  timestamp: string;
  notes: string;
  changes_json: string;
}

export type SecretType = 'Password' | 'IpAddress' | 'VpnKey' | 'LicenseFile';

export type ChangeType = 'VaultCreated' | 'SecretAdded' | 'SecretUpdated' | 'SecretDeleted' | 'VaultUpdated';

export interface VaultInfo {
  vault: IdentityVault;
  secrets: VaultSecret[];
  secret_count: number;
}

export interface CreateVaultRequest {
  asset_id: number;
  name: string;
  description: string;
  created_by: number;
}

export interface AddSecretRequest {
  vault_id: number;
  secret_type: SecretType;
  label: string;
  value: string;
  author_id: number;
}

export const secretTypeDisplayNames: Record<SecretType, string> = {
  Password: 'Password',
  IpAddress: 'IP Address',
  VpnKey: 'VPN Key',
  LicenseFile: 'License File',
};

export const secretTypeIcons: Record<SecretType, string> = {
  Password: 'lock',
  IpAddress: 'global',
  VpnKey: 'shield',
  LicenseFile: 'file-text',
};

export const changeTypeDisplayNames: Record<ChangeType, string> = {
  VaultCreated: 'Vault Created',
  SecretAdded: 'Secret Added',
  SecretUpdated: 'Secret Updated',
  SecretDeleted: 'Secret Deleted',
  VaultUpdated: 'Vault Updated',
};

// Password management types
export interface PasswordHistory {
  id: number;
  secret_id: number;
  password_hash: string;
  created_at: string;
  retired_at?: string;
}

export interface PasswordStrength {
  score: number;
  entropy: number;
  has_uppercase: boolean;
  has_lowercase: boolean;
  has_numbers: boolean;
  has_special: boolean;
  length: number;
  feedback: string[];
}

export interface GeneratePasswordRequest {
  length: number;
  include_uppercase: boolean;
  include_lowercase: boolean;
  include_numbers: boolean;
  include_special: boolean;
  exclude_ambiguous: boolean;
}

export interface UpdateCredentialPasswordRequest {
  secret_id: number;
  new_password: string;
  author_id: number;
}

export const getStrengthColor = (score: number): string => {
  if (score >= 80) return '#52c41a'; // Green
  if (score >= 60) return '#faad14'; // Orange
  if (score >= 40) return '#fa8c16'; // Dark orange
  return '#f5222d'; // Red
};

export const getStrengthLabel = (score: number): string => {
  if (score >= 80) return 'Excellent';
  if (score >= 60) return 'Good';
  if (score >= 40) return 'Fair';
  return 'Weak';
};

export const defaultPasswordRequest: GeneratePasswordRequest = {
  length: 16,
  include_uppercase: true,
  include_lowercase: true,
  include_numbers: true,
  include_special: true,
  exclude_ambiguous: true,
};