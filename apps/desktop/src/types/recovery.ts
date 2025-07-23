export interface RecoveryExportRequest {
  asset_id: number;
  config_version_id: number;
  firmware_version_id: number;
  export_directory: string;
}

export interface RecoveryManifest {
  asset_id: number;
  export_date: string;
  exported_by: string;
  configuration: ConfigurationExportInfo;
  firmware: FirmwareExportInfo;
  compatibility_verified: boolean;
}

export interface ConfigurationExportInfo {
  version_id: number;
  version_number: string;
  filename: string;
  checksum: string;
  file_size: number;
}

export interface FirmwareExportInfo {
  version_id: number;
  version: string;
  filename: string;
  checksum: string;
  vendor: string;
  model: string;
  file_size: number;
}

export interface ExportProgress {
  step: ExportStep;
  progress: number;
  message: string;
  timing?: ExportTiming;
}

export type ExportStep = 
  | 'selecting'
  | 'exporting_config' 
  | 'exporting_firmware'
  | 'creating_manifest'
  | 'completed'
  | 'error';

export interface ExportTiming {
  config_export_ms?: number;
  firmware_export_ms?: number;
  total_ms?: number;
}

export interface RecoveryDialogProps {
  visible: boolean;
  onClose: () => void;
  assetId: number;
  assetName: string;
  configurationVersions: ConfigurationVersionInfo[];
  firmwareVersions: FirmwareVersionInfo[];
}

export interface RecoveryProgressProps {
  progress: ExportProgress;
  onCancel?: () => void;
}

// Re-export existing types that are needed
export interface ConfigurationVersionInfo {
  id: number;
  asset_id: number;
  author_id: number;
  author_username: string;
  version_number: string;
  file_name: string;
  notes?: string;
  status: ConfigurationStatus;
  firmware_version_id?: number;
  file_size: number;
  content_hash: string;
  branch_id?: number;
  is_silver: boolean;
  promoted_from_branch_id?: number;
  promoted_from_version_id?: number;
  status_changed_at?: string;
  status_changed_by?: number;
  created_at: string;
}

export interface FirmwareVersionInfo {
  id: number;
  asset_id: number;
  author_id: number;
  author_username: string;
  vendor?: string;
  model?: string;
  version: string;
  notes?: string;
  status: FirmwareStatus;
  file_path: string;
  file_hash: string;
  file_size: number;
  created_at: string;
}

export type ConfigurationStatus = 'Draft' | 'Silver' | 'Golden' | 'Archived';
export type FirmwareStatus = 'Draft' | 'Approved' | 'Golden' | 'Archived';