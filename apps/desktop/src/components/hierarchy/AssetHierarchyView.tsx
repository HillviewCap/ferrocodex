import React, { useState, useEffect } from 'react';
import { 
  Layout, 
  Card, 
  Row, 
  Col, 
  Button, 
  Space, 
  Typography, 
  Spin, 
  Alert,
  message,
  Breadcrumb,
  Divider,
  Empty,
  Tooltip
} from 'antd';
import { 
  PlusOutlined, 
  ReloadOutlined, 
  FolderOutlined,
  ToolOutlined,
  HomeOutlined,
  HistoryOutlined,
  DatabaseOutlined,
  CalendarOutlined,
  UserOutlined,
  SafetyOutlined
} from '@ant-design/icons';
import { AssetTreeView } from './AssetTreeView';
import { CreateAssetModal } from './CreateAssetModal';
import { AssetHierarchy, AssetType, AssetInfo } from '../../types/assets';
import { useAuthStore } from '../../store/auth';
import { useHierarchyStore, useHierarchyData, useHierarchyLoading, useHierarchyError, useSelectedAsset } from '../../store/hierarchy';
import { invoke } from '@tauri-apps/api/core';
import { VaultInfo } from '../../types/vault';
import VaultAccessIndicator from '../VaultAccessIndicator';

const { Sider, Content } = Layout;
const { Title, Text } = Typography;

export interface AssetHierarchyViewProps {
  onAssetSelect?: (asset: AssetHierarchy | null) => void;
  onNavigateToHistory?: (asset: AssetInfo) => void;
}

