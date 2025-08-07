// Multi-select bulk operations types (different from bulk import functionality)

export type BulkOperationType = 'move' | 'delete' | 'export' | 'classify' | 'rename';

export type BulkOperationStatus = 'pending' | 'validating' | 'processing' | 'completed' | 'failed' | 'cancelled';

export interface BulkOperation {
  id: string;
  operation_type: BulkOperationType;
  asset_ids: number[];
  status: BulkOperationStatus;
  progress_percent: number;
  created_by: number;
  started_at: string;
  completed_at: string | null;
  error_details: string | null;
  metadata?: Record<string, unknown>;
}

export interface BulkOperationLog {
  id: string;
  bulk_operation_id: string;
  asset_id: number;
  action: string;
  status: 'success' | 'failed' | 'skipped';
  error_message: string | null;
  timestamp: string;
}

export interface SelectionState {
  selected_asset_ids: Set<number>;
  selection_mode: 'none' | 'single' | 'multiple';
  last_selection_anchor: number | null;
  selection_metadata: Record<number, AssetSelectionInfo>;
}

export interface AssetSelectionInfo {
  selected_at: string;
  context: 'tree' | 'search' | 'manual';
  asset_name?: string;
  asset_type?: string;
}

// Bulk Move Operation
export interface BulkMoveOptions {
  new_parent_id: number | null;
  validate_hierarchy: boolean;
  skip_conflicts: boolean;
}

export interface BulkMoveRequest {
  asset_ids: number[];
  new_parent_id: number | null;
  options: BulkMoveOptions;
}

// Bulk Delete Operation
export interface BulkDeleteOptions {
  force_delete: boolean;
  delete_children: boolean;
  skip_protected: boolean;
}

export interface BulkDeleteRequest {
  asset_ids: number[];
  options: BulkDeleteOptions;
}

// Bulk Export Operation
export type ExportFormat = 'csv' | 'json' | 'xml' | 'yaml';

export interface BulkExportOptions {
  format: ExportFormat;
  include_metadata: boolean;
  include_children: boolean;
  include_configurations: boolean;
  export_path?: string;
}

export interface BulkExportRequest {
  asset_ids: number[];
  format: ExportFormat;
  options: BulkExportOptions;
}

// Bulk Classification Operation
export interface BulkClassifyRequest {
  asset_ids: number[];
  new_classification: string;
  apply_to_children: boolean;
}

// Bulk Rename Operation
export interface BulkRenameOptions {
  pattern: string;
  use_pattern: boolean;
  preserve_extension: boolean;
  start_number: number;
  prefix?: string;
  suffix?: string;
}

export interface BulkRenameRequest {
  asset_ids: number[];
  options: BulkRenameOptions;
}

// Operation Progress and Monitoring
export interface BulkOperationProgress {
  operation_id: string;
  status: BulkOperationStatus;
  total_items: number;
  processed_items: number;
  failed_items: number;
  current_item: string | null;
  estimated_completion: string | null;
  processing_rate: number;
  errors: BulkOperationError[];
}

export interface BulkOperationError {
  asset_id: number;
  asset_name: string;
  error_type: string;
  error_message: string;
  error_details?: Record<string, unknown>;
}

// Validation and Conflict Resolution
export interface ValidationResult {
  is_valid: boolean;
  warnings: ValidationWarning[];
  errors: ValidationError[];
  conflicts: ValidationConflict[];
}

export interface ValidationWarning {
  asset_id: number;
  asset_name: string;
  warning_type: string;
  message: string;
  can_proceed: boolean;
}

export interface ValidationError {
  asset_id: number;
  asset_name: string;
  error_type: string;
  message: string;
  blocking: boolean;
}

export interface ValidationConflict {
  asset_id: number;
  asset_name: string;
  conflict_type: string;
  message: string;
  resolution_options: ConflictResolutionOption[];
}

export interface ConflictResolutionOption {
  option_id: string;
  description: string;
  action: 'skip' | 'force' | 'modify' | 'cancel';
  parameters?: Record<string, unknown>;
}

// Operation History and Undo
export interface BulkOperationHistory {
  operations: BulkOperation[];
  total_count: number;
  page: number;
  page_size: number;
}

export interface UndoResult {
  success: boolean;
  reverted_items: number;
  failed_reversions: UndoFailure[];
  message: string;
}

export interface UndoFailure {
  asset_id: number;
  asset_name: string;
  reason: string;
}

