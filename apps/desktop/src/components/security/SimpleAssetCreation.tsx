import React, { useState, useEffect, useMemo } from 'react';
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
  Spin,
  message
} from 'antd';
import {
  ToolOutlined,
  CheckCircleOutlined,
  RightOutlined
} from '@ant-design/icons';
import {
  AssetHierarchy,
  findAssetInHierarchy,
  getInheritedClassification,
  hierarchyHasFolders
} from '../../types/assets';
import { SecurityClassificationLevel, SECURITY_CLASSIFICATIONS } from '../../types/security';
import { AssetHierarchyPicker } from '../hierarchy/AssetHierarchyPicker';
import SecurityClassificationSelector from './SecurityClassificationSelector';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../../store/auth';

const { Text, Paragraph } = Typography;
const { TextArea } = Input;

// Creates devices only; folders have their own dialog (hierarchy/CreateFolderModal)
export interface SimpleAssetCreationProps {
  open: boolean;
  onCancel: () => void;
  onSuccess: (asset: any) => void;
  hierarchyData: AssetHierarchy[];
  initialParentId?: number | null;
}

interface FormData {
  name: string;
  description: string;
  parent_id: number | null;
  security_classification: SecurityClassificationLevel;
}

type StepKey = 'location' | 'details' | 'confirm';

export const SimpleAssetCreation: React.FC<SimpleAssetCreationProps> = ({
  open,
  onCancel,
  onSuccess,
  hierarchyData,
  initialParentId = null,
}) => {
  const { token, user } = useAuthStore();
  const [form] = Form.useForm<FormData>();
  const [currentStep, setCurrentStep] = useState(0);
  const [loading, setLoading] = useState(false);
  const [, forceUpdate] = useState({});
  const hasFolders = useMemo(() => hierarchyHasFolders(hierarchyData), [hierarchyData]);
  const [formData, setFormData] = useState<Partial<FormData>>({
    parent_id: initialParentId,
    security_classification: getInheritedClassification(hierarchyData, initialParentId)
  });

  // Reset form when modal opens/closes
  useEffect(() => {
    if (open) {
      const startParent = hasFolders ? initialParentId : null;
      setCurrentStep(0);
      form.resetFields();
      setFormData({
        parent_id: startParent,
        security_classification: getInheritedClassification(hierarchyData, startParent)
      });
    }
    // hierarchyData is deliberately read only when the modal opens, so a
    // background refresh doesn't reset what the user has chosen
  }, [open, form, initialParentId]);

  // Update form when formData changes
  useEffect(() => {
    form.setFieldsValue(formData);
  }, [form, formData]);

  // Pick a location only when there is a folder to pick
  const steps: { key: StepKey; title: string; content: string; icon: React.ReactNode }[] = [
    ...(hasFolders
      ? [{
          key: 'location' as const,
          title: 'Location',
          content: 'Choose parent folder',
          icon: <RightOutlined />,
        }]
      : []),
    {
      key: 'details',
      title: 'Details',
      content: 'Enter device information',
      icon: <ToolOutlined />,
    },
    {
      key: 'confirm',
      title: 'Confirm',
      content: 'Review and create',
      icon: <CheckCircleOutlined />,
    },
  ];
  const stepKey = steps[currentStep]?.key;

  // A device can't be classified below its folder; at the root anything goes
  const inheritedClassification = formData.parent_id != null
    ? getInheritedClassification(hierarchyData, formData.parent_id)
    : undefined;

  const handleParentChange = (parentId: number | null) => {
    // The classification follows the new folder
    setFormData(prev => ({
      ...prev,
      parent_id: parentId,
      security_classification: getInheritedClassification(hierarchyData, parentId)
    }));
  };

  const handleNext = async () => {
    try {
      if (stepKey === 'details') {
        // Validate details
        await form.validateFields(['name', 'description', 'security_classification']);
        const name = form.getFieldValue('name');
        const description = form.getFieldValue('description');
        const classification = form.getFieldValue('security_classification');
        if (!name || !name.trim()) {
          throw new Error('Please enter a device name');
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

      // Use formData for values that might not be in the form
      const finalValues = {
        ...formData,
        ...form.getFieldsValue()
      };

      if (!finalValues.name) {
        throw new Error('Required fields are missing');
      }

      const result = await invoke('create_asset', {
        token,
        name: finalValues.name.trim(),
        description: finalValues.description || '',
        assetType: 'device',
        parentId: finalValues.parent_id || null,
        securityClassification: finalValues.security_classification
      });

      onSuccess(result);
    } catch (error) {
      console.error('Failed to create device:', error);
      message.error(typeof error === 'string' ? error : error instanceof Error ? error.message : 'Failed to create device');
    } finally {
      setLoading(false);
    }
  };

  const renderStepContent = () => {
    switch (stepKey) {
      case 'location':
        return (
          <Card title="Select Parent Folder" size="small">
            <Alert
              title="Select the folder this device belongs in, or leave empty to create it at root level"
              type="info"
              showIcon
              style={{ marginBottom: 16 }}
            />
            <Form.Item name="parent_id">
              <AssetHierarchyPicker
                hierarchyData={hierarchyData}
                value={formData.parent_id}
                onChange={handleParentChange}
                placeholder="Select parent folder (optional)"
                allowClear
              />
            </Form.Item>
          </Card>
        );

      case 'details':
        return (
          <Card title="Device Details" size="small">
            <Form.Item
              name="name"
              label="Device Name"
              rules={[{ required: true, message: 'Please enter device name' }]}
            >
              <Input
                placeholder="e.g. PLC-Line5"
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
                placeholder="Enter device description (optional)"
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

      case 'confirm': {
        const parent = formData.parent_id != null ? findAssetInHierarchy(hierarchyData, formData.parent_id) : null;
        return (
          <Card title="Review Device" size="small">
            <Space orientation="vertical" style={{ width: '100%' }} size="middle">
              <div>
                <Text strong>Name:</Text> {formData.name || form.getFieldValue('name') || 'Not set'}
              </div>

              <div>
                <Text strong>Security Classification:</Text>{' '}
                {formData.security_classification
                  ? SECURITY_CLASSIFICATIONS[formData.security_classification].displayName
                  : 'Not set'}
              </div>

              <div>
                <Text strong>Parent Folder:</Text> {parent ? parent.name : 'Root level'}
              </div>

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
      }

      default:
        return null;
    }
  };

  const canProceed = () => {
    switch (stepKey) {
      case 'location':
        return true; // Parent is optional
      case 'details': {
        const name = form.getFieldValue('name');
        const classification = form.getFieldValue('security_classification');
        return !!(name && name.trim() && classification);
      }
      case 'confirm':
        return true;
      default:
        return false;
    }
  };

  return (
    <Modal
      title="Create Device"
      open={open}
      onCancel={onCancel}
      width={700}
      footer={null}
      mask={{ closable: false }}
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
                    Create Device
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
