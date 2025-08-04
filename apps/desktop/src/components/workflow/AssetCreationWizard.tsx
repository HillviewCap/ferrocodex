import React, { useEffect, useState } from 'react';
import { Steps, Card, Button, Space, message, Spin, Modal } from 'antd';
import { ExclamationCircleOutlined, SaveOutlined } from '@ant-design/icons';
import { useWorkflowStore } from '../../store/workflow';
import { WorkflowStepName, WORKFLOW_STEPS, getStepIndex, ValidationError, WorkflowData } from '../../types/workflow';
import { AssetTypeSelectionStep } from './steps/AssetTypeSelectionStep';
import { HierarchySelectionStep } from './steps/HierarchySelectionStep';
import { MetadataConfigurationStep } from './steps/MetadataConfigurationStep';
import { SecurityValidationStep } from './steps/SecurityValidationStep';
import { ReviewConfirmationStep } from './steps/ReviewConfirmationStep';
import { WorkflowProgressIndicator } from './WorkflowProgressIndicator';
import { DraftSaveIndicator } from './DraftSaveIndicator';

const { Step } = Steps;
const { confirm } = Modal;

interface AssetCreationWizardProps {
  onComplete?: (assetId: number) => void;
  onCancel?: () => void;
  initialData?: Record<string, any>;
}

