import React, { useState, useEffect } from 'react';
import { 
  Card, 
  Typography, 
  Space, 
  Tree, 
  Tag, 
  Tooltip, 
  Button,
  Switch,
  Select,
  Empty,
  Alert,
  Badge,
  Divider
} from 'antd';
import { 
  DatabaseOutlined,
  FileOutlined,
  FolderOutlined,
  LinkOutlined,
  BugOutlined,
  CheckCircleOutlined,
  ExclamationCircleOutlined,
  SettingOutlined
} from '@ant-design/icons';
import { AssociationInfo, AssetInfo, ValidationResult, HealthStatus } from '../../types/associations';
import useAssociationStore from '../../store/associations';

const { Title, Text } = Typography;
const { TreeNode } = Tree;
const { Option } = Select;

interface AssociationVisualizationProps {
  assetId?: number;
  showHealthStatus?: boolean;
  interactive?: boolean;
  compact?: boolean;
}

interface AssetNode {
  id: number;
  name: string;
  type: 'folder' | 'device';
  parentId?: number;
  children: AssetNode[];
  associations: AssociationInfo[];
  healthStatus?: HealthStatus;
}

interface VisualizationSettings {
  showEmptyAssets: boolean;
  groupByType: boolean;
  showValidationStatus: boolean;
  expandAll: boolean;
}

