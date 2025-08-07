import React, { useState, useEffect, useMemo } from 'react';
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
  Tooltip,
  Switch,
  Badge
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
  SafetyOutlined,
  SelectOutlined,
  UnorderedListOutlined
} from '@ant-design/icons';
import { AssetTreeView } from './AssetTreeView';
import { CreateAssetModal } from './CreateAssetModal';
import { AssetHierarchy, AssetType, AssetInfo } from '../../types/assets';
import useAuthStore from '../../store/auth';
import { useHierarchyStore, useHierarchyData, useHierarchyLoading, useHierarchyError, useSelectedAsset } from '../../store/hierarchy';
import useBulkOperationsStore from '../../store/bulkOperations';
import { invoke } from '@tauri-apps/api/core';
import { VaultInfo } from '../../types/vault';
import VaultAccessIndicator from '../VaultAccessIndicator';
import { 
  BulkSelectionManager, 
  BulkOperationToolbar, 
  BulkProgressModal 
} from '../bulk';

const { Sider, Content } = Layout;
const { Title, Text } = Typography;

export interface EnhancedAssetHierarchyViewProps {
  onAssetSelect?: (asset: AssetHierarchy | null) => void;
  onNavigateToHistory?: (asset: AssetInfo) => void;
  enableBulkOperations?: boolean;
}

