import React, { useState, useEffect } from 'react';
import { Typography, Space, Card, Alert } from 'antd';
import { FolderOutlined, HomeOutlined } from '@ant-design/icons';
import { BaseStepProps, HierarchySelectionData } from '../../../types/workflow';
import { AssetHierarchyPicker } from '../../hierarchy/AssetHierarchyPicker';
import { useWorkflowStore } from '../../../store/workflow';

const { Title, Text, Paragraph } = Typography;

export const HierarchySelectionStep: React.FC<BaseStepProps> = ({
  workflowId,
  data,
  onDataChange,
  onValidation
}) => {
  const { updateStep } = useWorkflowStore();
  const [localData, setLocalData] = useState<Partial<HierarchySelectionData>>({
    parent_id: data.parent_id,
    parent_path: data.parent_path
  });

  // Validate form whenever data changes
  useEffect(() => {
    validateForm();
  }, [localData, data.asset_type]);

  const validateForm = () => {
    const errors = [];
    
    // For devices, require a parent folder
    if (data.asset_type === 'Device' && !localData.parent_id) {
      errors.push({ 
        field: 'parent_id', 
        message: 'Devices must be placed inside a folder', 
        code: 'REQUIRED' 
      });
    }

    onValidation?.(errors.length === 0, errors);
  };

  const handleParentSelect = async (parentId: number | null, parentPath?: string) => {
    const updatedData = { 
      parent_id: parentId,
      parent_path: parentPath 
    };
    
    setLocalData(updatedData);

    // Update the workflow store
    try {
      await updateStep('hierarchy_selection', updatedData);
      onDataChange?.(updatedData);
    } catch (error) {
      console.error('Failed to update workflow step:', error);
    }
  };

  const getLocationDescription = () => {
    if (data.asset_type === 'Folder') {
      return 'Folders can be placed at the root level or inside other folders to create organizational hierarchies.';
    } else {
      return 'Devices must be placed inside a folder for proper organization and access control.';
    }
  };

  const getSelectionInfo = () => {
    if (localData.parent_id === null) {
      return {
        icon: <HomeOutlined style={{ color: '#1890ff' }} />,
        title: 'Root Level',
        description: 'This asset will be created at the root level of your hierarchy'
      };
    } else {
      return {
        icon: <FolderOutlined style={{ color: '#52c41a' }} />,
        title: 'Inside Folder',
        description: localData.parent_path ? `Location: ${localData.parent_path}` : 'Selected folder'
      };
    }
  };

  const selectionInfo = getSelectionInfo();

  return (
    <div>
      <div style={{ marginBottom: '32px', textAlign: 'center' }}>
        <Title level={3}>Select Location</Title>
        <Paragraph type="secondary">
          Choose where to place "{data.asset_name}" in your asset hierarchy.
        </Paragraph>
        <Text type="secondary" style={{ fontSize: '14px' }}>
          {getLocationDescription()}
        </Text>
      </div>

      {data.asset_type === 'Device' && (
        <Alert
          message="Device Placement Required"
          description="Devices must be placed inside a folder for proper organization and security. Please select a parent folder below."
          type="info"
          showIcon
          style={{ marginBottom: '24px' }}
        />
      )}

      <Card style={{ marginBottom: '24px' }}>
        <AssetHierarchyPicker
          value={localData.parent_id}
          onChange={handleParentSelect}
          allowRoot={data.asset_type === 'Folder'}
          placeholder="Select a parent folder"
          style={{ width: '100%' }}
        />
      </Card>

      {(localData.parent_id !== undefined) && (
        <Card 
          style={{ 
            backgroundColor: '#f6ffed', 
            border: '1px solid #b7eb8f',
            marginTop: '16px'
          }}
        >
          <Space align="start">
            {selectionInfo.icon}
            <div>
              <Text strong>{selectionInfo.title}</Text>
              <div style={{ marginTop: '4px' }}>
                <Text type="secondary" style={{ fontSize: '14px' }}>
                  {selectionInfo.description}
                </Text>
              </div>
            </div>
          </Space>
        </Card>
      )}

      <div style={{ marginTop: '24px', padding: '16px', backgroundColor: '#fafafa', borderRadius: '6px' }}>
        <Title level={5} style={{ margin: '0 0 8px 0' }}>Hierarchy Guidelines</Title>
        <ul style={{ margin: 0, paddingLeft: '20px' }}>
          <li style={{ marginBottom: '4px' }}>
            <Text type="secondary">Folders organize and group related assets</Text>
          </li>
          <li style={{ marginBottom: '4px' }}>
            <Text type="secondary">Devices contain configuration files and must be in folders</Text>
          </li>
          <li style={{ marginBottom: '4px' }}>
            <Text type="secondary">Use a logical structure that matches your physical or functional organization</Text>
          </li>
          {data.asset_type === 'Folder' && (
            <li style={{ marginBottom: '4px' }}>
              <Text type="secondary">Root-level folders create top-level categories</Text>
            </li>
          )}
        </ul>
      </div>
    </div>
  );
};