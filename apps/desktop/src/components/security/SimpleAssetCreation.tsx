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
  Spin,
  message 
} from 'antd';
import { 
  FolderOutlined, 
  ToolOutlined, 
  CheckCircleOutlined,
  RightOutlined
} from '@ant-design/icons';
import { AssetType, AssetHierarchy, CreateAssetRequest } from '../../types/assets';
import { SecurityClassificationLevel } from '../../types/security';
import { AssetTypeSelector } from '../hierarchy/AssetTypeSelector';
import { AssetHierarchyPicker } from '../hierarchy/AssetHierarchyPicker';
import SecurityClassificationSelector from './SecurityClassificationSelector';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../../store/auth';

const { Title, Text, Paragraph } = Typography;
const { TextArea } = Input;

export interface SimpleAssetCreationProps {
  open: boolean;
  onCancel: () => void;
  onSuccess: (asset: any) => void;
  hierarchyData: AssetHierarchy[];
  initialParentId?: number | null;
  initialAssetType?: AssetType;
  inheritedClassification?: SecurityClassificationLevel;
}

interface FormData {
  name: string;
  description: string;
  asset_type: AssetType;
  parent_id: number | null;
  security_classification: SecurityClassificationLevel;
}

export const SimpleAssetCreation: React.FC<SimpleAssetCreationProps> = ({
  open,
  onCancel,
  onSuccess,
  hierarchyData,
  initialParentId = null,
  initialAssetType,
  inheritedClassification
}) => {
  const { token, user } = useAuthStore();
  const [form] = Form.useForm<FormData>();
  const [currentStep, setCurrentStep] = useState(0);
  const [loading, setLoading] = useState(false);
  const [, forceUpdate] = useState({});
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
        // Validate details
        await form.validateFields(['name', 'description', 'security_classification']);
        const name = form.getFieldValue('name');
        const description = form.getFieldValue('description');
        const classification = form.getFieldValue('security_classification');
        if (!name || !name.trim()) {
          throw new Error('Please enter an asset name');
        }
        // Important: Save all values to formData
        setFormData(prev => ({ 
          ...prev, 
          name: name.trim(), 
          description: description || '', 
          security_classification: classification 
        }));
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

  const handleCreate = async () => {
    try {
      setLoading(true);
      
      // Get all form values
      const values = form.getFieldsValue();
      console.log('Form values:', values);
      console.log('FormData state:', formData);
      
      // Use formData for values that might not be in the form
      const finalValues = {
        ...formData,
        ...values
      };
      
      console.log('Final values:', finalValues);
      
      if (!finalValues.name || !finalValues.asset_type) {
        throw new Error('Required fields are missing');
      }

      const request: CreateAssetRequest = {
        name: finalValues.name.trim(),
        description: finalValues.description || '',
        asset_type: finalValues.asset_type,
        parent_id: finalValues.parent_id || null,
        manufacturer: '',
        model: '',
        serial_number: '',
        firmware_version: '',
        hardware_version: '',
        installation_date: null,
        last_maintenance_date: null,
        location: '',
        criticality: 'Low',
        status: 'Operational',
        metadata: {}
      };

      const result = await invoke('create_asset', {
        token,
        name: request.name,
        description: request.description,
        assetType: request.asset_type === 'Folder' ? 'folder' : 'device',
        parentId: request.parent_id
      });

      onSuccess(result);
    } catch (error) {
      console.error('Failed to create asset:', error);
      message.error(error instanceof Error ? error.message : 'Failed to create asset');
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
            <Alert 
              message="You can select a parent folder or leave empty to create at root level" 
              type="info" 
              showIcon 
              style={{ marginBottom: 16 }}
            />
            <Form.Item name="parent_id">
              <AssetHierarchyPicker
                hierarchyData={hierarchyData}
                value={formData.parent_id}
                onChange={(parentId) => setFormData(prev => ({ ...prev, parent_id: parentId }))}
                placeholder="Select parent folder (optional)"
                allowClear
              />
            </Form.Item>
          </Card>
        );

      case 2:
        return (
          <Card title="Asset Details" size="small">
            <Form.Item 
              name="name" 
              label="Asset Name"
              rules={[{ required: true, message: 'Please enter asset name' }]}
            >
              <Input
                placeholder="Enter asset name"
                maxLength={100}
                showCount
              />
            </Form.Item>
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
            <Form.Item 
              name="security_classification" 
              label="Security Classification"
              rules={[{ required: true, message: 'Please select security classification' }]}
            >
              <SecurityClassificationSelector
                value={formData.security_classification}
                onChange={(level) => setFormData(prev => ({ ...prev, security_classification: level }))}
                currentUserRole={user?.role}
                inheritedClassification={inheritedClassification}
                showDescription={false}
                showAccessRequirements={true}
              />
            </Form.Item>
          </Card>
        );

      case 3:
        return (
          <Card title="Review Asset Creation" size="small">
            <Space direction="vertical" style={{ width: '100%' }} size="middle">
              <div>
                <Text strong>Asset Type:</Text> {formData.asset_type}
              </div>
              
              <div>
                <Text strong>Name:</Text> {formData.name || form.getFieldValue('name') || 'Not set'}
              </div>
              
              <div>
                <Text strong>Security Classification:</Text> {formData.security_classification || form.getFieldValue('security_classification') || 'Not set'}
              </div>
              
              {formData.parent_id && (
                <div>
                  <Text strong>Parent Location:</Text> ID {formData.parent_id}
                </div>
              )}
              
              {formData.description && (
                <div>
                  <Text strong>Description:</Text>
                  <Paragraph style={{ marginBottom: 0, marginTop: 4 }}>
                    {formData.description}
                  </Paragraph>
                </div>
              )}
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
        return !!form.getFieldValue('asset_type');
      case 1:
        return true; // Parent is optional
      case 2:
        const name = form.getFieldValue('name');
        const classification = form.getFieldValue('security_classification');
        return !!(name && name.trim() && classification);
      case 3:
        return true;
      default:
        return false;
    }
  };

  return (
    <Modal
      title="Create Asset"
      open={open}
      onCancel={onCancel}
      width={700}
      footer={null}
      maskClosable={false}
    >
      <div style={{ padding: '16px 0' }}>
        <Steps current={currentStep} items={steps} style={{ marginBottom: 24 }} />
        
        <Form 
          form={form} 
          layout="vertical"
          onValuesChange={() => {
            // Force re-render to update button state
            forceUpdate({});
          }}
        >
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
              <>
                {loading ? (
                  <Spin />
                ) : (
                  <Button 
                    type="primary" 
                    onClick={handleCreate}
                    disabled={!canProceed()}
                  >
                    Create Asset
                  </Button>
                )}
              </>
            )}
          </Space>
        </div>
      </div>
    </Modal>
  );
};

export default SimpleAssetCreation;