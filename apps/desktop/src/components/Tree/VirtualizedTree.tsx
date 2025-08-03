import React, { useState, useCallback, useMemo, useRef, useEffect } from 'react';
import { FixedSizeList as List } from 'react-window';
import { Typography, Tooltip, Input, Empty } from 'antd';
import { 
  FolderOutlined, 
  FolderOpenOutlined, 
  ToolOutlined,
  SearchOutlined,
  RightOutlined,
  DownOutlined
} from '@ant-design/icons';
import { AssetHierarchy, AssetType } from '../../types/assets';
import { TreeContainer } from './TreeContainer';
import { useTreeVirtualization } from '../../hooks/useTreeVirtualization';

const { Text } = Typography;
const { Search } = Input;

export interface VirtualizedTreeProps {
  hierarchyData: AssetHierarchy[];
  onAssetSelect?: (asset: AssetHierarchy | null) => void;
  onAssetCreate?: (parentId: number | null, assetType: AssetType) => void;
  onAssetMove?: (assetId: number, newParentId: number | null) => void;
  selectedAssetId?: number | null;
  loading?: boolean;
  height?: number;
  itemHeight?: number;
  enableSearch?: boolean;
  maxItems?: number;
}

interface FlatTreeItem {
  id: number;
  asset: AssetHierarchy;
  level: number;
  isExpanded: boolean;
  hasChildren: boolean;
  isVisible: boolean;
  parentIds: number[];
}

