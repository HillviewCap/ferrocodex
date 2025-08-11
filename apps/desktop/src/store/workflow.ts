import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { 
  WorkflowState, 
  WorkflowData, 
  WorkflowStepName, 
  WorkflowSession,
  ValidationResults,
  AutoSaveConfig,
  StartWorkflowRequest,
  StartWorkflowResponse,
  UpdateWorkflowStepRequest,
  UpdateWorkflowStepResponse,
  CompleteWorkflowRequest,
  CompleteWorkflowResponse,
  getNextStep,
  getPreviousStep,
  validateWorkflowStep
} from '../types/workflow';

interface WorkflowStore {
  // Current workflow state
  currentWorkflow: WorkflowState | null;
  session: WorkflowSession | null;
  isLoading: boolean;
  error: string | null;
  
  // Auto-save state
  autoSave: AutoSaveConfig;
  lastSaveTime: Date | null;
  autoSaveIntervalId: number | null;
  
  // Actions
  startWorkflow: (request: StartWorkflowRequest) => Promise<void>;
  updateStep: (stepName: WorkflowStepName, data: Partial<WorkflowData>) => Promise<ValidationResults>;
  navigateToStep: (stepName: WorkflowStepName) => Promise<void>;
  nextStep: () => Promise<void>;
  previousStep: () => Promise<void>;
  saveWorkflowDraft: () => Promise<void>;
  resumeWorkflow: (workflowId: string) => Promise<void>;
  completeWorkflow: () => Promise<number>; // Returns asset_id
  cancelWorkflow: () => Promise<void>;
  
  // Validation
  validateCurrentStep: () => ValidationResults;
  canNavigateNext: () => boolean;
  canNavigatePrevious: () => boolean;
  
  // Auto-save management
  enableAutoSave: (interval?: number) => void;
  disableAutoSave: () => void;
  
  // Error handling
  setError: (error: string | null) => void;
  clearError: () => void;
  
  // Reset
  reset: () => void;
}

