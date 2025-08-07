// Association Management Components
export { default as FileAssociationWizard } from './FileAssociationWizard';
export { default as AssetFileSelector } from './AssetFileSelector';
export { default as FileUploadStep } from './FileUploadStep';
export { default as AssociationReview } from './AssociationReview';
export { default as AssociationVisualization } from './AssociationVisualization';

// Re-export types for convenience
export type {
  AssetFileAssociation,
  AssociationType,
  FileImportSession,
  ImportStatus,
  AssociationValidation,
  ValidationType,
  ValidationResult,
  AssociationInfo,
  AssociationSummary,
  HealthStatus,
  CreateAssociationRequest,
  DragDropAssociationData,
  AssociationWizardStep,
  FileAssociationWizardState,
  AssetSelectorProps,
  FileUploadProgress,
  AssociationPreview
} from '../../types/associations';