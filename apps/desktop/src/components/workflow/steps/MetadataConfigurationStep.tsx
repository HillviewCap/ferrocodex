import React, { useState, useEffect } from 'react';
import { Typography, Space, Card, Spin, Alert } from 'antd';
import { SettingOutlined, InfoCircleOutlined } from '@ant-design/icons';
import { BaseStepProps, MetadataConfigurationData } from '../../../types/workflow';
import DynamicMetadataForm from '../../forms/DynamicMetadataForm';
import { useWorkflowStore } from '../../../store/workflow';

const { Title, Text, Paragraph } = Typography;

export const MetadataConfigurationStep: React.FC<BaseStepProps> = ({
  workflowId,
  data,
  onDataChange,
  onValidation
}) => {
  const { updateStep } = useWorkflowStore();
  const [localData, setLocalData] = useState<Partial<MetadataConfigurationData>>({
    metadata_schema_id: data.metadata_schema_id,
    metadata_values: data.metadata_values || {}
  });
  const [isLoading, setIsLoading] = useState(false);
  const [formValid, setFormValid] = useState(false);

  // Validate form whenever data changes
  useEffect(() => {
    validateForm();
  }, [localData, formValid]);

  const validateForm = () => {
    const errors = [];
    
    if (!localData.metadata_schema_id) {
      errors.push({ 
        field: 'metadata_schema_id', 
        message: 'Metadata schema is required', 
        code: 'REQUIRED' 
      });
    }

    // Additional validation will be handled by the DynamicMetadataForm
    if (localData.metadata_schema_id && !formValid) {
      errors.push({
        field: 'metadata_values',
        message: 'Please complete all required metadata fields',
        code: 'INVALID_METADATA'
      });
    }

    onValidation?.(errors.length === 0, errors);
  };

  const handleSchemaChange = async (schemaId: number) => {
    setIsLoading(true);
    
    const updatedData = { 
      ...localData,
      metadata_schema_id: schemaId,
      metadata_values: {} // Reset values when schema changes
    };
    
    setLocalData(updatedData);

    try {
      await updateStep('metadata_configuration', updatedData);
      onDataChange?.(updatedData);
    } catch (error) {
      console.error('Failed to update workflow step:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleMetadataChange = async (values: Record<string, any>) => {
    const updatedData = { 
      ...localData,
      metadata_values: values
    };
    
    setLocalData(updatedData);

    try {
      await updateStep('metadata_configuration', updatedData);
      onDataChange?.(updatedData);
    } catch (error) {
      console.error('Failed to update workflow step:', error);
    }
  };

  const handleFormValidation = (isValid: boolean) => {
    setFormValid(isValid);
  };

  const getAssetTypeInfo = () => {
    if (data.asset_type === 'Folder') {
      return {
        title: 'Folder Metadata',
        description: 'Configure organizational and descriptive metadata for this folder. This information helps categorize and manage the folder within your hierarchy.'
      };
    } else {
      return {
        title: 'Device Metadata',
        description: 'Configure technical specifications and properties for this device. This metadata is crucial for configuration management and compliance tracking.'
      };
    }
  };

  const assetInfo = getAssetTypeInfo();

  return (
    <div>
      <div style={{ marginBottom: '32px', textAlign: 'center' }}>
        <Title level={3}>{assetInfo.title}</Title>
        <Paragraph type="secondary">
          {assetInfo.description}
        </Paragraph>
      </div>

      <Card style={{ marginBottom: '24px' }}>
        <Space align="start" style={{ width: '100%' }}>
          <SettingOutlined style={{ color: '#1890ff', fontSize: '20px', marginTop: '4px' }} />
          <div style={{ flex: 1 }}>
            <Title level={5} style={{ margin: '0 0 8px 0' }}>
              Asset: {data.asset_name}
            </Title>
            <Text type="secondary">
              Type: {data.asset_type} • Location: {data.parent_path || 'Root Level'}
            </Text>
          </div>
        </Space>
      </Card>

      <Spin spinning={isLoading}>
        <Card>
          <DynamicMetadataForm
            assetType={data.asset_type}
            selectedSchemaId={localData.metadata_schema_id}
            initialValues={localData.metadata_values}
            onSchemaChange={handleSchemaChange}
            onValuesChange={handleMetadataChange}
            onValidationChange={handleFormValidation}
            showSchemaSelector={true}
            showTitle={false}
          />
        </Card>
      </Spin>

      {localData.metadata_schema_id && (
        <Alert
          message="Metadata Configuration"
          description="Complete all required fields to proceed. Optional fields can be filled now or updated later after the asset is created."
          type="info"
          icon={<InfoCircleOutlined />}
          style={{ marginTop: '16px' }}
        />
      )}

      <div style={{ marginTop: '24px', padding: '16px', backgroundColor: '#fafafa', borderRadius: '6px' }}>
        <Title level={5} style={{ margin: '0 0 8px 0' }}>Metadata Guidelines</Title>
        <ul style={{ margin: 0, paddingLeft: '20px' }}>
          <li style={{ marginBottom: '4px' }}>
            <Text type="secondary">Required fields must be completed before proceeding</Text>
          </li>
          <li style={{ marginBottom: '4px' }}>
            <Text type="secondary">Metadata can be updated after asset creation</Text>
          </li>
          <li style={{ marginBottom: '4px' }}>
            <Text type="secondary">Use consistent naming conventions for better organization</Text>
          </li>
          <li style={{ marginBottom: '4px' }}>
            <Text type="secondary">Technical specifications help with configuration management</Text>
          </li>
        </ul>
      </div>
    </div>
  );
};