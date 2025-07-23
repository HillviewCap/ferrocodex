import React, { useEffect, useState } from 'react';
import {
  Typography,
  Button,
  Spin,
  Empty,
  message
} from 'antd';
import {
  UploadOutlined,
  CloudUploadOutlined
} from '@ant-design/icons';
import { AssetInfo } from '../types/assets';
import useAuthStore from '../store/auth';
import useFirmwareStore from '../store/firmware';
import FirmwareUploadModal from './FirmwareUploadModal';
import FirmwareVersionList from './FirmwareVersionList';

const { Title, Text } = Typography;

interface FirmwareManagementProps {
  asset: AssetInfo;
}

const FirmwareManagement: React.FC<FirmwareManagementProps> = ({ asset }) => {
  const { user } = useAuthStore();
  const { 
    firmwareVersions,
    isLoading,
    error,
    loadFirmwareVersions,
    clearError
  } = useFirmwareStore();
  
  const [uploadModalVisible, setUploadModalVisible] = useState(false);
  const assetFirmware = firmwareVersions[asset.id] || [];
  const isEngineer = user?.role === 'Engineer';

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

  const handleUploadSuccess = () => {
    setUploadModalVisible(false);
    message.success('Firmware uploaded successfully!');
  };

  return (
    <div>
      <div style={{ marginBottom: '24px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <Text type="secondary">
            Manage firmware files for crash recovery and maintenance procedures
          </Text>
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
    </div>
  );
};

export default FirmwareManagement;