export const EnhancedAssetHierarchyView: React.FC<EnhancedAssetHierarchyViewProps> = ({
  onAssetSelect,
  onNavigateToHistory,
  enableBulkOperations = true,
}) => {
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [vaultInfo, setVaultInfo] = useState<VaultInfo | null>(null);
  const [isBulkModeEnabled, setIsBulkModeEnabled] = useState(false);
  
  // Store hooks
  const { user } = useAuthStore();
  const hierarchyData = useHierarchyData();
  const isLoading = useHierarchyLoading();
  const error = useHierarchyError();
  const selectedAsset = useSelectedAsset();
  const { loadHierarchy, setSelectedAsset } = useHierarchyStore();
  
  // Bulk operations store
  const {
    getSelectedCount,
    getSelectedAssets,
    ui: bulkUI,
    resetSelection,
  } = useBulkOperationsStore();

  const selectedBulkCount = getSelectedCount();

  // Load hierarchy data on mount
  useEffect(() => {
    if (!hierarchyData) {
      loadHierarchy();
    }
  }, [hierarchyData, loadHierarchy]);

  // Load vault info when asset is selected
  useEffect(() => {
    const loadVaultInfo = async () => {
      if (selectedAsset) {
        try {
          const vault = await invoke<VaultInfo | null>('get_vault_by_asset_id', { 
            assetId: selectedAsset.id 
          });
          setVaultInfo(vault);
        } catch (error) {
          console.warn('No vault found for asset:', error);
          setVaultInfo(null);
        }
      } else {
        setVaultInfo(null);
      }
    };

    loadVaultInfo();
  }, [selectedAsset]);

  // Handle asset selection - integrate with bulk operations
  const handleAssetSelect = (asset: AssetHierarchy | null) => {
    setSelectedAsset(asset);
    if (onAssetSelect) {
      onAssetSelect(asset);
    }
  };

  const handleRefresh = async () => {
    setIsRefreshing(true);
    try {
      await loadHierarchy();
      message.success('Asset hierarchy refreshed');
    } catch (err) {
      message.error('Failed to refresh asset hierarchy');
    } finally {
      setIsRefreshing(false);
    }
  };

  const handleCreateAsset = () => {
    setShowCreateModal(true);
  };

  const handleAssetCreated = () => {
    setShowCreateModal(false);
    loadHierarchy();
  };

  const handleViewHistory = () => {
    if (selectedAsset && onNavigateToHistory) {
      const assetInfo: AssetInfo = {
        id: selectedAsset.id,
        name: selectedAsset.name,
        description: selectedAsset.description,
        asset_type: selectedAsset.asset_type,
        parent_id: selectedAsset.parent_id,
        sort_order: selectedAsset.sort_order,
        created_by: selectedAsset.created_by,
        created_by_username: '',
        created_at: selectedAsset.created_at,
        version_count: 0,
        latest_version: null,
        latest_version_notes: null,
      };
      onNavigateToHistory(assetInfo);
    }
  };

  const toggleBulkMode = (enabled: boolean) => {
    setIsBulkModeEnabled(enabled);
    if (!enabled) {
      resetSelection();
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

  // Extract asset IDs for bulk operations
  const availableAssetIds = useMemo(() => {
    const extractIds = (assets: AssetHierarchy[]): number[] => {
      const ids: number[] = [];
      for (const asset of assets) {
        ids.push(asset.id);
        if (asset.children && asset.children.length > 0) {
          ids.push(...extractIds(asset.children));
        }
      }
      return ids;
    };
    
    return hierarchyData ? extractIds(hierarchyData) : [];
  }, [hierarchyData]);

  const renderToolbar = () => (
    <Space size="middle" style={{ marginBottom: '16px' }}>
      <Button 
        type="primary" 
        icon={<PlusOutlined />} 
        onClick={handleCreateAsset}
        disabled={!user || isLoading}
      >
        Create Asset
      </Button>
      
      <Button 
        icon={<ReloadOutlined />} 
        onClick={handleRefresh}
        loading={isRefreshing}
        disabled={isLoading}
      >
        Refresh
      </Button>

      {enableBulkOperations && (
        <>
          <Divider type="vertical" />
          <Space align="center">
            <Text>Bulk Mode:</Text>
            <Switch
              checked={isBulkModeEnabled}
              onChange={toggleBulkMode}
              checkedChildren={<SelectOutlined />}
              unCheckedChildren={<UnorderedListOutlined />}
            />
            {selectedBulkCount > 0 && (
              <Badge 
                count={selectedBulkCount} 
                style={{ backgroundColor: '#1890ff' }}
              />
            )}
          </Space>
        </>
      )}
    </Space>
  );

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
        style={{ height: '100%' }}
        title={
          <Space align="center">
            {isDevice ? <ToolOutlined /> : <FolderOutlined />}
            <Text strong>{selectedAsset.name}</Text>
            {vaultInfo && <VaultAccessIndicator />}
          </Space>
        }
        extra={
          <Space>
            <Tooltip title="View Configuration History">
              <Button 
                type="text" 
                icon={<HistoryOutlined />}
                onClick={handleViewHistory}
                disabled={!isDevice}
              />
            </Tooltip>
          </Space>
        }
      >
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          <div>
            <Space direction="vertical" size="small">
              <div>
                <Text strong>Type: </Text>
                <Text>{selectedAsset.asset_type}</Text>
              </div>
              
              <div>
                <Text strong>Description: </Text>
                <Text>{selectedAsset.description || 'No description provided'}</Text>
              </div>

              <div>
                <Text strong>Created: </Text>
                <Text>{formatDate(selectedAsset.created_at)}</Text>
              </div>

              <div>
                <Text strong>ID: </Text>
                <Text code>{selectedAsset.id}</Text>
              </div>
            </Space>
          </div>

          {isDevice && (
            <>
              <Divider />
              <div>
                <Space direction="vertical" size="small" style={{ width: '100%' }}>
                  <Text strong>Device Information</Text>
                  
                  <Row gutter={[16, 8]}>
                    <Col span={12}>
                      <Space align="center">
                        <DatabaseOutlined />
                        <Text type="secondary">Configurations</Text>
                      </Space>
                    </Col>
                    <Col span={12}>
                      <Text>Available</Text>
                    </Col>
                  </Row>

                  <Row gutter={[16, 8]}>
                    <Col span={12}>
                      <Space align="center">
                        <SafetyOutlined />
                        <Text type="secondary">Vault</Text>
                      </Space>
                    </Col>
                    <Col span={12}>
                      <Text>{vaultInfo ? 'Protected' : 'None'}</Text>
                    </Col>
                  </Row>
                </Space>
              </div>
            </>
          )}
        </Space>
      </Card>
    );
  };

  if (error) {
    return (
      <Alert
        message="Error Loading Asset Hierarchy"
        description={error}
        type="error"
        action={
          <Button size="small" danger onClick={loadHierarchy}>
            Retry
          </Button>
        }
      />
    );
  }

  return (
    <div className="enhanced-asset-hierarchy-view">
      <Layout style={{ minHeight: '600px' }}>
        <Sider 
          width={400} 
          style={{ 
            background: '#fff',
            borderRight: '1px solid #f0f0f0'
          }}
        >
          <div style={{ padding: '16px' }}>
            <Space direction="vertical" size="middle" style={{ width: '100%' }}>
              <div>
                <Breadcrumb>
                  <Breadcrumb.Item>
                    <HomeOutlined />
                    <span style={{ marginLeft: '4px' }}>Assets</span>
                  </Breadcrumb.Item>
                </Breadcrumb>
              </div>

              {renderToolbar()}

              {/* Bulk Selection Manager */}
              {enableBulkOperations && isBulkModeEnabled && (
                <BulkSelectionManager 
                  availableAssetIds={availableAssetIds}
                  className="asset-hierarchy-bulk-manager"
                />
              )}

              {/* Bulk Operation Toolbar */}
              {enableBulkOperations && bulkUI.showSelectionToolbar && (
                <BulkOperationToolbar className="asset-hierarchy-bulk-toolbar" />
              )}

              <Card 
                size="small" 
                title="Asset Hierarchy" 
                style={{ height: isBulkModeEnabled ? 'calc(100vh - 350px)' : 'calc(100vh - 250px)' }}
                bodyStyle={{ 
                  padding: '8px', 
                  height: 'calc(100% - 40px)', 
                  overflow: 'auto' 
                }}
              >
                {isLoading ? (
                  <div style={{ textAlign: 'center', padding: '40px' }}>
                    <Spin size="large" />
                    <div style={{ marginTop: '16px' }}>
                      <Text type="secondary">Loading asset hierarchy...</Text>
                    </div>
                  </div>
                ) : hierarchyData && hierarchyData.length > 0 ? (
                  <AssetTreeView
                    assets={hierarchyData}
                    selectedAsset={selectedAsset}
                    onAssetSelect={handleAssetSelect}
                    enableBulkSelection={isBulkModeEnabled}
                    bulkSelectionContext="tree"
                  />
                ) : (
                  <Empty 
                    description="No assets found"
                    image={Empty.PRESENTED_IMAGE_SIMPLE}
                  />
                )}
              </Card>
            </Space>
          </div>
        </Sider>

        <Content style={{ padding: '16px', background: '#fff' }}>
          {renderSelectedAssetDetails()}
        </Content>
      </Layout>

      {/* Modals */}
      <CreateAssetModal
        visible={showCreateModal}
        onClose={() => setShowCreateModal(false)}
        onAssetCreated={handleAssetCreated}
        parentAsset={selectedAsset?.asset_type === 'Folder' ? selectedAsset : null}
      />

      {/* Bulk Operations Progress Modal */}
      {enableBulkOperations && <BulkProgressModal />}
    </div>
  );
};

export default EnhancedAssetHierarchyView;