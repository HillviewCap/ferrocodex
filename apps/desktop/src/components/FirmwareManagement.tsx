import React, { useEffect, useState } from 'react';
import {
  Typography,
  Button,
  Spin,
  Empty,
  message,
  Space
} from 'antd';
import {
  UploadOutlined,
  CloudUploadOutlined,
  ExportOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { AssetInfo } from '../types/assets';
import { ConfigurationVersionInfo, FirmwareVersionInfo as RecoveryFirmwareVersionInfo } from '../types/recovery';
import useAuthStore from '../store/auth';
import useFirmwareStore from '../store/firmware';
import FirmwareUploadModal from './FirmwareUploadModal';
import FirmwareVersionList from './FirmwareVersionList';
import { CompleteRecoveryModal } from './recovery/CompleteRecoveryModal';

const { Title, Text } = Typography;

interface FirmwareManagementProps {
  asset: AssetInfo;
}

const FirmwareManagement: React.FC<FirmwareManagementProps> = ({ asset }) => {
  const { user, token } = useAuthStore();
  const { 
    firmwareVersions,
    isLoading,
    error,
    loadFirmwareVersions,
    clearError
  } = useFirmwareStore();
  
  const [uploadModalVisible, setUploadModalVisible] = useState(false);
  const [recoveryModalVisible, setRecoveryModalVisible] = useState(false);
  const [configurationVersions, setConfigurationVersions] = useState<ConfigurationVersionInfo[]>([]);
  const [configVersionsLoading, setConfigVersionsLoading] = useState(false);
  
  const assetFirmware = firmwareVersions[asset.id] || [];
  const isEngineer = user?.role === 'Engineer' || user?.role === 'Administrator';

  useEffect(() => {
    loadFirmwareVersions(asset.id).catch(err => {
      console.error('Failed to load firmware versions:', err);
    });
  }, [asset.id, loadFirmwareVersions]);

  useEffect(() => {
    if (error) {
      message.error(error);
      clearError();
    }
  }, [error, clearError]);

  const canShowRecoveryOption = (): boolean => {
    return assetFirmware.length > 0 && isEngineer;
  };

  const handleUploadSuccess = () => {
    setUploadModalVisible(false);
    message.success('Firmware uploaded successfully!');
  };

  const loadConfigurationVersions = async () => {
    if (!token) return;
    
    setConfigVersionsLoading(true);
    try {
      const configs = await invoke<ConfigurationVersionInfo[]>('get_configuration_versions', {
        token,
        assetId: asset.id,
      });
      setConfigurationVersions(configs);
    } catch (error) {
      console.error('Failed to load configuration versions:', error);
      message.error('Failed to load configuration versions');
    } finally {
      setConfigVersionsLoading(false);
    }
  };

  const handleOpenRecoveryModal = async () => {
    await loadConfigurationVersions();
    setRecoveryModalVisible(true);
  };

  // Keyboard shortcut for complete recovery (Ctrl+E)
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key === 'e' && canShowRecoveryOption()) {
        event.preventDefault();
        handleOpenRecoveryModal();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [assetFirmware.length, isEngineer]);

  return (
    <div>
      <div style={{ marginBottom: '24px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <Text type="secondary">
            Manage firmware files for crash recovery and maintenance procedures
          </Text>
          <Space>
            {canShowRecoveryOption() && (
              <Button
                icon={<ExportOutlined />}
                onClick={handleOpenRecoveryModal}
                size="large"
                loading={configVersionsLoading}
                title="Export complete recovery package with firmware and configuration"
              >
                Complete Recovery
              </Button>
            )}
            {isEngineer && (
              <Button
                type="primary"
                icon={<UploadOutlined />}
                onClick={() => setUploadModalVisible(true)}
                size="large"
              >
                Upload Firmware
              </Button>
            )}
          </Space>
        </div>
      </div>

      {isLoading ? (
        <div style={{ textAlign: 'center', padding: '48px' }}>
          <Spin size="large" />
          <div style={{ marginTop: '16px' }}>
            <Text type="secondary">Loading firmware versions...</Text>
          </div>
        </div>
      ) : assetFirmware.length === 0 ? (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={
            <div>
              <Title level={5}>No Firmware Files</Title>
              <Text type="secondary">
                {isEngineer 
                  ? 'Upload firmware files to enable complete system recovery'
                  : 'No firmware files have been uploaded for this asset'}
              </Text>
            </div>
          }
        >
          {isEngineer && (
            <Button 
              type="primary" 
              icon={<CloudUploadOutlined />}
              onClick={() => setUploadModalVisible(true)}
            >
              Upload First Firmware
            </Button>
          )}
        </Empty>
      ) : (
        <FirmwareVersionList 
          versions={assetFirmware}
          onDelete={async () => {
            // Refresh the list after deletion
            await loadFirmwareVersions(asset.id);
          }}
        />
      )}

      {uploadModalVisible && (
        <FirmwareUploadModal
          visible={uploadModalVisible}
          onCancel={() => setUploadModalVisible(false)}
          onSuccess={handleUploadSuccess}
          assetId={asset.id}
        />
      )}

      {recoveryModalVisible && (
        <CompleteRecoveryModal
          visible={recoveryModalVisible}
          onClose={() => setRecoveryModalVisible(false)}
          assetId={asset.id}
          assetName={asset.name}
          configurationVersions={configurationVersions}
          firmwareVersions={assetFirmware as unknown as RecoveryFirmwareVersionInfo[]}
        />
      )}
    </div>
  );
};

export default FirmwareManagement;