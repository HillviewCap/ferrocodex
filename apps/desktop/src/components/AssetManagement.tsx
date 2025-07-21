import React, { useState, useEffect } from 'react';
import { 
  Typography, 
  Button, 
  Card, 
  Row, 
  Col, 
  Avatar, 
  Space,
  Spin,
  Empty,
  message,
  Tag,
  Tooltip,
  Divider
} from 'antd';
import { 
  ImportOutlined, 
  DatabaseOutlined, 
  CalendarOutlined,
  UserOutlined,
  PlusOutlined,
  HistoryOutlined
} from '@ant-design/icons';
import { AssetInfo } from '../types/assets';
import { formatVersion } from '../types/assets';
import useAuthStore from '../store/auth';
import useAssetStore from '../store/assets';
import ImportConfigurationModal from './ImportConfigurationModal';
import ConfigurationHistoryView from './ConfigurationHistoryView';

const { Title, Text } = Typography;

const AssetManagement: React.FC = () => {
  const { token } = useAuthStore();
  const { 
    assets, 
    isLoading, 
    error, 
    fetchAssets, 
    clearError,
    currentView,
    selectedAsset,
    navigateToHistory,
    navigateToDashboard
  } = useAssetStore();
  const [importModalVisible, setImportModalVisible] = useState(false);

  useEffect(() => {
    if (token) {
      fetchAssets(token);
    }
  }, [token, fetchAssets]);

  useEffect(() => {
    if (error) {
      message.error(error);
      clearError();
    }
  }, [error, clearError]);

  const handleImportSuccess = (_asset: AssetInfo) => {
    setImportModalVisible(false);
    message.success('Configuration imported successfully!');
    // Refresh the assets list to ensure the new asset is displayed
    if (token) {
      fetchAssets(token);
    }
  };

  const formatDate = (dateString: string) => {
    try {
      return new Date(dateString).toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
      });
    } catch {
      return dateString;
    }
  };

  const handleViewHistory = (asset: AssetInfo) => {
    navigateToHistory(asset);
  };

  const handleAddVersion = (asset: AssetInfo) => {
    // Navigate to history view and show import modal
    navigateToHistory(asset);
    message.info('Navigate to Version History to import a new version');
  };

  const AssetCard: React.FC<{ asset: AssetInfo }> = ({ asset }) => (
    <Card 
      hoverable
      onClick={() => handleViewHistory(asset)}
      style={{ 
        height: '100%',
        minHeight: '200px',
        cursor: 'pointer'
      }}
      bodyStyle={{ 
        padding: '20px',
        display: 'flex',
        flexDirection: 'column',
        height: 'calc(100% - 57px)' // Account for actions bar
      }}
      actions={[
        <Tooltip title="Version History">
          <HistoryOutlined key="history" onClick={(e) => {
            e.stopPropagation();
            handleViewHistory(asset);
          }} />
        </Tooltip>,
        <Tooltip title="Import New Version">
          <PlusOutlined key="add" onClick={(e) => {
            e.stopPropagation();
            handleAddVersion(asset);
          }} />
        </Tooltip>
      ]}
    >
      <div style={{ display: 'flex', alignItems: 'flex-start', gap: '12px', marginBottom: '16px' }}>
        <Avatar 
          icon={<DatabaseOutlined />} 
          style={{ backgroundColor: '#52c41a' }}
          size={48}
        />
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: '8px', marginBottom: '4px' }}>
            <Text strong style={{ fontSize: '16px', lineHeight: '24px' }} ellipsis>
              {asset.name}
            </Text>
            <Tag color="blue" style={{ flexShrink: 0 }}>
              {formatVersion(asset.latest_version || 'v1')}
            </Tag>
          </div>
          <Tag color="green">
            {asset.version_count} {asset.version_count === 1 ? 'version' : 'versions'}
          </Tag>
        </div>
      </div>
      
      <div style={{ flex: 1, marginBottom: '12px' }}>
        <Text 
          type="secondary" 
          style={{ 
            fontSize: '14px',
            display: '-webkit-box',
            WebkitLineClamp: 2,
            WebkitBoxOrient: 'vertical',
            overflow: 'hidden'
          }}
        >
          {asset.latest_version_notes || asset.description || 'No description available'}
        </Text>
      </div>
      
      <div style={{ marginTop: 'auto' }}>
        <Space size={4} style={{ marginBottom: '4px' }}>
          <CalendarOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
          <Text type="secondary" style={{ fontSize: '12px' }}>
            {formatDate(asset.created_at)}
          </Text>
        </Space>
        <br />
        <Space size={4}>
          <UserOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
          <Text type="secondary" style={{ fontSize: '12px' }}>
            {asset.created_by_username}
          </Text>
        </Space>
      </div>
    </Card>
  );

  // Render history view if selected
  if (currentView === 'history' && selectedAsset) {
    return (
      <ConfigurationHistoryView
        asset={selectedAsset}
        onBack={navigateToDashboard}
      />
    );
  }

  // Render dashboard view
  return (
    <div style={{ padding: '24px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '24px' }}>
        <div>
          <Title level={2} style={{ margin: 0 }}>
            Configuration Assets
          </Title>
          <Text type="secondary">
            Manage your configuration files and track versions
          </Text>
        </div>
        <Button 
          type="primary" 
          icon={<ImportOutlined />}
          onClick={() => setImportModalVisible(true)}
          size="large"
        >
          Import Configuration
        </Button>
      </div>

      <Divider style={{ margin: '24px 0' }} />

      {isLoading ? (
        <div style={{ textAlign: 'center', padding: '48px' }}>
          <Spin size="large" />
          <div style={{ marginTop: '16px' }}>
            <Text type="secondary">Loading assets...</Text>
          </div>
        </div>
      ) : assets.length === 0 ? (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={
            <div>
              <Title level={4}>No Configuration Assets</Title>
              <Text type="secondary">
                Get started by importing your first configuration file
              </Text>
            </div>
          }
        >
          <Button 
            type="primary" 
            icon={<ImportOutlined />}
            onClick={() => setImportModalVisible(true)}
          >
            Import Configuration
          </Button>
        </Empty>
      ) : (
        <Row gutter={[16, 16]}>
          {assets.map(asset => (
            <Col xs={24} sm={24} md={12} lg={8} xl={8} key={asset.id}>
              <AssetCard asset={asset} />
            </Col>
          ))}
        </Row>
      )}

      <ImportConfigurationModal
        visible={importModalVisible}
        onCancel={() => setImportModalVisible(false)}
        onSuccess={handleImportSuccess}
      />
    </div>
  );
};

export default AssetManagement;