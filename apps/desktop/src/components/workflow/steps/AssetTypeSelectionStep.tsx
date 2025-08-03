import React, { useState, useEffect } from 'react';
import { Form, Radio, Input, Typography, Space, Card, Row, Col } from 'antd';
import { FolderOutlined, DesktopOutlined } from '@ant-design/icons';
import { BaseStepProps, AssetTypeSelectionData } from '../../../types/workflow';
import { AssetType } from '../../../types/assets';
import { validateAssetName, validateAssetDescription } from '../../../types/assets';
import { useWorkflowStore } from '../../../store/workflow';

const { Title, Text, Paragraph } = Typography;
const { TextArea } = Input;

export const AssetTypeSelectionStep: React.FC<BaseStepProps> = ({
  workflowId,
  data,
  onDataChange,
  onValidation
}) => {
  const [form] = Form.useForm();
  const { updateStep } = useWorkflowStore();
  const [localData, setLocalData] = useState<Partial<AssetTypeSelectionData>>({
    asset_type: data.asset_type,
    asset_name: data.asset_name || '',
    asset_description: data.asset_description || ''
  });

  // Initialize form with existing data
  useEffect(() => {
    form.setFieldsValue(localData);
  }, [form, localData]);

  // Validate form whenever data changes
  useEffect(() => {
    validateForm();
  }, [localData]);

  const validateForm = () => {
    const errors = [];
    
    if (!localData.asset_type) {
      errors.push({ field: 'asset_type', message: 'Asset type is required', code: 'REQUIRED' });
    }
    
    if (!localData.asset_name?.trim()) {
      errors.push({ field: 'asset_name', message: 'Asset name is required', code: 'REQUIRED' });
    } else {
      const nameError = validateAssetName(localData.asset_name);
      if (nameError) {
        errors.push({ field: 'asset_name', message: nameError, code: 'INVALID_FORMAT' });
      }
    }

    if (localData.asset_description) {
      const descError = validateAssetDescription(localData.asset_description);
      if (descError) {
        errors.push({ field: 'asset_description', message: descError, code: 'INVALID_FORMAT' });
      }
    }

    onValidation?.(errors.length === 0, errors);
  };

  const handleFieldChange = async (field: keyof AssetTypeSelectionData, value: any) => {
    const updatedData = { ...localData, [field]: value };
    setLocalData(updatedData);

    // Update the workflow store
    try {
      await updateStep('asset_type_selection', updatedData);
      onDataChange?.(updatedData);
    } catch (error) {
      console.error('Failed to update workflow step:', error);
    }
  };

  const assetTypeOptions = [
    {
      value: 'Folder' as AssetType,
      icon: <FolderOutlined style={{ fontSize: '48px', color: '#1890ff' }} />,
      title: 'Folder',
      description: 'A container for organizing other assets and folders. Use folders to create a hierarchical structure for your equipment and configurations.',
      features: ['Contains other assets', 'Organizational structure', 'No configuration files']
    },
    {
      value: 'Device' as AssetType,
      icon: <DesktopOutlined style={{ fontSize: '48px', color: '#52c41a' }} />,
      title: 'Device',
      description: 'A physical device or equipment that can have configuration files. Devices contain the actual configuration data and firmware.',
      features: ['Stores configuration files', 'Version management', 'Firmware tracking']
    }
  ];

  return (
    <div>
      <div style={{ marginBottom: '32px', textAlign: 'center' }}>
        <Title level={3}>Select Asset Type</Title>
        <Paragraph type="secondary">
          Choose the type of asset you want to create. This determines how the asset will be organized and what features are available.
        </Paragraph>
      </div>

      <Form
        form={form}
        layout="vertical"
        onValuesChange={(changedValues) => {
          Object.keys(changedValues).forEach(key => {
            handleFieldChange(key as keyof AssetTypeSelectionData, changedValues[key]);
          });
        }}
      >
        <Form.Item
          name="asset_type"
          label="Asset Type"
          rules={[{ required: true, message: 'Please select an asset type' }]}
        >
          <Radio.Group style={{ width: '100%' }}>
            <Row gutter={16}>
              {assetTypeOptions.map((option) => (
                <Col span={12} key={option.value}>
                  <Card
                    hoverable
                    style={{
                      height: '280px',
                      border: localData.asset_type === option.value ? '2px solid #1890ff' : '1px solid #d9d9d9',
                      cursor: 'pointer'
                    }}
                    bodyStyle={{ padding: '24px', textAlign: 'center', height: '100%' }}
                    onClick={() => handleFieldChange('asset_type', option.value)}
                  >
                    <Radio value={option.value} style={{ display: 'none' }} />
                    <Space direction="vertical" size="large" style={{ width: '100%' }}>
                      {option.icon}
                      <div>
                        <Title level={4} style={{ margin: '0 0 8px 0' }}>
                          {option.title}
                        </Title>
                        <Paragraph style={{ fontSize: '14px', margin: '0 0 16px 0', minHeight: '60px' }}>
                          {option.description}
                        </Paragraph>
                        <div style={{ textAlign: 'left' }}>
                          {option.features.map((feature, index) => (
                            <div key={index} style={{ fontSize: '12px', color: '#666', marginBottom: '4px' }}>
                              • {feature}
                            </div>
                          ))}
                        </div>
                      </div>
                    </Space>
                  </Card>
                </Col>
              ))}
            </Row>
          </Radio.Group>
        </Form.Item>

        <Row gutter={16}>
          <Col span={12}>
            <Form.Item
              name="asset_name"
              label="Asset Name"
              rules={[
                { required: true, message: 'Asset name is required' },
                { min: 2, message: 'Asset name must be at least 2 characters' },
                { max: 100, message: 'Asset name cannot exceed 100 characters' },
                {
                  pattern: /^[a-zA-Z0-9\s\-_\.]+$/,
                  message: 'Asset name can only contain letters, numbers, spaces, hyphens, underscores, and periods'
                }
              ]}
            >
              <Input
                placeholder="Enter a name for this asset"
                size="large"
                value={localData.asset_name}
                onChange={(e) => handleFieldChange('asset_name', e.target.value)}
              />
            </Form.Item>
          </Col>
          <Col span={12}>
            <Form.Item
              name="asset_description"
              label="Description (Optional)"
              rules={[
                { max: 500, message: 'Description cannot exceed 500 characters' }
              ]}
            >
              <TextArea
                placeholder="Provide a brief description of this asset"
                rows={4}
                value={localData.asset_description}
                onChange={(e) => handleFieldChange('asset_description', e.target.value)}
                showCount
                maxLength={500}
              />
            </Form.Item>
          </Col>
        </Row>

        {localData.asset_type && (
          <Card style={{ backgroundColor: '#f6ffed', border: '1px solid #b7eb8f' }}>
            <Text type="secondary">
              <strong>Selected:</strong> {localData.asset_type} - {assetTypeOptions.find(opt => opt.value === localData.asset_type)?.description}
            </Text>
          </Card>
        )}
      </Form>
    </div>
  );
};