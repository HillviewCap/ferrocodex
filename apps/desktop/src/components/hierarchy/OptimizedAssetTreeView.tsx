import React, { useState, useCallback, useMemo, useRef, useEffect } from 'react';
import { Tree, Typography, Input, Spin, Empty } from 'antd';
import { SearchOutlined, FolderOutlined, FolderOpenOutlined, ToolOutlined } from '@ant-design/icons';
import type { TreeProps, DataNode } from 'antd/es/tree';
import { AssetHierarchy, AssetType } from '../../types/assets';
import { 
  HierarchyPerformanceOptimizer, 
  memoizeTreeNode, 
  createDebouncedSearch,
  flattenTree 
} from '../../store/hierarchyOptimizations';

const { Text } = Typography;
const { Search } = Input;

export interface OptimizedAssetTreeViewProps {
  hierarchyData: AssetHierarchy[];
  onAssetSelect?: (asset: AssetHierarchy | null) => void;
  onAssetCreate?: (parentId: number | null, assetType: AssetType) => void;
  onAssetMove?: (assetId: number, newParentId: number | null) => void;
  selectedAssetId?: number | null;
  loading?: boolean;
  allowDragDrop?: boolean;
  enableSearch?: boolean;
  enableVirtualization?: boolean;
}

interface OptimizedTreeDataNode extends DataNode {
  asset: AssetHierarchy;
  isFolder: boolean;
}

