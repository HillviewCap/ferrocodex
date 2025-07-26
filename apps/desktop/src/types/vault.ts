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