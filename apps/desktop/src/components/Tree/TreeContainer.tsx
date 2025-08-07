import React, { forwardRef, useCallback, useEffect, useRef } from 'react';
import { Card, Breadcrumb, Spin } from 'antd';
import { HomeOutlined, FolderOutlined, ToolOutlined } from '@ant-design/icons';
import { AssetInfo } from '../../types/assets';

export interface TreeContainerProps {
  children: React.ReactNode;
  height?: number;
  loading?: boolean;
  breadcrumbPath?: AssetInfo[];
  onBreadcrumbClick?: (asset: AssetInfo) => void;
  onKeyDown?: (e: React.KeyboardEvent) => void;
  title?: string;
  extra?: React.ReactNode;
  className?: string;
  style?: React.CSSProperties;
}

const TreeContainer = forwardRef<HTMLDivElement, TreeContainerProps>(({
  children,
  height = 400,
  loading = false,
  breadcrumbPath = [],
  onBreadcrumbClick,
  onKeyDown,
  title = "Asset Tree",
  extra,
  className,
  style,
}, ref) => {
  const containerRef = useRef<HTMLDivElement>(null);

  // Handle keyboard focus management
  useEffect(() => {
    const container = containerRef.current;
    if (container) {
      container.tabIndex = 0; // Make container focusable
    }
  }, []);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    onKeyDown?.(e);
  }, [onKeyDown]);

  const handleBreadcrumbClick = useCallback((asset: AssetInfo) => {
    onBreadcrumbClick?.(asset);
  }, [onBreadcrumbClick]);

  const renderBreadcrumb = () => {
    if (breadcrumbPath.length === 0) return null;

    const items = [
      {
        key: 'root',
        title: (
          <span>
            <HomeOutlined style={{ marginRight: 4 }} />
            Root
          </span>
        ),
        onClick: () => {
          // Navigate to root
        }
      },
      ...breadcrumbPath.map((asset, index) => ({
        key: asset.id,
        title: (
          <span
            style={{ cursor: 'pointer' }}
            onClick={() => handleBreadcrumbClick(asset)}
          >
            {asset.asset_type === 'Folder' ? (
              <FolderOutlined style={{ marginRight: 4 }} />
            ) : (
              <ToolOutlined style={{ marginRight: 4 }} />
            )}
            {asset.name}
          </span>
        ),
      }))
    ];

    return (
      <div style={{ 
        padding: '8px 12px', 
        borderBottom: '1px solid #f0f0f0',
        background: '#fafafa'
      }}>
        <Breadcrumb items={items} />
      </div>
    );
  };

  return (
    <div
      ref={ref || containerRef}
      className={className}
      style={{
        height,
        display: 'flex',
        flexDirection: 'column',
        border: '1px solid #d9d9d9',
        borderRadius: 6,
        overflow: 'hidden',
        backgroundColor: '#fff',
        ...style,
      }}
      onKeyDown={handleKeyDown}
      tabIndex={0}
      role="tree"
      aria-label={title}
    >
      {/* Header */}
      {(title || extra) && (
        <div style={{
          padding: '12px 16px',
          borderBottom: '1px solid #f0f0f0',
          background: '#fafafa',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
        }}>
          {title && (
            <span style={{ fontWeight: 500, fontSize: 14 }}>
              {title}
            </span>
          )}
          {extra}
        </div>
      )}

      {/* Breadcrumb */}
      {renderBreadcrumb()}

      {/* Content */}
      <div style={{ 
        flex: 1, 
        display: 'flex', 
        flexDirection: 'column',
        overflow: 'hidden',
        position: 'relative'
      }}>
        <Spin spinning={loading} style={{ height: '100%' }}>
          {children}
        </Spin>
      </div>
    </div>
  );
});

TreeContainer.displayName = 'TreeContainer';

export { TreeContainer };