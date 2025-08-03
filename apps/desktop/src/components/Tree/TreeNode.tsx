import React, { memo, useCallback } from 'react';
import { Typography, Tooltip, Dropdown, Menu } from 'antd';
import { 
  FolderOutlined, 
  FolderOpenOutlined, 
  ToolOutlined,
  RightOutlined,
  DownOutlined,
  EditOutlined,
  DeleteOutlined,
  PlusOutlined,
  CopyOutlined,
  ScissorOutlined
} from '@ant-design/icons';
import { AssetHierarchy, AssetType } from '../../types/assets';

const { Text } = Typography;

export interface TreeNodeProps {
  asset: AssetHierarchy;
  level: number;
  isExpanded: boolean;
  hasChildren: boolean;
  isSelected: boolean;
  style?: React.CSSProperties;
  onSelect: (asset: AssetHierarchy) => void;
  onToggleExpand: (assetId: number) => void;
  onAssetCreate?: (parentId: number | null, assetType: AssetType) => void;
  onAssetEdit?: (asset: AssetHierarchy) => void;
  onAssetDelete?: (asset: AssetHierarchy) => void;
  onAssetCopy?: (asset: AssetHierarchy) => void;
  onAssetCut?: (asset: AssetHierarchy) => void;
  enableContextMenu?: boolean;
  enableDragDrop?: boolean;
}

const TreeNode: React.FC<TreeNodeProps> = memo(({
  asset,
  level,
  isExpanded,
  hasChildren,
  isSelected,
  style,
  onSelect,
  onToggleExpand,
  onAssetCreate,
  onAssetEdit,
  onAssetDelete,
  onAssetCopy,
  onAssetCut,
  enableContextMenu = true,
  enableDragDrop = false,
}) => {
  const isFolder = asset.asset_type === 'Folder';
  const indent = level * 20;

  const handleToggleExpand = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    if (hasChildren) {
      onToggleExpand(asset.id);
    }
  }, [hasChildren, onToggleExpand, asset.id]);

  const handleSelect = useCallback(() => {
    onSelect(asset);
  }, [onSelect, asset]);

  // Context menu items
  const contextMenuItems = [
    ...(isFolder ? [
      {
        key: 'create-folder',
        label: 'Create Folder',
        icon: <PlusOutlined />,
        onClick: () => onAssetCreate?.(asset.id, 'Folder'),
      },
      {
        key: 'create-device',
        label: 'Create Device',
        icon: <PlusOutlined />,
        onClick: () => onAssetCreate?.(asset.id, 'Device'),
      },
      { type: 'divider' as const },
    ] : []),
    {
      key: 'edit',
      label: 'Edit',
      icon: <EditOutlined />,
      onClick: () => onAssetEdit?.(asset),
    },
    {
      key: 'copy',
      label: 'Copy',
      icon: <CopyOutlined />,
      onClick: () => onAssetCopy?.(asset),
    },
    {
      key: 'cut',
      label: 'Cut',
      icon: <ScissorOutlined />,
      onClick: () => onAssetCut?.(asset),
    },
    { type: 'divider' as const },
    {
      key: 'delete',
      label: 'Delete',
      icon: <DeleteOutlined />,
      danger: true,
      onClick: () => onAssetDelete?.(asset),
    },
  ];

  const nodeContent = (
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
        minHeight: 32,
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
      draggable={enableDragDrop}
      onDragStart={(e) => {
        if (enableDragDrop) {
          e.dataTransfer.setData('text/plain', asset.id.toString());
          e.dataTransfer.effectAllowed = 'move';
        }
      }}
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

      {/* Modified Indicator */}
      {asset.updated_at !== asset.created_at && (
        <div
          style={{
            width: 6,
            height: 6,
            borderRadius: '50%',
            backgroundColor: '#faad14',
            marginLeft: 8,
          }}
          title="Modified"
        />
      )}
    </div>
  );

  // Wrap with context menu if enabled
  if (enableContextMenu) {
    return (
      <Dropdown
        menu={{ items: contextMenuItems }}
        trigger={['contextMenu']}
        disabled={!enableContextMenu}
      >
        {nodeContent}
      </Dropdown>
    );
  }

  return nodeContent;
});

TreeNode.displayName = 'TreeNode';

export default TreeNode;