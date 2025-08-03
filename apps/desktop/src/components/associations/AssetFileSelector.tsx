import React, { useState, useEffect } from 'react';
import { 
  Card, 
  Tree, 
  Input, 
  Typography, 
  Space, 
  Tag, 
  Empty,
  Spin,
  Alert,
  Tooltip
} from 'antd';
import { 
  FolderOutlined, 
  DatabaseOutlined, 
  SearchOutlined,
  InfoCircleOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { AssetInfo, AssetSelectorProps, AssociationType } from '../../types/associations';

const { Search } = Input;
const { Title, Text } = Typography;
const { TreeNode } = Tree;

interface AssetHierarchy {
  id: number;
  name: string;
  description: string;
  assetType: 'Folder' | 'Device';
  parentId?: number;
  sortOrder: number;
  children: AssetHierarchy[];
  createdBy: number;
  createdAt: string;
  updatedAt: string;
}

const AssetFileSelector: React.FC<AssetSelectorProps> = ({
  onAssetSelect,
  selectedAsset,
  fileType,
  disabled = false
}) => {
  const [assets, setAssets] = useState<AssetHierarchy[]>([]);
  const [filteredAssets, setFilteredAssets] = useState<AssetHierarchy[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchValue, setSearchValue] = useState('');
  const [expandedKeys, setExpandedKeys] = useState<string[]>([]);
  const [selectedKeys, setSelectedKeys] = useState<string[]>([]);

  useEffect(() => {
    loadAssets();
  }, []);

  useEffect(() => {
    if (selectedAsset) {
      setSelectedKeys([selectedAsset.id.toString()]);
    }
  }, [selectedAsset]);

  useEffect(() => {
    if (searchValue) {
      const filtered = filterAssets(assets, searchValue.toLowerCase());
      setFilteredAssets(filtered);
      // Auto-expand filtered results
      const expandKeys = getAllKeys(filtered);
      setExpandedKeys(expandKeys);
    } else {
      setFilteredAssets(assets);
    }
  }, [searchValue, assets]);

  const loadAssets = async () => {
    setLoading(true);
    setError(null);
    
    try {
      const hierarchy = await invoke<AssetHierarchy[]>('get_asset_hierarchy');
      setAssets(hierarchy);
      setFilteredAssets(hierarchy);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load assets';
      setError(errorMessage);
    } finally {
      setLoading(false);
    }
  };

  const filterAssets = (assetList: AssetHierarchy[], searchTerm: string): AssetHierarchy[] => {
    return assetList.reduce<AssetHierarchy[]>((filtered, asset) => {
      const matchesSearch = asset.name.toLowerCase().includes(searchTerm) ||
                           asset.description.toLowerCase().includes(searchTerm);
      
      const filteredChildren = filterAssets(asset.children, searchTerm);
      
      if (matchesSearch || filteredChildren.length > 0) {
        filtered.push({
          ...asset,
          children: filteredChildren
        });
      }
      
      return filtered;
    }, []);
  };

  const getAllKeys = (assetList: AssetHierarchy[]): string[] => {
    const keys: string[] = [];
    
    const traverse = (assets: AssetHierarchy[]) => {
      assets.forEach(asset => {
        keys.push(asset.id.toString());
        if (asset.children.length > 0) {
          traverse(asset.children);
        }
      });
    };
    
    traverse(assetList);
    return keys;
  };

  const findAssetById = (assetList: AssetHierarchy[], id: number): AssetHierarchy | null => {
    for (const asset of assetList) {
      if (asset.id === id) {
        return asset;
      }
      const found = findAssetById(asset.children, id);
      if (found) {
        return found;
      }
    }
    return null;
  };

  const handleSelect = (selectedKeys: React.Key[]) => {
    if (disabled || selectedKeys.length === 0) return;
    
    const assetId = parseInt(selectedKeys[0] as string);
    const asset = findAssetById(assets, assetId);
    
    if (asset) {
      // Only allow selection of devices for file associations
      if (asset.assetType === 'Device') {
        setSelectedKeys([assetId.toString()]);
        
        // Convert to AssetInfo format
        const assetInfo: AssetInfo = {
          id: asset.id,
          name: asset.name,
          description: asset.description,
          assetType: asset.assetType,
          parentId: asset.parentId,
          sortOrder: asset.sortOrder,
          createdBy: asset.createdBy,
          createdByUsername: '', // This would be populated in a real scenario
          createdAt: asset.createdAt,
          versionCount: 0, // This would be populated in a real scenario
        };
        
        onAssetSelect(assetInfo);
      }
    }
  };

  const handleExpand = (expandedKeys: React.Key[]) => {
    setExpandedKeys(expandedKeys as string[]);
  };

  const renderTreeNodes = (assetList: AssetHierarchy[]): React.ReactNode => {
    return assetList.map(asset => {
      const isDevice = asset.assetType === 'Device';
      const isSelectable = isDevice && !disabled;
      
      return (
        <TreeNode
          key={asset.id.toString()}
          title={
            <Space>
              {isDevice ? (
                <DatabaseOutlined style={{ color: '#1890ff' }} />
              ) : (
                <FolderOutlined style={{ color: '#faad14' }} />
              )}
              <span style={{ 
                fontWeight: isDevice ? 'normal' : 'bold',
                color: isSelectable ? '#000' : '#999'
              }}>
                {asset.name}
              </span>
              <Tag color={isDevice ? 'blue' : 'orange'} size="small">
                {asset.assetType}
              </Tag>
              {!isSelectable && isDevice && (
                <Tooltip title="This device may not be compatible with the selected file type">
                  <InfoCircleOutlined style={{ color: '#faad14' }} />
                </Tooltip>
              )}
            </Space>
          }
          selectable={isSelectable}
          disabled={!isSelectable}
        >
          {asset.children.length > 0 && renderTreeNodes(asset.children)}
        </TreeNode>
      );
    });
  };

  const getInstructions = () => {
    if (fileType) {
      return `Select a device to associate with ${fileType.toLowerCase()} files. Only devices can have file associations.`;
    }
    return 'Select a device to associate files with. Only devices can have file associations.';
  };

  if (loading) {
    return (
      <Card>
        <div style={{ textAlign: 'center', padding: '40px 0' }}>
          <Spin size="large" />
          <div style={{ marginTop: 16 }}>
            <Text>Loading assets...</Text>
          </div>
        </div>
      </Card>
    );
  }

  if (error) {
    return (
      <Card>
        <Alert
          message="Error Loading Assets"
          description={error}
          type="error"
          showIcon
          action={
            <Button size="small" onClick={loadAssets}>
              Retry
            </Button>
          }
        />
      </Card>
    );
  }

  return (
    <Card 
      title={
        <Space>
          <DatabaseOutlined />
          <Title level={4} style={{ margin: 0 }}>
            Select Target Asset
          </Title>
        </Space>
      }
    >
      <Space direction="vertical" style={{ width: '100%' }}>
        <Alert
          message="Instructions"
          description={getInstructions()}
          type="info"
          showIcon
          style={{ marginBottom: 16 }}
        />

        <Search
          placeholder="Search assets..."
          value={searchValue}
          onChange={(e) => setSearchValue(e.target.value)}
          prefix={<SearchOutlined />}
          allowClear
          disabled={disabled}
        />

        {selectedAsset && (
          <Alert
            message="Selected Asset"
            description={
              <Space>
                <DatabaseOutlined />
                <Text strong>{selectedAsset.name}</Text>
                <Text type="secondary">- {selectedAsset.description}</Text>
              </Space>
            }
            type="success"
            style={{ marginBottom: 16 }}
          />
        )}

        {filteredAssets.length === 0 ? (
          <Empty
            description={searchValue ? 'No assets match your search' : 'No assets available'}
            style={{ margin: '40px 0' }}
          />
        ) : (
          <Tree
            showLine={{ showLeafIcon: false }}
            showIcon={false}
            selectedKeys={selectedKeys}
            expandedKeys={expandedKeys}
            onSelect={handleSelect}
            onExpand={handleExpand}
            style={{ 
              background: '#fafafa', 
              padding: '16px', 
              borderRadius: '6px',
              maxHeight: '400px',
              overflowY: 'auto'
            }}
          >
            {renderTreeNodes(filteredAssets)}
          </Tree>
        )}
      </Space>
    </Card>
  );
};

export default AssetFileSelector;