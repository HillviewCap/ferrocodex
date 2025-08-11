import React, { useState, useEffect, useCallback } from 'react';
import {
  Button,
  Drawer,
  List,
  Typography,
  Space,
  Tooltip,
  Tag,
  Empty,
  message,
  Popconfirm
} from 'antd';
import {
  UndoOutlined,
  RedoOutlined,
  HistoryOutlined,
  DeleteOutlined,
  InfoCircleOutlined
} from '@ant-design/icons';
import { HistoryManager as HistoryManagerUtil, HistoryOperation } from '../../utils/historyManager';
import useBulkOperationsStore from '../../store/bulkOperations';
import { BulkOperationType } from '../../types/bulkOperations';

const { Text } = Typography;

interface HistoryManagerProps {
  visible?: boolean;
  onVisibilityChange?: (visible: boolean) => void;
  className?: string;
  style?: React.CSSProperties;
}

export const HistoryManagerComponent: React.FC<HistoryManagerProps> = ({
  onVisibilityChange,
  className,
  style
}) => {
  const [historyOperations, setHistoryOperations] = useState<HistoryOperation[]>([]);
  const [canUndo, setCanUndo] = useState(false);
  const [canRedo, setCanRedo] = useState(false);
  const [isUndoing, setIsUndoing] = useState(false);
  const [isRedoing, setIsRedoing] = useState(false);
  const [drawerVisible, setDrawerVisible] = useState(false);

  const historyManager = HistoryManagerUtil.getInstance();
  const { undoOperation } = useBulkOperationsStore();

  // Update state when history changes
  const updateHistoryState = useCallback(() => {
    setHistoryOperations(historyManager.getRecentOperations(20));
    setCanUndo(historyManager.canUndo());
    setCanRedo(historyManager.canRedo());
  }, [historyManager]);

  // Subscribe to history changes
  useEffect(() => {
    const unsubscribe = historyManager.subscribe(updateHistoryState);
    updateHistoryState(); // Initial load
    return unsubscribe;
  }, [historyManager, updateHistoryState]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.ctrlKey && event.key === 'z' && !event.shiftKey) {
        event.preventDefault();
        handleUndo();
      } else if ((event.ctrlKey && event.key === 'y') || (event.ctrlKey && event.shiftKey && event.key === 'Z')) {
        event.preventDefault();
        handleRedo();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, []);

  // Handle undo operation
  const handleUndo = useCallback(async () => {
    if (!canUndo || isUndoing) return;

    const operation = historyManager.getUndoOperation();
    if (!operation) return;

    setIsUndoing(true);
    try {
      // For bulk operations, use the existing undo system
      await undoOperation(operation.id);
      
      // Update local history
      historyManager.undo();
      
      message.success(`Undid: ${operation.description}`);
    } catch (error) {
      console.error('Undo failed:', error);
      message.error('Failed to undo operation');
    } finally {
      setIsUndoing(false);
    }
  }, [canUndo, isUndoing, historyManager, undoOperation]);

  // Handle redo operation
  const handleRedo = useCallback(async () => {
    if (!canRedo || isRedoing) return;

    const operation = historyManager.getRedoOperation();
    if (!operation) return;

    setIsRedoing(true);
    try {
      // For now, redo is not implemented in backend
      // This would require storing forward operations
      message.info('Redo functionality coming soon');
      
      // historyManager.redo();
    } catch (error) {
      console.error('Redo failed:', error);
      message.error('Failed to redo operation');
    } finally {
      setIsRedoing(false);
    }
  }, [canRedo, isRedoing, historyManager]);

  // Clear history
  const handleClearHistory = useCallback(() => {
    historyManager.clear();
    message.success('History cleared');
  }, [historyManager]);

  // Toggle drawer visibility
  const toggleDrawer = useCallback(() => {
    const newVisible = !drawerVisible;
    setDrawerVisible(newVisible);
    onVisibilityChange?.(newVisible);
  }, [drawerVisible, onVisibilityChange]);

  // Get operation type color
  const getOperationTypeColor = (type: BulkOperationType | 'rename'): string => {
    switch (type) {
      case 'move': return 'blue';
      case 'delete': return 'red';
      case 'export': return 'green';
      case 'classify': return 'purple';
      case 'rename': return 'orange';
      default: return 'default';
    }
  };

  // Get operation type icon
  const getOperationTypeIcon = (type: BulkOperationType | 'rename'): React.ReactNode => {
    switch (type) {
      case 'move': return '↔️';
      case 'delete': return '🗑️';
      case 'export': return '📤';
      case 'classify': return '🏷️';
      case 'rename': return '✏️';
      default: return '⚙️';
    }
  };

  // Format timestamp
  const formatTimestamp = (timestamp: string): string => {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    
    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    
    const diffHours = Math.floor(diffMins / 60);
    if (diffHours < 24) return `${diffHours}h ago`;
    
    const diffDays = Math.floor(diffHours / 24);
    if (diffDays < 7) return `${diffDays}d ago`;
    
    return date.toLocaleDateString();
  };

  return (
    <div className={className} style={style}>
      {/* Undo/Redo Buttons */}
      <Space>
        <Tooltip title="Undo (Ctrl+Z)">
          <Button
            icon={<UndoOutlined />}
            disabled={!canUndo}
            loading={isUndoing}
            onClick={handleUndo}
            size="small"
          >
            Undo
          </Button>
        </Tooltip>
        
        <Tooltip title="Redo (Ctrl+Y)">
          <Button
            icon={<RedoOutlined />}
            disabled={!canRedo}
            loading={isRedoing}
            onClick={handleRedo}
            size="small"
          >
            Redo
          </Button>
        </Tooltip>
        
        <Tooltip title="View History">
          <Button
            icon={<HistoryOutlined />}
            onClick={toggleDrawer}
            size="small"
          >
            History
          </Button>
        </Tooltip>
      </Space>

      {/* History Drawer */}
      <Drawer
        title={
          <Space>
            <HistoryOutlined />
            <span>Operation History</span>
            <Tag color="blue">{historyOperations.length} operations</Tag>
          </Space>
        }
        placement="right"
        width={400}
        open={drawerVisible}
        onClose={() => {
          setDrawerVisible(false);
          onVisibilityChange?.(false);
        }}
        extra={
          <Space>
            <Popconfirm
              title="Clear all history?"
              description="This action cannot be undone."
              onConfirm={handleClearHistory}
              okText="Clear"
              cancelText="Cancel"
            >
              <Button 
                icon={<DeleteOutlined />} 
                size="small"
                danger
                disabled={historyOperations.length === 0}
              >
                Clear
              </Button>
            </Popconfirm>
          </Space>
        }
      >
        {historyOperations.length === 0 ? (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description="No operations yet"
          />
        ) : (
          <>
            <div style={{ marginBottom: 16 }}>
              <Text type="secondary">
                <InfoCircleOutlined style={{ marginRight: 4 }} />
                Use Ctrl+Z to undo, Ctrl+Y to redo
              </Text>
            </div>
            
            <List
              dataSource={historyOperations}
              renderItem={(operation, index) => {
                const isLastOperation = index === 0; // Most recent first
                const canUndoThis = operation.canUndo && index === 0;
                
                return (
                  <List.Item
                    style={{
                      opacity: isLastOperation ? 1 : 0.7,
                      border: isLastOperation ? '1px solid #1890ff' : 'none',
                      borderRadius: isLastOperation ? 4 : 0,
                      padding: 12,
                      backgroundColor: isLastOperation ? '#f0f8ff' : 'transparent'
                    }}
                  >
                    <List.Item.Meta
                      avatar={
                        <span style={{ fontSize: 16 }}>
                          {getOperationTypeIcon(operation.type)}
                        </span>
                      }
                      title={
                        <Space>
                          <Text strong={isLastOperation}>
                            {operation.description}
                          </Text>
                          <Tag color={getOperationTypeColor(operation.type)} size="small">
                            {operation.type.toUpperCase()}
                          </Tag>
                          {isLastOperation && (
                            <Tag color="blue" size="small">LAST</Tag>
                          )}
                        </Space>
                      }
                      description={
                        <Space direction="vertical" size={4}>
                          <Text type="secondary" style={{ fontSize: 12 }}>
                            {formatTimestamp(operation.timestamp)}
                          </Text>
                          <Text type="secondary" style={{ fontSize: 12 }}>
                            {operation.assetIds.length} asset{operation.assetIds.length !== 1 ? 's' : ''}
                          </Text>
                        </Space>
                      }
                    />
                  </List.Item>
                );
              }}
            />
          </>
        )}
      </Drawer>
    </div>
  );
};

// Export both as named and default
export default HistoryManagerComponent;