export const useWorkflowStore = create<WorkflowStore>((set, get) => ({
  // Initial state
  currentWorkflow: null,
  session: null,
  isLoading: false,
  error: null,
  autoSave: {
    enabled: false,
    interval: 30,
    save_in_progress: false
  },
  lastSaveTime: null,
  autoSaveIntervalId: null,

  // Start a new workflow
  startWorkflow: async (request: StartWorkflowRequest) => {
    set({ isLoading: true, error: null });
    
    try {
      const response = await invoke<StartWorkflowResponse>('start_asset_creation_workflow', {
        workflowType: request.workflow_type,
        initialData: request.initial_data
      });
      
      set({
        currentWorkflow: response.state,
        session: response.session,
        isLoading: false
      });
      
      // Enable auto-save by default
      get().enableAutoSave();
      
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Failed to start workflow',
        isLoading: false
      });
    }
  },

  // Update current workflow step
  updateStep: async (stepName: WorkflowStepName, data: Partial<WorkflowData>): Promise<ValidationResults> => {
    const { currentWorkflow } = get();
    if (!currentWorkflow) {
      throw new Error('No active workflow');
    }

    set({ isLoading: true, error: null });

    try {
      const response = await invoke<UpdateWorkflowStepResponse>('update_workflow_step', {
        workflowId: currentWorkflow.id,
        stepName,
        stepData: data
      });

      set({
        currentWorkflow: response.state,
        isLoading: false
      });

      return response.validation;
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to update workflow step';
      set({ error: errorMessage, isLoading: false });
      throw new Error(errorMessage);
    }
  },

  // Navigate to specific step
  navigateToStep: async (stepName: WorkflowStepName) => {
    const { currentWorkflow } = get();
    if (!currentWorkflow) {
      throw new Error('No active workflow');
    }

    // Validate current step before navigation
    const validation = get().validateCurrentStep();
    if (!validation.is_valid && stepName !== currentWorkflow.current_step) {
      throw new Error('Current step validation failed');
    }

    set({ isLoading: true, error: null });

    try {
      await invoke('advance_workflow_step', {
        workflowId: currentWorkflow.id,
        targetStep: stepName
      });

      set((state) => ({
        currentWorkflow: state.currentWorkflow ? {
          ...state.currentWorkflow,
          current_step: stepName,
          updated_at: new Date().toISOString()
        } : null,
        isLoading: false
      }));
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Failed to navigate to step',
        isLoading: false
      });
    }
  },

  // Navigate to next step
  nextStep: async () => {
    const { currentWorkflow } = get();
    if (!currentWorkflow) {
      throw new Error('No active workflow');
    }

    const nextStepName = getNextStep(currentWorkflow.current_step);
    if (!nextStepName) {
      throw new Error('Already at last step');
    }

    await get().navigateToStep(nextStepName);
  },

  // Navigate to previous step
  previousStep: async () => {
    const { currentWorkflow } = get();
    if (!currentWorkflow) {
      throw new Error('No active workflow');
    }

    const previousStepName = getPreviousStep(currentWorkflow.current_step);
    if (!previousStepName) {
      throw new Error('Already at first step');
    }

    await get().navigateToStep(previousStepName);
  },

  // Save workflow draft
  saveWorkflowDraft: async () => {
    const { currentWorkflow, autoSave } = get();
    if (!currentWorkflow) {
      return;
    }

    if (autoSave.save_in_progress) {
      return; // Prevent concurrent saves
    }

    set((state) => ({
      autoSave: { ...state.autoSave, save_in_progress: true }
    }));

    try {
      await invoke('save_workflow_draft', {
        workflowId: currentWorkflow.id,
        draftData: currentWorkflow.data
      });

      set({
        lastSaveTime: new Date(),
        autoSave: { ...get().autoSave, save_in_progress: false }
      });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Failed to save draft',
        autoSave: { ...get().autoSave, save_in_progress: false }
      });
    }
  },

  // Resume existing workflow
  resumeWorkflow: async (workflowId: string) => {
    set({ isLoading: true, error: null });

    try {
      const state = await invoke<WorkflowState>('resume_workflow', { workflowId });
      
      set({
        currentWorkflow: state,
        isLoading: false
      });

      // Re-enable auto-save
      get().enableAutoSave();
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Failed to resume workflow',
        isLoading: false
      });
    }
  },

  // Complete workflow and create asset
  completeWorkflow: async (): Promise<number> => {
    const { currentWorkflow } = get();
    if (!currentWorkflow) {
      throw new Error('No active workflow');
    }

    // Final validation
    const validation = get().validateCurrentStep();
    if (!validation.is_valid) {
      throw new Error('Workflow validation failed');
    }

    set({ isLoading: true, error: null });

    try {
      const response = await invoke<CompleteWorkflowResponse>('complete_workflow', {
        workflowId: currentWorkflow.id
      });

      // Disable auto-save and clear workflow
      get().disableAutoSave();
      set({
        currentWorkflow: null,
        session: null,
        isLoading: false
      });

      return response.asset_id;
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Failed to complete workflow',
        isLoading: false
      });
      throw error;
    }
  },

  // Cancel workflow
  cancelWorkflow: async () => {
    const { currentWorkflow } = get();
    if (!currentWorkflow) {
      return;
    }

    set({ isLoading: true, error: null });

    try {
      await invoke('cancel_workflow', {
        workflowId: currentWorkflow.id
      });

      get().disableAutoSave();
      get().reset();
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : 'Failed to cancel workflow',
        isLoading: false
      });
    }
  },

  // Validate current step
  validateCurrentStep: (): ValidationResults => {
    const { currentWorkflow } = get();
    if (!currentWorkflow) {
      return {
        is_valid: false,
        errors: [{ field: 'workflow', message: 'No active workflow', code: 'NO_WORKFLOW' }],
        warnings: []
      };
    }

    return validateWorkflowStep(currentWorkflow.current_step, currentWorkflow.data);
  },

  // Check if can navigate to next step
  canNavigateNext: (): boolean => {
    const { currentWorkflow } = get();
    if (!currentWorkflow) {
      console.log('canNavigateNext: No current workflow');
      return false;
    }

    console.log('canNavigateNext: Current step:', currentWorkflow.current_step);
    console.log('canNavigateNext: Workflow data:', currentWorkflow.data);

    // First check if step component has provided validation results
    if (currentWorkflow.data.validation_results !== undefined) {
      const stepValidation = currentWorkflow.data.validation_results;
      const nextStep = getNextStep(currentWorkflow.current_step);
      console.log('canNavigateNext: Using step validation:', stepValidation);
      console.log('canNavigateNext: Next step available:', nextStep);
      return stepValidation.is_valid && nextStep !== null;
    }

    // Fall back to central validation if no step-specific validation
    const validation = get().validateCurrentStep();
    const nextStep = getNextStep(currentWorkflow.current_step);
    console.log('canNavigateNext: Using central validation:', validation);
    console.log('canNavigateNext: Next step available:', nextStep);
    
    return validation.is_valid && nextStep !== null;
  },

  // Check if can navigate to previous step
  canNavigatePrevious: (): boolean => {
    const { currentWorkflow } = get();
    if (!currentWorkflow) return false;

    return getPreviousStep(currentWorkflow.current_step) !== null;
  },

  // Enable auto-save
  enableAutoSave: (interval = 30) => {
    // Clear existing interval
    get().disableAutoSave();

    const autoSaveInterval = window.setInterval(() => {
      get().saveWorkflowDraft();
    }, interval * 1000);

    set({
      autoSave: {
        enabled: true,
        interval,
        save_in_progress: false
      },
      autoSaveIntervalId: autoSaveInterval
    });
  },

  // Disable auto-save
  disableAutoSave: () => {
    const { autoSaveIntervalId } = get();
    
    // Clear existing interval if it exists
    if (autoSaveIntervalId !== null) {
      window.clearInterval(autoSaveIntervalId);
    }
    
    set({
      autoSave: {
        enabled: false,
        interval: 30,
        save_in_progress: false
      },
      autoSaveIntervalId: null
    });
  },

  // Set error
  setError: (error: string | null) => {
    set({ error });
  },

  // Clear error
  clearError: () => {
    set({ error: null });
  },

  // Reset store
  reset: () => {
    // Clear auto-save interval before reset
    get().disableAutoSave();
    
    set({
      currentWorkflow: null,
      session: null,
      isLoading: false,
      error: null,
      autoSave: {
        enabled: false,
        interval: 30,
        save_in_progress: false
      },
      lastSaveTime: null,
      autoSaveIntervalId: null
    });
  }
}));