export const AssetHierarchyView: React.FC<AssetHierarchyViewProps> = ({
  onAssetSelect,
  onNavigateToHistory,
}) => {
  const { token } = useAuthStore();
  const hierarchyData = useHierarchyData();
  const selectedAsset = useSelectedAsset();
  const isLoading = useHierarchyLoading();
  const error = useHierarchyError();
  
  const { 
    loadHierarchy, 
    selectAsset, 
    moveAsset,
    getAssetPath,
    refreshHierarchy 
  } = useHierarchyStore();

  const [createModalVisible, setCreateModalVisible] = useState(false);
  const [createModalParentId, setCreateModalParentId] = useState<number | null>(null);
  const [createModalAssetType, setCreateModalAssetType] = useState<AssetType | undefined>();
  const [assetPath, setAssetPath] = useState<AssetInfo[]>([]);
  const [vaultInfo, setVaultInfo] = useState<VaultInfo | null>(null);
  const [vaultLoading, setVaultLoading] = useState(false);

  // Load hierarchy on mount
  useEffect(() => {
    if (token) {
      loadHierarchy(token);
    }
  }, [token, loadHierarchy]);

  // Update asset path when selection changes
  useEffect(() => {
    if (selectedAsset && token) {
      getAssetPath(token, selectedAsset.id)
        .then(setAssetPath)
        .catch(err => {
          console.error('Failed to get asset path:', err);
          setAssetPath([]);
        });
    } else {
      setAssetPath([]);
    }
  }, [selectedAsset, token, getAssetPath]);

  // Load vault info for selected asset
  useEffect(() => {
    if (selectedAsset && token) {
      setVaultLoading(true);
      invoke<VaultInfo | null>('get_vault_by_asset_id', {
        token,
        assetId: selectedAsset.id
      })
        .then(setVaultInfo)
        .catch(err => {
          console.error('Failed to load vault info:', err);
          setVaultInfo(null);
        })
        .finally(() => setVaultLoading(false));
    } else {
      setVaultInfo(null);
    }
  }, [selectedAsset, token]);

  const handleAssetSelect = (asset: AssetHierarchy | null) => {
    selectAsset(asset);
    onAssetSelect?.(asset);
  };

  const handleAssetCreate = (parentId: number | null, assetType: AssetType) => {
    setCreateModalParentId(parentId);
    setCreateModalAssetType(assetType);
    setCreateModalVisible(true);
  };

  const handleCreateSuccess = () => {
    setCreateModalVisible(false);
    if (token) {
      refreshHierarchy(token);
    }
  };

  const handleAssetMove = async (assetId: number, newParentId: number | null) => {
    if (!token) return;
    
    try {
      await moveAsset(token, {
        asset_id: assetId,
        new_parent_id: newParentId,
      });
      message.success('Asset moved successfully');
    } catch (error) {
      // Error handling is done in the store
    }
  };

  const handleRefresh = () => {
    if (token) {
      refreshHierarchy(token);
    }
  };

  const handleViewHistory = () => {
    if (selectedAsset && onNavigateToHistory) {
      // Convert AssetHierarchy to AssetInfo for compatibility
      const assetInfo: AssetInfo = {
        id: selectedAsset.id,
        name: selectedAsset.name,
        description: selectedAsset.description,
        asset_type: selectedAsset.asset_type,
        parent_id: selectedAsset.parent_id,
        sort_order: selectedAsset.sort_order,
        created_by: selectedAsset.created_by,
        created_by_username: '', // This would need to be fetched
        created_at: selectedAsset.created_at,
        version_count: 0, // This would need to be fetched
        latest_version: null,
        latest_version_notes: null,
      };
      onNavigateToHistory(assetInfo);
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

  const renderSelectedAssetDetails = () => {
    if (!selectedAsset) {
      return (
        <Card style={{ height: '100%' }}>
          <Empty 
            description="Select an asset to view details"
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          />
        </Card>
      );
    }

    const isDevice = selectedAsset.asset_type === 'Device';

    return (
      <Card 
        title={
          <Space>
            {selectedAsset.asset_type === 'Folder' ? (
              <FolderOutlined style={{ color: '#1890ff' }} />
            ) : (
              <ToolOutlined style={{ color: '#52c41a' }} />
            )}
            <span>{selectedAsset.name}</span>
          </Space>
        }
        extra={
          <Space>
            {isDevice && (
              <Tooltip title="View Configuration History">
                <Button 
                  icon={<HistoryOutlined />} 
                  onClick={handleViewHistory}
                  type="text"
                />
              </Tooltip>
            )}
            <Tooltip title="Refresh">
              <Button 
                icon={<ReloadOutlined />} 
                onClick={handleRefresh}
                type="text"
              />
            </Tooltip>
          </Space>
        }
        style={{ height: '100%' }}
      >
        <Space direction="vertical" style={{ width: '100%' }} size="middle">
          {/* Breadcrumb */}
          {assetPath.length > 0 && (
            <div>
              <Text type="secondary" style={{ fontSize: '12px' }}>Location:</Text>
              <Breadcrumb style={{ marginTop: 4 }}>
                <Breadcrumb.Item>
                  <HomeOutlined />
                  <span>Root</span>
                </Breadcrumb.Item>
                {assetPath.slice(0, -1).map((asset, index) => (
                  <Breadcrumb.Item key={asset.id}>
                    <FolderOutlined />
                    <span>{asset.name}</span>
                  </Breadcrumb.Item>
                ))}
              </Breadcrumb>
            </div>
          )}

          {/* Asset Type */}
          <div>
            <Text strong>Type: </Text>
            <Space>
              {selectedAsset.asset_type === 'Folder' ? (
                <>
                  <FolderOutlined style={{ color: '#1890ff' }} />
                  <Text>Folder</Text>
                </>
              ) : (
                <>
                  <ToolOutlined style={{ color: '#52c41a' }} />
                  <Text>Device</Text>
                </>
              )}
            </Space>
          </div>

          {/* Description */}
          {selectedAsset.description && (
            <div>
              <Text strong>Description: </Text>
              <Text>{selectedAsset.description}</Text>
            </div>
          )}

          {/* Vault Access */}
          {isDevice && !vaultLoading && (
            <div>
              <Text strong>Identity Vault: </Text>
              <VaultAccessIndicator 
                assetId={selectedAsset.id}
                vaultInfo={vaultInfo}
              />
            </div>
          )}

          {/* Creation Info */}
          <Divider style={{ margin: '12px 0' }} />
          <div>
            <Space direction="vertical" size="small">
              <div>
                <CalendarOutlined style={{ marginRight: 8, color: '#666' }} />
                <Text type="secondary">Created: {formatDate(selectedAsset.created_at)}</Text>
              </div>
              <div>
                <UserOutlined style={{ marginRight: 8, color: '#666' }} />
                <Text type="secondary">Created by: User #{selectedAsset.created_by}</Text>
              </div>
            </Space>
          </div>

          {/* Folder Contents */}
          {selectedAsset.asset_type === 'Folder' && (
            <>
              <Divider style={{ margin: '12px 0' }} />
              <div>
                <Text strong>Contents: </Text>
                <Text>{selectedAsset.children.length} items</Text>
                {selectedAsset.children.length > 0 && (
                  <div style={{ marginTop: 8 }}>
                    <Space wrap>
                      {selectedAsset.children.slice(0, 5).map(child => (
                        <div key={child.id} style={{ fontSize: '12px' }}>
                          {child.asset_type === 'Folder' ? (
                            <FolderOutlined style={{ color: '#1890ff', marginRight: 4 }} />
                          ) : (
                            <ToolOutlined style={{ color: '#52c41a', marginRight: 4 }} />
                          )}
                          <Text type="secondary">{child.name}</Text>
                        </div>
                      ))}
                      {selectedAsset.children.length > 5 && (
                        <Text type="secondary" style={{ fontSize: '12px' }}>
                          +{selectedAsset.children.length - 5} more
                        </Text>
                      )}
                    </Space>
                  </div>
                )}
              </div>
            </>
          )}
        </Space>
      </Card>
    );
  };

  return (
    <div style={{ height: '100%' }}>
      <div style={{ marginBottom: 16 }}>
        <Row justify="space-between" align="middle">
          <Col>
            <Title level={3} style={{ margin: 0 }}>Asset Hierarchy</Title>
          </Col>
          <Col>
            <Space>
              <Button
                type="primary"
                icon={<PlusOutlined />}
                onClick={() => handleAssetCreate(null, 'Folder')}
              >
                Create Folder
              </Button>
              <Button
                icon={<PlusOutlined />}
                onClick={() => handleAssetCreate(null, 'Device')}
              >
                Create Device
              </Button>
              <Button
                icon={<ReloadOutlined />}
                onClick={handleRefresh}
                loading={isLoading}
              >
                Refresh
              </Button>
            </Space>
          </Col>
        </Row>
      </div>

      {error && (
        <Alert
          message="Error loading hierarchy"
          description={error}
          type="error"
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}

      <Layout style={{ height: 'calc(100vh - 200px)' }}>
        <Sider 
          width={400} 
          style={{ background: '#fff', borderRight: '1px solid #f0f0f0' }}
        >
          <Card 
            title="Asset Tree" 
            bodyStyle={{ padding: 0, height: 'calc(100% - 60px)' }}
            style={{ height: '100%', border: 'none' }}
          >
            <Spin spinning={isLoading}>
              <AssetTreeView
                hierarchyData={hierarchyData}
                onAssetSelect={handleAssetSelect}
                onAssetCreate={handleAssetCreate}
                onAssetMove={handleAssetMove}
                selectedAssetId={selectedAsset?.id || null}
                loading={isLoading}
                allowDragDrop={true}
              />
            </Spin>
          </Card>
        </Sider>
        
        <Content style={{ padding: '0 16px' }}>
          {renderSelectedAssetDetails()}
        </Content>
      </Layout>

      <CreateAssetModal
        open={createModalVisible}
        onCancel={() => setCreateModalVisible(false)}
        onSuccess={handleCreateSuccess}
        hierarchyData={hierarchyData}
        initialParentId={createModalParentId}
        initialAssetType={createModalAssetType}
      />
    </div>
  );
};

export default AssetHierarchyView;