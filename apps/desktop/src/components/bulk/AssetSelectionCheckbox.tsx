import React, { useCallback } from 'react';
import { Checkbox } from 'antd';
import type { CheckboxChangeEvent } from 'antd/es/checkbox';
import useBulkOperationsStore from '../../store/bulkOperations';

interface AssetSelectionCheckboxProps {
  assetId: number;
  assetName?: string;
  assetType?: string;
  context?: 'tree' | 'search';
  disabled?: boolean;
  className?: string;
  onClick?: (event: React.MouseEvent) => void;
}

const AssetSelectionCheckbox: React.FC<AssetSelectionCheckboxProps> = ({
  assetId,
  assetName,
  context = 'search',
  disabled = false,
  className,
  onClick,
}) => {
  const {
    isSelected,
    toggleAsset,
    selectAsset,
    deselectAsset,
    setLastFocusedAsset,
    setSelectionAnchor,
    selection,
  } = useBulkOperationsStore();

  const selected = isSelected(assetId);

  // Handle checkbox change
  const handleChange = useCallback((e: CheckboxChangeEvent) => {
    const isChecked = e.target.checked;
    const nativeEvent = e.nativeEvent as MouseEvent;
    
    // Handle different selection modes based on modifier keys
    if (nativeEvent.shiftKey && selection.last_selection_anchor) {
      // Range selection with Shift+click
      // For range selection, we would need the ordered list of asset IDs
      // This is a simplified implementation
      if (isChecked) {
        selectAsset(assetId, context);
      } else {
        deselectAsset(assetId);
      }
    } else if (nativeEvent.ctrlKey || nativeEvent.metaKey) {
      // Toggle individual item with Ctrl+click
      toggleAsset(assetId, context);
    } else {
      // Normal single selection
      if (isChecked) {
        selectAsset(assetId, context);
      } else {
        deselectAsset(assetId);
      }
    }

    // Update focus and anchor for keyboard navigation
    setLastFocusedAsset(assetId);
    if (!nativeEvent.shiftKey) {
      setSelectionAnchor(assetId);
    }
  }, [
    assetId,
    context,
    selectAsset,
    deselectAsset,
    toggleAsset,
    setLastFocusedAsset,
    setSelectionAnchor,
    selection.last_selection_anchor,
  ]);

  // Note: Keyboard navigation would need to be implemented at the parent component level

  // Handle focus events for keyboard navigation
  const handleFocus = useCallback(() => {
    setLastFocusedAsset(assetId);
  }, [assetId, setLastFocusedAsset]);

  // Handle click event passthrough
  const handleClick = useCallback((event: React.MouseEvent) => {
    if (onClick) {
      onClick(event);
    }
  }, [onClick]);

  return (
    <Checkbox
      checked={selected}
      onChange={handleChange}
      onClick={handleClick}
      onFocus={handleFocus}
      disabled={disabled}
      className={`asset-selection-checkbox ${className || ''}`}
      title={`${selected ? 'Deselect' : 'Select'} ${assetName || `Asset ${assetId}`}`}
      style={{
        userSelect: 'none',
        ...(selected && {
          backgroundColor: '#e6f7ff',
          borderRadius: '2px',
        }),
      }}
    />
  );
};

export default AssetSelectionCheckbox;