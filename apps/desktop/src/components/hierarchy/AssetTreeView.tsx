import React, { useState, useCallback, useMemo, useRef, useEffect } from 'react';
import { Tree, Typography, Modal, message, TreeSelect, Spin } from 'antd';
import { 
  FolderOutlined, 
  FolderOpenOutlined, 
  ToolOutlined, 
  EditOutlined, 
  DeleteOutlined,
  ScissorOutlined,
  FolderAddOutlined,
  AppstoreAddOutlined
} from '@ant-design/icons';
import type { TreeProps, DataNode } from 'antd/es/tree';
import { AssetHierarchy, AssetType } from '../../types/assets';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../../store/auth';
import { TreeSearchFilter } from './TreeSearchFilter';

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
  showSearch?: boolean;
  searchPlaceholder?: string;
}

interface AssetTreeDataNode extends DataNode {
  key: React.Key;
  asset: AssetHierarchy;
  isFolder: boolean;
  children?: AssetTreeDataNode[];
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
  showSearch = true,
  searchPlaceholder = "Search assets...",
}) => {
  const { token } = useAuthStore();
  const [expandedKeys, setExpandedKeys] = useState<React.Key[]>([]);
  const [selectedKeys, setSelectedKeys] = useState<React.Key[]>([]);
  const [isDragging, setIsDragging] = useState(false);
  const [cutAsset, setCutAsset] = useState<AssetHierarchy | null>(null);
  const [moveModalVisible, setMoveModalVisible] = useState(false);
  const [assetToMove, setAssetToMove] = useState<AssetHierarchy | null>(null);
  const [selectedDestination, setSelectedDestination] = useState<number | null>(null);
  const [contextMenu, setContextMenu] = useState<{
    visible: boolean;
    x: number;
    y: number;
    asset: AssetHierarchy | null;
  }>({ visible: false, x: 0, y: 0, asset: null });
  const contextMenuRef = useRef<HTMLDivElement>(null);
  const [filteredNodeIds, setFilteredNodeIds] = useState<Set<number>>(new Set());
  const [searchQuery, setSearchQuery] = useState('');
  const [assetTypeFilter, setAssetTypeFilter] = useState<AssetType | 'All'>('All');

  // Convert AssetHierarchy to Tree Data with search filtering
  const convertToTreeData = useCallback((assets: AssetHierarchy[]): AssetTreeDataNode[] => {
    // If no search or filter is active, show all assets
    const isFilteringActive = searchQuery.trim() || assetTypeFilter !== 'All';
    
    const processAsset = (asset: AssetHierarchy): AssetTreeDataNode | null => {
      let shouldShow = true;
      
      if (isFilteringActive) {
        // If search is active, check if this asset matches
        if (searchQuery.trim() && filteredNodeIds.size > 0) {
          shouldShow = filteredNodeIds.has(asset.id);
        }
        
        // If type filter is active, check type
        if (assetTypeFilter !== 'All') {
          shouldShow = shouldShow && asset.asset_type === assetTypeFilter;
        }
      }
      
      // Process children regardless of parent visibility
      const processedChildren = asset.children
        .map(processAsset)
        .filter((child): child is AssetTreeDataNode => child !== null);
      
      // If this asset doesn't match but has matching children, show it
      if (!shouldShow && processedChildren.length > 0) {
        shouldShow = true;
      }
      
      if (!shouldShow && !processedChildren.length) {
        return null;
      }

      const isFolder = asset.asset_type === 'Folder';
      const isHighlighted = searchQuery && filteredNodeIds.has(asset.id);
      
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
            <Text 
              strong={isFolder}
              style={{
                backgroundColor: isHighlighted ? '#fff2e8' : 'transparent',
                padding: isHighlighted ? '2px 4px' : '0',
                borderRadius: '4px',
              }}
            >
              {asset.name}
            </Text>
            {isFolder && processedChildren.length > 0 && (
              <Text type="secondary" style={{ fontSize: '12px' }}>
                ({processedChildren.length})
              </Text>
            )}
          </div>
        ),
        children: processedChildren.length > 0 ? processedChildren : undefined,
        isLeaf: !isFolder || processedChildren.length === 0,
        asset,
        isFolder,
      };
    };

    return assets
      .map(processAsset)
      .filter((node): node is AssetTreeDataNode => node !== null);
  }, [expandedKeys, filteredNodeIds, searchQuery, assetTypeFilter]);

  const treeData = useMemo(() => convertToTreeData(hierarchyData), [hierarchyData, convertToTreeData]);

  // Context menu items
  const getContextMenuItems = useCallback((asset: AssetHierarchy | null) => {
    // If no asset (root level), show root creation options
    if (!asset) {
      return [
        {
          key: 'create-root-folder',
          label: 'Create Root Folder',
          icon: <FolderAddOutlined />,
          disabled: false,
        },
        {
          key: 'create-root-device',
          label: 'Create Root Device',
          icon: <AppstoreAddOutlined />,
          disabled: false,
        },
      ];
    }
    
    const isFolder = asset.asset_type === 'Folder';
    const items = [
      {
        key: 'create-folder',
        label: 'Create Folder',
        icon: <FolderAddOutlined />,
        disabled: !isFolder,
      },
      {
        key: 'create-device',
        label: 'Create Device',
        icon: <AppstoreAddOutlined />,
        disabled: !isFolder,
      },
      { type: 'divider' as const, key: 'divider-1' },
      {
        key: 'move',
        label: 'Move to...',
        icon: <ScissorOutlined />,
        disabled: false,
      },
      {
        key: 'edit',
        label: 'Edit',
        icon: <EditOutlined />,
        disabled: false,
      },
      {
        key: 'delete',
        label: 'Delete',
        icon: <DeleteOutlined />,
        danger: true,
        disabled: false,
      },
    ];

    return items;
  }, []);

  // Tree event handlers
  const handleSelect: TreeProps['onSelect'] = (selectedKeys, info) => {
    const key = selectedKeys[0];
    setSelectedKeys(selectedKeys);
    
    if (key) {
      const nodeData = info.node as unknown as AssetTreeDataNode;
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
    event.stopPropagation();
    const nodeData = node as unknown as AssetTreeDataNode;
    
    setContextMenu({
      visible: true,
      x: event.pageX,
      y: event.pageY,
      asset: nodeData.asset,
    });
  };

  // Handle right click on empty area for root level context menu
  const handleEmptyAreaRightClick = (event: React.MouseEvent) => {
    // Only trigger if clicking on the container div, not on tree nodes
    if ((event.target as HTMLElement).closest('.ant-tree-treenode')) {
      return;
    }
    
    event.preventDefault();
    event.stopPropagation();
    
    setContextMenu({
      visible: true,
      x: event.pageX,
      y: event.pageY,
      asset: null, // null indicates root level
    });
  };

  // Close context menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (contextMenuRef.current && !contextMenuRef.current.contains(event.target as Node)) {
        setContextMenu({ visible: false, x: 0, y: 0, asset: null });
      }
    };

    if (contextMenu.visible) {
      document.addEventListener('mousedown', handleClickOutside);
      return () => document.removeEventListener('mousedown', handleClickOutside);
    }
  }, [contextMenu.visible]);

  // Handle context menu item clicks
  const handleContextMenuClick = (key: string, asset: AssetHierarchy | null) => {
    setContextMenu({ visible: false, x: 0, y: 0, asset: null });
    
    switch (key) {
      case 'create-folder':
      case 'create-root-folder':
        onAssetCreate?.(asset?.id || null, 'Folder');
        break;
      case 'create-device':
      case 'create-root-device':
        onAssetCreate?.(asset?.id || null, 'Device');
        break;
      case 'move':
        if (asset) {
          setAssetToMove(asset);
          setMoveModalVisible(true);
        }
        break;
      case 'edit':
        if (asset) onAssetEdit?.(asset);
        break;
      case 'delete':
        if (asset) onAssetDelete?.(asset);
        break;
      default:
        break;
    }
  };

  // Keyboard event handler
  const handleKeyDown = useCallback((event: KeyboardEvent) => {
    if (selectedKeys.length === 0) return;
    
    const selectedKey = selectedKeys[0];
    const selectedNode = findNodeByKey(treeData, selectedKey);
    if (!selectedNode) return;
    
    switch (event.key) {
      case 'Delete':
        event.preventDefault();
        onAssetDelete?.(selectedNode.asset);
        break;
      case 'F2':
        event.preventDefault();
        onAssetEdit?.(selectedNode.asset);
        break;
      case 'Enter':
        event.preventDefault();
        if (selectedNode.isFolder) {
          const isExpanded = expandedKeys.includes(selectedKey);
          if (isExpanded) {
            setExpandedKeys(expandedKeys.filter(key => key !== selectedKey));
          } else {
            setExpandedKeys([...expandedKeys, selectedKey]);
          }
        }
        break;
      default:
        break;
    }
  }, [selectedKeys, treeData, onAssetDelete, onAssetEdit, expandedKeys]);

  // Helper function to find node by key
  const findNodeByKey = (nodes: AssetTreeDataNode[], key: React.Key): AssetTreeDataNode | null => {
    for (const node of nodes) {
      if (node.key === key) return node;
      if (node.children) {
        const found = findNodeByKey(node.children as AssetTreeDataNode[], key);
        if (found) return found;
      }
    }
    return null;
  };

  // Add keyboard event listener
  useEffect(() => {
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);

  // Handle search changes from TreeSearchFilter
  const handleSearchChange = useCallback((query: string, matchedNodes: Set<number>, expandKeys: React.Key[]) => {
    setSearchQuery(query);
    setFilteredNodeIds(matchedNodes);
    
    // Auto-expand nodes to show search results
    if (query && expandKeys.length > 0) {
      setExpandedKeys(expandKeys);
    }
  }, []);

  // Handle filter changes from TreeSearchFilter
  const handleFilterChange = useCallback((assetType: AssetType | 'All') => {
    setAssetTypeFilter(assetType);
  }, []);

  // Handle move operation from modal
  const handleMoveAsset = async () => {
    if (!assetToMove || selectedDestination === null || !token) {
      message.error('Please select a destination');
      return;
    }

    // Don't allow moving to itself
    if (assetToMove.id === selectedDestination) {
      message.error('Cannot move asset to itself');
      return;
    }

    try {
      // Validate the move first
      const isValid = await invoke<boolean>('validate_asset_move', {
        token,
        assetId: assetToMove.id,
        newParentId: selectedDestination === 0 ? null : selectedDestination,
      });

      if (!isValid) {
        message.error('Cannot move asset to this location');
        return;
      }

      // Execute the move
      await invoke('move_asset', {
        token,
        assetId: assetToMove.id,
        newParentId: selectedDestination === 0 ? null : selectedDestination,
        newSortOrder: null,
      });

      onAssetMove?.(assetToMove.id, selectedDestination === 0 ? null : selectedDestination);
      message.success('Asset moved successfully');
      
      // Close modal and reset state
      setMoveModalVisible(false);
      setAssetToMove(null);
      setSelectedDestination(null);
    } catch (error) {
      console.error('Failed to move asset:', error);
      message.error(`Failed to move asset: ${error}`);
    }
  };

  // Convert hierarchy to TreeSelect data
  const convertToTreeSelectData = useCallback((assets: AssetHierarchy[], excludeId?: number): any[] => {
    return assets
      .filter(asset => asset.id !== excludeId)
      .map(asset => ({
        value: asset.id,
        title: asset.name,
        disabled: asset.asset_type !== 'Folder',
        children: asset.children.length > 0 
          ? convertToTreeSelectData(asset.children, excludeId)
          : undefined,
      }));
  }, []);

  // Drag and drop handlers
  const handleDrop: TreeProps['onDrop'] = async (info) => {
    console.log('Drop event:', {
      dragKey: info.dragNode.key,
      dropKey: info.node.key,  
      dropToGap: info.dropToGap,
      dropPosition: info.dropPosition,
      allowDragDrop,
      hasToken: !!token,
      info: info
    });

    if (!token) {
      console.log('Drop rejected: no token');
      return;
    }

    const dropKey = info.node.key;
    const dragKey = info.dragNode.key;
    const dropToGap = info.dropToGap;

    const dragNodeData = info.dragNode as unknown as AssetTreeDataNode;
    const dropNodeData = info.node as unknown as AssetTreeDataNode;

    // Prevent dropping onto self
    if (dragKey === dropKey) {
      console.log('Drop rejected: dropping onto self');
      return;
    }

    // Determine the new parent ID based on drop position
    let newParentId: number | null;
    
    if (!dropToGap) {
      // Dropping onto a node (must be a folder)
      if (!dropNodeData.isFolder) {
        message.error('Can only drop assets into folders');
        console.log('Drop rejected: target is not a folder');
        return;
      }
      newParentId = dropNodeData.asset.id;
    } else {
      // Dropping between nodes (at the same level as the drop target)
      newParentId = dropNodeData.asset.parent_id;
    }
    
    console.log('Attempting move:', {
      assetId: parseInt(dragKey.toString()),
      newParentId: newParentId
    });
    
    try {
      // First validate the move
      const isValid = await invoke<boolean>('validate_asset_move', {
        token,
        assetId: parseInt(dragKey.toString()),
        newParentId: newParentId,
      });

      if (!isValid) {
        message.error('Cannot move asset to this location');
        console.log('Move validation failed');
        return;
      }

      // Execute the move
      await invoke('move_asset', {
        token,
        assetId: parseInt(dragKey.toString()),
        newParentId: newParentId,
        newSortOrder: null, // Let backend determine sort order
      });

      onAssetMove?.(parseInt(dragKey.toString()), newParentId);
      message.success('Asset moved successfully');
      console.log('Move successful');
    } catch (error) {
      console.error('Failed to move asset:', error);
      message.error(`Failed to move asset: ${error}`);
    }
  };

  const handleDragEnter: TreeProps['onDragEnter'] = (info) => {
    console.log('Drag enter:', {
      nodeKey: info.node.key,
      nodePos: info.node.pos,
      event: info.event
    });
    // setDragOverNodeKey(info.node.key);
  };

  const handleDragLeave: TreeProps['onDragLeave'] = (info) => {
    console.log('Drag leave:', {
      nodeKey: info.node.key,
      event: info.event
    });
    // setDragOverNodeKey(null);
  };
  
  const handleDragStart: TreeProps['onDragStart'] = (info) => {
    console.log('Drag start:', {
      nodeKey: info.node.key,
      nodeData: info.node,
      event: info.event
    });
  };
  
  const handleDragEnd: TreeProps['onDragEnd'] = (info) => {
    console.log('Drag end:', {
      nodeKey: info.node.key,
      event: info.event
    });
  };
  
  const handleDragOver: TreeProps['onDragOver'] = (info) => {
    console.log('Drag over:', {
      nodeKey: info.node.key,
      event: info.event
    });
  };

  // Add global style to ensure drag and drop works and prevent default drag behavior
  React.useEffect(() => {
    const style = document.createElement('style');
    style.innerHTML = `
      .ant-tree-draggable-icon {
        display: inline-block !important;
      }
      .ant-tree-treenode-draggable {
        cursor: move !important;
        -webkit-user-drag: element !important;
        user-drag: element !important;
      }
      .ant-tree-treenode-draggable .ant-tree-node-content-wrapper {
        cursor: move !important;
      }
      .drag-over-gap-top > .ant-tree-node-content-wrapper {
        border-top: 2px solid #1890ff !important;
      }
      .drag-over-gap-bottom > .ant-tree-node-content-wrapper {
        border-bottom: 2px solid #1890ff !important;
      }
      .drag-over > .ant-tree-node-content-wrapper {
        background: #bae7ff !important;
      }
      /* Prevent text selection during drag */
      .ant-tree-dragging {
        -webkit-user-select: none !important;
        -moz-user-select: none !important;
        -ms-user-select: none !important;
        user-select: none !important;
      }
    `;
    document.head.appendChild(style);
    
    // Prevent default drag behavior on the document
    const preventDefaultDrag = (e: DragEvent) => {
      if (e.target && (e.target as HTMLElement).closest('.ant-tree')) {
        e.preventDefault();
        e.stopPropagation();
      }
    };
    
    const allowDrop = (e: DragEvent) => {
      if (e.target && (e.target as HTMLElement).closest('.ant-tree')) {
        e.preventDefault();
        return false;
      }
    };
    
    // Add event listeners to handle drag events properly
    document.addEventListener('dragover', allowDrop);
    document.addEventListener('drop', allowDrop);
    
    return () => {
      document.head.removeChild(style);
      document.removeEventListener('dragover', allowDrop);
      document.removeEventListener('drop', allowDrop);
    };
  }, []);

  return (
    <div style={{ height: '100%', overflow: 'auto' }}>
      {showSearch && (
        <div style={{ padding: '8px', borderBottom: '1px solid #f0f0f0' }}>
          <TreeSearchFilter
            hierarchyData={hierarchyData}
            onSearchChange={handleSearchChange}
            onFilterChange={handleFilterChange}
            placeholder={searchPlaceholder}
            showTypeFilter={true}
          />
        </div>
      )}
      
      <div 
        style={{ 
          minHeight: '100%', 
          padding: '8px', 
          position: 'relative'
        }} 
        tabIndex={0}
        onContextMenu={handleEmptyAreaRightClick}
      >
        <Spin spinning={loading || false}>
          <Tree
            treeData={treeData}
          onSelect={handleSelect}
          onExpand={handleExpand}
          onRightClick={handleRightClick}
          selectedKeys={selectedAssetId ? [selectedAssetId.toString()] : selectedKeys}
          expandedKeys={expandedKeys}
          draggable={{
            icon: false,
            nodeDraggable: () => true
          }}
          allowDrop={({ dropNode, dragNode, dropPosition }) => {
            console.log('allowDrop called:', { 
              dragKey: dragNode?.key, 
              dropKey: dropNode?.key,
              dropPosition 
            });
            
            // Prevent dropping on itself
            if (dragNode?.key === dropNode?.key) {
              return false;
            }
            
            // Check if dropping into a folder or between nodes
            const dropNodeData = dropNode as unknown as AssetTreeDataNode;
            const isTargetFolder = dropNodeData?.isFolder;
            const isDropToGap = dropPosition === -1 || dropPosition === 1;
            
            // Allow dropping into folders or between any nodes
            if (!isDropToGap && !isTargetFolder) {
              console.log('Rejecting drop: target is not a folder');
              return false;
            }
            
            return true;
          }}
          onDrop={(info) => {
            console.log('DROP EVENT:', {
              dragNode: info.dragNode?.key,
              dropNode: info.node?.key,
              dropPosition: info.dropPosition,
              dropToGap: info.dropToGap
            });
            
            if (!info.dragNode || !info.node) {
              console.error('Missing drag or drop node');
              return;
            }
            
            handleDrop(info);
          }}
          onDragStart={(info) => {
            console.log('Drag started:', info.node?.key);
            setIsDragging(true);
            // setDraggedNode(info.node?.key || null);
            // Ensure the drag effect is set
            if (info.event && info.event.dataTransfer) {
              info.event.dataTransfer.effectAllowed = 'move';
              info.event.dataTransfer.setData('text/plain', String(info.node?.key || ''));
            }
          }}
          onDragEnd={(info) => {
            console.log('Drag ended:', info.node?.key, 'wasDragging:', isDragging);
            setIsDragging(false);
            // setDraggedNode(null);
            // setDragOverNodeKey(null);
          }}
          onDragEnter={(info) => {
            console.log('Drag enter:', info.node?.key);
            info.expandedKeys = expandedKeys;
          }}
          onDragOver={(info) => {
            console.log('Drag over:', info.node?.key);
          }}
          onDragLeave={(info) => {
            console.log('Drag leave');
          }}
          showLine={{ showLeafIcon: false }}
          blockNode
          dropIndicatorRender={(props) => {
            console.log('Drop indicator render:', props);
            return null; // Use default indicator
          }}
          style={{ 
            background: 'transparent',
            fontSize: '14px',
          }}
          />
        </Spin>
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
      
      {/* Custom Context Menu */}
      {contextMenu.visible && (
        <div
          ref={contextMenuRef}
          style={{
            position: 'fixed',
            left: contextMenu.x,
            top: contextMenu.y,
            zIndex: 9999,
            background: 'white',
            border: '1px solid #d9d9d9',
            borderRadius: '6px',
            boxShadow: '0 6px 16px 0 rgba(0, 0, 0, 0.08), 0 3px 6px -4px rgba(0, 0, 0, 0.12), 0 9px 28px 8px rgba(0, 0, 0, 0.05)',
            padding: '4px 0',
            minWidth: '160px',
          }}
        >
          {getContextMenuItems(contextMenu.asset).map((item) => {
            if (item.type === 'divider') {
              return (
                <div
                  key={item.key || 'divider'}
                  style={{
                    height: '1px',
                    background: '#f0f0f0',
                    margin: '4px 0',
                  }}
                />
              );
            }
            
            return (
              <div
                key={item.key}
                style={{
                  padding: '8px 12px',
                  cursor: item.disabled ? 'not-allowed' : 'pointer',
                  color: item.disabled ? '#d9d9d9' : (item.danger ? '#ff4d4f' : '#262626'),
                  display: 'flex',
                  alignItems: 'center',
                  gap: '8px',
                  fontSize: '14px',
                  opacity: item.disabled ? 0.5 : 1,
                }}
                onMouseEnter={(e) => {
                  if (!item.disabled) {
                    e.currentTarget.style.background = '#f5f5f5';
                  }
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = 'transparent';
                }}
                onClick={() => {
                  if (!item.disabled) {
                    handleContextMenuClick(item.key as string, contextMenu.asset);
                  }
                }}
              >
                {item.icon}
                <span>{item.label}</span>
              </div>
            );
          })}
        </div>
      )}
      
      {/* Move Asset Modal */}
      <Modal
        title={`Move "${assetToMove?.name || ''}" to...`}
        open={moveModalVisible}
        onOk={handleMoveAsset}
        onCancel={() => {
          setMoveModalVisible(false);
          setAssetToMove(null);
          setSelectedDestination(null);
        }}
        okText="Move"
        cancelText="Cancel"
        width={500}
      >
        <div style={{ marginBottom: 16 }}>
          <Text>Select the destination folder:</Text>
        </div>
        <TreeSelect
          style={{ width: '100%' }}
          value={selectedDestination}
          dropdownStyle={{ maxHeight: 400, overflow: 'auto' }}
          placeholder="Select destination folder"
          treeDefaultExpandAll
          onChange={(value) => setSelectedDestination(value)}
          treeData={[
            {
              value: 0,
              title: 'Root (Top Level)',
              children: convertToTreeSelectData(hierarchyData, assetToMove?.id)
            }
          ]}
          showSearch
          filterTreeNode={(search, item) => {
            const titleStr = item.title?.toString() || '';
            return titleStr.toLowerCase().indexOf(search.toLowerCase()) >= 0;
          }}
        />
        {selectedDestination !== null && (
          <div style={{ marginTop: 16 }}>
            <Text type="secondary">
              The asset will be moved to: {selectedDestination === 0 ? 'Root Level' : `Selected Folder`}
            </Text>
          </div>
        )}
      </Modal>
    </div>
  );
};

export default AssetTreeView;