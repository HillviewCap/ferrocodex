import React, { useCallback, useEffect, useRef } from 'react';
import { Checkbox } from 'antd';
import type { CheckboxChangeEvent } from 'antd/es/checkbox';
import useBulkOperationsStore from '../../store/bulkOperations';

interface AssetSelectionCheckboxProps {
  assetId: number;
  assetName?: string;
  assetType?: string;
  context?: 'tree' | 'search' | 'manual';
  disabled?: boolean;
  className?: string;
  size?: 'small' | 'middle' | 'large';
  onClick?: (event: React.MouseEvent) => void;
}

const AssetSelectionCheckbox: React.FC<AssetSelectionCheckboxProps> = ({
  assetId,
  assetName,
  assetType,
  context = 'manual',
  disabled = false,
  className,
  size = 'small',
  onClick,
}) => {
  const {
    isSelected,
    toggleAsset,
    selectAsset,
    deselectAsset,
    selectRange,
    setLastFocusedAsset,
    setSelectionAnchor,
    keyboardState,
    selection,
  } = useBulkOperationsStore();

  const checkboxRef = useRef<HTMLInputElement>(null);
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

  // Handle keyboard navigation
  useEffect(() => {
    const checkbox = checkboxRef.current;
    if (!checkbox) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      // Handle space key for toggling
      if (e.code === 'Space' && document.activeElement === checkbox) {
        e.preventDefault();
        toggleAsset(assetId, context);
        return;
      }

      // Handle arrow keys for navigation (would need parent container support)
      if (['ArrowUp', 'ArrowDown'].includes(e.code)) {
        // This would need to be implemented in coordination with the parent list component
        e.preventDefault();
      }
    };

    checkbox.addEventListener('keydown', handleKeyDown);
    return () => checkbox.removeEventListener('keydown', handleKeyDown);
  }, [assetId, context, toggleAsset]);

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
      ref={checkboxRef}
      checked={selected}
      onChange={handleChange}
      onClick={handleClick}
      onFocus={handleFocus}
      disabled={disabled}
      className={`asset-selection-checkbox ${className || ''}`}
      size={size}
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