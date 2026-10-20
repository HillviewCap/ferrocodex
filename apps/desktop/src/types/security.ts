// Security-related types for the frontend UI
// These mirror the backend Rust structures from src-tauri/src/security/mod.rs

export interface SecurityValidationResult {
  isValid: boolean;
  errorCode?: string;
  errorMessage?: string;
  suggestedCorrections: string[];
  securityFlags: string[];
  validationTimestamp?: string;
}

export interface FileIntegrityResult {
  sha256Hash: string;
  fileSize: number;
  isVerified: boolean;
  securityScanPassed: boolean;
  detectedIssues: string[];
}

export interface AuditEvent {
  id: number;
  timestamp: string;
  eventType: string;
  userId?: number;
  details: string;
  result: string;
}

export interface AuditFilter {
  startDate?: string;
  endDate?: string;
  eventType?: string;
  userId?: number;
  limit?: number;
}

// UI-specific types for security components

export interface FileValidationStatus {
  filename: string;
  validationStage: 'pending' | 'filename' | 'content' | 'hash' | 'classification' | 'complete' | 'failed';
  progressPercentage: number;
  hashStatus: 'pending' | 'calculating' | 'complete' | 'failed';
  complianceStatus: 'pending' | 'checking' | 'passed' | 'failed';
  error?: string;
}

export interface SecurityClassificationUI {
  level: SecurityClassificationLevel;
  displayName: string;
  color: string;
  icon: string;
  description: string;
  accessRequirements: string[];
}

export enum SecurityClassificationLevel {
  PUBLIC = 'PUBLIC',
  INTERNAL = 'INTERNAL',
  CONFIDENTIAL = 'CONFIDENTIAL',
  RESTRICTED = 'RESTRICTED',
  SECRET = 'SECRET'
}

export interface AuditEventUI {
  timestamp: string;
  eventType: string;
  userName: string;
  assetName: string;
  actionResult: 'success' | 'failure' | 'warning';
  severity: 'info' | 'warning' | 'critical';
  details: string;
}

// Security overview returned by get_security_overview (mirrors security_commands.rs).
// Timestamps are the DB's UTC "YYYY-MM-DD HH:MM:SS" strings with no zone suffix;
// parse them with utils/utcTimestamp.
export type SecurityOverviewPeriod = '24h' | '7d' | '30d';
export type SecurityOverviewStatus = 'ok' | 'warning' | 'critical';
export type SecuritySeverity = 'critical' | 'warning' | 'info';

export interface AuthenticationStats {
  successfulLogins: number;
  failedLogins: number;
  lockouts: number;
  distinctFailedUsernames: number;
  currentlyLockedAccounts: string[];
}

export interface AccessControlStats {
  vaultAccessDenied: number;
  vaultExports: number;
  secretReads: number;
  pendingPermissionRequests: number;
  grantsExpiringSoon: number;
}

export interface UserStats {
  activeUsers: number;
  activeAdministrators: number;
  inactiveUsers: number;
  usersPendingKeyUpgrade: number;
  roleChanges: number;
}

export interface EncryptionStats {
  activeKeyVersion: number | null;
  activeKeyCreatedAt: string | null;
  activeKeyAgeDays: number | null;
  rotationInProgress: boolean;
  lastRotationCompletedAt: string | null;
  recoveryKeyLastReplacedAt: string | null;
}

export interface AssetStats {
  totalAssets: number;
  // Every classification level is present, zero-filled
  classificationBreakdown: Record<string, number>;
}

export interface SecurityFinding {
  id: string;
  severity: SecuritySeverity;
  title: string;
  detail: string;
}

export interface SecurityEventSummary {
  id: number;
  timestamp: string;
  eventCode: string;
  category: 'authentication' | 'user' | 'access' | 'encryption';
  severity: SecuritySeverity;
  username: string | null;
  description: string;
}

export interface SecurityOverview {
  period: SecurityOverviewPeriod;
  generatedAt: string;
  status: SecurityOverviewStatus;
  authentication: AuthenticationStats;
  accessControl: AccessControlStats;
  users: UserStats;
  encryption: EncryptionStats;
  assets: AssetStats;
  findings: SecurityFinding[];
  recentEvents: SecurityEventSummary[];
}

