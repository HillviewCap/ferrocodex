import { AssetType } from './assets';

// Workflow State Management
export interface WorkflowState {
  id: string;
  workflow_type: WorkflowType;
  current_step: WorkflowStepName;
  user_id: number;
  status: WorkflowStatus;
  data: WorkflowData;
  created_at: string;
  updated_at: string;
  completed_at?: string;
}

export type WorkflowType = 'asset_creation';

export type WorkflowStatus = 'active' | 'paused' | 'completed' | 'cancelled' | 'error';

export type WorkflowStepName = 
  | 'asset_type_selection'
  | 'hierarchy_selection' 
  | 'metadata_configuration'
  | 'security_validation'
  | 'review_confirmation';

// Workflow Data Structure
export interface WorkflowData {
  asset_type?: AssetType;
  asset_name?: string;
  asset_description?: string;
  parent_id?: number | null;
  parent_path?: string;
  metadata_schema_id?: number;
  metadata_values?: Record<string, any>;
  security_classification?: string;
  validation_results?: ValidationResults;
}

// Step Configuration
export interface WorkflowStepConfig {
  name: WorkflowStepName;
  title: string;
  description?: string;
  component: string;
  validation_rules: ValidationRule[];
  required_fields: string[];
  optional_fields: string[];
}

export interface ValidationRule {
  field: string;
  type: 'required' | 'pattern' | 'custom';
  value?: any;
  message: string;
}

export interface ValidationResults {
  is_valid: boolean;
  errors: ValidationError[];
  warnings: ValidationWarning[];
}

export interface ValidationError {
  field: string;
  message: string;
  code: string;
}

export interface ValidationWarning {
  field: string;
  message: string;
  code: string;
}

// Draft Management
export interface WorkflowDraft {
  id: string;
  workflow_id: string;
  draft_data: WorkflowData;
  created_at: string;
  updated_at: string;
}

// Step Components Props
export interface BaseStepProps {
  workflowId: string;
  data: WorkflowData;
  onDataChange: (data: Partial<WorkflowData>) => void;
  onValidation: (isValid: boolean, errors?: ValidationError[]) => void;
  onNext: () => void;
  onPrevious: () => void;
}

export interface AssetTypeSelectionData {
  asset_type: AssetType;
  asset_name: string;
  asset_description: string;
}

export interface HierarchySelectionData {
  parent_id: number | null;
  parent_path?: string;
}

export interface MetadataConfigurationData {
  metadata_schema_id: number;
  metadata_values: Record<string, any>;
}

export interface SecurityValidationData {
  security_classification: string;
  naming_compliance: boolean;
  validation_passed: boolean;
}

// Wizard Navigation
export interface WizardStep {
  key: WorkflowStepName;
  title: string;
  description?: string;
  status: 'wait' | 'process' | 'finish' | 'error';
  disabled?: boolean;
}

export interface WizardNavigation {
  steps: WizardStep[];
  current: number;
  canGoNext: boolean;
  canGoPrevious: boolean;
  isFirstStep: boolean;
  isLastStep: boolean;
}

// Auto-save Configuration
export interface AutoSaveConfig {
  enabled: boolean;
  interval: number; // seconds
  last_saved?: string;
  save_in_progress: boolean;
}

// Workflow Session
export interface WorkflowSession {
  workflow_id: string;
  session_token: string;
  expires_at: string;
  auto_save: AutoSaveConfig;
}

// IPC Command Interfaces
export interface StartWorkflowRequest {
  workflow_type: WorkflowType;
  initial_data?: Partial<WorkflowData>;
}

export interface StartWorkflowResponse {
  session: WorkflowSession;
  state: WorkflowState;
}

export interface UpdateWorkflowStepRequest {
  workflow_id: string;
  step_name: WorkflowStepName;
  step_data: Partial<WorkflowData>;
}

export interface UpdateWorkflowStepResponse {
  state: WorkflowState;
  validation: ValidationResults;
}

export interface CompleteWorkflowRequest {
  workflow_id: string;
}

export interface CompleteWorkflowResponse {
  asset_id: number;
  workflow_state: WorkflowState;
}

