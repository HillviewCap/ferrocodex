import React, { useEffect } from 'react';
import { 
  Modal, 
  Steps, 
  Button, 
  Typography, 
  Space, 
  Alert,
  notification
} from 'antd';
import { CheckCircleOutlined } from '@ant-design/icons';
import useAssociationStore from '../../store/associations';
import AssetFileSelector from './AssetFileSelector';
import FileUploadStep from './FileUploadStep';
import AssociationReview from './AssociationReview';
import { AssetInfo, AssociationType } from '../../types/associations';

const { Title } = Typography;
const { Step } = Steps;

interface FileAssociationWizardProps {
  visible: boolean;
  onClose: () => void;
  preselectedAsset?: AssetInfo;
  fileType?: AssociationType;
}

const FileAssociationWizard: React.FC<FileAssociationWizardProps> = ({
  visible,
  onClose,
  preselectedAsset,
  fileType
}) => {
  const {
    wizardState,
    loading,
    error,
    startImportWizard,
    updateWizardState,
    nextWizardStep,
    previousWizardStep,
    completeWizard,
    cancelWizard,
    clearError
  } = useAssociationStore();

  // Initialize wizard when modal opens
  useEffect(() => {
    if (visible && !wizardState) {
      startImportWizard(preselectedAsset?.id);
      if (preselectedAsset) {
        updateWizardState({ selectedAsset: preselectedAsset });
      }
    }
  }, [visible, wizardState, preselectedAsset, startImportWizard, updateWizardState]);

  const handleClose = () => {
    cancelWizard();
    onClose();
  };

  const handleNext = async () => {
    if (!wizardState) return;

    try {
      // Validate current step before proceeding
      switch (wizardState.currentStep) {
        case 0: // Asset Selection
          if (!wizardState.selectedAsset) {
            notification.error({
              message: 'Asset Required',
              description: 'Please select an asset before proceeding.',
            });
            return;
          }
          break;
        case 1: // File Upload
          if (wizardState.selectedFiles.length === 0) {
            notification.error({
              message: 'Files Required',
              description: 'Please upload at least one file before proceeding.',
            });
            return;
          }
          break;
        case 2: // Review
          if (wizardState.associations.length === 0) {
            notification.error({
              message: 'No Associations',
              description: 'No valid associations were created. Please review your files.',
            });
            return;
          }
          break;
      }

      if (wizardState.currentStep === 2) {
        // Final step - complete the wizard
        await completeWizard();
        notification.success({
          message: 'Import Completed',
          description: `Successfully created ${wizardState.associations.length} file association(s).`,
        });
        handleClose();
      } else {
        nextWizardStep();
      }
    } catch (error) {
      console.error('Error in wizard step:', error);
      notification.error({
        message: 'Step Failed',
        description: error instanceof Error ? error.message : 'An unexpected error occurred.',
      });
    }
  };

  const handlePrevious = () => {
    previousWizardStep();
  };

  const handleAssetSelect = (asset: AssetInfo) => {
    updateWizardState({ selectedAsset: asset });
  };

  const renderCurrentStep = () => {
    if (!wizardState) return null;

    switch (wizardState.currentStep) {
      case 0:
        return (
          <AssetFileSelector
            onAssetSelect={handleAssetSelect}
            selectedAsset={wizardState.selectedAsset}
            fileType={fileType}
            disabled={loading}
          />
        );
      case 1:
        return (
          <FileUploadStep
            selectedAsset={wizardState.selectedAsset!}
            fileType={fileType}
            onFilesSelected={(files) => updateWizardState({ selectedFiles: files })}
            onUploadProgress={(uploadedFiles) => updateWizardState({ uploadedFiles })}
          />
        );
      case 2:
        return (
          <AssociationReview
            wizardState={wizardState}
            onAssociationsUpdate={(associations) => updateWizardState({ associations })}
          />
        );
      default:
        return null;
    }
  };

  const getStepStatus = (stepIndex: number) => {
    if (!wizardState) return 'wait';
    
    if (stepIndex < wizardState.currentStep) return 'finish';
    if (stepIndex === wizardState.currentStep) return 'process';
    return 'wait';
  };

  const canProceed = () => {
    if (!wizardState) return false;

    switch (wizardState.currentStep) {
      case 0:
        return !!wizardState.selectedAsset;
      case 1:
        return wizardState.selectedFiles.length > 0 && 
               wizardState.uploadedFiles.every(f => f.status === 'completed');
      case 2:
        return wizardState.associations.length > 0;
      default:
        return false;
    }
  };

  const getNextButtonText = () => {
    if (!wizardState) return 'Next';
    
    switch (wizardState.currentStep) {
      case 2:
        return 'Complete Import';
      default:
        return 'Next';
    }
  };

  return (
    <Modal
      title={
        <Space>
          <CheckCircleOutlined style={{ color: '#52c41a' }} />
          <Title level={4} style={{ margin: 0 }}>
            File Association Wizard
          </Title>
        </Space>
      }
      open={visible}
      onCancel={handleClose}
      width={800}
      maskClosable={false}
      footer={null}
      destroyOnClose
    >
      <div style={{ padding: '20px 0' }}>
        {/* Progress Steps */}
        <Steps 
          current={wizardState?.currentStep || 0} 
          style={{ marginBottom: 32 }}
          size="small"
        >
          <Step 
            title="Select Asset" 
            description="Choose target asset"
            status={getStepStatus(0)}
          />
          <Step 
            title="Upload Files" 
            description="Upload configuration/firmware files"
            status={getStepStatus(1)}
          />
          <Step 
            title="Review & Confirm" 
            description="Review associations and complete"
            status={getStepStatus(2)}
          />
        </Steps>

        {/* Error Display */}
        {error && (
          <Alert
            message="Error"
            description={error}
            type="error"
            showIcon
            closable
            onClose={clearError}
            style={{ marginBottom: 16 }}
          />
        )}

        {/* Step Content */}
        <div style={{ minHeight: 400, padding: '20px 0' }}>
          {renderCurrentStep()}
        </div>

        {/* Navigation Buttons */}
        <div style={{ 
          display: 'flex', 
          justifyContent: 'space-between', 
          marginTop: 24,
          borderTop: '1px solid #f0f0f0',
          paddingTop: 16
        }}>
          <Button 
            onClick={handleClose}
            disabled={loading}
          >
            Cancel
          </Button>
          
          <Space>
            <Button 
              onClick={handlePrevious}
              disabled={!wizardState || wizardState.currentStep === 0 || loading}
            >
              Previous
            </Button>
            <Button 
              type="primary"
              onClick={handleNext}
              loading={loading}
              disabled={!canProceed()}
            >
              {getNextButtonText()}
            </Button>
          </Space>
        </div>
      </div>
    </Modal>
  );
};

export default FileAssociationWizard;