export interface SecurityAlert {
  id: string;
  timestamp: string;
  severity: 'info' | 'warning' | 'critical';
  title: string;
  description: string;
  assetName?: string;
  userName?: string;
  acknowledged: boolean;
  resolvedAt?: string;
}

export interface ComplianceStatus {
  level: 'excellent' | 'good' | 'fair' | 'needs-attention';
  score: number;
  issues: string[];
  recommendations: string[];
}

// Form types for security validation
export interface SecurityValidationInputProps {
  value: string;
  onChange: (value: string) => void;
  onValidationChange: (result: SecurityValidationResult) => void;
  placeholder?: string;
  disabled?: boolean;
  showSuggestions?: boolean;
  validationDelay?: number; // ms, default 100
}

export interface FileSecurityUploadProps {
  onUploadComplete: (results: FileIntegrityResult[]) => void;
  onUploadProgress: (status: FileValidationStatus[]) => void;
  onUploadError: (error: string) => void;
  acceptedFileTypes?: string[];
  maxFileSize?: number; // bytes
  maxFiles?: number;
  autoClassification?: SecurityClassificationLevel;
}

// Security classification constants
export const SECURITY_CLASSIFICATIONS: Record<SecurityClassificationLevel, SecurityClassificationUI> = {
  [SecurityClassificationLevel.PUBLIC]: {
    level: SecurityClassificationLevel.PUBLIC,
    displayName: 'Public',
    color: '#52c41a',
    icon: 'GlobalOutlined',
    description: 'Information that can be freely shared with anyone',
    accessRequirements: ['No special access required']
  },
  [SecurityClassificationLevel.INTERNAL]: {
    level: SecurityClassificationLevel.INTERNAL,
    displayName: 'Internal',
    color: '#1890ff',
    icon: 'TeamOutlined',
    description: 'Information for internal organizational use only',
    accessRequirements: ['Valid employee/contractor status']
  },
  [SecurityClassificationLevel.CONFIDENTIAL]: {
    level: SecurityClassificationLevel.CONFIDENTIAL,
    displayName: 'Confidential',
    color: '#faad14',
    icon: 'LockOutlined',
    description: 'Sensitive information requiring controlled access',
    accessRequirements: ['Manager approval', 'Confidentiality agreement']
  },
  [SecurityClassificationLevel.RESTRICTED]: {
    level: SecurityClassificationLevel.RESTRICTED,
    displayName: 'Restricted',
    color: '#ff7a45',
    icon: 'SafetyCertificateOutlined',
    description: 'Highly sensitive information with limited access',
    accessRequirements: ['Executive approval', 'Security clearance', 'Need-to-know basis']
  },
  [SecurityClassificationLevel.SECRET]: {
    level: SecurityClassificationLevel.SECRET,
    displayName: 'Secret',
    color: '#f5222d',
    icon: 'SafetyCertificateOutlined',
    description: 'Top secret information requiring highest security level',
    accessRequirements: ['Top secret clearance', 'Administrator approval', 'Compartmentalized access']
  }
};

// Helper functions for security UI
export const getClassificationColor = (level: SecurityClassificationLevel): string => {
  return SECURITY_CLASSIFICATIONS[level].color;
};

export const getClassificationIcon = (level: SecurityClassificationLevel): string => {
  return SECURITY_CLASSIFICATIONS[level].icon;
};

export const getClassificationDescription = (level: SecurityClassificationLevel): string => {
  return SECURITY_CLASSIFICATIONS[level].description;
};

export const isHigherClassification = (level1: SecurityClassificationLevel, level2: SecurityClassificationLevel): boolean => {
  const hierarchy = [
    SecurityClassificationLevel.PUBLIC,
    SecurityClassificationLevel.INTERNAL,
    SecurityClassificationLevel.CONFIDENTIAL,
    SecurityClassificationLevel.RESTRICTED,
    SecurityClassificationLevel.SECRET
  ];
  
  return hierarchy.indexOf(level1) > hierarchy.indexOf(level2);
};