export const AssetCreationWizard: React.FC<AssetCreationWizardProps> = ({
  onComplete,
  onCancel,
  initialData
}) => {
  const {
    currentWorkflow,
    isLoading,
    error,
    autoSave,
    lastSaveTime,
    startWorkflow,
    nextStep,
    previousStep,
    completeWorkflow,
    cancelWorkflow,
    validateCurrentStep,
    canNavigateNext,
    canNavigatePrevious,
    saveWorkflowDraft,
    clearError
  } = useWorkflowStore();

  const [isInitialized, setIsInitialized] = useState(false);
  const [currentStepKey, setCurrentStepKey] = useState<WorkflowStepName>('asset_type_selection');

  // Initialize workflow on mount
  useEffect(() => {
    if (!isInitialized && !currentWorkflow) {
      initializeWorkflow();
    }
  }, [isInitialized, currentWorkflow]);

  // Update current step when workflow changes
  useEffect(() => {
    if (currentWorkflow) {
      setCurrentStepKey(currentWorkflow.current_step);
    }
  }, [currentWorkflow]);

  // Handle errors
  useEffect(() => {
    if (error) {
      message.error(error);
      clearError();
    }
  }, [error, clearError]);

  const initializeWorkflow = async () => {
    try {
      await startWorkflow({
        workflow_type: 'asset_creation',
        initial_data: initialData
      });
      setIsInitialized(true);
    } catch (err) {
      message.error('Failed to initialize asset creation workflow');
      onCancel?.();
    }
  };

  const handleNext = async () => {
    try {
      const validation = validateCurrentStep();
      if (!validation.is_valid) {
        message.error('Please complete all required fields before continuing');
        return;
      }

      await nextStep();
    } catch (err) {
      message.error('Failed to advance to next step');
    }
  };

  const handlePrevious = async () => {
    try {
      await previousStep();
    } catch (err) {
      message.error('Failed to go back to previous step');
    }
  };

  const handleComplete = async () => {
    try {
      const validation = validateCurrentStep();
      if (!validation.is_valid) {
        message.error('Please complete all required fields');
        return;
      }

      const assetId = await completeWorkflow();
      message.success('Asset created successfully!');
      onComplete?.(assetId);
    } catch (err) {
      message.error('Failed to create asset');
    }
  };

  const handleCancel = () => {
    confirm({
      title: 'Cancel Asset Creation',
      icon: <ExclamationCircleOutlined />,
      content: 'Are you sure you want to cancel? All progress will be lost.',
      okText: 'Yes, Cancel',
      okType: 'danger',
      cancelText: 'Continue Working',
      onOk: async () => {
        try {
          await cancelWorkflow();
          onCancel?.();
        } catch (err) {
          message.error('Failed to cancel workflow');
        }
      }
    });
  };

  const handleManualSave = async () => {
    try {
      await saveWorkflowDraft();
      message.success('Progress saved');
    } catch (err) {
      message.error('Failed to save progress');
    }
  };

  const handleDataChange = async (data: Partial<WorkflowData>) => {
    if (!currentWorkflow) return;
    
    try {
      await updateStep(currentWorkflow.current_step, data);
    } catch (error) {
      console.error('Failed to update step data:', error);
    }
  };

  const handleValidation = (isValid: boolean, errors?: ValidationError[]) => {
    // Store validation results in the workflow data for canNavigateNext to use
    if (currentWorkflow) {
      const validationResults = {
        is_valid: isValid,
        errors: errors || [],
        warnings: []
      };
      handleDataChange({ validation_results: validationResults });
    }
  };

  const renderStepContent = () => {
    if (!currentWorkflow) {
      return <div>Loading workflow...</div>;
    }

    // Debug logging
    console.log('Current step key:', currentStepKey);
    console.log('Current workflow data:', currentWorkflow.data);
    console.log('Can navigate next:', canNavigateNext());

    const baseProps = {
      workflowId: currentWorkflow.id,
      data: currentWorkflow.data,
      onDataChange: handleDataChange,
      onValidation: handleValidation,
      onNext: handleNext,
      onPrevious: handlePrevious
    };

    switch (currentStepKey) {
      case 'asset_type_selection':
        return <AssetTypeSelectionStep {...baseProps} />;
      case 'hierarchy_selection':
        return <HierarchySelectionStep {...baseProps} />;
      case 'metadata_configuration':
        return <MetadataConfigurationStep {...baseProps} />;
      case 'security_validation':
        return <SecurityValidationStep {...baseProps} />;
      case 'review_confirmation':
        return <ReviewConfirmationStep {...baseProps} onComplete={handleComplete} />;
      default:
        return <div>Unknown step: {currentStepKey}</div>;
    }
  };

  const getCurrentStepIndex = () => {
    return getStepIndex(currentStepKey);
  };

  const renderSteps = () => {
    return WORKFLOW_STEPS.map((step, index) => {
      let status: 'wait' | 'process' | 'finish' | 'error' = 'wait';
      
      if (index < getCurrentStepIndex()) {
        status = 'finish';
      } else if (index === getCurrentStepIndex()) {
        status = 'process';
      }

      return (
        <Step
          key={step.name}
          title={step.title}
          description={step.description}
          status={status}
        />
      );
    });
  };

  if (!isInitialized || !currentWorkflow) {
    return (
      <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '400px' }}>
        <Spin size="large" tip="Initializing asset creation workflow..." />
      </div>
    );
  }

  const currentStepIndex = getCurrentStepIndex();
  const isFirstStep = currentStepIndex === 0;
  const isLastStep = currentStepIndex === WORKFLOW_STEPS.length - 1;

  return (
    <div style={{ maxWidth: '1000px', margin: '0 auto', padding: '24px' }}>
      <Card>
        <div style={{ marginBottom: '24px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
            <h2 style={{ margin: 0 }}>Create New Asset</h2>
            <Space>
              <DraftSaveIndicator
                autoSave={autoSave}
                lastSaveTime={lastSaveTime}
                onManualSave={handleManualSave}
              />
              <Button
                icon={<SaveOutlined />}
                onClick={handleManualSave}
                loading={autoSave.save_in_progress}
                disabled={isLoading}
              >
                Save Draft
              </Button>
            </Space>
          </div>
          
          <WorkflowProgressIndicator
            currentStep={currentStepIndex}
            totalSteps={WORKFLOW_STEPS.length}
            steps={WORKFLOW_STEPS}
          />
        </div>

        <Steps current={currentStepIndex} style={{ marginBottom: '32px' }}>
          {renderSteps()}
        </Steps>

        <div style={{ minHeight: '400px', marginBottom: '24px' }}>
          <Spin spinning={isLoading}>
            {renderStepContent()}
          </Spin>
        </div>

        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <Button onClick={handleCancel} disabled={isLoading}>
            Cancel
          </Button>
          
          <Space>
            <Button
              onClick={handlePrevious}
              disabled={isFirstStep || !canNavigatePrevious() || isLoading}
            >
              Previous
            </Button>
            
            {isLastStep ? (
              <Button
                type="primary"
                onClick={handleComplete}
                disabled={!canNavigateNext() || isLoading}
                loading={isLoading}
              >
                Create Asset
              </Button>
            ) : (
              <Button
                type="primary"
                onClick={handleNext}
                disabled={!canNavigateNext() || isLoading}
                loading={isLoading}
              >
                Next
              </Button>
            )}
          </Space>
        </div>
      </Card>
    </div>
  );
};