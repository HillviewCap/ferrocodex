import React, { useCallback, useEffect } from 'react';
import { Checkbox, Button, Space, Typography, Badge, Popover } from 'antd';
import {
  SelectOutlined,
  ClearOutlined,
  SwapOutlined,
  InfoCircleOutlined,
} from '@ant-design/icons';
import useBulkOperationsStore from '../../store/bulkOperations';

const { Text } = Typography;

interface BulkSelectionManagerProps {
  availableAssetIds: number[];
  className?: string;
}

const BulkSelectionManager: React.FC<BulkSelectionManagerProps> = ({
  availableAssetIds,
  className,
}) => {
  const {
    selection,
    getSelectedCount,
    getSelectedAssets,
    selectAll,
    selectNone,
    invertSelection,
    showSelectionToolbar,
  } = useBulkOperationsStore();

  const selectedCount = getSelectedCount();
  const totalAvailable = availableAssetIds.length;
  const hasSelection = selectedCount > 0;

  // Handle select all checkbox state
  const isAllSelected = selectedCount === totalAvailable && totalAvailable > 0;
  const isIndeterminate = selectedCount > 0 && selectedCount < totalAvailable;

  const handleSelectAll = useCallback(() => {
    if (isAllSelected) {
      selectNone();
    } else {
      selectAll(availableAssetIds, 'search');
    }
  }, [isAllSelected, selectAll, selectNone, availableAssetIds]);

  const handleSelectNone = useCallback(() => {
    selectNone();
  }, [selectNone]);

  const handleInvertSelection = useCallback(() => {
    invertSelection(availableAssetIds);
  }, [invertSelection, availableAssetIds]);

  // Show selection toolbar when items are selected
  useEffect(() => {
    showSelectionToolbar(hasSelection);
  }, [hasSelection, showSelectionToolbar]);

  const SelectionInfo = () => (
    <div style={{ padding: '8px 12px', maxWidth: '300px' }}>
      <Text strong>Selection Details</Text>
      <div style={{ marginTop: '8px' }}>
        <Text>Selected: {selectedCount}</Text>
        <br />
        <Text>Available: {totalAvailable}</Text>
        <br />
        <Text>Mode: {selection.selection_mode}</Text>
      </div>
      {selectedCount > 0 && (
        <div style={{ marginTop: '8px' }}>
          <Text type="secondary" style={{ fontSize: '12px' }}>
            Selected Asset IDs: {getSelectedAssets().slice(0, 10).join(', ')}
            {selectedCount > 10 && '...'}
          </Text>
        </div>
      )}
    </div>
  );

  return (
    <div className={`bulk-selection-manager ${className || ''}`}>
      <Space size="small" align="center">
        {/* Master checkbox for select all/none */}
        <Checkbox
          checked={isAllSelected}
          indeterminate={isIndeterminate}
          onChange={handleSelectAll}
          disabled={totalAvailable === 0}
        >
          <Text style={{ userSelect: 'none' }}>
            {isAllSelected ? 'Deselect All' : 'Select All'}
          </Text>
        </Checkbox>

        {/* Selection count with info */}
        <Badge
          count={selectedCount}
          style={{ backgroundColor: hasSelection ? '#1890ff' : '#d9d9d9' }}
          showZero
        />
        
        <Popover
          content={<SelectionInfo />}
          title="Selection Information"
          trigger="hover"
          placement="bottomLeft"
        >
          <Button
            type="text"
            size="small"
            icon={<InfoCircleOutlined />}
            style={{ color: '#8c8c8c' }}
          />
        </Popover>

        {/* Selection tools */}
        {totalAvailable > 0 && (
          <Space.Compact size="small">
            <Button
              size="small"
              icon={<SelectOutlined />}
              onClick={handleSelectAll}
              disabled={isAllSelected}
              title="Select All"
            />
            <Button
              size="small"
              icon={<ClearOutlined />}
              onClick={handleSelectNone}
              disabled={selectedCount === 0}
              title="Select None"
            />
            <Button
              size="small"
              icon={<SwapOutlined />}
              onClick={handleInvertSelection}
              disabled={totalAvailable === 0}
              title="Invert Selection"
            />
          </Space.Compact>
        )}

        {/* Quick selection info */}
        {hasSelection && (
          <Text type="secondary" style={{ fontSize: '12px', marginLeft: '8px' }}>
            {selectedCount} of {totalAvailable} selected
          </Text>
        )}
      </Space>

      {/* Keyboard shortcuts hint */}
      {hasSelection && (
        <div style={{ marginTop: '4px', fontSize: '11px', color: '#8c8c8c' }}>
          <Text type="secondary">
            💡 Tip: Use Ctrl+Click for individual selection, Shift+Click for range selection
          </Text>
        </div>
      )}
    </div>
  );
};

export default BulkSelectionManager;