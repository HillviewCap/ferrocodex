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
  Tooltip,
  Modal,
  Tabs
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
  CloudUploadOutlined,
  KeyOutlined
} from '@ant-design/icons';
import { AssetTreeView } from './AssetTreeView';
import { SimpleAssetCreation } from '../security/SimpleAssetCreation';
import { AssetHierarchy, AssetType, AssetInfo } from '../../types/assets';
import useAuthStore from '../../store/auth';
import { useHierarchyStore, useHierarchyData, useHierarchyLoading, useHierarchyError, useSelectedAsset } from '../../store/hierarchy';
import { invoke } from '@tauri-apps/api/core';
import { VaultInfo } from '../../types/vault';
import VaultAccessIndicator from '../VaultAccessIndicator';
import IdentityVault from '../IdentityVault';
import QuickConfigImportModal from './QuickConfigImportModal';
import QuickFirmwareUploadModal from './QuickFirmwareUploadModal';
import MetadataDisplay from '../metadata/MetadataDisplay';

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
  const [configModalVisible, setConfigModalVisible] = useState(false);
  const [firmwareModalVisible, setFirmwareModalVisible] = useState(false);
  const [vaultModalVisible, setVaultModalVisible] = useState(false);
  const [assetFileStats, setAssetFileStats] = useState<{configCount: number, firmwareCount: number} | null>(null);

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

  // Load asset file statistics
  useEffect(() => {
    if (selectedAsset && selectedAsset.asset_type === 'Device' && token) {
      Promise.all([
        invoke<any[]>('list_configurations_for_asset', { token, assetId: selectedAsset.id }).catch(() => []),
        invoke<any[]>('list_firmware_for_asset', { token, assetId: selectedAsset.id }).catch(() => [])
      ])
        .then(([configs, firmware]) => {
          setAssetFileStats({
            configCount: configs.length,
            firmwareCount: firmware.length
          });
        })
        .catch(err => {
          console.error('Failed to load asset file stats:', err);
          setAssetFileStats({ configCount: 0, firmwareCount: 0 });
        });
    } else {
      setAssetFileStats(null);
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

  const handleAddConfig = () => {
    if (selectedAsset && selectedAsset.asset_type === 'Device') {
      setConfigModalVisible(true);
    }
  };

  const handleUploadFirmware = () => {
    if (selectedAsset && selectedAsset.asset_type === 'Device') {
      setFirmwareModalVisible(true);
    }
  };

  const handleConfigImportSuccess = () => {
    setConfigModalVisible(false);
    message.success('Configuration imported successfully!');
    // Refresh file stats
    if (selectedAsset && token) {
      invoke<any[]>('list_configurations_for_asset', { token, assetId: selectedAsset.id })
        .then(configs => {
          setAssetFileStats(prev => prev ? { ...prev, configCount: configs.length } : null);
        })
        .catch(console.error);
    }
  };

  const handleFirmwareUploadSuccess = () => {
    setFirmwareModalVisible(false);
    message.success('Firmware uploaded successfully!');
    // Refresh file stats
    if (selectedAsset && token) {
      invoke<any[]>('list_firmware_for_asset', { token, assetId: selectedAsset.id })
        .then(firmware => {
          setAssetFileStats(prev => prev ? { ...prev, firmwareCount: firmware.length } : null);
        })
        .catch(console.error);
    }
  };

  const handleManageVault = () => {
    if (selectedAsset && selectedAsset.asset_type === 'Device') {
      setVaultModalVisible(true);
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
              <>
                <Tooltip title="Add Configuration">
                  <Button 
                    icon={<DatabaseOutlined />} 
                    onClick={handleAddConfig}
                    type="primary"
                    size="small"
                  >
                    Add Config
                  </Button>
                </Tooltip>
                <Tooltip title="Upload Firmware">
                  <Button 
                    icon={<CloudUploadOutlined />} 
                    onClick={handleUploadFirmware}
                    size="small"
                  >
                    Add Firmware
                  </Button>
                </Tooltip>
                <Tooltip title="View Configuration History">
                  <Button 
                    icon={<HistoryOutlined />} 
                    onClick={handleViewHistory}
                    type="text"
                  />
                </Tooltip>
              </>
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
        bodyStyle={{ padding: 0 }}
      >
        <Tabs defaultActiveKey="details" style={{ height: '100%' }}>
          <Tabs.TabPane tab="Details" key="details">
            <div style={{ padding: '16px' }}>
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
                vaultId={vaultInfo?.vault.id || 0}
                compact={true}
              />
            </div>
          )}

          {/* Asset File Statistics */}
          {isDevice && assetFileStats && (
            <div>
              <Text strong>Files: </Text>
              <Space>
                <div>
                  <DatabaseOutlined style={{ color: '#1890ff', marginRight: 4 }} />
                  <Text type="secondary">{assetFileStats.configCount} Config{assetFileStats.configCount !== 1 ? 's' : ''}</Text>
                </div>
                <div>
                  <CloudUploadOutlined style={{ color: '#52c41a', marginRight: 4 }} />
                  <Text type="secondary">{assetFileStats.firmwareCount} Firmware</Text>
                </div>
              </Space>
            </div>
          )}

          {/* Quick Actions */}
          {isDevice && (
            <div>
              <Text strong>Quick Actions: </Text>
              <Space wrap style={{ marginTop: 8 }}>
                <Button 
                  size="small" 
                  icon={<DatabaseOutlined />} 
                  onClick={handleAddConfig}
                  type="primary"
                >
                  Add Configuration
                </Button>
                <Button 
                  size="small" 
                  icon={<CloudUploadOutlined />} 
                  onClick={handleUploadFirmware}
                >
                  Upload Firmware
                </Button>
                <Button 
                  size="small" 
                  icon={<KeyOutlined />} 
                  onClick={handleManageVault}
                >
                  Identity Vault
                </Button>
                <Button 
                  size="small" 
                  icon={<HistoryOutlined />} 
                  onClick={handleViewHistory}
                  type="default"
                >
                  View History
                </Button>
              </Space>
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
            </div>
          </Tabs.TabPane>
          
          {isDevice && (
            <Tabs.TabPane tab="Metadata" key="metadata">
              <div style={{ height: '100%', overflow: 'auto' }}>
                <MetadataDisplay
                  assetId={selectedAsset.id}
                  schemaId={1} // TODO: Get schema ID from asset type
                  onRefresh={() => {
                    // Optionally refresh other data
                    console.log('Metadata refreshed');
                  }}
                />
              </div>
            </Tabs.TabPane>
          )}
        </Tabs>
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

      <SimpleAssetCreation
        open={createModalVisible}
        onCancel={() => setCreateModalVisible(false)}
        onSuccess={handleCreateSuccess}
        hierarchyData={hierarchyData}
        initialParentId={createModalParentId}
        initialAssetType={createModalAssetType}
      />

      {/* Quick Config Import Modal */}
      {selectedAsset && (
        <QuickConfigImportModal
          visible={configModalVisible}
          onCancel={() => setConfigModalVisible(false)}
          onSuccess={handleConfigImportSuccess}
          assetId={selectedAsset.id}
          assetName={selectedAsset.name}
        />
      )}

      {/* Quick Firmware Upload Modal */}
      {selectedAsset && (
        <QuickFirmwareUploadModal
          visible={firmwareModalVisible}
          onCancel={() => setFirmwareModalVisible(false)}
          onSuccess={handleFirmwareUploadSuccess}
          assetId={selectedAsset.id}
          assetName={selectedAsset.name}
        />
      )}

      {/* Identity Vault Modal */}
      {selectedAsset && selectedAsset.asset_type === 'Device' && (
        <Modal
          title={
            <Space>
              <KeyOutlined />
              <span>Identity Vault - {selectedAsset.name}</span>
            </Space>
          }
          open={vaultModalVisible}
          onCancel={() => setVaultModalVisible(false)}
          footer={null}
          width={1000}
          destroyOnClose
        >
          <IdentityVault 
            asset={{
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
            }}
          />
        </Modal>
      )}
    </div>
  );
};

export default AssetHierarchyView;