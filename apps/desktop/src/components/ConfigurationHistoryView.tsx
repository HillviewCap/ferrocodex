import React, { useEffect } from 'react';
import {
  Typography,
  Button,
  Card,
  Space,
  Spin,
  Empty,
  message,
  Breadcrumb,
  Divider,
  Avatar,
  Tag
} from 'antd';
import {
  ArrowLeftOutlined,
  DatabaseOutlined,
  HistoryOutlined,
  CalendarOutlined,
  UserOutlined,
  FileOutlined
} from '@ant-design/icons';
import { AssetInfo } from '../types/assets';
import { formatVersion } from '../types/assets';
import useAuthStore from '../store/auth';
import useAssetStore from '../store/assets';
import VersionHistoryList from './VersionHistoryList';

const { Title, Text } = Typography;

interface ConfigurationHistoryViewProps {
  asset: AssetInfo;
  onBack: () => void;
}

const ConfigurationHistoryView: React.FC<ConfigurationHistoryViewProps> = ({ asset, onBack }) => {
  const { token } = useAuthStore();
  const { 
    versions, 
    versionsLoading, 
    error, 
    fetchVersions, 
    clearError 
  } = useAssetStore();

  useEffect(() => {
    if (token && asset.id) {
      fetchVersions(token, asset.id);
    }
  }, [token, asset.id, fetchVersions]);

  useEffect(() => {
    if (error) {
      message.error(error);
      clearError();
    }
  }, [error, clearError]);

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

  return (
    <div style={{ padding: '24px' }}>
      {/* Header with breadcrumb and navigation */}
      <div style={{ marginBottom: '24px' }}>
        <Breadcrumb style={{ marginBottom: '16px' }}>
          <Breadcrumb.Item>
            <Button 
              type="link" 
              icon={<ArrowLeftOutlined />}
              onClick={onBack}
              style={{ padding: 0 }}
            >
              Configuration Assets
            </Button>
          </Breadcrumb.Item>
          <Breadcrumb.Item>
            <HistoryOutlined /> Version History
          </Breadcrumb.Item>
        </Breadcrumb>

        <Card style={{ marginBottom: '24px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
            <Avatar 
              icon={<DatabaseOutlined />} 
              size={64}
              style={{ backgroundColor: '#52c41a' }}
            />
            <div style={{ flex: 1 }}>
              <Title level={3} style={{ margin: 0 }}>
                {asset.name}
              </Title>
              <Text type="secondary" style={{ fontSize: '14px', display: 'block', marginBottom: '8px' }}>
                {asset.description || 'No description'}
              </Text>
              <Space wrap>
                <Space size={4}>
                  <CalendarOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                  <Text type="secondary" style={{ fontSize: '12px' }}>
                    Created: {formatDate(asset.created_at)}
                  </Text>
                </Space>
                <Space size={4}>
                  <UserOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                  <Text type="secondary" style={{ fontSize: '12px' }}>
                    Owner: {asset.created_by}
                  </Text>
                </Space>
                <Space size={4}>
                  <FileOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                  <Text type="secondary" style={{ fontSize: '12px' }}>
                    {asset.version_count} {asset.version_count === 1 ? 'version' : 'versions'}
                  </Text>
                </Space>
                {asset.latest_version && (
                  <Tag color="blue">
                    Latest: {formatVersion(asset.latest_version)}
                  </Tag>
                )}
              </Space>
            </div>
          </div>
        </Card>
      </div>

      <Divider style={{ margin: '24px 0' }} />

      {/* Version History Section */}
      <div>
        <Title level={4} style={{ marginBottom: '16px' }}>
          <HistoryOutlined /> Version History
        </Title>
        <Text type="secondary" style={{ display: 'block', marginBottom: '24px' }}>
          Complete audit trail of all configuration changes
        </Text>

        {versionsLoading ? (
          <div style={{ textAlign: 'center', padding: '48px' }}>
            <Spin size="large" />
            <div style={{ marginTop: '16px' }}>
              <Text type="secondary">Loading version history...</Text>
            </div>
          </div>
        ) : versions.length === 0 ? (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={
              <div>
                <Title level={5}>No Version History</Title>
                <Text type="secondary">
                  This asset has no configuration versions yet
                </Text>
              </div>
            }
          />
        ) : (
          <VersionHistoryList versions={versions} />
        )}
      </div>
    </div>
  );
};

export default ConfigurationHistoryView;