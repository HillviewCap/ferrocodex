import React, { useState, useCallback, useMemo } from 'react';
import { Tree, Typography, Dropdown, Menu, Modal, message, Space, Checkbox } from 'antd';
import { 
  FolderOutlined, 
  FolderOpenOutlined, 
  ToolOutlined, 
  PlusOutlined, 
  EditOutlined, 
  DeleteOutlined,
  ScissorOutlined,
  CopyOutlined,
  FolderAddOutlined,
  AppstoreAddOutlined
} from '@ant-design/icons';
import type { TreeProps, TreeDataNode } from 'antd/es/tree';
import { AssetHierarchy, AssetType, AssetInfo } from '../../types/assets';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../../store/auth';
import useBulkOperationsStore from '../../store/bulkOperations';
import { AssetSelectionCheckbox } from '../bulk';

const { Text } = Typography;

export interface EnhancedAssetTreeViewProps {
  assets: AssetHierarchy[];
  selectedAsset?: AssetHierarchy | null;
  onAssetSelect?: (asset: AssetHierarchy | null) => void;
  onAssetCreate?: (parentId: number | null, assetType: AssetType) => void;
  onAssetEdit?: (asset: AssetHierarchy) => void;
  onAssetDelete?: (asset: AssetHierarchy) => void;
  onAssetMove?: (assetId: number, newParentId: number | null) => void;
  loading?: boolean;
  allowDragDrop?: boolean;
  enableBulkSelection?: boolean;
  bulkSelectionContext?: 'tree' | 'search' | 'manual';
}

interface EnhancedAssetTreeDataNode extends TreeDataNode {
  asset: AssetHierarchy;
  isFolder: boolean;
}

export const EnhancedAssetTreeView: React.FC<EnhancedAssetTreeViewProps> = ({
  assets,
  selectedAsset,
  onAssetSelect,
  onAssetCreate,
  onAssetEdit,
  onAssetDelete,
  onAssetMove,
  loading = false,
  allowDragDrop = true,
  enableBulkSelection = false,
  bulkSelectionContext = 'tree',
}) => {
  const { user } = useAuthStore();
  const [expandedKeys, setExpandedKeys] = useState<React.Key[]>([]);
  const [checkedKeys, setCheckedKeys] = useState<React.Key[]>([]);
  const [selectedKeys, setSelectedKeys] = useState<React.Key[]>([]);
  
  // Bulk operations store
  const {
    isSelected: isBulkSelected,
    selectAsset: selectBulkAsset,
    deselectAsset: deselectBulkAsset,
  } = useBulkOperationsStore();

  // Convert hierarchy data to tree data nodes
  const treeData = useMemo(() => {
    const convertToTreeNode = (asset: AssetHierarchy): EnhancedAssetTreeDataNode => {
      const isFolder = asset.asset_type === 'Folder';
      const isSelected = isBulkSelected(asset.id);
      
      return {
        key: asset.id.toString(),
        title: (
          <Space size="small" align="center">
            {enableBulkSelection && (
              <AssetSelectionCheckbox
                assetId={asset.id}
                assetName={asset.name}
                assetType={asset.asset_type}
                context={bulkSelectionContext}
                size="small"
                onClick={(e) => e.stopPropagation()}
              />
            )}
            <span 
              style={{ 
                fontWeight: isSelected ? 'bold' : 'normal',
                color: isSelected ? '#1890ff' : 'inherit'
              }}
            >
              {asset.name}
            </span>
          </Space>
        ),
        icon: isFolder 
          ? (expandedKeys.includes(asset.id.toString()) ? <FolderOpenOutlined /> : <FolderOutlined />)
          : <ToolOutlined />,
        children: asset.children?.map(convertToTreeNode) || [],
        asset,
        isFolder,
        selectable: true,
        disabled: loading,
      };
    };

    return assets.map(convertToTreeNode);
  }, [assets, expandedKeys, enableBulkSelection, bulkSelectionContext, isBulkSelected, loading]);

  // Handle tree selection
  const handleSelect = useCallback((selectedKeys: React.Key[], info: any) => {
    const key = selectedKeys[0];
    if (key) {
      const node = info.node as EnhancedAssetTreeDataNode;
      setSelectedKeys([key]);
      if (onAssetSelect) {
        onAssetSelect(node.asset);
      }
    } else {
      setSelectedKeys([]);
      if (onAssetSelect) {
        onAssetSelect(null);
      }
    }
  }, [onAssetSelect]);

  // Handle tree expansion
  const handleExpand = useCallback((expandedKeys: React.Key[]) => {
    setExpandedKeys(expandedKeys);
  }, []);

  // Handle context menu
  const handleRightClick = useCallback(({ event, node }: any) => {
    if (!user) return;
    
    const treeNode = node as EnhancedAssetTreeDataNode;
    const asset = treeNode.asset;
    
    event.preventDefault();
    // Context menu would be implemented here
  }, [user]);

  // Handle drag and drop
  const handleDrop = useCallback(async (info: any) => {
    if (!allowDragDrop || !onAssetMove) return;

    const dragNode = info.dragNode as EnhancedAssetTreeDataNode;
    const dropNode = info.node as EnhancedAssetTreeDataNode;
    const dragAsset = dragNode.asset;
    const dropAsset = dropNode.asset;

    // Validate drop target
    if (dropAsset.asset_type !== 'Folder') {
      message.error('Can only move assets into folders');
      return;
    }

    // Prevent moving into self or children
    if (dragAsset.id === dropAsset.id) {
      message.error('Cannot move asset into itself');
      return;
    }

    try {
      await onAssetMove(dragAsset.id, dropAsset.id);
      message.success(`Moved ${dragAsset.name} to ${dropAsset.name}`);
    } catch (error) {
      message.error('Failed to move asset');
      console.error('Move error:', error);
    }
  }, [allowDragDrop, onAssetMove]);

  // Get tree props based on selection mode
  const treeProps: TreeProps<EnhancedAssetTreeDataNode> = {
    treeData,
    selectedKeys,
    expandedKeys,
    onSelect: handleSelect,
    onExpand: handleExpand,
    onRightClick: handleRightClick,
    showIcon: true,
    blockNode: true,
    loading,
    ...(allowDragDrop && {
      draggable: {
        icon: false,
        nodeDraggable: (node) => !loading && user !== null,
      },
      onDrop: handleDrop,
    }),
  };

  // Sync selected asset with tree selection
  React.useEffect(() => {
    if (selectedAsset) {
      setSelectedKeys([selectedAsset.id.toString()]);
    } else {
      setSelectedKeys([]);
    }
  }, [selectedAsset]);

  return (
    <div className="enhanced-asset-tree-view">
      <Tree<EnhancedAssetTreeDataNode>
        {...treeProps}
        style={{ 
          background: 'transparent',
          fontSize: '14px',
        }}
        titleRender={(node) => node.title}
      />
    </div>
  );
};

// For backward compatibility, also export with the original name
export const AssetTreeView = EnhancedAssetTreeView;

export default EnhancedAssetTreeView;