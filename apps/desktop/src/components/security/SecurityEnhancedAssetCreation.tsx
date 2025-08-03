import React, { useState, useEffect } from 'react';
import { 
  Modal, 
  Form, 
  Input,
  Steps, 
  Button, 
  Space, 
  Typography, 
  Card,
  Alert,
  Result,
  Spin 
} from 'antd';
import { 
  FolderOutlined, 
  ToolOutlined, 
  CheckCircleOutlined,
  RightOutlined,
  ShieldCheckOutlined,
  SafetyCertificateOutlined
} from '@ant-design/icons';
import { AssetType, AssetHierarchy, CreateAssetRequest } from '../../types/assets';
import { SecurityValidationResult, SecurityClassificationLevel } from '../../types/security';
import { AssetTypeSelector } from '../hierarchy/AssetTypeSelector';
import { AssetHierarchyPicker } from '../hierarchy/AssetHierarchyPicker';
import SecurityValidationInput from './SecurityValidationInput';
import SecurityClassificationSelector from './SecurityClassificationSelector';
import ComplianceStatusIndicator from './ComplianceStatusIndicator';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../../store/auth';
import useSecurityStore from '../../store/security';

const { Title, Text, Paragraph } = Typography;
const { TextArea } = Input;

export interface SecurityEnhancedAssetCreationProps {
  open: boolean;
  onCancel: () => void;
  onSuccess: (asset: any) => void;
  hierarchyData: AssetHierarchy[];
  initialParentId?: number | null;
  initialAssetType?: AssetType;
  requiresSecurityValidation?: boolean;
  inheritedClassification?: SecurityClassificationLevel;
}

interface FormData {
  name: string;
  description: string;
  asset_type: AssetType;
  parent_id: number | null;
  security_classification: SecurityClassificationLevel;
}

