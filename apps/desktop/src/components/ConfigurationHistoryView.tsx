import React, { useEffect, useState } from 'react';
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
  Tag,
  Tabs
} from 'antd';
import {
  ArrowLeftOutlined,
  DatabaseOutlined,
  HistoryOutlined,
  CalendarOutlined,
  UserOutlined,
  FileOutlined,
  BranchesOutlined
} from '@ant-design/icons';
import { AssetInfo, ConfigurationVersionInfo } from '../types/assets';
import { formatVersion } from '../types/assets';
import useAuthStore from '../store/auth';
import useAssetStore from '../store/assets';
import useBranchStore from '../store/branches';
import VersionHistoryList from './VersionHistoryList';
import CreateBranchModal from './CreateBranchModal';
import BranchManagement from './BranchManagement';

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
  const { 
    fetchBranches, 
    clearError: clearBranchError 
  } = useBranchStore();
  
  const [createBranchModalVisible, setCreateBranchModalVisible] = useState(false);
  const [selectedVersionForBranch, setSelectedVersionForBranch] = useState<ConfigurationVersionInfo | null>(null);
  const [activeTab, setActiveTab] = useState('versions');

  useEffect(() => {
    if (token && asset.id) {
      fetchVersions(token, asset.id);
      fetchBranches(token, asset.id);
    }
  }, [token, asset.id, fetchVersions, fetchBranches]);

  useEffect(() => {
    if (error) {
      message.error(error);
      clearError();
    }
  }, [error, clearError]);

  const handleCreateBranch = (version: ConfigurationVersionInfo) => {
    setSelectedVersionForBranch(version);
    setCreateBranchModalVisible(true);
  };

  const handleBranchCreated = () => {
    setCreateBranchModalVisible(false);
    setSelectedVersionForBranch(null);
    clearBranchError();
    // Refresh branches after creation
    if (token && asset.id) {
      fetchBranches(token, asset.id);
    }
  };

  const handleCancelBranchCreation = () => {
    setCreateBranchModalVisible(false);
    setSelectedVersionForBranch(null);
    clearBranchError();
  };

  const handleStatusChange = () => {
    // Refresh the versions list when status changes
    if (token && asset.id) {
      fetchVersions(token, asset.id);
    }
  };

  const handleCreateBranchFromManagement = () => {
    // For now, we'll just show a message as we need a version to create a branch from
    message.info('Please select a version from the history to create a branch from');
    setActiveTab('versions');
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

      {/* Tabs for Version History and Branch Management */}
      <Tabs 
        activeKey={activeTab} 
        onChange={setActiveTab}
        items={[
          {
            key: 'versions',
            label: (
              <span>
                <HistoryOutlined />
                Version History
              </span>
            ),
            children: (
              <div>
                <Text type="secondary" style={{ display: 'block', marginBottom: '24px' }}>
                  Complete audit trail of all configuration changes. Select any version to create a branch from it.
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
                  <VersionHistoryList 
                    versions={versions} 
                    onCreateBranch={handleCreateBranch}
                    showCreateBranch={true}
                    onStatusChange={handleStatusChange}
                  />
                )}
              </div>
            )
          },
          {
            key: 'branches',
            label: (
              <span>
                <BranchesOutlined />
                Branch Management
              </span>
            ),
            children: (
              <BranchManagement 
                asset={asset}
                onCreateBranch={handleCreateBranchFromManagement}
                showCreateButton={false}
                showSelectActions={false}
              />
            )
          }
        ]}
      />

      {/* Create Branch Modal */}
      {selectedVersionForBranch && (
        <CreateBranchModal
          visible={createBranchModalVisible}
          onCancel={handleCancelBranchCreation}
          onSuccess={handleBranchCreated}
          parentVersion={selectedVersionForBranch}
          assetId={asset.id}
        />
      )}
    </div>
  );
};

export default ConfigurationHistoryView;