const AssociationVisualization: React.FC<AssociationVisualizationProps> = ({
  assetId,
  showHealthStatus = true,
  interactive = true,
  compact = false
}) => {
  const {
    associations,
    healthStatuses,
    loading,
    error,
    loadAssociations,
    loadHealthStatus
  } = useAssociationStore();

  const [assetHierarchy, setAssetHierarchy] = useState<AssetNode[]>([]);
  const [expandedKeys, setExpandedKeys] = useState<string[]>([]);
  const [selectedKeys, setSelectedKeys] = useState<string[]>([]);
  const [settings, setSettings] = useState<VisualizationSettings>({
    showEmptyAssets: true,
    groupByType: false,
    showValidationStatus: true,
    expandAll: false
  });

  useEffect(() => {
    if (assetId) {
      loadData();
    }
  }, [assetId]);

  useEffect(() => {
    if (settings.expandAll) {
      const allKeys = getAllNodeKeys(assetHierarchy);
      setExpandedKeys(allKeys);
    } else {
      setExpandedKeys([]);
    }
  }, [settings.expandAll, assetHierarchy]);

  const loadData = async () => {
    if (!assetId) return;

    try {
      await Promise.all([
        loadAssociations(assetId),
        showHealthStatus ? loadHealthStatus(assetId) : Promise.resolve()
      ]);
      
      // Load asset hierarchy and build visualization
      await loadAssetHierarchy();
    } catch (error) {
      console.error('Failed to load visualization data:', error);
    }
  };

  const loadAssetHierarchy = async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const hierarchy = await invoke<any[]>('get_asset_hierarchy');
      
      // Build asset nodes with associations
      const assetNodes = buildAssetNodes(hierarchy);
      setAssetHierarchy(assetNodes);
    } catch (error) {
      console.error('Failed to load asset hierarchy:', error);
    }
  };

  const buildAssetNodes = (hierarchy: any[]): AssetNode[] => {
    return hierarchy.map(asset => ({
      id: asset.id,
      name: asset.name,
      type: asset.asset_type === 'Folder' ? 'folder' : 'device',
      parentId: asset.parent_id,
      children: buildAssetNodes(asset.children || []),
      associations: associations.filter(a => a.assetId === asset.id),
      healthStatus: healthStatuses.get(asset.id)
    }));
  };

  const getAllNodeKeys = (nodes: AssetNode[]): string[] => {
    const keys: string[] = [];
    
    const traverse = (nodeList: AssetNode[]) => {
      nodeList.forEach(node => {
        keys.push(node.id.toString());
        if (node.children.length > 0) {
          traverse(node.children);
        }
      });
    };
    
    traverse(nodes);
    return keys;
  };

  const getAssociationSummary = (associations: AssociationInfo[]) => {
    const configCount = associations.filter(a => a.fileType === 'Configuration').length;
    const firmwareCount = associations.filter(a => a.fileType === 'Firmware').length;
    const warningCount = associations.filter(a => a.validationStatus === 'Warning').length;
    const errorCount = associations.filter(a => a.validationStatus === 'Failed').length;
    
    return { configCount, firmwareCount, warningCount, errorCount, total: associations.length };
  };

  const getHealthIcon = (healthStatus?: HealthStatus) => {
    if (!healthStatus) return null;
    
    if (healthStatus.healthy) {
      return <CheckCircleOutlined style={{ color: '#52c41a' }} />;
    } else {
      return <ExclamationCircleOutlined style={{ color: '#ff4d4f' }} />;
    }
  };

  const renderAssetNode = (node: AssetNode): React.ReactNode => {
    const { configCount, firmwareCount, warningCount, errorCount, total } = getAssociationSummary(node.associations);
    
    // Filter based on settings
    if (!settings.showEmptyAssets && total === 0 && node.type === 'device') {
      return null;
    }

    const hasIssues = errorCount > 0 || warningCount > 0;
    
    return (
      <TreeNode
        key={node.id.toString()}
        title={
          <Space size="small">
            {/* Asset Icon and Name */}
            {node.type === 'folder' ? (
              <FolderOutlined style={{ color: '#faad14' }} />
            ) : (
              <DatabaseOutlined style={{ color: '#1890ff' }} />
            )}
            
            <Text strong={node.type === 'folder'}>
              {node.name}
            </Text>

            {/* Association Count Badges */}
            {total > 0 && (
              <Space size={4}>
                {configCount > 0 && (
                  <Badge count={configCount} size="small" style={{ backgroundColor: '#52c41a' }}>
                    <Tag size="small" color="green">Config</Tag>
                  </Badge>
                )}
                {firmwareCount > 0 && (
                  <Badge count={firmwareCount} size="small" style={{ backgroundColor: '#1890ff' }}>
                    <Tag size="small" color="blue">Firmware</Tag>
                  </Badge>
                )}
              </Space>
            )}

            {/* Validation Status */}
            {settings.showValidationStatus && hasIssues && (
              <Space size={2}>
                {errorCount > 0 && (
                  <Tooltip title={`${errorCount} validation error(s)`}>
                    <Badge count={errorCount} size="small" style={{ backgroundColor: '#ff4d4f' }}>
                      <BugOutlined style={{ color: '#ff4d4f' }} />
                    </Badge>
                  </Tooltip>
                )}
                {warningCount > 0 && (
                  <Tooltip title={`${warningCount} validation warning(s)`}>
                    <Badge count={warningCount} size="small" style={{ backgroundColor: '#faad14' }}>
                      <ExclamationCircleOutlined style={{ color: '#faad14' }} />
                    </Badge>
                  </Tooltip>
                )}
              </Space>
            )}

            {/* Health Status */}
            {showHealthStatus && node.healthStatus && (
              <Tooltip title={
                node.healthStatus.healthy 
                  ? 'All associations healthy' 
                  : `Issues: ${node.healthStatus.issues.join(', ')}`
              }>
                {getHealthIcon(node.healthStatus)}
              </Tooltip>
            )}

            {/* Association Link Icon */}
            {total > 0 && (
              <LinkOutlined style={{ color: '#52c41a', fontSize: '12px' }} />
            )}
          </Space>
        }
        selectable={interactive}
      >
        {node.children.map(child => renderAssetNode(child)).filter(Boolean)}
        
        {/* Render individual file associations if this is a device */}
        {node.type === 'device' && node.associations.length > 0 && (
          node.associations.map(association => (
            <TreeNode
              key={`assoc-${association.id}`}
              title={
                <Space size="small">
                  <FileOutlined style={{ 
                    color: association.fileType === 'Configuration' ? '#52c41a' : '#1890ff' 
                  }} />
                  <Text>{association.fileName}</Text>
                  <Tag 
                    size="small" 
                    color={association.fileType === 'Configuration' ? 'green' : 'blue'}
                  >
                    {association.fileType}
                  </Tag>
                  {settings.showValidationStatus && association.validationStatus !== 'Passed' && (
                    <Tag 
                      size="small" 
                      color={association.validationStatus === 'Failed' ? 'red' : 'orange'}
                    >
                      {association.validationStatus}
                    </Tag>
                  )}
                  <Text type="secondary" style={{ fontSize: '11px' }}>
                    Order: {association.associationOrder}
                  </Text>
                </Space>
              }
              selectable={false}
              isLeaf
            />
          ))
        )}
      </TreeNode>
    );
  };

  const handleNodeSelect = (selectedKeys: React.Key[], info: any) => {
    if (!interactive) return;
    
    setSelectedKeys(selectedKeys as string[]);
    
    // If a device is selected, we could trigger additional actions
    if (selectedKeys.length > 0) {
      const nodeId = parseInt(selectedKeys[0] as string);
      // Could trigger association details view, etc.
    }
  };

  const handleSettingChange = (key: keyof VisualizationSettings, value: boolean) => {
    setSettings(prev => ({ ...prev, [key]: value }));
  };

  if (loading) {
    return (
      <Card title="Asset-File Relationships">
        <div style={{ textAlign: 'center', padding: '40px 0' }}>
          <Text>Loading relationship visualization...</Text>
        </div>
      </Card>
    );
  }

  if (error) {
    return (
      <Card title="Asset-File Relationships">
        <Alert
          message="Error Loading Relationships"
          description={error}
          type="error"
          showIcon
        />
      </Card>
    );
  }

  return (
    <Card 
      title={
        <Space>
          <LinkOutlined />
          <Title level={4} style={{ margin: 0 }}>
            Asset-File Relationships
          </Title>
        </Space>
      }
      extra={
        interactive && (
          <Button 
            icon={<SettingOutlined />} 
            size="small"
            onClick={() => {/* Could open settings modal */}}
          >
            Settings
          </Button>
        )
      }
      size={compact ? 'small' : 'default'}
    >
      <Space direction="vertical" style={{ width: '100%' }} size="middle">
        {/* Settings Panel */}
        {interactive && !compact && (
          <Card size="small" style={{ backgroundColor: '#fafafa' }}>
            <Space wrap>
              <Space>
                <Text>Show empty assets:</Text>
                <Switch
                  size="small"
                  checked={settings.showEmptyAssets}
                  onChange={(checked) => handleSettingChange('showEmptyAssets', checked)}
                />
              </Space>
              <Space>
                <Text>Show validation status:</Text>
                <Switch
                  size="small"
                  checked={settings.showValidationStatus}
                  onChange={(checked) => handleSettingChange('showValidationStatus', checked)}
                />
              </Space>
              <Space>
                <Text>Expand all:</Text>
                <Switch
                  size="small"
                  checked={settings.expandAll}
                  onChange={(checked) => handleSettingChange('expandAll', checked)}
                />
              </Space>
            </Space>
          </Card>
        )}

        {/* Visualization Tree */}
        {assetHierarchy.length === 0 ? (
          <Empty 
            description="No asset relationships to display"
            style={{ margin: '40px 0' }}
          />
        ) : (
          <Tree
            showLine={{ showLeafIcon: false }}
            showIcon={false}
            selectedKeys={selectedKeys}
            expandedKeys={expandedKeys}
            onSelect={handleNodeSelect}
            onExpand={(expandedKeys) => setExpandedKeys(expandedKeys as string[])}
            style={{ 
              background: '#fafafa', 
              padding: '16px', 
              borderRadius: '6px',
              maxHeight: compact ? '300px' : '600px',
              overflowY: 'auto'
            }}
          >
            {assetHierarchy.map(node => renderAssetNode(node)).filter(Boolean)}
          </Tree>
        )}

        {/* Legend */}
        {!compact && (
          <>
            <Divider />
            <Card size="small" title="Legend" style={{ backgroundColor: '#fafafa' }}>
              <Space direction="vertical" size="small">
                <Space wrap>
                  <Space size="small">
                    <FolderOutlined style={{ color: '#faad14' }} />
                    <Text>Folder</Text>
                  </Space>
                  <Space size="small">
                    <DatabaseOutlined style={{ color: '#1890ff' }} />
                    <Text>Device</Text>
                  </Space>
                  <Space size="small">
                    <FileOutlined style={{ color: '#52c41a' }} />
                    <Text>Configuration File</Text>
                  </Space>
                  <Space size="small">
                    <FileOutlined style={{ color: '#1890ff' }} />
                    <Text>Firmware File</Text>
                  </Space>
                </Space>
                <Space wrap>
                  <Space size="small">
                    <CheckCircleOutlined style={{ color: '#52c41a' }} />
                    <Text>Healthy</Text>
                  </Space>
                  <Space size="small">
                    <ExclamationCircleOutlined style={{ color: '#faad14' }} />
                    <Text>Warning</Text>
                  </Space>
                  <Space size="small">
                    <BugOutlined style={{ color: '#ff4d4f' }} />
                    <Text>Error</Text>
                  </Space>
                  <Space size="small">
                    <LinkOutlined style={{ color: '#52c41a' }} />
                    <Text>Has Associations</Text>
                  </Space>
                </Space>
              </Space>
            </Card>
          </>
        )}
      </Space>
    </Card>
  );
};

export default AssociationVisualization;