// UI State Management
export interface BulkOperationUIState {
  showSelectionToolbar: boolean;
  showProgressModal: boolean;
  showValidationDialog: boolean;
  showHistoryPanel: boolean;
  currentOperation: BulkOperation | null;
  validationResults: ValidationResult | null;
  resolutionChoices: Record<string, string>;
}

// Selection Management
export interface SelectionManager {
  selectAsset: (assetId: number, context: 'tree' | 'search' | 'manual') => void;
  deselectAsset: (assetId: number) => void;
  toggleAsset: (assetId: number, context: 'tree' | 'search' | 'manual') => void;
  selectAll: (assetIds: number[], context: 'tree' | 'search') => void;
  selectNone: () => void;
  invertSelection: (availableAssetIds: number[]) => void;
  selectRange: (fromId: number, toId: number, assetIds: number[]) => void;
  isSelected: (assetId: number) => boolean;
  getSelectedCount: () => number;
  getSelectedAssets: () => number[];
  getSelectionInfo: (assetId: number) => AssetSelectionInfo | null;
}

// Performance Optimization Types
export interface SelectionPerformanceMetrics {
  selection_time: number;
  render_time: number;
  memory_usage: number;
  operation_count: number;
}

export interface BulkOperationPerformanceMetrics {
  operation_id: string;
  total_duration: number;
  validation_duration: number;
  processing_duration: number;
  throughput: number;
  memory_peak: number;
  cpu_usage: number;
}

// Keyboard and Accessibility Support
export interface KeyboardSelectionState {
  last_focused_asset: number | null;
  selection_anchor: number | null;
  keyboard_navigation_mode: boolean;
}

// Validation Helpers
export const validateBulkMoveRequest = (request: BulkMoveRequest): string | null => {
  if (!request.asset_ids || request.asset_ids.length === 0) {
    return 'No assets selected for move operation';
  }
  
  if (request.asset_ids.length > 1000) {
    return 'Cannot move more than 1000 assets in a single operation';
  }
  
  return null;
};

export const validateBulkDeleteRequest = (request: BulkDeleteRequest): string | null => {
  if (!request.asset_ids || request.asset_ids.length === 0) {
    return 'No assets selected for delete operation';
  }
  
  if (request.asset_ids.length > 500) {
    return 'Cannot delete more than 500 assets in a single operation';
  }
  
  return null;
};

export const validateBulkExportRequest = (request: BulkExportRequest): string | null => {
  if (!request.asset_ids || request.asset_ids.length === 0) {
    return 'No assets selected for export operation';
  }
  
  if (!['csv', 'json', 'xml', 'yaml'].includes(request.format)) {
    return 'Invalid export format specified';
  }
  
  return null;
};

export const validateBulkRenameRequest = (request: BulkRenameRequest): string | null => {
  if (!request.asset_ids || request.asset_ids.length === 0) {
    return 'No assets selected for rename operation';
  }
  
  if (request.asset_ids.length > 1000) {
    return 'Cannot rename more than 1000 assets in a single operation';
  }
  
  if (request.options.use_pattern && (!request.options.pattern || request.options.pattern.trim() === '')) {
    return 'Pattern is required when using pattern-based renaming';
  }
  
  return null;
};

// Utility Functions
export const formatBulkOperationDuration = (milliseconds: number): string => {
  const seconds = Math.floor(milliseconds / 1000);
  if (seconds < 60) {
    return `${seconds}s`;
  }
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  if (minutes < 60) {
    return `${minutes}m ${remainingSeconds}s`;
  }
  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  return `${hours}h ${remainingMinutes}m`;
};

export const calculateETA = (processedItems: number, totalItems: number, startTime: string): string | null => {
  if (processedItems === 0) return null;
  
  const elapsed = Date.now() - new Date(startTime).getTime();
  const rate = processedItems / elapsed;
  const remaining = totalItems - processedItems;
  const eta = remaining / rate;
  
  return formatBulkOperationDuration(eta);
};

export const getBulkOperationIcon = (operationType: BulkOperationType): string => {
  switch (operationType) {
    case 'move': return 'move';
    case 'delete': return 'delete';
    case 'export': return 'export';
    case 'classify': return 'tag';
    case 'rename': return 'edit';
    default: return 'operation';
  }
};

export const getBulkOperationColor = (status: BulkOperationStatus): string => {
  switch (status) {
    case 'pending': return 'default';
    case 'validating': return 'processing';
    case 'processing': return 'processing';
    case 'completed': return 'success';
    case 'failed': return 'error';
    case 'cancelled': return 'warning';
    default: return 'default';
  }
};