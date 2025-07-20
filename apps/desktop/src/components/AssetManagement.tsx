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
  HistoryOutlined,
  EyeOutlined
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

  const handleImportSuccess = () => {
    setImportModalVisible(false);
    message.success('Configuration imported successfully!');
    // The asset will be added to the store by the import modal
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

  const handleViewDetails = (asset: AssetInfo) => {
    // For now, navigate to history view as it shows all details
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
      style={{ height: '100%' }}
      bodyStyle={{ padding: '16px' }}
      actions={[
        <Tooltip title="View Details">
          <EyeOutlined key="view" onClick={() => handleViewDetails(asset)} />
        </Tooltip>,
        <Tooltip title="Version History">
          <HistoryOutlined key="history" onClick={() => handleViewHistory(asset)} />
        </Tooltip>,
        <Tooltip title="Add Version">
          <PlusOutlined key="add" onClick={() => handleAddVersion(asset)} />
        </Tooltip>
      ]}
    >
      <Card.Meta
        avatar={
          <Avatar 
            icon={<DatabaseOutlined />} 
            style={{ backgroundColor: '#52c41a' }}
          />
        }
        title={
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <Text strong style={{ fontSize: '16px' }}>{asset.name}</Text>
            <Space>
              {asset.version_count > 0 && (
                <Tag color="blue">
                  {formatVersion(asset.latest_version || 'v1')}
                </Tag>
              )}
              <Tag color="green">
                {asset.version_count} {asset.version_count === 1 ? 'version' : 'versions'}
              </Tag>
            </Space>
          </div>
        }
        description={
          <div style={{ marginTop: '8px' }}>
            <Text type="secondary" style={{ fontSize: '14px' }}>
              {asset.description || 'No description'}
            </Text>
            <div style={{ marginTop: '8px', display: 'flex', alignItems: 'center', gap: '16px' }}>
              <Space size={4}>
                <CalendarOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                <Text type="secondary" style={{ fontSize: '12px' }}>
                  {formatDate(asset.created_at)}
                </Text>
              </Space>
              <Space size={4}>
                <UserOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                <Text type="secondary" style={{ fontSize: '12px' }}>
                  ID: {asset.created_by}
                </Text>
              </Space>
            </div>
          </div>
        }
      />
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
            <Col xs={24} sm={12} lg={8} xl={6} key={asset.id}>
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