// Asset File Association Types
// These types mirror the Rust structures in src-tauri/src/associations/mod.rs

export interface AssetFileAssociation {
  id: number;
  assetId: number;
  fileId: number;
  fileType: AssociationType;
  associationOrder: number;
  metadata?: string;
  createdBy: number;
  createdAt: string;
  updatedAt: string;
}

export type AssociationType = 'Configuration' | 'Firmware';

export interface FileImportSession {
  id: number;
  sessionName: string;
  assetId: number;
  filePaths: string[];
  importStatus: ImportStatus;
  validationResults?: string;
  createdBy: number;
  createdAt: string;
}

export type ImportStatus = 'Pending' | 'InProgress' | 'Completed' | 'Failed';

export interface AssociationValidation {
  id: number;
  associationId: number;
  validationType: ValidationType;
  validationResult: ValidationResult;
  validationMessage: string;
  validatedAt: string;
}

export type ValidationType = 
  | 'SecurityClassification'
  | 'FileTypeCompatibility'
  | 'AssetTypeCompatibility'
  | 'DuplicateCheck'
  | 'ReferentialIntegrity';

export type ValidationResult = 'Passed' | 'Failed' | 'Warning';

export interface AssociationInfo {
  id: number;
  assetId: number;
  assetName: string;
  fileId: number;
  fileName: string;
  fileType: AssociationType;
  associationOrder: number;
  metadata?: string;
  createdBy: number;
  createdByUsername: string;
  createdAt: string;
  validationStatus: ValidationResult;
}

export interface AssociationSummary {
  assetId: number;
  assetName: string;
  configurationCount: number;
  firmwareCount: number;
  totalAssociations: number;
  validationIssues: number;
}

export interface HealthStatus {
  healthy: boolean;
  issues: string[];
  warnings: string[];
  lastChecked: string;
}

export interface CreateAssociationRequest {
  assetId: number;
  fileId: number;
  fileType: AssociationType;
  metadata?: string;
  createdBy: number;
}

// UI-specific types
export interface DragDropAssociationData {
  fileId: number;
  fileName: string;
  fileType: AssociationType;
  fileSize: number;
  lastModified: string;
}

export interface AssociationWizardStep {
  id: string;
  title: string;
  description: string;
  completed: boolean;
  active: boolean;
}

export interface FileAssociationWizardState {
  currentStep: number;
  selectedAsset?: {
    id: number;
    name: string;
    description: string;
    type: string;
  };
  selectedFiles: File[];
  uploadedFiles: {
    file: File;
    fileId?: number;
    progress: number;
    status: 'pending' | 'uploading' | 'completed' | 'failed';
    error?: string;
  }[];
  associations: CreateAssociationRequest[];
  validationResults: AssociationValidation[];
  importSession?: FileImportSession;
}

export interface AssetSelectorProps {
  onAssetSelect: (asset: AssetInfo) => void;
  selectedAsset?: AssetInfo;
  fileType?: AssociationType;
  disabled?: boolean;
}

export interface FileUploadProgress {
  fileId: string;
  fileName: string;
  progress: number;
  status: 'uploading' | 'validating' | 'completed' | 'failed';
  error?: string;
}

export interface AssociationPreview {
  asset: {
    id: number;
    name: string;
    type: string;
  };
  files: {
    id: number;
    name: string;
    type: AssociationType;
    size: number;
  }[];
  estimatedAssociations: number;
  validationStatus: ValidationResult;
  warnings: string[];
}

// Import from existing types
export interface AssetInfo {
  id: number;
  name: string;
  description: string;
  assetType: 'Folder' | 'Device';
  parentId?: number;
  sortOrder: number;
  createdBy: number;
  createdByUsername: string;
  createdAt: string;
  versionCount: number;
  latestVersion?: string;
  latestVersionNotes?: string;
}