const VirtualizedTree: React.FC<VirtualizedTreeProps> = ({
  hierarchyData,
  onAssetSelect,
  onAssetCreate,
  onAssetMove,
  selectedAssetId,
  loading = false,
  height = 400,
  itemHeight = 32,
  enableSearch = true,
  maxItems = 10000,
}) => {
  const [expandedKeys, setExpandedKeys] = useState<Set<number>>(new Set());
  const [searchValue, setSearchValue] = useState('');
  const [filteredItems, setFilteredItems] = useState<FlatTreeItem[]>([]);
  
  const listRef = useRef<List>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  
  const {
    flattenedItems,
    visibleItems,
    expandNode,
    collapseNode,
    searchItems,
    performanceMetrics
  } = useTreeVirtualization({
    data: hierarchyData,
    expandedKeys,
    searchValue,
    maxItems
  });

  // Use the optimized hook's flattening instead of duplicating logic
  // The useTreeVirtualization hook already provides flattenedItems

  // Search filtering is now handled by useTreeVirtualization hook

  // Use the hook's processed items directly since it already handles search
  const processedItems = useMemo(() => {
    // Auto-expand search results for better UX
    if (searchValue.trim() && visibleItems.length > 0 && visibleItems.length < 100) {
      const newExpandedKeys = new Set(expandedKeys);
      visibleItems.forEach(item => {
        if (item.hasChildren && item.searchMatch) {
          newExpandedKeys.add(item.id);
        }
      });
      if (newExpandedKeys.size !== expandedKeys.size) {
        setExpandedKeys(newExpandedKeys);
      }
    }
    
    return visibleItems.slice(0, maxItems); // Apply final limit for performance
  }, [visibleItems, searchValue, expandedKeys, maxItems]);

  // Update filtered items
  useEffect(() => {
    setFilteredItems(processedItems);
  }, [processedItems]);

  // Tree item renderer for virtualization
  const TreeItemRenderer = useCallback(({ index, style }: { index: number; style: any }) => {
    const item = filteredItems[index];
    if (!item) return null;

    const { asset, level, isExpanded, hasChildren } = item;
    const isSelected = selectedAssetId === asset.id;
    const isFolder = asset.asset_type === 'Folder';
    const indent = level * 20;

    const handleToggleExpand = (e: React.MouseEvent) => {
      e.stopPropagation();
      if (hasChildren) {
        const newExpandedKeys = new Set(expandedKeys);
        if (isExpanded) {
          newExpandedKeys.delete(asset.id);
        } else {
          newExpandedKeys.add(asset.id);
        }
        setExpandedKeys(newExpandedKeys);
      }
    };

    const handleSelect = () => {
      onAssetSelect?.(asset);
    };

    return (
      <div
        style={{
          ...style,
          display: 'flex',
          alignItems: 'center',
          paddingLeft: 8 + indent,
          paddingRight: 8,
          backgroundColor: isSelected ? '#e6f7ff' : 'transparent',
          borderLeft: isSelected ? '3px solid #1890ff' : '3px solid transparent',
          cursor: 'pointer',
          transition: 'background-color 0.2s ease',
        }}
        onClick={handleSelect}
        onMouseEnter={(e) => {
          if (!isSelected) {
            e.currentTarget.style.backgroundColor = '#f5f5f5';
          }
        }}
        onMouseLeave={(e) => {
          if (!isSelected) {
            e.currentTarget.style.backgroundColor = 'transparent';
          }
        }}
        role="treeitem"
        aria-expanded={hasChildren ? isExpanded : undefined}
        aria-selected={isSelected}
        aria-level={level + 1}
        tabIndex={-1}
      >
        {/* Expand/Collapse Button */}
        <div
          style={{
            width: 16,
            height: 16,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginRight: 4,
            cursor: hasChildren ? 'pointer' : 'default',
          }}
          onClick={hasChildren ? handleToggleExpand : undefined}
        >
          {hasChildren ? (
            isExpanded ? (
              <DownOutlined style={{ fontSize: 10, color: '#666' }} />
            ) : (
              <RightOutlined style={{ fontSize: 10, color: '#666' }} />
            )
          ) : (
            <span style={{ width: 10 }} />
          )}
        </div>

        {/* Asset Icon */}
        <div style={{ marginRight: 8, display: 'flex', alignItems: 'center' }}>
          {isFolder ? (
            isExpanded ? (
              <FolderOpenOutlined style={{ color: '#1890ff', fontSize: 16 }} />
            ) : (
              <FolderOutlined style={{ color: '#1890ff', fontSize: 16 }} />
            )
          ) : (
            <ToolOutlined style={{ color: '#52c41a', fontSize: 16 }} />
          )}
        </div>

        {/* Asset Name */}
        <div style={{ flex: 1, overflow: 'hidden' }}>
          <Tooltip title={asset.description || asset.name} placement="topLeft">
            <Text
              strong={isFolder}
              style={{
                color: isSelected ? '#1890ff' : isFolder ? '#262626' : '#595959',
                fontSize: 14,
                whiteSpace: 'nowrap',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
              }}
            >
              {asset.name}
            </Text>
          </Tooltip>
        </div>

        {/* Item Count for Folders */}
        {isFolder && asset.children.length > 0 && (
          <Text type="secondary" style={{ fontSize: 12, marginLeft: 8 }}>
            ({asset.children.length})
          </Text>
        )}
      </div>
    );
  }, [filteredItems, selectedAssetId, expandedKeys, onAssetSelect]);

  // Handle search
  const handleSearch = useCallback((value: string) => {
    setSearchValue(value);
  }, []);

  // Keyboard navigation
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (!filteredItems.length) return;

    const currentIndex = selectedAssetId 
      ? filteredItems.findIndex(item => item.id === selectedAssetId)
      : -1;

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        const nextIndex = Math.min(currentIndex + 1, filteredItems.length - 1);
        if (nextIndex >= 0) {
          onAssetSelect?.(filteredItems[nextIndex].asset);
          listRef.current?.scrollToItem(nextIndex, 'smart');
        }
        break;
        
      case 'ArrowUp':
        e.preventDefault();
        const prevIndex = Math.max(currentIndex - 1, 0);
        if (prevIndex >= 0 && currentIndex > 0) {
          onAssetSelect?.(filteredItems[prevIndex].asset);
          listRef.current?.scrollToItem(prevIndex, 'smart');
        }
        break;
        
      case 'ArrowRight':
        e.preventDefault();
        if (currentIndex >= 0) {
          const item = filteredItems[currentIndex];
          if (item.hasChildren && !item.isExpanded) {
            const newExpandedKeys = new Set(expandedKeys);
            newExpandedKeys.add(item.id);
            setExpandedKeys(newExpandedKeys);
          }
        }
        break;
        
      case 'ArrowLeft':
        e.preventDefault();
        if (currentIndex >= 0) {
          const item = filteredItems[currentIndex];
          if (item.hasChildren && item.isExpanded) {
            const newExpandedKeys = new Set(expandedKeys);
            newExpandedKeys.delete(item.id);
            setExpandedKeys(newExpandedKeys);
          }
        }
        break;
        
      case 'Enter':
      case ' ':
        e.preventDefault();
        if (currentIndex >= 0) {
          const item = filteredItems[currentIndex];
          if (item.hasChildren) {
            const newExpandedKeys = new Set(expandedKeys);
            if (item.isExpanded) {
              newExpandedKeys.delete(item.id);
            } else {
              newExpandedKeys.add(item.id);
            }
            setExpandedKeys(newExpandedKeys);
          }
        }
        break;
    }
  }, [filteredItems, selectedAssetId, expandedKeys, onAssetSelect]);

  if (loading) {
    return (
      <TreeContainer height={height}>
        <div style={{ 
          display: 'flex', 
          alignItems: 'center', 
          justifyContent: 'center', 
          height: '100%' 
        }}>
          Loading...
        </div>
      </TreeContainer>
    );
  }

  return (
    <TreeContainer height={height} onKeyDown={handleKeyDown}>
      {enableSearch && (
        <div style={{ padding: '8px 12px', borderBottom: '1px solid #f0f0f0' }}>
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
      
      <div 
        ref={containerRef}
        style={{ 
          flex: 1,
          overflow: 'hidden'
        }}
      >
        {filteredItems.length === 0 ? (
          <Empty 
            description={searchValue ? "No assets found matching your search" : "No assets found"}
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            style={{ padding: '32px' }}
          />
        ) : (
          <List
            ref={listRef}
            height={height - (enableSearch ? 48 : 0)}
            itemCount={filteredItems.length}
            itemSize={itemHeight}
            width="100%"
            itemData={filteredItems}
          >
            {TreeItemRenderer}
          </List>
        )}
      </div>

      {/* Performance Info in Development */}
      {process.env.NODE_ENV === 'development' && (
        <div style={{ 
          fontSize: '10px', 
          color: '#999', 
          padding: '4px 8px',
          borderTop: '1px solid #f0f0f0',
          background: '#fafafa'
        }}>
          Items: {filteredItems.length}/{processedItems.length} | 
          Expanded: {expandedKeys.size} | 
          Search: {searchValue ? 'Active' : 'Inactive'}
        </div>
      )}
    </TreeContainer>
  );
};

export default VirtualizedTree;