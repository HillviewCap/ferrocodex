export type FirmwareStatus = 'Draft' | 'Golden' | 'Archived';

export interface FirmwareVersion {
  id: number;
  asset_id: number;
  author_id: number;
  vendor: string | null;
  model: string | null;
  version: string;
  notes: string | null;
  status: FirmwareStatus;
  file_path: string;
  file_hash: string;
  file_size: number;
  created_at: string;
}

export interface FirmwareVersionInfo {
  id: number;
  asset_id: number;
  author_id: number;
  author_username: string;
  vendor: string | null;
  model: string | null;
  version: string;
  notes: string | null;
  status: FirmwareStatus;
  file_path: string;
  file_hash: string;
  file_size: number;
  created_at: string;
}

export interface UploadFirmwareRequest {
  asset_id: number;
  vendor: string | null;
  model: string | null;
  version: string;
  notes: string | null;
  file_path: string;
}

export interface FirmwareUploadProgress {
  progress: number;
  status: 'preparing' | 'uploading' | 'processing' | 'complete' | 'error';
  message?: string;
}

// Validation schemas
export const FirmwareValidation = {
  vendor: {
    maxLength: 100,
    required: false,
    message: 'Vendor name cannot exceed 100 characters'
  },
  model: {
    maxLength: 100,
    required: false,
    message: 'Model name cannot exceed 100 characters'
  },
  version: {
    minLength: 1,
    maxLength: 50,
    required: true,
    pattern: /^[a-zA-Z0-9\.\-_]+$/,
    message: 'Version must be 1-50 characters and contain only letters, numbers, dots, hyphens, and underscores'
  },
  notes: {
    maxLength: 1000,
    required: false,
    message: 'Notes cannot exceed 1000 characters'
  },
  fileSize: {
    maxSize: 2 * 1024 * 1024 * 1024, // 2GB
    message: 'File size cannot exceed 2GB'
  },
  allowedExtensions: [
    'bin', 'hex', 'img', 'rom', 'fw', 'elf', 'dfu', 'upd', 
    'dat', 'firmware', 'update', 'pkg', 'ipk', 'tar', 'gz',
    'bz2', 'xz', 'zip', 'rar', '7z', 'cab', 'iso', 'dmg'
  ]
};

// Helper functions
export const validateFirmwareVendor = (vendor: string): string | null => {
  if (vendor.length > FirmwareValidation.vendor.maxLength) {
    return FirmwareValidation.vendor.message;
  }
  return null;
};

export const validateFirmwareModel = (model: string): string | null => {
  if (model.length > FirmwareValidation.model.maxLength) {
    return FirmwareValidation.model.message;
  }
  return null;
};

export const validateFirmwareVersion = (version: string): string | null => {
  if (!version.trim()) {
    return 'Firmware version is required';
  }
  if (version.length < FirmwareValidation.version.minLength) {
    return `Version must be at least ${FirmwareValidation.version.minLength} character`;
  }
  if (version.length > FirmwareValidation.version.maxLength) {
    return `Version cannot exceed ${FirmwareValidation.version.maxLength} characters`;
  }
  if (!FirmwareValidation.version.pattern.test(version)) {
    return FirmwareValidation.version.message;
  }
  return null;
};

export const validateFirmwareNotes = (notes: string): string | null => {
  if (notes.length > FirmwareValidation.notes.maxLength) {
    return FirmwareValidation.notes.message;
  }
  return null;
};

export const validateFirmwareFileSize = (size: number): string | null => {
  if (size > FirmwareValidation.fileSize.maxSize) {
    return FirmwareValidation.fileSize.message;
  }
  return null;
};

export const formatFirmwareFileSize = (bytes: number): string => {
  if (bytes === 0) return '0 Bytes';
  
  const k = 1024;
  const sizes = ['Bytes', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
};

export const formatFirmwareHash = (hash: string): string => {
  // Show first 8 characters of hash
  return hash.length > 8 ? `${hash.substring(0, 8)}...` : hash;
};

export const sortFirmwareVersions = (versions: FirmwareVersionInfo[]): FirmwareVersionInfo[] => {
  return [...versions].sort((a, b) => {
    // Sort by created_at descending (latest first)
    return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
  });
};

export const validateFirmwareFileExtension = (filename: string): string | null => {
  const extension = filename.split('.').pop()?.toLowerCase();
  if (!extension) {
    return 'File must have an extension';
  }
  
  if (!FirmwareValidation.allowedExtensions.includes(extension)) {
    return `File type .${extension} is not allowed. Allowed types: ${FirmwareValidation.allowedExtensions.join(', ')}`;
  }
  
  return null;
};

// Firmware Analysis Types
export type AnalysisStatus = 'pending' | 'in_progress' | 'completed' | 'failed';

export type SecuritySeverity = 'info' | 'low' | 'medium' | 'high' | 'critical';

export interface SecurityFinding {
  severity: SecuritySeverity;
  findingType: string;
  description: string;
  offset?: number;
}

export interface FirmwareAnalysisResult {
  id: number;
  firmwareVersionId: number;
  analysisStatus: AnalysisStatus;
  fileType?: string;
  detectedVersions?: string[];
  entropyScore?: number;
  securityFindings?: SecurityFinding[];
  rawResults?: string;
  startedAt?: string;
  completedAt?: string;
  errorMessage?: string;
  createdAt: string;
}

export interface AnalysisEvent {
  firmwareId: number;
  status: string;
  progress?: number;
  message?: string;
}