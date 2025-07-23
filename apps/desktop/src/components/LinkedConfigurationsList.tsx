import React, { useEffect, useState } from 'react';
import { Space, Tag, Tooltip, message } from 'antd';
import { LinkOutlined, FileOutlined } from '@ant-design/icons';
import { ConfigurationVersionInfo } from '../types/assets';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../store/auth';
import useAssetStore from '../store/assets';

interface LinkedConfigurationsListProps {
  firmwareId: number;
  assetId: number;
}

const LinkedConfigurationsList: React.FC<LinkedConfigurationsListProps> = ({ 
  firmwareId,
  assetId
}) => {
  const { token } = useAuthStore();
  const { navigateToHistory } = useAssetStore();
  const [linkedConfigs, setLinkedConfigs] = useState<ConfigurationVersionInfo[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    fetchLinkedConfigurations();
  }, [firmwareId, token]);

  const fetchLinkedConfigurations = async () => {
    if (!token) return;
    
    setLoading(true);
    try {
      // Get configurations by firmware from backend
      const configs = await invoke<ConfigurationVersionInfo[]>('get_configurations_by_firmware', {
        token,
        firmwareId
      });
      setLinkedConfigs(configs);
    } catch (err) {
      console.error('Failed to fetch linked configurations:', err);
      message.error('Failed to load linked configurations');
    } finally {
      setLoading(false);
    }
  };

  const handleNavigateToConfig = () => {
    // Navigate to the asset's configuration history view
    const asset = {
      id: assetId,
      name: '', // These will be populated by the store
      description: '',
      created_by: 0,
      created_by_username: '',
      created_at: '',
      version_count: 0,
      latest_version: null,
      latest_version_notes: null
    };
    navigateToHistory(asset);
  };

  if (loading) {
    return <Tag icon={<LinkOutlined />}>Loading...</Tag>;
  }

  if (linkedConfigs.length === 0) {
    return (
      <Tooltip title="No configurations linked to this firmware">
        <Tag icon={<LinkOutlined />} color="default">
          No linked configurations
        </Tag>
      </Tooltip>
    );
  }

  return (
    <Space size={4} wrap>
      <Tag icon={<LinkOutlined />} color="blue">
        {linkedConfigs.length} linked {linkedConfigs.length === 1 ? 'configuration' : 'configurations'}
      </Tag>
      {linkedConfigs.map(config => (
        <Tooltip 
          key={config.id} 
          title={`Version ${config.version_number} - ${config.status}`}
        >
          <Tag 
            icon={<FileOutlined />}
            color={config.status === 'Golden' ? 'gold' : 'default'}
            style={{ cursor: 'pointer' }}
            onClick={handleNavigateToConfig}
          >
            {config.version_number}
          </Tag>
        </Tooltip>
      ))}
    </Space>
  );
};

export default LinkedConfigurationsList;