export const SecurityEnhancedAssetCreation: React.FC<SecurityEnhancedAssetCreationProps> = ({
  open,
  onCancel,
  onSuccess,
  hierarchyData,
  initialParentId = null,
  initialAssetType,
  requiresSecurityValidation = true,
  inheritedClassification
}) => {
  const { token, user } = useAuthStore();
  const addAuditEvent = useSecurityStore(state => state.addAuditEvent);
  const addAlert = useSecurityStore(state => state.addAlert);
  const [form] = Form.useForm<FormData>();
  const [currentStep, setCurrentStep] = useState(0);
  const [loading, setLoading] = useState(false);
  const [validationResult, setValidationResult] = useState<SecurityValidationResult | null>(null);
  const [formData, setFormData] = useState<Partial<FormData>>({
    parent_id: initialParentId,
    asset_type: initialAssetType,
    security_classification: inheritedClassification || SecurityClassificationLevel.INTERNAL
  });

  // Reset form when modal opens/closes
  useEffect(() => {
    if (open) {
      setCurrentStep(0);
      form.resetFields();
      setValidationResult(null);
      setFormData({
        parent_id: initialParentId,
        asset_type: initialAssetType,
        security_classification: inheritedClassification || SecurityClassificationLevel.INTERNAL
      });
    }
  }, [open, form, initialParentId, initialAssetType, inheritedClassification]);

  // Update form when formData changes
  useEffect(() => {
    form.setFieldsValue(formData);
  }, [form, formData]);

  const steps = [
    {
      title: 'Asset Type',
      description: 'Choose asset type',
      icon: <FolderOutlined />,
    },
    {
      title: 'Location',
      description: 'Choose parent folder',
      icon: <RightOutlined />,
    },
    {
      title: 'Security',
      description: 'Security validation & classification',
      icon: <ShieldCheckOutlined />,
    },
    {
      title: 'Details',
      description: 'Enter asset information',
      icon: <ToolOutlined />,
    },
    {
      title: 'Confirm',
      description: 'Review and create',
      icon: <CheckCircleOutlined />,
    },
  ];

  const handleNext = async () => {
    try {
      if (currentStep === 0) {
        // Validate asset type
        const assetType = form.getFieldValue('asset_type');
        if (!assetType) {
          throw new Error('Please select an asset type');
        }
        setFormData(prev => ({ ...prev, asset_type: assetType }));
      } else if (currentStep === 1) {
        // Validate parent selection (optional)
        const parentId = form.getFieldValue('parent_id') || null;
        setFormData(prev => ({ ...prev, parent_id: parentId }));
      } else if (currentStep === 2) {
        // Validate security settings
        if (requiresSecurityValidation) {
          if (!validationResult?.isValid) {
            throw new Error('Asset name must pass security validation');
          }
          const classification = form.getFieldValue('security_classification');
          if (!classification) {
            throw new Error('Please select a security classification');
          }
          setFormData(prev => ({ ...prev, security_classification: classification }));
        }
      } else if (currentStep === 3) {
        // Validate details
        await form.validateFields(['description']);
        const description = form.getFieldValue('description');
        setFormData(prev => ({ ...prev, description }));
      }
      
      setCurrentStep(prev => prev + 1);
    } catch (error) {
      console.error('Step validation error:', error);
      // Error will be shown by form validation or custom error handling
    }
  };

  const handlePrevious = () => {
    setCurrentStep(prev => prev - 1);
  };

  const handleValidationChange = (result: SecurityValidationResult) => {
    setValidationResult(result);
    if (result.isValid && result.suggestedCorrections.length === 0) {
      // Auto-update form data with validated name
      setFormData(prev => ({ ...prev, name: form.getFieldValue('name') }));
    }
  };

  const handleCreate = async () => {
    try {
      setLoading(true);
      
      const request: CreateAssetRequest & { security_classification: SecurityClassificationLevel } = {
        name: formData.name!,
        description: formData.description || '',
        asset_type: formData.asset_type!,
        parent_id: formData.parent_id,
        security_classification: formData.security_classification!,
      };

      // Log security validation attempt
      addAuditEvent({
        id: Date.now(),
        timestamp: new Date().toISOString(),
        eventType: 'asset_creation_attempt',
        userId: user?.id,
        details: `Attempting to create asset: ${request.name} with classification: ${request.security_classification}`,
        result: 'pending'
      });

      const result = await invoke('create_asset_with_security', {
        token,
        name: request.name,
        description: request.description,
        assetType: request.asset_type,
        parentId: request.parent_id,
        securityClassification: request.security_classification
      });

      // Log successful creation
      addAuditEvent({
        id: Date.now() + 1,
        timestamp: new Date().toISOString(),
        eventType: 'asset_creation_success',
        userId: user?.id,
        details: `Successfully created asset: ${request.name}`,
        result: 'success'
      });

      // Add security alert if high classification
      if (request.security_classification === SecurityClassificationLevel.RESTRICTED || 
          request.security_classification === SecurityClassificationLevel.SECRET) {
        addAlert({
          timestamp: new Date().toISOString(),
          severity: 'warning',
          title: 'High Classification Asset Created',
          description: `Asset "${request.name}" created with ${request.security_classification} classification`,
          assetName: request.name,
          userName: user?.username,
          acknowledged: false
        });
      }

      onSuccess(result);
      
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Asset creation failed';
      
      // Log failed creation
      addAuditEvent({
        id: Date.now() + 2,
        timestamp: new Date().toISOString(),
        eventType: 'asset_creation_failure',
        userId: user?.id,
        details: `Failed to create asset: ${formData.name} - ${errorMessage}`,
        result: 'failure'
      });

      addAlert({
        timestamp: new Date().toISOString(),
        severity: 'critical',
        title: 'Asset Creation Failed',
        description: `Failed to create asset "${formData.name}": ${errorMessage}`,
        assetName: formData.name,
        userName: user?.username,
        acknowledged: false
      });

      console.error('Asset creation error:', error);
    } finally {
      setLoading(false);
    }
  };

  const renderStepContent = () => {
    switch (currentStep) {
      case 0:
        return (
          <Card title="Select Asset Type" size="small">
            <Form.Item name="asset_type" rules={[{ required: true, message: 'Please select asset type' }]}>
              <AssetTypeSelector
                value={formData.asset_type}
                onChange={(type) => setFormData(prev => ({ ...prev, asset_type: type }))}
              />
            </Form.Item>
          </Card>
        );

      case 1:
        return (
          <Card title="Select Parent Location" size="small">
            <Form.Item name="parent_id">
              <AssetHierarchyPicker
                hierarchyData={hierarchyData}
                value={formData.parent_id}
                onChange={(parentId) => setFormData(prev => ({ ...prev, parent_id: parentId }))}
                placeholder="Select parent folder (optional)"
              />
            </Form.Item>
          </Card>
        );

      case 2:
        return (
          <Card 
            title={
              <Space>
                <SafetyCertificateOutlined />
                <span>Security Validation & Classification</span>
              </Space>
            } 
            size="small"
          >
            <Space direction="vertical" style={{ width: '100%' }} size="large">
              {/* Asset name validation */}
              <div>
                <Title level={5}>Asset Name</Title>
                <Form.Item 
                  name="name" 
                  rules={[
                    { required: true, message: 'Please enter asset name' },
                    { 
                      validator: () => 
                        validationResult?.isValid ? Promise.resolve() : Promise.reject('Name must pass security validation') 
                    }
                  ]}
                >
                  <SecurityValidationInput
                    value={form.getFieldValue('name') || ''}
                    onChange={(value) => {
                      form.setFieldValue('name', value);
                      setFormData(prev => ({ ...prev, name: value }));
                    }}
                    onValidationChange={handleValidationChange}
                    placeholder="Enter secure asset name"
                    showSuggestions={true}
                  />
                </Form.Item>
              </div>

              {/* Security classification */}
              <div>
                <Title level={5}>Security Classification</Title>
                <Form.Item 
                  name="security_classification" 
                  rules={[{ required: true, message: 'Please select security classification' }]}
                >
                  <SecurityClassificationSelector
                    value={formData.security_classification}
                    onChange={(level) => setFormData(prev => ({ ...prev, security_classification: level }))}
                    currentUserRole={user?.role}
                    inheritedClassification={inheritedClassification}
                    showDescription={true}
                    showAccessRequirements={true}
                  />
                </Form.Item>
              </div>

              {/* Validation status */}
              {validationResult && (
                <div>
                  <Title level={5}>Validation Status</Title>
                  <ComplianceStatusIndicator
                    status={{
                      level: validationResult.isValid ? 'excellent' : 'needs-attention',
                      score: validationResult.isValid ? 100 : 0,
                      issues: validationResult.isValid ? [] : [validationResult.errorMessage || 'Validation failed'],
                      recommendations: validationResult.suggestedCorrections
                    }}
                    showScore={true}
                    showDetails={true}
                  />
                </div>
              )}
            </Space>
          </Card>
        );

      case 3:
        return (
          <Card title="Asset Details" size="small">
            <Form.Item 
              name="description" 
              label="Description"
              rules={[{ required: false }]}
            >
              <TextArea
                rows={4}
                placeholder="Enter asset description (optional)"
                maxLength={500}
                showCount
              />
            </Form.Item>
          </Card>
        );

      case 4:
        return (
          <Card title="Review Asset Creation" size="small">
            <Space direction="vertical" style={{ width: '100%' }} size="middle">
              <div>
                <Text strong>Asset Type:</Text> {formData.asset_type}
              </div>
              
              <div>
                <Text strong>Name:</Text> {formData.name}
                {validationResult?.isValid && (
                  <CheckCircleOutlined style={{ color: '#52c41a', marginLeft: 8 }} />
                )}
              </div>
              
              <div>
                <Text strong>Security Classification:</Text> {formData.security_classification}
              </div>
              
              {formData.parent_id && (
                <div>
                  <Text strong>Parent Location:</Text> Selected folder ID {formData.parent_id}
                </div>
              )}
              
              {formData.description && (
                <div>
                  <Text strong>Description:</Text>
                  <Paragraph style={{ marginTop: 4, fontSize: '13px' }}>
                    {formData.description}
                  </Paragraph>
                </div>
              )}

              {/* Security summary */}
              <Alert
                message="Security Summary"
                description={
                  <div>
                    <div>✓ Asset name passed security validation</div>
                    <div>✓ Security classification: {formData.security_classification}</div>
                    {inheritedClassification && (
                      <div>ℹ️ Inherits classification from parent: {inheritedClassification}</div>
                    )}
                  </div>
                }
                type="success"
                showIcon
                style={{ marginTop: 16 }}
              />
            </Space>
          </Card>
        );

      default:
        return null;
    }
  };

  const canProceed = () => {
    switch (currentStep) {
      case 0:
        return !!formData.asset_type;
      case 1:
        return true; // Parent is optional
      case 2:
        return validationResult?.isValid && !!formData.security_classification;
      case 3:
        return true; // Description is optional
      default:
        return false;
    }
  };

  return (
    <Modal
      title={
        <Space>
          <SafetyCertificateOutlined />
          <span>Create Secure Asset</span>
        </Space>
      }
      open={open}
      onCancel={onCancel}
      width={800}
      footer={null}
      destroyOnClose
    >
      <div style={{ padding: '16px 0' }}>
        <Steps current={currentStep} items={steps} style={{ marginBottom: 24 }} />
        
        <Form form={form} layout="vertical">
          {renderStepContent()}
        </Form>

        <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: 24 }}>
          <Button onClick={currentStep > 0 ? handlePrevious : onCancel}>
            {currentStep > 0 ? 'Previous' : 'Cancel'}
          </Button>
          
          <Space>
            {currentStep < steps.length - 1 ? (
              <Button 
                type="primary" 
                onClick={handleNext}
                disabled={!canProceed()}
              >
                Next
              </Button>
            ) : (
              <Button 
                type="primary" 
                onClick={handleCreate}
                loading={loading}
                disabled={!canProceed()}
              >
                Create Asset
              </Button>
            )}
          </Space>
        </div>
      </div>
    </Modal>
  );
};

export default SecurityEnhancedAssetCreation;