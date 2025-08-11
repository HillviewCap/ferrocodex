export interface BulkImportSession {
  id: number;
  session_name: string;
  import_type: string;
  total_items: number;
  processed_items: number;
  failed_items: number;
  status: BulkImportStatus;
  template_path: string | null;
  error_log: string | null;
  created_by: number;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
}

export type BulkImportStatus = 'Created' | 'Validating' | 'Processing' | 'Paused' | 'Completed' | 'Failed' | 'Cancelled';

export interface BulkImportItem {
  id: number;
  session_id: number;
  item_data_json: string;
  processing_status: BulkItemStatus;
  error_message: string | null;
  asset_id: number | null;
  processed_at: string | null;
}

export type BulkItemStatus = 'Pending' | 'Processing' | 'Completed' | 'Failed' | 'Skipped';

export type ValidationMode = 'strict' | 'permissive';

export interface BulkImportSessionDetails {
  session: BulkImportSession;
  items: BulkImportItem[];
  errors: BulkImportError[];
}

export interface BulkImportError {
  id: number;
  session_id: number;
  item_id: number | null;
  error_type: string;
  error_message: string;
  error_details: string | null;
  created_at: string;
}

export interface CreateBulkImportSessionRequest {
  session_name: string;
  import_type: string;
  template_path?: string;
}

export interface ValidationSummary {
  total_items: number;
  valid_items: number;
  invalid_items: number;
  errors: ValidationError[];
}

export interface ValidationError {
  row: number;
  field: string;
  value: string;
  message: string;
}

export interface ValidationResults {
  is_valid: boolean;
  errors: ValidationError[];
  warnings: ValidationWarning[];
  preview_items: AssetPreview[];
}

export interface ValidationWarning {
  row: number;
  field: string;
  value: string;
  message: string;
}

export interface AssetPreview {
  row: number;
  name: string;
  description: string;
  asset_type: string;
  parent_name: string | null;
  metadata: Record<string, unknown>;
}

export interface ProcessingOptions {
  skip_existing: boolean;
  update_existing: boolean;
  create_missing_parents: boolean;
  validation_mode: 'strict' | 'permissive';
}

export interface ProgressStatus {
  session_id: number;
  total_items: number;
  processed_items: number;
  failed_items: number;
  current_item: string | null;
  estimated_completion: string | null;
  processing_rate: number;
  status: BulkImportStatus;
}

export interface ImportTemplate {
  id: number;
  template_name: string;
  template_type: string;
  field_mapping: Record<string, string>;
  required_fields: string[];
  optional_fields: string[];
  validation_rules: Record<string, unknown>;
  created_by: number;
  created_at: string;
  updated_at: string;
}

export interface ImportTemplateConfig {
  template_name: string;
  template_type: string;
  asset_type: string;
  field_mapping: Record<string, string>;
  required_fields: string[];
  optional_fields: string[];
  validation_rules: Record<string, unknown>;
}

export interface CSVParseResult {
  headers: string[];
  rows: Record<string, string>[];
  total_rows: number;
  errors: CSVParseError[];
}

export interface CSVParseError {
  row: number;
  column: string;
  message: string;
}

export interface BulkOperationStats {
  total_sessions: number;
  active_sessions: number;
  completed_sessions: number;
  failed_sessions: number;
  total_items_processed: number;
  average_processing_time: number;
  success_rate: number;
}

// Validation schemas and helpers
export const BulkImportValidation = {
  sessionName: {
    minLength: 3,
    maxLength: 100,
    required: true,
    pattern: /^[a-zA-Z0-9\s\-_\.]+$/,
    message: 'Session name must be 3-100 characters and contain only letters, numbers, spaces, hyphens, underscores, and periods'
  },
  importType: {
    allowedTypes: ['assets', 'configurations', 'metadata'],
    required: true,
    message: 'Import type must be one of: assets, configurations, metadata'
  },
  fileSize: {
    maxSize: 50 * 1024 * 1024, // 50MB for CSV files
    message: 'CSV file size cannot exceed 50MB'
  },
  csvHeaders: {
    required: ['name', 'asset_type'],
    optional: ['description', 'parent_name', 'metadata'],
    message: 'CSV must contain required headers: name, asset_type'
  }
};

export const validateSessionName = (name: string): string | null => {
  if (!name.trim()) {
    return 'Session name is required';
  }
  if (name.length < BulkImportValidation.sessionName.minLength) {
    return `Session name must be at least ${BulkImportValidation.sessionName.minLength} characters`;
  }
  if (name.length > BulkImportValidation.sessionName.maxLength) {
    return `Session name cannot exceed ${BulkImportValidation.sessionName.maxLength} characters`;
  }
  if (!BulkImportValidation.sessionName.pattern.test(name)) {
    return BulkImportValidation.sessionName.message;
  }
  return null;
};

export const validateImportType = (type: string): string | null => {
  if (!type) {
    return 'Import type is required';
  }
  if (!BulkImportValidation.importType.allowedTypes.includes(type)) {
    return BulkImportValidation.importType.message;
  }
  return null;
};

export const validateCSVFile = (file: File): string | null => {
  if (!file) {
    return 'File is required';
  }
  if (!file.name.toLowerCase().endsWith('.csv')) {
    return 'File must be a CSV file (.csv extension)';
  }
  if (file.size > BulkImportValidation.fileSize.maxSize) {
    return BulkImportValidation.fileSize.message;
  }
  return null;
};

export const formatProcessingRate = (rate: number): string => {
  if (rate < 1) {
    return `${(rate * 60).toFixed(1)} items/min`;
  }
  return `${rate.toFixed(1)} items/sec`;
};

export const formatEstimatedTime = (seconds: number): string => {
  if (seconds < 60) {
    return `${Math.round(seconds)}s`;
  }
  if (seconds < 3600) {
    const minutes = Math.round(seconds / 60);
    return `${minutes}m`;
  }
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.round((seconds % 3600) / 60);
  return `${hours}h ${minutes}m`;
};

export const calculateSuccessRate = (processed: number, failed: number): number => {
  const total = processed + failed;
  if (total === 0) return 100;
  return Math.round(((processed - failed) / total) * 100);
};