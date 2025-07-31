import React, { useState, useEffect } from 'react';
import { 
  Modal, 
  Form, 
  Input, 
  Steps, 
  Button, 
  Space, 
  Typography, 
  Divider,
  message,
  Spin 
} from 'antd';
import { 
  FolderOutlined, 
  ToolOutlined, 
  CheckCircleOutlined,
  RightOutlined 
} from '@ant-design/icons';
import { AssetType, AssetHierarchy, CreateAssetRequest } from '../../types/assets';
import { AssetTypeSelector } from './AssetTypeSelector';
import { AssetHierarchyPicker } from './AssetHierarchyPicker';
import { validateAssetName, validateAssetDescription } from '../../types/assets';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '../../store/auth';

const { Title, Text, Paragraph } = Typography;
const { TextArea } = Input;

export interface CreateAssetModalProps {
  open: boolean;
  onCancel: () => void;
  onSuccess: (asset: any) => void;
  hierarchyData: AssetHierarchy[];
  initialParentId?: number | null;
  initialAssetType?: AssetType;
}

interface FormData {
  name: string;
  description: string;
  asset_type: AssetType;
  parent_id: number | null;
}

export const CreateAssetModal: React.FC<CreateAssetModalProps> = ({
  open,
  onCancel,
  onSuccess,
  hierarchyData,
  initialParentId = null,
  initialAssetType,
}) => {
  const { token } = useAuthStore();
  const [form] = Form.useForm<FormData>();
  const [currentStep, setCurrentStep] = useState(0);
  const [loading, setLoading] = useState(false);
  const [formData, setFormData] = useState<Partial<FormData>>({
    parent_id: initialParentId,
    asset_type: initialAssetType,
  });

  // Reset form when modal opens/closes
  useEffect(() => {
    if (open) {
      setCurrentStep(0);
      form.resetFields();
      setFormData({
        parent_id: initialParentId,
        asset_type: initialAssetType,
      });
    }
  }, [open, form, initialParentId, initialAssetType]);

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
          message.error('Please select an asset type');
          return;
        }
        setFormData(prev => ({ ...prev, asset_type: assetType }));
      } else if (currentStep === 1) {
        // Validate parent selection (optional)
        const parentId = form.getFieldValue('parent_id') || null;
        setFormData(prev => ({ ...prev, parent_id: parentId }));
      } else if (currentStep === 2) {
        // Validate name and description
        await form.validateFields(['name', 'description']);
        const values = form.getFieldsValue(['name', 'description']);
        setFormData(prev => ({ ...prev, ...values }));
      }
      
      setCurrentStep(prev => prev + 1);
    } catch (error) {
      // Form validation failed
      console.error('Form validation error:', error);
    }
  };

  const handlePrevious = () => {
    setCurrentStep(prev => prev - 1);
  };

  const handleCreate = async () => {
    try {
      setLoading(true);
      
      const request: CreateAssetRequest = {
        name: formData.name!,
        description: formData.description || '',
        asset_type: formData.asset_type!,
        parent_id: formData.parent_id,
      };

      const result = await invoke('create_asset', {
        token,
        name: request.name,
        description: request.description,
        assetType: request.asset_type === 'Folder' ? 'folder' : 'device',
        parentId: request.parent_id,
      });

      message.success(`${formData.asset_type} created successfully!`);
      onSuccess(result);
      handleCancel();
    } catch (error) {
      console.error('Failed to create asset:', error);
      message.error(`Failed to create ${formData.asset_type?.toLowerCase()}: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleCancel = () => {
    setCurrentStep(0);
    form.resetFields();
    setFormData({});
    onCancel();
  };

  const getParentPath = (parentId: number | null): string => {
    if (!parentId) return 'Root Level';
    
    const findPath = (assets: AssetHierarchy[], targetId: number, path: string[] = []): string[] | null => {
      for (const asset of assets) {
        const currentPath = [...path, asset.name];
        if (asset.id === targetId) {
          return currentPath;
        }
        if (asset.children.length > 0) {
          const childPath = findPath(asset.children, targetId, currentPath);
          if (childPath) return childPath;
        }
      }
      return null;
    };

    const path = findPath(hierarchyData, parentId);
    return path ? path.join(' > ') : 'Unknown Location';
  };

  const renderStepContent = () => {
    switch (currentStep) {
      case 0:
        return (
          <Form.Item name="asset_type" rules={[{ required: true, message: 'Please select an asset type' }]}>
            <AssetTypeSelector
              value={formData.asset_type}
              onChange={(value) => setFormData(prev => ({ ...prev, asset_type: value }))}
            />
          </Form.Item>
        );

      case 1:
        return (
          <div>
            <Title level={5} style={{ marginBottom: 16 }}>Parent Location</Title>
            <Form.Item name="parent_id">
              <AssetHierarchyPicker
                hierarchyData={hierarchyData}
                value={formData.parent_id}
                onChange={(value) => setFormData(prev => ({ ...prev, parent_id: value }))}
                placeholder="Select parent folder (optional)"
              />
            </Form.Item>
          </div>
        );

      case 2:
        return (
          <Space direction="vertical" size="middle" style={{ width: '100%' }}>
            <div>
              <Title level={5} style={{ marginBottom: 16 }}>Asset Information</Title>
              <Form.Item
                name="name"
                label="Name"
                rules={[
                  { required: true, message: 'Asset name is required' },
                  { validator: (_, value) => {
                    const error = validateAssetName(value);
                    return error ? Promise.reject(error) : Promise.resolve();
                  }}
                ]}
              >
                <Input
                  placeholder={`Enter ${formData.asset_type?.toLowerCase()} name`}
                  size="large"
                />
              </Form.Item>

              <Form.Item
                name="description"
                label="Description"
                rules={[{
                  validator: (_, value) => {
                    const error = validateAssetDescription(value || '');
                    return error ? Promise.reject(error) : Promise.resolve();
                  }
                }]}
              >
                <TextArea
                  placeholder={`Describe this ${formData.asset_type?.toLowerCase()}`}
                  rows={3}
                  showCount
                  maxLength={500}
                />
              </Form.Item>
            </div>
          </Space>
        );

      case 3:
        return (
          <div>
            <Title level={5} style={{ marginBottom: 16 }}>Review and Confirm</Title>
            <Space direction="vertical" size="middle" style={{ width: '100%' }}>
              <div>
                <Text strong>Asset Type:</Text>
                <div style={{ marginLeft: 16, marginTop: 4 }}>
                  <Space>
                    {formData.asset_type === 'Folder' ? (
                      <FolderOutlined style={{ color: '#1890ff' }} />
                    ) : (
                      <ToolOutlined style={{ color: '#52c41a' }} />
                    )}
                    <Text>{formData.asset_type}</Text>
                  </Space>
                </div>
              </div>

              <div>
                <Text strong>Location:</Text>
                <div style={{ marginLeft: 16, marginTop: 4 }}>
                  <Text type="secondary">{getParentPath(formData.parent_id)}</Text>
                </div>
              </div>

              <div>
                <Text strong>Name:</Text>
                <div style={{ marginLeft: 16, marginTop: 4 }}>
                  <Text>{formData.name}</Text>
                </div>
              </div>

              {formData.description && (
                <div>
                  <Text strong>Description:</Text>
                  <div style={{ marginLeft: 16, marginTop: 4 }}>
                    <Paragraph style={{ margin: 0 }}>{formData.description}</Paragraph>
                  </div>
                </div>
              )}
            </Space>
          </div>
        );

      default:
        return null;
    }
  };

  const canGoNext = () => {
    switch (currentStep) {
      case 0:
        return !!formData.asset_type;
      case 1:
        return true; // Parent is optional
      case 2:
        return !!formData.name && !validateAssetName(formData.name);
      default:
        return false;
    }
  };

  return (
    <Modal
      title={
        <Space>
          <FolderOutlined />
          Create New Asset
        </Space>
      }
      open={open}
      onCancel={handleCancel}
      width={600}
      footer={null}
      destroyOnClose
    >
      <Spin spinning={loading}>
        <div style={{ marginBottom: 24 }}>
          <Steps
            current={currentStep}
            size="small"
            items={steps.map(step => ({
              title: step.title,
              description: step.description,
            }))}
          />
        </div>

        <Form
          form={form}
          layout="vertical"
          initialValues={formData}
        >
          <div style={{ minHeight: 300, marginBottom: 24 }}>
            {renderStepContent()}
          </div>
        </Form>

        <Divider />

        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <Space>
            {currentStep > 0 && (
              <Button onClick={handlePrevious}>
                Previous
              </Button>
            )}
          </Space>
          
          <Space>
            <Button onClick={handleCancel}>
              Cancel
            </Button>
            {currentStep < steps.length - 1 ? (
              <Button 
                type="primary" 
                onClick={handleNext}
                disabled={!canGoNext()}
              >
                Next
              </Button>
            ) : (
              <Button 
                type="primary" 
                onClick={handleCreate}
                loading={loading}
              >
                Create {formData.asset_type}
              </Button>
            )}
          </Space>
        </div>
      </Spin>
    </Modal>
  );
};

export default CreateAssetModal;