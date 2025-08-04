import React, { useState, useCallback, useMemo } from 'react';
import { Tree, Typography, Dropdown, Menu, Modal, message } from 'antd';
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

const { Text } = Typography;

export interface AssetTreeViewProps {
  hierarchyData: AssetHierarchy[];
  onAssetSelect?: (asset: AssetHierarchy | null) => void;
  onAssetCreate?: (parentId: number | null, assetType: AssetType) => void;
  onAssetEdit?: (asset: AssetHierarchy) => void;
  onAssetDelete?: (asset: AssetHierarchy) => void;
  onAssetMove?: (assetId: number, newParentId: number | null) => void;
  selectedAssetId?: number | null;
  loading?: boolean;
  allowDragDrop?: boolean;
}

interface AssetTreeDataNode extends TreeDataNode {
  asset: AssetHierarchy;
  isFolder: boolean;
}

export const AssetTreeView: React.FC<AssetTreeViewProps> = ({
  hierarchyData,
  onAssetSelect,
  onAssetCreate,
  onAssetEdit,
  onAssetDelete,
  onAssetMove,
  selectedAssetId,
  loading = false,
  allowDragDrop = true,
}) => {
  const { token } = useAuthStore();
  const [expandedKeys, setExpandedKeys] = useState<React.Key[]>([]);
  const [selectedKeys, setSelectedKeys] = useState<React.Key[]>([]);
  const [dragOverNodeKey, setDragOverNodeKey] = useState<React.Key | null>(null);

  // Convert AssetHierarchy to Tree Data
  const convertToTreeData = useCallback((assets: AssetHierarchy[]): AssetTreeDataNode[] => {
    return assets.map(asset => {
      const isFolder = asset.asset_type === 'Folder';
      const nodeData: AssetTreeDataNode = {
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
        children: asset.children.length > 0 ? convertToTreeData(asset.children) : undefined,
        isLeaf: !isFolder || asset.children.length === 0,
        asset,
        isFolder,
      };
      return nodeData;
    });
  }, [expandedKeys]);

  const treeData = useMemo(() => convertToTreeData(hierarchyData), [hierarchyData, convertToTreeData]);

  // Context menu items
  const getContextMenuItems = useCallback((asset: AssetHierarchy) => {
    const isFolder = asset.asset_type === 'Folder';
    const items = [
      {
        key: 'create-folder',
        label: 'Create Folder',
        icon: <FolderAddOutlined />,
        disabled: !isFolder,
        onClick: () => onAssetCreate?.(asset.id, 'Folder'),
      },
      {
        key: 'create-device',
        label: 'Create Device',
        icon: <AppstoreAddOutlined />,
        disabled: !isFolder,
        onClick: () => onAssetCreate?.(asset.id, 'Device'),
      },
      { type: 'divider' as const },
      {
        key: 'edit',
        label: 'Edit',
        icon: <EditOutlined />,
        onClick: () => onAssetEdit?.(asset),
      },
      {
        key: 'delete',
        label: 'Delete',
        icon: <DeleteOutlined />,
        danger: true,
        onClick: () => onAssetDelete?.(asset),
      },
    ];

    return items;
  }, [onAssetCreate, onAssetEdit, onAssetDelete]);

  // Tree event handlers
  const handleSelect: TreeProps['onSelect'] = (selectedKeys, info) => {
    const key = selectedKeys[0];
    setSelectedKeys(selectedKeys);
    
    if (key) {
      const nodeData = info.node as AssetTreeDataNode;
      onAssetSelect?.(nodeData.asset);
    } else {
      onAssetSelect?.(null);
    }
  };

  const handleExpand: TreeProps['onExpand'] = (expandedKeys) => {
    setExpandedKeys(expandedKeys);
  };

  const handleRightClick: TreeProps['onRightClick'] = ({ event, node }) => {
    event.preventDefault();
    const nodeData = node as AssetTreeDataNode;
    
    const contextMenu = (
      <Menu items={getContextMenuItems(nodeData.asset)} />
    );

    // Note: This is a simplified context menu implementation
    // In a production app, you might want to use a proper context menu library
    console.log('Right click on asset:', nodeData.asset.name);
  };

  // Drag and drop handlers
  const handleDrop: TreeProps['onDrop'] = async (info) => {
    if (!allowDragDrop) return;

    const dropKey = info.node.key;
    const dragKey = info.dragNode.key;
    const dropToGap = info.dropToGap;

    const dragNodeData = info.dragNode as AssetTreeDataNode;
    const dropNodeData = info.node as AssetTreeDataNode;

    // Can only drop onto folders or root level
    if (!dropToGap && !dropNodeData.isFolder) {
      message.error('Can only drop assets into folders');
      return;
    }

    const newParentId = dropToGap ? dropNodeData.asset.parent_id : dropNodeData.asset.id;
    
    try {
      // Validate the move first
      const isValid = await invoke<boolean>('validate_asset_move', {
        token,
        assetId: parseInt(dragKey.toString()),
        newParentId,
      });

      if (!isValid) {
        message.error('Invalid move operation');
        return;
      }

      // Execute the move
      await invoke('move_asset', {
        token,
        assetId: parseInt(dragKey.toString()),
        newParentId,
        newSortOrder: null, // Let backend determine sort order
      });

      onAssetMove?.(parseInt(dragKey.toString()), newParentId);
      message.success('Asset moved successfully');
    } catch (error) {
      console.error('Failed to move asset:', error);
      message.error('Failed to move asset');
    }
  };

  const handleDragEnter: TreeProps['onDragEnter'] = (info) => {
    setDragOverNodeKey(info.node.key);
  };

  const handleDragLeave: TreeProps['onDragLeave'] = () => {
    setDragOverNodeKey(null);
  };

  // Root level context menu (for creating root assets)
  const rootContextMenuItems = [
    {
      key: 'create-root-folder',
      label: 'Create Root Folder',
      icon: <FolderAddOutlined />,
      onClick: () => onAssetCreate?.(null, 'Folder'),
    },
    {
      key: 'create-root-device',
      label: 'Create Root Device',
      icon: <AppstoreAddOutlined />,
      onClick: () => onAssetCreate?.(null, 'Device'),
    },
  ];

  return (
    <div style={{ height: '100%', overflow: 'auto' }}>
      <Dropdown
        menu={{ items: rootContextMenuItems }}
        trigger={['contextMenu']}
      >
        <div style={{ minHeight: '100%', padding: '8px' }}>
          <Tree
            treeData={treeData}
            onSelect={handleSelect}
            onExpand={handleExpand}
            onRightClick={handleRightClick}
            onDrop={allowDragDrop ? handleDrop : undefined}
            onDragEnter={allowDragDrop ? handleDragEnter : undefined}
            onDragLeave={allowDragDrop ? handleDragLeave : undefined}
            selectedKeys={selectedAssetId ? [selectedAssetId.toString()] : selectedKeys}
            expandedKeys={expandedKeys}
            draggable={allowDragDrop}
            loading={loading}
            showLine={{ showLeafIcon: false }}
            blockNode
            style={{ 
              background: 'transparent',
              fontSize: '14px',
            }}
          />
          {treeData.length === 0 && !loading && (
            <div style={{ 
              textAlign: 'center', 
              padding: '32px', 
              color: '#999',
              fontSize: '14px' 
            }}>
              <FolderOutlined style={{ fontSize: '32px', marginBottom: '16px', display: 'block' }} />
              No assets found. Right-click to create your first asset.
            </div>
          )}
        </div>
      </Dropdown>
    </div>
  );
};

export default AssetTreeView;