import React, { useEffect, useCallback, useRef } from 'react';
import { message } from 'antd';
import useBulkOperationsStore from '../../store/bulkOperations';
import { AssetHierarchy } from '../../types/assets';

interface KeyboardShortcutsProps {
  assets: AssetHierarchy[];
  selectedAsset?: AssetHierarchy | null;
  onAssetSelect?: (asset: AssetHierarchy | null) => void;
  onRenameStart?: (asset: AssetHierarchy) => void;
  onDeleteStart?: () => void;
  enabled?: boolean;
  children: React.ReactNode;
}

export const KeyboardShortcuts: React.FC<KeyboardShortcutsProps> = ({
  assets,
  selectedAsset,
  onAssetSelect,
  onRenameStart,
  onDeleteStart,
  enabled = true,
  children
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const lastFocusTime = useRef<number>(0);

  const {
    selectAll,
    selectNone,
    getSelectedAssets,
    getSelectedCount,
    isSelected,
    setKeyboardNavigationMode,
    setLastFocusedAsset,
    setSelectionAnchor,
    keyboardState
  } = useBulkOperationsStore();

  // Get flat list of assets for navigation
  const getFlatAssetList = useCallback((): AssetHierarchy[] => {
    const flatList: AssetHierarchy[] = [];
    
    const traverse = (assetList: AssetHierarchy[]) => {
      for (const asset of assetList) {
        flatList.push(asset);
        if (asset.children && asset.children.length > 0) {
          traverse(asset.children);
        }
      }
    };
    
    traverse(assets);
    return flatList;
  }, [assets]);

  // Find asset by ID in flat list
  const findAssetById = useCallback((id: number): AssetHierarchy | null => {
    const flatList = getFlatAssetList();
    return flatList.find(asset => asset.id === id) || null;
  }, [getFlatAssetList]);

  // Navigate to next/previous asset
  const navigateAsset = useCallback((direction: 'up' | 'down') => {
    const flatList = getFlatAssetList();
    if (flatList.length === 0) return;

    const currentAssetId = selectedAsset?.id || keyboardState.last_focused_asset;
    if (!currentAssetId) {
      // No current selection, select first asset
      const firstAsset = flatList[0];
      onAssetSelect?.(firstAsset);
      setLastFocusedAsset(firstAsset.id);
      return;
    }

    const currentIndex = flatList.findIndex(asset => asset.id === currentAssetId);
    if (currentIndex === -1) return;

    let nextIndex;
    if (direction === 'down') {
      nextIndex = currentIndex + 1;
      if (nextIndex >= flatList.length) nextIndex = 0; // Loop to beginning
    } else {
      nextIndex = currentIndex - 1;
      if (nextIndex < 0) nextIndex = flatList.length - 1; // Loop to end
    }

    const nextAsset = flatList[nextIndex];
    onAssetSelect?.(nextAsset);
    setLastFocusedAsset(nextAsset.id);
  }, [getFlatAssetList, selectedAsset, keyboardState.last_focused_asset, onAssetSelect, setLastFocusedAsset]);

  // Handle keyboard events
  const handleKeyDown = useCallback((event: KeyboardEvent) => {
    if (!enabled) return;

    // Check if we're in an input field
    const target = event.target as HTMLElement;
    if (target && (
      target.tagName === 'INPUT' ||
      target.tagName === 'TEXTAREA' ||
      target.contentEditable === 'true' ||
      target.closest('.ant-input') ||
      target.closest('.ant-select') ||
      target.closest('.ant-modal')
    )) {
      // Don't handle shortcuts when typing in input fields
      return;
    }

    // Update last focus time for navigation mode
    const now = Date.now();
    if (now - lastFocusTime.current > 100) {
      setKeyboardNavigationMode(true);
      lastFocusTime.current = now;
    }

    switch (true) {
      // Select All (Ctrl+A)
      case event.ctrlKey && event.key === 'a': {
        event.preventDefault();
        const flatList = getFlatAssetList();
        const allAssetIds = flatList.map(asset => asset.id);
        selectAll(allAssetIds, 'manual');
        message.info(`Selected all ${allAssetIds.length} assets`);
        break;
      }

      // Clear Selection (Escape)
      case event.key === 'Escape': {
        event.preventDefault();
        selectNone();
        onAssetSelect?.(null);
        setLastFocusedAsset(null);
        setSelectionAnchor(null);
        message.info('Selection cleared');
        break;
      }

      // Delete Selected Assets (Delete key)
      case event.key === 'Delete': {
        event.preventDefault();
        const selectedCount = getSelectedCount();
        if (selectedCount > 0) {
          onDeleteStart?.();
        } else if (selectedAsset) {
          // Delete currently focused asset if no bulk selection
          onDeleteStart?.();
        } else {
          message.info('No assets selected for deletion');
        }
        break;
      }

      // Rename Asset (F2)
      case event.key === 'F2': {
        event.preventDefault();
        if (selectedAsset) {
          onRenameStart?.(selectedAsset);
        } else {
          const focusedAssetId = keyboardState.last_focused_asset;
          if (focusedAssetId) {
            const focusedAsset = findAssetById(focusedAssetId);
            if (focusedAsset) {
              onRenameStart?.(focusedAsset);
            }
          } else {
            message.info('No asset selected for renaming');
          }
        }
        break;
      }

      // Navigate Up (Arrow Up)
      case event.key === 'ArrowUp': {
        event.preventDefault();
        navigateAsset('up');
        break;
      }

      // Navigate Down (Arrow Down)
      case event.key === 'ArrowDown': {
        event.preventDefault();
        navigateAsset('down');
        break;
      }

      // Enter - Select/Toggle Asset
      case event.key === 'Enter': {
        event.preventDefault();
        const focusedAssetId = keyboardState.last_focused_asset || selectedAsset?.id;
        if (focusedAssetId) {
          const focusedAsset = findAssetById(focusedAssetId);
          if (focusedAsset) {
            if (event.ctrlKey) {
              // Ctrl+Enter: Toggle selection
              if (isSelected(focusedAssetId)) {
                // deselectAsset is handled by the bulk operations store
              } else {
                // selectAsset is handled by the bulk operations store
              }
            } else {
              // Enter: Single select
              onAssetSelect?.(focusedAsset);
            }
          }
        }
        break;
      }

      // Space - Toggle Selection
      case event.key === ' ': {
        event.preventDefault();
        const focusedAssetId = keyboardState.last_focused_asset || selectedAsset?.id;
        if (focusedAssetId) {
          // Toggle selection handled by bulk operations store
          message.info(isSelected(focusedAssetId) ? 'Asset deselected' : 'Asset selected');
        }
        break;
      }

      // Shift+Arrow for range selection
      case event.shiftKey && (event.key === 'ArrowUp' || event.key === 'ArrowDown'): {
        event.preventDefault();
        const flatList = getFlatAssetList();
        const currentAssetId = selectedAsset?.id || keyboardState.last_focused_asset;
        const anchorId = keyboardState.selection_anchor;
        
        if (!currentAssetId || !anchorId) {
          // No anchor set, just navigate
          navigateAsset(event.key === 'ArrowDown' ? 'down' : 'up');
          return;
        }

        const currentIndex = flatList.findIndex(asset => asset.id === currentAssetId);
        const anchorIndex = flatList.findIndex(asset => asset.id === anchorId);
        
        if (currentIndex === -1 || anchorIndex === -1) return;

        // Calculate range
        const nextIndex = event.key === 'ArrowDown' ? currentIndex + 1 : currentIndex - 1;
        if (nextIndex < 0 || nextIndex >= flatList.length) return;

        const rangeStart = Math.min(anchorIndex, nextIndex);
        const rangeEnd = Math.max(anchorIndex, nextIndex);
        const rangeAssetIds = flatList.slice(rangeStart, rangeEnd + 1).map(asset => asset.id);

        selectAll(rangeAssetIds, 'manual');
        
        // Move focus to next asset
        const nextAsset = flatList[nextIndex];
        onAssetSelect?.(nextAsset);
        setLastFocusedAsset(nextAsset.id);
        break;
      }

      default:
        break;
    }
  }, [
    enabled,
    getFlatAssetList,
    selectedAsset,
    keyboardState,
    selectAll,
    selectNone,
    getSelectedCount,
    isSelected,
    setKeyboardNavigationMode,
    setLastFocusedAsset,
    setSelectionAnchor,
    onAssetSelect,
    onRenameStart,
    onDeleteStart,
    navigateAsset,
    findAssetById
  ]);

  // Handle mouse events to disable keyboard navigation mode
  const handleMouseEvent = useCallback(() => {
    if (keyboardState.keyboard_navigation_mode) {
      setKeyboardNavigationMode(false);
    }
  }, [keyboardState.keyboard_navigation_mode, setKeyboardNavigationMode]);

  // Set up event listeners
  useEffect(() => {
    if (!enabled) return;

    const container = containerRef.current;
    if (!container) return;

    // Make container focusable
    if (!container.hasAttribute('tabIndex')) {
      container.setAttribute('tabIndex', '-1');
    }

    // Add event listeners
    document.addEventListener('keydown', handleKeyDown);
    container.addEventListener('mousedown', handleMouseEvent);
    container.addEventListener('click', handleMouseEvent);

    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      container.removeEventListener('mousedown', handleMouseEvent);
      container.removeEventListener('click', handleMouseEvent);
    };
  }, [enabled, handleKeyDown, handleMouseEvent]);

  // Focus container when keyboard navigation is enabled
  useEffect(() => {
    if (keyboardState.keyboard_navigation_mode && containerRef.current) {
      containerRef.current.focus();
    }
  }, [keyboardState.keyboard_navigation_mode]);

  return (
    <div
      ref={containerRef}
      style={{
        outline: keyboardState.keyboard_navigation_mode ? '2px solid #1890ff' : 'none',
        outlineOffset: '2px',
        borderRadius: '4px',
        transition: 'outline 0.2s ease-out'
      }}
      onFocus={() => setKeyboardNavigationMode(true)}
      onBlur={() => setKeyboardNavigationMode(false)}
    >
      {children}
    </div>
  );
};

export default KeyboardShortcuts;