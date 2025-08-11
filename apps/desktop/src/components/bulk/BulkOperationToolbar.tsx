import React, { useState, useCallback } from 'react';
import { Button, Space, Dropdown, Modal, message, Typography } from 'antd';
import {
  DeleteOutlined,
  ExportOutlined,
  FolderOpenOutlined,
  TagOutlined,
  DownOutlined,
  HistoryOutlined,
  ExclamationCircleOutlined,
} from '@ant-design/icons';
import type { MenuProps } from 'antd';
import useBulkOperationsStore from '../../store/bulkOperations';
import {
  BulkMoveOptions,
  BulkDeleteOptions,
  BulkExportOptions,
  ExportFormat,
} from '../../types/bulkOperations';

const { Text } = Typography;
const { confirm } = Modal;

interface BulkOperationToolbarProps {
  className?: string;
}

const BulkOperationToolbar: React.FC<BulkOperationToolbarProps> = ({ className }) => {
  const {
    getSelectedAssets,
    getSelectedCount,
    startBulkMove,
    startBulkDelete,
    startBulkExport,
    startBulkClassify,
    showProgressModal,
    showHistoryPanel,
    resetSelection,
    error,
    isLoading,
  } = useBulkOperationsStore();

  const [isOperationInProgress, setIsOperationInProgress] = useState(false);

  const selectedAssets = getSelectedAssets();
  const selectedCount = getSelectedCount();
  const hasSelection = selectedCount > 0;

  // Handle bulk move operation
  const handleBulkMove = useCallback(async (targetParentId?: number) => {
    if (selectedAssets.length === 0) {
      message.warning('No assets selected for move operation');
      return;
    }

    const options: BulkMoveOptions = {
      new_parent_id: targetParentId || null,
      validate_hierarchy: true,
      skip_conflicts: false,
    };

    try {
      setIsOperationInProgress(true);
      const operationId = await startBulkMove({
        asset_ids: selectedAssets,
        new_parent_id: targetParentId || null,
        options,
      });

      message.success(`Started bulk move operation: ${operationId.slice(0, 8)}...`);
      showProgressModal(true);
      resetSelection();
    } catch (err) {
      console.error('Failed to start bulk move:', err);
      message.error('Failed to start bulk move operation');
    } finally {
      setIsOperationInProgress(false);
    }
  }, [selectedAssets, startBulkMove, showProgressModal, resetSelection]);

  // Handle bulk delete operation
  const handleBulkDelete = useCallback(() => {
    if (selectedAssets.length === 0) {
      message.warning('No assets selected for delete operation');
      return;
    }

    confirm({
      title: 'Confirm Bulk Delete',
      icon: <ExclamationCircleOutlined />,
      content: (
        <div>
          <p>Are you sure you want to delete {selectedCount} selected assets?</p>
          <Text type="warning">This action cannot be undone.</Text>
        </div>
      ),
      okText: 'Delete',
      okType: 'danger',
      cancelText: 'Cancel',
      onOk: async () => {
        const options: BulkDeleteOptions = {
          force_delete: false,
          delete_children: false,
          skip_protected: true,
        };

        try {
          setIsOperationInProgress(true);
          const operationId = await startBulkDelete({
            asset_ids: selectedAssets,
            options,
          });

          message.success(`Started bulk delete operation: ${operationId.slice(0, 8)}...`);
          showProgressModal(true);
          resetSelection();
        } catch (err) {
          console.error('Failed to start bulk delete:', err);
          message.error('Failed to start bulk delete operation');
        } finally {
          setIsOperationInProgress(false);
        }
      },
    });
  }, [selectedAssets, selectedCount, startBulkDelete, showProgressModal, resetSelection]);

  // Handle bulk export operation
  const handleBulkExport = useCallback(async (format: ExportFormat) => {
    if (selectedAssets.length === 0) {
      message.warning('No assets selected for export operation');
      return;
    }

    const options: BulkExportOptions = {
      format,
      include_metadata: true,
      include_children: false,
      include_configurations: true,
      export_path: undefined,
    };

    try {
      setIsOperationInProgress(true);
      const operationId = await startBulkExport({
        asset_ids: selectedAssets,
        format,
        options,
      });

      message.success(`Started bulk export operation: ${operationId.slice(0, 8)}...`);
      showProgressModal(true);
      resetSelection();
    } catch (err) {
      console.error('Failed to start bulk export:', err);
      message.error('Failed to start bulk export operation');
    } finally {
      setIsOperationInProgress(false);
    }
  }, [selectedAssets, startBulkExport, showProgressModal, resetSelection]);

  // Handle bulk classify operation
  const handleBulkClassify = useCallback(async (classification: string) => {
    if (selectedAssets.length === 0) {
      message.warning('No assets selected for classification operation');
      return;
    }

    try {
      setIsOperationInProgress(true);
      const operationId = await startBulkClassify({
        asset_ids: selectedAssets,
        new_classification: classification,
        apply_to_children: false,
      });

      message.success(`Started bulk classify operation: ${operationId.slice(0, 8)}...`);
      showProgressModal(true);
      resetSelection();
    } catch (err) {
      console.error('Failed to start bulk classify:', err);
      message.error('Failed to start bulk classify operation');
    } finally {
      setIsOperationInProgress(false);
    }
  }, [selectedAssets, startBulkClassify, showProgressModal, resetSelection]);

  // Export format menu
  const exportMenuItems: MenuProps['items'] = [
    {
      key: 'csv',
      label: 'Export as CSV',
      icon: <ExportOutlined />,
      onClick: () => handleBulkExport('csv' as ExportFormat),
    },
    {
      key: 'json',
      label: 'Export as JSON',
      icon: <ExportOutlined />,
      onClick: () => handleBulkExport('json' as ExportFormat),
    },
    {
      key: 'xml',
      label: 'Export as XML',
      icon: <ExportOutlined />,
      onClick: () => handleBulkExport('xml' as ExportFormat),
    },
    {
      key: 'yaml',
      label: 'Export as YAML',
      icon: <ExportOutlined />,
      onClick: () => handleBulkExport('yaml' as ExportFormat),
    },
  ];

  // Classification menu
  const classificationMenuItems: MenuProps['items'] = [
    {
      key: 'unclassified',
      label: 'Unclassified',
      onClick: () => handleBulkClassify('unclassified'),
    },
    {
      key: 'internal',
      label: 'Internal',
      onClick: () => handleBulkClassify('internal'),
    },
    {
      key: 'confidential',
      label: 'Confidential',
      onClick: () => handleBulkClassify('confidential'),
    },
    {
      key: 'restricted',
      label: 'Restricted',
      onClick: () => handleBulkClassify('restricted'),
    },
  ];

  // Move menu (simplified - would need tree picker in real implementation)
  const moveMenuItems: MenuProps['items'] = [
    {
      key: 'root',
      label: 'Move to Root',
      onClick: () => handleBulkMove(undefined),
    },
    {
      key: 'select',
      label: 'Select Destination...',
      onClick: () => {
        // TODO: Open destination picker modal
        message.info('Destination picker not yet implemented');
      },
    },
  ];

  if (!hasSelection) {
    return null;
  }

  return (
    <div className={`bulk-operation-toolbar ${className || ''}`} style={{ 
      padding: '8px 16px',
      background: '#f6f6f6',
      borderRadius: '6px',
      border: '1px solid #d9d9d9',
    }}>
      <Space size="small" align="center">
        <Text strong style={{ marginRight: '8px' }}>
          Bulk Actions ({selectedCount} selected):
        </Text>

        {/* Move operation */}
        <Dropdown
          menu={{ items: moveMenuItems }}
          trigger={['click']}
          disabled={isOperationInProgress || isLoading}
        >
          <Button
            type="default"
            icon={<FolderOpenOutlined />}
            size="small"
            loading={isOperationInProgress}
          >
            Move <DownOutlined />
          </Button>
        </Dropdown>

        {/* Delete operation */}
        <Button
          type="default"
          danger
          icon={<DeleteOutlined />}
          size="small"
          onClick={handleBulkDelete}
          disabled={isOperationInProgress || isLoading}
          loading={isOperationInProgress}
        >
          Delete
        </Button>

        {/* Export operation */}
        <Dropdown
          menu={{ items: exportMenuItems }}
          trigger={['click']}
          disabled={isOperationInProgress || isLoading}
        >
          <Button
            type="default"
            icon={<ExportOutlined />}
            size="small"
            loading={isOperationInProgress}
          >
            Export <DownOutlined />
          </Button>
        </Dropdown>

        {/* Classify operation */}
        <Dropdown
          menu={{ items: classificationMenuItems }}
          trigger={['click']}
          disabled={isOperationInProgress || isLoading}
        >
          <Button
            type="default"
            icon={<TagOutlined />}
            size="small"
            loading={isOperationInProgress}
          >
            Classify <DownOutlined />
          </Button>
        </Dropdown>

        {/* History button */}
        <Button
          type="text"
          icon={<HistoryOutlined />}
          size="small"
          onClick={() => showHistoryPanel(true)}
          title="View Operation History"
        >
          History
        </Button>
      </Space>

      {/* Error display */}
      {error && (
        <div style={{ marginTop: '8px' }}>
          <Text type="danger" style={{ fontSize: '12px' }}>
            Error: {error}
          </Text>
        </div>
      )}
    </div>
  );
};

export default BulkOperationToolbar;