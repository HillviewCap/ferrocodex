export interface Asset {
  id: number;
  name: string;
  description: string;
  created_by: number;
  created_at: string;
  updated_at: string;
}

export interface AssetInfo {
  id: number;
  name: string;
  description: string;
  created_by: number;
  created_at: string;
  version_count: number;
  latest_version: string | null;
}

export interface ConfigurationVersion {
  id: number;
  asset_id: number;
  version_number: string;
  file_name: string;
  file_content: Uint8Array;
  file_size: number;
  content_hash: string;
  author: number;
  notes: string;
  created_at: string;
}

export type ConfigurationStatus = 'Draft' | 'Silver' | 'Approved' | 'Golden' | 'Archived';

export interface ConfigurationVersionInfo {
  id: number;
  asset_id: number;
  version_number: string;
  file_name: string;
  file_size: number;
  content_hash: string;
  author: number;
  author_username: string;
  notes: string;
  status: ConfigurationStatus;
  status_changed_by?: number;
  status_changed_at?: string;
  created_at: string;
}

export interface StatusChangeRecord {
  id: number;
  version_id: number;
  old_status?: string;
  new_status: string;
  changed_by: number;
  changed_by_username: string;
  change_reason?: string;
  created_at: string;
}

export interface FileMetadata {
  name: string;
  size: number;
  content_type: string;
  hash: string;
}

export interface CreateAssetRequest {
  name: string;
  description: string;
}

export interface ImportConfigurationRequest {
  asset_name: string;
  file_path: string;
  notes: string;
}

export interface FileImportStatus {
  isImporting: boolean;
  progress: number;
  status: string;
  error?: string;
}

// Validation schemas
export const AssetValidation = {
  name: {
    minLength: 2,
    maxLength: 100,
    required: true,
    pattern: /^[a-zA-Z0-9\s\-_\.]+$/,
    message: 'Asset name must be 2-100 characters and contain only letters, numbers, spaces, hyphens, underscores, and periods'
  },
  description: {
    maxLength: 500,
    required: false,
    message: 'Description cannot exceed 500 characters'
  }
};

export const ConfigurationValidation = {
  notes: {
    maxLength: 1000,
    required: false,
    message: 'Notes cannot exceed 1000 characters'
  },
  fileSize: {
    maxSize: 100 * 1024 * 1024, // 100MB
    message: 'File size cannot exceed 100MB'
  },
  allowedExtensions: [
    'json', 'xml', 'yaml', 'yml', 'txt', 'cfg', 'conf', 'ini',
    'csv', 'log', 'properties', 'config', 'settings', 'toml',
    'bin', 'dat', 'hex', 'raw', 'dump'
  ]
};

// Helper functions
export const validateAssetName = (name: string): string | null => {
  if (!name.trim()) {
    return 'Asset name is required';
  }
  if (name.length < AssetValidation.name.minLength) {
    return `Asset name must be at least ${AssetValidation.name.minLength} characters`;
  }
  if (name.length > AssetValidation.name.maxLength) {
    return `Asset name cannot exceed ${AssetValidation.name.maxLength} characters`;
  }
  if (!AssetValidation.name.pattern.test(name)) {
    return AssetValidation.name.message;
  }
  return null;
};

export const validateAssetDescription = (description: string): string | null => {
  if (description.length > AssetValidation.description.maxLength) {
    return AssetValidation.description.message;
  }
  return null;
};

export const validateConfigurationNotes = (notes: string): string | null => {
  if (notes.length > ConfigurationValidation.notes.maxLength) {
    return ConfigurationValidation.notes.message;
  }
  return null;
};

export const validateFileSize = (size: number): string | null => {
  if (size > ConfigurationValidation.fileSize.maxSize) {
    return ConfigurationValidation.fileSize.message;
  }
  return null;
};

export const validateFileExtension = (filename: string): string | null => {
  const extension = filename.split('.').pop()?.toLowerCase();
  if (!extension) {
    return 'File must have an extension';
  }
  if (!ConfigurationValidation.allowedExtensions.includes(extension)) {
    return `File type .${extension} is not supported. Allowed types: ${ConfigurationValidation.allowedExtensions.join(', ')}`;
  }
  return null;
};

export const formatFileSize = (bytes: number): string => {
  if (bytes === 0) return '0 Bytes';
  
  const k = 1024;
  const sizes = ['Bytes', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
};

export const formatVersion = (version: string): string => {
  return version.startsWith('v') ? version : `v${version}`;
};

export const parseVersion = (version: string): number => {
  const match = version.match(/v?(\d+)/);
  return match ? parseInt(match[1], 10) : 0;
};

export const sortVersions = (versions: ConfigurationVersionInfo[]): ConfigurationVersionInfo[] => {
  return [...versions].sort((a, b) => {
    const aNum = parseVersion(a.version_number);
    const bNum = parseVersion(b.version_number);
    return bNum - aNum; // Descending order (latest first)
  });
};