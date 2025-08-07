import React, { useState, useCallback, useRef, useEffect } from 'react';
import { message, Typography } from 'antd';
import { AssetHierarchy } from '../../types/assets';
import useBulkOperationsStore from '../../store/bulkOperations';

const { Text } = Typography;

export interface DragPreviewData {
  assetIds: number[];
  assetNames: string[];
  dragStartPosition: { x: number; y: number };
}

export interface DropZoneValidation {
  isValid: boolean;
  canDrop: boolean;
  message?: string;
  dropEffect: 'move' | 'copy' | 'none';
}

interface AdvancedDragDropProps {
  children: React.ReactNode;
  assets: AssetHierarchy[];
  onMultiAssetMove?: (assetIds: number[], targetId: number | null) => Promise<void>;
  onDragStart?: (dragData: DragPreviewData) => void;
  onDragEnd?: () => void;
  className?: string;
  style?: React.CSSProperties;
}

export const AdvancedDragDrop: React.FC<AdvancedDragDropProps> = ({
  children,
  assets,
  onMultiAssetMove,
  onDragStart,
  onDragEnd,
  className,
  style
}) => {
  const [isDragging, setIsDragging] = useState(false);
  const [dragPreview, setDragPreview] = useState<DragPreviewData | null>(null);
  const [dropZone, setDropZone] = useState<{ targetId: number | null; validation: DropZoneValidation } | null>(null);
  const dragPreviewRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const {
    getSelectedAssets,
    getSelectedCount,
    isSelected,
    validateBulkMove
  } = useBulkOperationsStore();

  // Custom drag preview component
  const DragPreview: React.FC<{ data: DragPreviewData }> = ({ data }) => (
    <div
      ref={dragPreviewRef}
      style={{
        position: 'fixed',
        top: data.dragStartPosition.y - 10,
        left: data.dragStartPosition.x + 20,
        backgroundColor: '#1890ff',
        color: 'white',
        padding: '4px 8px',
        borderRadius: '4px',
        fontSize: '12px',
        fontWeight: 'bold',
        boxShadow: '0 2px 8px rgba(0,0,0,0.3)',
        zIndex: 10000,
        pointerEvents: 'none',
        opacity: 0.9,
        transform: 'translateZ(0)', // Force hardware acceleration
        transition: 'opacity 0.2s ease-out'
      }}
    >
      {data.assetIds.length === 1 ? (
        <span>{data.assetNames[0]}</span>
      ) : (
        <span>{data.assetIds.length} assets</span>
      )}
    </div>
  );

  // Update drag preview position
  const updateDragPreviewPosition = useCallback((event: DragEvent) => {
    if (dragPreviewRef.current) {
      dragPreviewRef.current.style.top = `${event.clientY - 10}px`;
      dragPreviewRef.current.style.left = `${event.clientX + 20}px`;
    }
  }, []);

  // Validate drop target
  const validateDropTarget = useCallback(async (targetId: number | null, assetIds: number[]): Promise<DropZoneValidation> => {
    if (assetIds.length === 0) {
      return { isValid: false, canDrop: false, dropEffect: 'none', message: 'No assets selected' };
    }

    // Find target asset
    const findAsset = (assets: AssetHierarchy[], id: number): AssetHierarchy | null => {
      for (const asset of assets) {
        if (asset.id === id) return asset;
        if (asset.children) {
          const found = findAsset(asset.children, id);
          if (found) return found;
        }
      }
      return null;
    };

    const targetAsset = targetId ? findAsset(assets, targetId) : null;

    // Validate target is a folder
    if (targetId && (!targetAsset || targetAsset.asset_type !== 'Folder')) {
      return { 
        isValid: false, 
        canDrop: false, 
        dropEffect: 'none', 
        message: 'Can only drop into folders' 
      };
    }

    // Check if any of the assets being dragged are ancestors of the target
    const isAncestor = (ancestorId: number, descendantId: number | null): boolean => {
      if (!descendantId) return false;
      
      const checkPath = (asset: AssetHierarchy): boolean => {
        if (asset.id === descendantId) return true;
        if (asset.id === ancestorId) return false; // Found ancestor first
        return asset.children?.some(checkPath) || false;
      };

      return assets.some(checkPath);
    };

    for (const assetId of assetIds) {
      if (isAncestor(assetId, targetId)) {
        return { 
          isValid: false, 
          canDrop: false, 
          dropEffect: 'none', 
          message: 'Cannot move asset into its own child' 
        };
      }
    }

    try {
      // Use backend validation
      const result = await validateBulkMove(assetIds, targetId);
      return {
        isValid: result.is_valid,
        canDrop: result.is_valid && result.errors.filter(e => e.blocking).length === 0,
        dropEffect: 'move',
        message: result.errors.length > 0 ? result.errors[0].message : undefined
      };
    } catch (error) {
      return { 
        isValid: false, 
        canDrop: false, 
        dropEffect: 'none', 
        message: 'Validation failed' 
      };
    }
  }, [assets, validateBulkMove]);

  // Handle drag start
  const handleDragStart = useCallback((event: DragEvent, assetId: number) => {
    const selectedAssets = getSelectedAssets();
    const dragAssetIds = isSelected(assetId) ? selectedAssets : [assetId];
    
    if (dragAssetIds.length === 0) {
      event.preventDefault();
      return;
    }

    // Get asset names for preview
    const getAssetName = (id: number): string => {
      const findAsset = (assets: AssetHierarchy[]): string | null => {
        for (const asset of assets) {
          if (asset.id === id) return asset.name;
          if (asset.children) {
            const found = findAsset(asset.children);
            if (found) return found;
          }
        }
        return null;
      };
      return findAsset(assets) || `Asset ${id}`;
    };

    const dragData: DragPreviewData = {
      assetIds: dragAssetIds,
      assetNames: dragAssetIds.map(getAssetName),
      dragStartPosition: { x: event.clientX, y: event.clientY }
    };

    setIsDragging(true);
    setDragPreview(dragData);
    
    // Set drag data
    event.dataTransfer?.setData('application/json', JSON.stringify({
      type: 'ferrocodex-assets',
      assetIds: dragAssetIds
    }));

    // Hide default drag image
    const emptyImg = new Image();
    emptyImg.src = 'data:image/gif;base64,R0lGODlhAQABAIAAAAUEBAAAACwAAAAAAQABAAACAkQBADs=';
    event.dataTransfer?.setDragImage(emptyImg, 0, 0);

    onDragStart?.(dragData);
  }, [getSelectedAssets, isSelected, assets, onDragStart]);

  // Handle drag over
  const handleDragOver = useCallback(async (event: DragEvent, targetId: number | null) => {
    event.preventDefault();
    
    if (!dragPreview) return;

    const validation = await validateDropTarget(targetId, dragPreview.assetIds);
    setDropZone({ targetId, validation });

    // Set drop effect
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = validation.dropEffect;
    }
  }, [dragPreview, validateDropTarget]);

  // Handle drop
  const handleDrop = useCallback(async (event: DragEvent, targetId: number | null) => {
    event.preventDefault();
    
    if (!dropZone || !dropZone.validation.canDrop || !dragPreview) {
      message.error(dropZone?.validation.message || 'Invalid drop target');
      return;
    }

    try {
      await onMultiAssetMove?.(dragPreview.assetIds, targetId);
      
      const assetCount = dragPreview.assetIds.length;
      const assetText = assetCount === 1 ? 'asset' : 'assets';
      const targetText = targetId ? 
        assets.find(a => a.id === targetId)?.name || 'folder' : 
        'root';
      
      message.success(`Moved ${assetCount} ${assetText} to ${targetText}`);
    } catch (error) {
      message.error('Failed to move assets');
      console.error('Drop error:', error);
    }
  }, [dropZone, dragPreview, onMultiAssetMove, assets]);

  // Handle drag end
  const handleDragEnd = useCallback(() => {
    setIsDragging(false);
    setDragPreview(null);
    setDropZone(null);
    onDragEnd?.();
  }, [onDragEnd]);

  // Add global drag event listeners
  useEffect(() => {
    if (isDragging) {
      document.addEventListener('dragover', updateDragPreviewPosition);
      document.addEventListener('dragend', handleDragEnd);
    }

    return () => {
      document.removeEventListener('dragover', updateDragPreviewPosition);
      document.removeEventListener('dragend', handleDragEnd);
    };
  }, [isDragging, updateDragPreviewPosition, handleDragEnd]);

  // Add reduced motion support
  const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  // Get drop zone styling
  const getDropZoneStyle = (targetId: number | null) => {
    if (!dropZone || dropZone.targetId !== targetId) return {};
    
    const { validation } = dropZone;
    const baseStyle = {
      transition: prefersReducedMotion ? 'none' : 'all 0.2s ease-out',
      transform: prefersReducedMotion ? 'none' : 'scale(1.02)',
    };

    if (validation.canDrop) {
      return {
        ...baseStyle,
        backgroundColor: 'rgba(82, 196, 26, 0.1)',
        border: '2px dashed #52c41a',
        borderRadius: '4px'
      };
    } else {
      return {
        ...baseStyle,
        backgroundColor: 'rgba(255, 77, 79, 0.1)',
        border: '2px dashed #ff4d4f',
        borderRadius: '4px'
      };
    }
  };

  return (
    <div
      ref={containerRef}
      className={className}
      style={style}
      onDragStart={(e) => {
        const target = e.target as HTMLElement;
        const assetId = target.getAttribute('data-asset-id');
        if (assetId) {
          handleDragStart(e.nativeEvent, parseInt(assetId, 10));
        }
      }}
      onDragOver={(e) => {
        const target = e.target as HTMLElement;
        const assetId = target.getAttribute('data-drop-target-id');
        handleDragOver(e.nativeEvent, assetId ? parseInt(assetId, 10) : null);
      }}
      onDrop={(e) => {
        const target = e.target as HTMLElement;
        const assetId = target.getAttribute('data-drop-target-id');
        handleDrop(e.nativeEvent, assetId ? parseInt(assetId, 10) : null);
      }}
    >
      {children}
      
      {/* Custom drag preview */}
      {isDragging && dragPreview && (
        <DragPreview data={dragPreview} />
      )}
      
      {/* Drop zone indicator */}
      {dropZone && dropZone.validation.message && (
        <div
          style={{
            position: 'fixed',
            bottom: '20px',
            left: '50%',
            transform: 'translateX(-50%)',
            backgroundColor: dropZone.validation.canDrop ? '#52c41a' : '#ff4d4f',
            color: 'white',
            padding: '8px 16px',
            borderRadius: '4px',
            fontSize: '12px',
            fontWeight: 'bold',
            zIndex: 9999,
            pointerEvents: 'none',
            opacity: 0.9
          }}
        >
          {dropZone.validation.message}
        </div>
      )}
    </div>
  );
};

export default AdvancedDragDrop;