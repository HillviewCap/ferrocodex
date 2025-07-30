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

export interface UpdateVaultSecretRequest {
  secret_id: number;
  label?: string;
  value?: string;
  author_id: number;
}

export interface DeleteVaultSecretRequest {
  secret_id: number;
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

// Standalone credential types for Story 4.3
export interface StandaloneCredential {
  id: number;
  name: string;
  description: string;
  credential_type: SecretType;
  category_id?: number;
  encrypted_data: string;
  created_by: number;
  created_at: string;
  updated_at: string;
  last_accessed?: string;
}

export interface CredentialCategory {
  id: number;
  name: string;
  description?: string;
  parent_category_id?: number;
  color_code?: string;
  icon?: string;
  created_at: string;
}

export interface StandaloneCredentialHistory {
  id: number;
  credential_id: number;
  change_type: StandaloneChangeType;
  author: number;
  timestamp: string;
  notes?: string;
  changes_json?: string;
}

export type StandaloneChangeType = 'Created' | 'Updated' | 'Accessed' | 'Deleted';

export interface StandaloneCredentialInfo {
  credential: StandaloneCredential;
  category?: CredentialCategory;
  tags: string[];
}

export interface CreateStandaloneCredentialRequest {
  name: string;
  description: string;
  credential_type: SecretType;
  category_id?: number;
  value: string;
  tags?: string[];
  created_by: number;
}

export interface UpdateStandaloneCredentialRequest {
  id: number;
  name?: string;
  description?: string;
  category_id?: number;
  value?: string;
  author_id: number;
}

export interface SearchCredentialsRequest {
  query?: string;
  credential_type?: SecretType;
  category_id?: number;
  tags?: string[];
  created_after?: string;
  created_before?: string;
  limit?: number;
  offset?: number;
}

export interface SearchCredentialsResponse {
  credentials: StandaloneCredentialInfo[];
  total_count: number;
  page: number;
  page_size: number;
}

export interface CreateCategoryRequest {
  name: string;
  description?: string;
  parent_category_id?: number;
  color_code?: string;
  icon?: string;
}

export interface CategoryWithChildren {
  category: CredentialCategory;
  children: CategoryWithChildren[];
  credential_count: number;
}

export const standaloneChangeTypeDisplayNames: Record<StandaloneChangeType, string> = {
  Created: 'Created',
  Updated: 'Updated',
  Accessed: 'Accessed',
  Deleted: 'Deleted',
};

export const defaultCategoryIcons: Record<string, string> = {
  'Jump Hosts': 'server',
  'Databases': 'database',
  'Network Equipment': 'network',
  'Applications': 'apps',
  'Cloud Services': 'cloud',
};

// Vault permission types for Story 4.5
export interface VaultPermission {
  permission_id: number;
  user_id: number;
  vault_id: number;
  permission_type: PermissionType;
  granted_by: number;
  granted_at: string;
  expires_at?: string;
  is_active: boolean;
}

export type PermissionType = 'Read' | 'Write' | 'Export' | 'Share';

export interface VaultAccessLog {
  access_id: number;
  user_id: number;
  vault_id: number;
  access_type: AccessType;
  accessed_at: string;
  ip_address?: string;
  user_agent?: string;
  result: AccessResult;
  error_message?: string;
}

export type AccessType = 'View' | 'Edit' | 'Export' | 'Share' | 'Denied';
export type AccessResult = 'Success' | 'Denied' | 'Error';

export interface PermissionRequest {
  request_id: number;
  user_id: number;
  vault_id: number;
  requested_permission: PermissionType;
  requested_by: number;
  status: RequestStatus;
  approved_by?: number;
  created_at: string;
  updated_at: string;
  approval_notes?: string;
}

export type RequestStatus = 'Pending' | 'Approved' | 'Denied' | 'Expired';

export interface GrantVaultAccessRequest {
  user_id: number;
  vault_id: number;
  permission_type: PermissionType;
  granted_by: number;
  expires_at?: string;
}

export interface RevokeVaultAccessRequest {
  user_id: number;
  vault_id: number;
  permission_type?: PermissionType;
  revoked_by: number;
}

export interface CheckVaultAccessRequest {
  user_id: number;
  vault_id: number;
  permission_type: PermissionType;
}

export interface VaultAccessInfo {
  has_access: boolean;
  permissions: VaultPermission[];
  is_administrator: boolean;
}

export interface CreatePermissionRequest {
  vault_id: number;
  requested_permission: PermissionType;
  requested_by: number;
}

export const permissionTypeDisplayNames: Record<PermissionType, string> = {
  Read: 'Read',
  Write: 'Write',
  Export: 'Export',
  Share: 'Share',
};

export const permissionTypeIcons: Record<PermissionType, string> = {
  Read: 'eye',
  Write: 'edit',
  Export: 'export',
  Share: 'share-alt',
};

export const requestStatusColors: Record<RequestStatus, string> = {
  Pending: '#faad14', // Yellow
  Approved: '#52c41a', // Green
  Denied: '#f5222d', // Red
  Expired: '#8c8c8c', // Gray
};

export const accessResultIcons: Record<AccessResult, string> = {
  Success: 'check-circle',
  Denied: 'close-circle',
  Error: 'exclamation-circle',
};

// Password rotation types for Story 4.6
export interface PasswordRotationRequest {
  secret_id: number;
  new_password: string;
  rotation_reason: string;
  author_id: number;
  batch_id?: number;
}

export interface RotationSchedule {
  schedule_id: number;
  vault_id: number;
  rotation_interval: number; // days
  alert_days_before: number;
  is_active: boolean;
  created_at: string;
  created_by: number;
  updated_at: string;
}

export interface RotationBatch {
  batch_id: number;
  batch_name: string;
  created_by: number;
  started_at: string;
  completed_at?: string;
  status: BatchStatus;
  notes?: string;
}

export type BatchStatus = 'pending' | 'in_progress' | 'completed' | 'failed' | 'cancelled';

export interface PasswordRotationHistory {
  rotation_id: number;
  secret_id: number;
  old_password_hash: string;
  rotation_reason: string;
  rotated_by: number;
  rotated_at: string;
  batch_id?: number;
}

export interface RotationAlert {
  secret_id: number;
  vault_id: number;
  secret_label: string;
  asset_name: string;
  days_until_rotation: number;
  next_rotation_due: string;
  last_rotated?: string;
}

export interface CreateRotationScheduleRequest {
  vault_id: number;
  rotation_interval: number;
  alert_days_before: number;
  created_by: number;
}

export interface UpdateRotationScheduleRequest {
  schedule_id: number;
  rotation_interval?: number;
  alert_days_before?: number;
  is_active?: boolean;
}

export interface CreateRotationBatchRequest {
  batch_name: string;
  created_by: number;
  notes?: string;
}

export interface BatchRotationItem {
  secret_id: number;
  new_password: string;
  rotation_reason: string;
}

export interface BatchRotationRequest {
  batch_id: number;
  items: BatchRotationItem[];
  author_id: number;
}

export const batchStatusColors: Record<BatchStatus, string> = {
  pending: '#faad14', // Yellow
  in_progress: '#1890ff', // Blue
  completed: '#52c41a', // Green
  failed: '#f5222d', // Red
  cancelled: '#8c8c8c', // Gray
};

export const batchStatusIcons: Record<BatchStatus, string> = {
  pending: 'clock-circle',
  in_progress: 'sync',
  completed: 'check-circle',
  failed: 'close-circle',
  cancelled: 'stop',
};

export const rotationIntervalOptions = [
  { value: 30, label: '30 days' },
  { value: 60, label: '60 days' },
  { value: 90, label: '90 days' },
  { value: 120, label: '120 days' },
  { value: 180, label: '180 days' },
  { value: 365, label: '1 year' },
];

export const rotationAlertOptions = [
  { value: 7, label: '7 days before' },
  { value: 14, label: '14 days before' },
  { value: 30, label: '30 days before' },
];

export const rotationTemplates = [
  { key: 'all_plc_credentials', name: 'All PLC Credentials', description: 'Rotate all PLC passwords in an asset' },
  { key: 'vendor_default_passwords', name: 'Vendor Default Passwords', description: 'Rotate all vendor default passwords' },
  { key: 'expired_passwords', name: 'Expired Passwords', description: 'Rotate all overdue passwords' },
  { key: 'security_incident_response', name: 'Security Incident Response', description: 'Emergency rotation for security incident' },
  { key: 'quarterly_rotation', name: 'Quarterly Rotation', description: 'Scheduled quarterly password rotation' },
];

export interface RotationComplianceMetrics {
  total_passwords: number;
  overdue_passwords: number;
  due_within_7_days: number;
  avg_days_since_rotation: number;
  compliance_percentage: number;
}