// Workflow Configuration
export const WORKFLOW_STEPS: WorkflowStepConfig[] = [
  {
    name: 'asset_type_selection',
    title: 'Asset Type',
    description: 'Choose the type of asset to create',
    component: 'AssetTypeSelectionStep',
    validation_rules: [
      { field: 'asset_type', type: 'required', message: 'Asset type is required' },
      { field: 'asset_name', type: 'required', message: 'Asset name is required' },
      { field: 'asset_name', type: 'pattern', value: /^[a-zA-Z0-9\s\-_\.]+$/, message: 'Invalid asset name format' }
    ],
    required_fields: ['asset_type', 'asset_name'],
    optional_fields: ['asset_description']
  },
  {
    name: 'hierarchy_selection',
    title: 'Location',
    description: 'Select where to place this asset in the hierarchy',
    component: 'HierarchySelectionStep',
    validation_rules: [
      { field: 'parent_id', type: 'custom', message: 'Valid parent folder required for devices' }
    ],
    required_fields: [],
    optional_fields: ['parent_id']
  },
  {
    name: 'metadata_configuration',
    title: 'Metadata',
    description: 'Configure asset metadata and properties',
    component: 'MetadataConfigurationStep',
    validation_rules: [
      { field: 'metadata_schema_id', type: 'required', message: 'Metadata schema is required' }
    ],
    required_fields: ['metadata_schema_id'],
    optional_fields: ['metadata_values']
  },
  {
    name: 'security_validation',
    title: 'Security',
    description: 'Set security classification and validate compliance',
    component: 'SecurityValidationStep',
    validation_rules: [
      { field: 'security_classification', type: 'required', message: 'Security classification is required' },
      { field: 'naming_compliance', type: 'custom', message: 'Asset name must comply with security standards' }
    ],
    required_fields: ['security_classification'],
    optional_fields: []
  },
  {
    name: 'review_confirmation',
    title: 'Review',
    description: 'Review and confirm asset creation',
    component: 'ReviewConfirmationStep',
    validation_rules: [],
    required_fields: [],
    optional_fields: []
  }
];

// Validation Helpers
export const validateWorkflowStep = (
  step: WorkflowStepName, 
  data: WorkflowData
): ValidationResults => {
  const stepConfig = WORKFLOW_STEPS.find(s => s.name === step);
  if (!stepConfig) {
    return {
      is_valid: false,
      errors: [{ field: 'step', message: 'Invalid workflow step', code: 'INVALID_STEP' }],
      warnings: []
    };
  }

  const errors: ValidationError[] = [];
  const warnings: ValidationWarning[] = [];

  // Check required fields
  for (const field of stepConfig.required_fields) {
    if (!data[field as keyof WorkflowData]) {
      errors.push({
        field,
        message: `${field} is required`,
        code: 'REQUIRED_FIELD'
      });
    }
  }

  // Run validation rules
  for (const rule of stepConfig.validation_rules) {
    const fieldValue = data[rule.field as keyof WorkflowData];
    
    switch (rule.type) {
      case 'required':
        if (!fieldValue) {
          errors.push({
            field: rule.field,
            message: rule.message,
            code: 'REQUIRED'
          });
        }
        break;
      case 'pattern':
        if (fieldValue && typeof fieldValue === 'string' && rule.value) {
          if (!rule.value.test(fieldValue)) {
            errors.push({
              field: rule.field,
              message: rule.message,
              code: 'PATTERN_MISMATCH'
            });
          }
        }
        break;
      case 'custom':
        // Custom validation handled by individual step components
        break;
    }
  }

  return {
    is_valid: errors.length === 0,
    errors,
    warnings
  };
};

export const getStepIndex = (stepName: WorkflowStepName): number => {
  return WORKFLOW_STEPS.findIndex(s => s.name === stepName);
};

export const getNextStep = (currentStep: WorkflowStepName): WorkflowStepName | null => {
  const currentIndex = getStepIndex(currentStep);
  if (currentIndex >= 0 && currentIndex < WORKFLOW_STEPS.length - 1) {
    return WORKFLOW_STEPS[currentIndex + 1].name;
  }
  return null;
};

export const getPreviousStep = (currentStep: WorkflowStepName): WorkflowStepName | null => {
  const currentIndex = getStepIndex(currentStep);
  if (currentIndex > 0) {
    return WORKFLOW_STEPS[currentIndex - 1].name;
  }
  return null;
};