export const OptimizedAssetTreeView: React.FC<OptimizedAssetTreeViewProps> = ({
  hierarchyData,
  onAssetSelect,
  selectedAssetId,
  loading = false,
  allowDragDrop = true,
  enableSearch = true,
  enableVirtualization = false,
}) => {
  const [expandedKeys, setExpandedKeys] = useState<React.Key[]>([]);
  const [selectedKeys, setSelectedKeys] = useState<React.Key[]>([]);
  const [searchValue, setSearchValue] = useState('');
  const [filteredData, setFilteredData] = useState<AssetHierarchy[]>([]);
  
  const treeRef = useRef<any>();

  // Calculate performance metrics
  const performanceMetrics = useMemo(() => {
    const calculateTreeSize = (nodes: AssetHierarchy[]): number => {
      return nodes.reduce((total, node) => {
        return total + 1 + calculateTreeSize(node.children);
      }, 0);
    };

    const calculateMaxDepth = (nodes: AssetHierarchy[], depth = 0): number => {
      if (nodes.length === 0) return depth;
      return Math.max(...nodes.map(node => 
        Math.max(depth + 1, calculateMaxDepth(node.children, depth + 1))
      ));
    };

    const treeSize = calculateTreeSize(hierarchyData);
    const maxDepth = calculateMaxDepth(hierarchyData);

    return {
      treeSize,
      maxDepth,
      renderTime: 0,
      lastOptimization: new Date(),
    };
  }, [hierarchyData]);

  // Get performance settings
  const performanceSettings = useMemo(() => 
    HierarchyPerformanceOptimizer.getPerformanceSettings(performanceMetrics),
    [performanceMetrics]
  );

  // Memoized tree node converter
  const convertToTreeData = useMemo(() => {
    const converter = memoizeTreeNode((asset: AssetHierarchy): OptimizedTreeDataNode => {
      const isFolder = asset.asset_type === 'Folder';
      
      return {
        key: asset.id.toString(),
        title: (
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            {isFolder ? (
              expandedKeys.includes(asset.id.toString()) ? 
                <FolderOpenOutlined style={{ color: '#1890ff' }} /> : 
                <FolderOutlined style={{ color: '#1890ff' }} />
            ) : (
              <ToolOutlined style={{ color: '#52c41a' }} />
            )}
            <Text strong={isFolder}>{asset.name}</Text>
            {isFolder && asset.children.length > 0 && (
              <Text type="secondary" style={{ fontSize: '12px' }}>
                ({asset.children.length})
              </Text>
            )}
          </div>
        ),
        children: asset.children.length > 0 ? 
          asset.children.map(child => converter(child)) : undefined,
        isLeaf: !isFolder || asset.children.length === 0,
        asset,
        isFolder,
      };
    });

    return (assets: AssetHierarchy[]): OptimizedTreeDataNode[] => {
      return assets.map(asset => converter(asset));
    };
  }, [expandedKeys]);

  // Debounced search function
  const debouncedSearch = useMemo(() => {
    if (!performanceSettings.useDebouncing) {
      return (term: string) => performSearch(term);
    }
    
    return createDebouncedSearch(300)((term: string) => performSearch(term));
  }, [performanceSettings.useDebouncing]);

  // Search functionality
  const performSearch = useCallback((searchTerm: string) => {
    if (!searchTerm.trim()) {
      setFilteredData(hierarchyData);
      return;
    }

    const filterTree = (nodes: AssetHierarchy[]): AssetHierarchy[] => {
      return nodes.reduce((filtered: AssetHierarchy[], node) => {
        const matchesSearch = node.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
                             node.description.toLowerCase().includes(searchTerm.toLowerCase());
        
        const filteredChildren = filterTree(node.children);
        
        if (matchesSearch || filteredChildren.length > 0) {
          filtered.push({
            ...node,
            children: filteredChildren,
          });
        }
        
        return filtered;
      }, []);
    };

    setFilteredData(filterTree(hierarchyData));
  }, [hierarchyData]);

  // Update filtered data when hierarchy changes
  useEffect(() => {
    if (searchValue.trim()) {
      performSearch(searchValue);
    } else {
      setFilteredData(hierarchyData);
    }
  }, [hierarchyData, searchValue, performSearch]);

  // Tree data with search filtering
  const treeData = useMemo(() => {
    const dataToUse = searchValue.trim() ? filteredData : hierarchyData;
    return convertToTreeData(dataToUse);
  }, [filteredData, hierarchyData, searchValue, convertToTreeData]);

  // Flattened tree for virtualization
  const flatTree = useMemo(() => {
    if (!enableVirtualization && !performanceSettings.useVirtualization) {
      return null;
    }
    
    const expandedSet = new Set(expandedKeys);
    return flattenTree(hierarchyData, expandedSet);
  }, [hierarchyData, expandedKeys, enableVirtualization, performanceSettings.useVirtualization]);

  // Event handlers
  const handleSelect: TreeProps['onSelect'] = useCallback((selectedKeys, info) => {
    const key = selectedKeys[0];
    setSelectedKeys(selectedKeys);
    
    if (key) {
      const nodeData = info.node as OptimizedTreeDataNode;
      onAssetSelect?.(nodeData.asset);
    } else {
      onAssetSelect?.(null);
    }
  }, [onAssetSelect]);

  const handleExpand: TreeProps['onExpand'] = useCallback((expandedKeys) => {
    setExpandedKeys(expandedKeys);
  }, []);

  const handleSearch = useCallback((value: string) => {
    setSearchValue(value);
    debouncedSearch(value);
  }, [debouncedSearch]);

  // Auto-expand search results for better UX
  useEffect(() => {
    if (searchValue.trim() && filteredData.length > 0) {
      const getAllKeys = (nodes: AssetHierarchy[]): string[] => {
        return nodes.reduce((keys: string[], node) => {
          keys.push(node.id.toString());
          if (node.children.length > 0) {
            keys.push(...getAllKeys(node.children));
          }
          return keys;
        }, []);
      };
      
      setExpandedKeys(getAllKeys(filteredData));
    }
  }, [searchValue, filteredData]);

  // Render performance metrics in development
  const renderPerformanceInfo = () => {
    if (process.env.NODE_ENV !== 'development') return null;
    
    return (
      <div style={{ 
        fontSize: '10px', 
        color: '#999', 
        padding: '4px 8px',
        borderBottom: '1px solid #f0f0f0'
      }}>
        Tree: {performanceMetrics.treeSize} assets, Depth: {performanceMetrics.maxDepth}
        {performanceSettings.useVirtualization && ' | Virtualized'}
        {performanceSettings.useLazyLoading && ' | Lazy Loading'}
        {performanceSettings.useDebouncing && ' | Debounced'}
      </div>
    );
  };

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {renderPerformanceInfo()}
      
      {enableSearch && performanceSettings.enableSearch && (
        <div style={{ padding: '8px' }}>
          <Search
            placeholder="Search assets..."
            allowClear
            value={searchValue}
            onChange={(e) => handleSearch(e.target.value)}
            prefix={<SearchOutlined />}
            size="small"
          />
        </div>
      )}

      <div style={{ flex: 1, overflow: 'auto' }}>
        <Spin spinning={loading}>
          {treeData.length === 0 && !loading ? (
            <Empty 
              description={searchValue ? "No assets found matching your search" : "No assets found"}
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              style={{ padding: '32px' }}
            />
          ) : (
            <Tree
              ref={treeRef}
              treeData={treeData}
              onSelect={handleSelect}
              onExpand={handleExpand}
              selectedKeys={selectedAssetId ? [selectedAssetId.toString()] : selectedKeys}
              expandedKeys={expandedKeys}
              draggable={allowDragDrop}
              showLine={{ showLeafIcon: false }}
              blockNode
              virtual={enableVirtualization || performanceSettings.useVirtualization}
              height={enableVirtualization ? 400 : undefined}
              style={{ 
                background: 'transparent',
                fontSize: '14px',
              }}
            />
          )}
        </Spin>
      </div>

      {searchValue && (
        <div style={{ 
          fontSize: '12px', 
          color: '#666', 
          padding: '4px 8px',
          borderTop: '1px solid #f0f0f0',
          background: '#fafafa'
        }}>
          {filteredData.length === 0 ? 'No results found' : 
           `${filteredData.length} result${filteredData.length !== 1 ? 's' : ''} found`}
        </div>
      )}
    </div>
  );
};

export default OptimizedAssetTreeView;