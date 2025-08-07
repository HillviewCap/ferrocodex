import React, { useState, useEffect, useCallback } from 'react';
import { message } from 'antd';
import { 
  WorkflowStepName, 
  WorkflowStepConfig, 
  ValidationResults,
  WORKFLOW_STEPS,
  validateWorkflowStep,
  getNextStep,
  getPreviousStep,
  getStepIndex
} from '../../types/workflow';
import { useWorkflowStore } from '../../store/workflow';

interface WorkflowStepManagerProps {
  workflowId: string;
  currentStep: WorkflowStepName;
  onStepChange: (step: WorkflowStepName) => void;
  onValidationChange: (isValid: boolean, errors?: any[]) => void;
}

export const WorkflowStepManager: React.FC<WorkflowStepManagerProps> = ({
  workflowId,
  currentStep,
  onStepChange,
  onValidationChange
}) => {
  const { currentWorkflow, updateStep, navigateToStep } = useWorkflowStore();
  const [stepRegistry] = useState<Map<WorkflowStepName, WorkflowStepConfig>>(
    new Map(WORKFLOW_STEPS.map(step => [step.name, step]))
  );
  const [validationCache] = useState<Map<WorkflowStepName, ValidationResults>>(new Map());
  const [stepTransitionLock, setStepTransitionLock] = useState(false);

  // Get current step configuration
  const getCurrentStepConfig = useCallback((): WorkflowStepConfig | null => {
    return stepRegistry.get(currentStep) || null;
  }, [currentStep, stepRegistry]);

  // Validate a specific step
  const validateStep = useCallback((
    stepName: WorkflowStepName, 
    stepData?: any
  ): ValidationResults => {
    const workflowData = stepData || currentWorkflow?.data || {};
    const validation = validateWorkflowStep(stepName, workflowData);
    
    // Cache validation result
    validationCache.set(stepName, validation);
    
    return validation;
  }, [currentWorkflow, validationCache]);

  // Check if step navigation is allowed
  const canNavigateToStep = useCallback((targetStep: WorkflowStepName): boolean => {
    const currentIndex = getStepIndex(currentStep);
    const targetIndex = getStepIndex(targetStep);
    
    if (currentIndex === -1 || targetIndex === -1) {
      return false;
    }

    // Can always go backwards
    if (targetIndex < currentIndex) {
      return true;
    }

    // Can go forward only if current step is valid
    const currentValidation = validateStep(currentStep);
    return currentValidation.is_valid;
  }, [currentStep, validateStep]);

  // Handle step transition with validation
  const handleStepTransition = useCallback(async (
    targetStep: WorkflowStepName,
    skipValidation: boolean = false
  ): Promise<boolean> => {
    if (stepTransitionLock) {
      return false;
    }

    setStepTransitionLock(true);

    try {
      // Check if navigation is allowed
      if (!skipValidation && !canNavigateToStep(targetStep)) {
        message.error('Cannot navigate to this step. Please complete current step first.');
        return false;
      }

      // Validate current step before transition (unless skipping)
      if (!skipValidation) {
        const currentValidation = validateStep(currentStep);
        if (!currentValidation.is_valid) {
          message.error('Please complete all required fields before proceeding.');
          onValidationChange(false, currentValidation.errors);
          return false;
        }
      }

      // Perform step transition
      await navigateToStep(targetStep);
      onStepChange(targetStep);
      
      // Validate new step
      const newStepValidation = validateStep(targetStep);
      onValidationChange(newStepValidation.is_valid, newStepValidation.errors);

      return true;
    } catch (error) {
      console.error('Step transition failed:', error);
      message.error('Failed to navigate to step');
      return false;
    } finally {
      setStepTransitionLock(false);
    }
  }, [
    stepTransitionLock,
    canNavigateToStep,
    validateStep,
    currentStep,
    navigateToStep,
    onStepChange,
    onValidationChange
  ]);

  // Navigate to next step
  const goToNextStep = useCallback(async (): Promise<boolean> => {
    const nextStep = getNextStep(currentStep);
    if (!nextStep) {
      return false;
    }
    return await handleStepTransition(nextStep);
  }, [currentStep, handleStepTransition]);

  // Navigate to previous step
  const goToPreviousStep = useCallback(async (): Promise<boolean> => {
    const previousStep = getPreviousStep(currentStep);
    if (!previousStep) {
      return false;
    }
    return await handleStepTransition(previousStep, true); // Skip validation when going back
  }, [currentStep, handleStepTransition]);

  // Update step data with validation
  const updateStepData = useCallback(async (
    stepName: WorkflowStepName,
    data: any
  ): Promise<ValidationResults> => {
    try {
      // Update the workflow step
      const validation = await updateStep(stepName, data);
      
      // Cache validation result
      validationCache.set(stepName, validation);
      
      // Notify parent of validation change if this is the current step
      if (stepName === currentStep) {
        onValidationChange(validation.is_valid, validation.errors);
      }
      
      return validation;
    } catch (error) {
      console.error('Failed to update step data:', error);
      const errorValidation: ValidationResults = {
        is_valid: false,
        errors: [{ field: 'update', message: 'Failed to update step data', code: 'UPDATE_ERROR' }],
        warnings: []
      };
      
      validationCache.set(stepName, errorValidation);
      if (stepName === currentStep) {
        onValidationChange(false, errorValidation.errors);
      }
      
      return errorValidation;
    }
  }, [updateStep, validationCache, currentStep, onValidationChange]);

  // Get step completion status
  const getStepCompletionStatus = useCallback((stepName: WorkflowStepName): {
    isCompleted: boolean;
    isValid: boolean;
    canNavigate: boolean;
  } => {
    const validation = validationCache.get(stepName) || validateStep(stepName);
    const canNavigate = canNavigateToStep(stepName);
    
    return {
      isCompleted: validation.is_valid,
      isValid: validation.is_valid,
      canNavigate
    };
  }, [validationCache, validateStep, canNavigateToStep]);

  // Get all steps with their status
  const getAllStepsStatus = useCallback(() => {
    return WORKFLOW_STEPS.map(step => ({
      ...step,
      ...getStepCompletionStatus(step.name),
      isCurrent: step.name === currentStep
    }));
  }, [getStepCompletionStatus, currentStep]);

  // Validate current step on mount and data changes
  useEffect(() => {
    if (currentWorkflow) {
      const validation = validateStep(currentStep);
      onValidationChange(validation.is_valid, validation.errors);
    }
  }, [currentStep, currentWorkflow, validateStep, onValidationChange]);

  // Expose methods through ref or context if needed
  const stepManager = {
    currentStep,
    getCurrentStepConfig,
    validateStep,
    canNavigateToStep,
    goToNextStep,
    goToPreviousStep,
    handleStepTransition,
    updateStepData,
    getStepCompletionStatus,
    getAllStepsStatus,
    stepRegistry,
    validationCache,
    isTransitioning: stepTransitionLock
  };

  // This component doesn't render anything directly
  // It's a logic component that manages step coordination
  return null;
};

// Hook to use step manager functionality
export const useStepManager = () => {
  const { currentWorkflow } = useWorkflowStore();
  
  if (!currentWorkflow) {
    throw new Error('useStepManager can only be used within an active workflow');
  }

  return {
    currentStep: currentWorkflow.current_step,
    workflowId: currentWorkflow.id,
    data: currentWorkflow.data
  };
};