import React, { useState, useEffect } from 'react';
import { Progress, Typography, Spin, Space, Tag } from 'antd';
import {
  LoadingOutlined,
  CheckCircleOutlined,
  ExclamationCircleOutlined,
  CloseCircleOutlined
} from '@ant-design/icons';
import { BulkOperationProgress, BulkOperationStatus } from '../../types/bulkOperations';

const { Text } = Typography;

interface ProgressIndicatorProps {
  progress?: BulkOperationProgress;
  status?: BulkOperationStatus;
  className?: string;
  style?: React.CSSProperties;
  compact?: boolean;
  showDetails?: boolean;
}

export const ProgressIndicator: React.FC<ProgressIndicatorProps> = ({
  progress,
  status = 'pending',
  className,
  style,
  compact = false,
  showDetails = true
}) => {
  const [animationClass, setAnimationClass] = useState('');
  const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  // Animate status changes
  useEffect(() => {
    if (prefersReducedMotion) return;

    setAnimationClass('progress-indicator-update');
    const timer = setTimeout(() => setAnimationClass(''), 300);
    return () => clearTimeout(timer);
  }, [status, progress?.processed_items, prefersReducedMotion]);

  // Get status color and icon
  const getStatusConfig = (currentStatus: BulkOperationStatus) => {
    switch (currentStatus) {
      case 'pending':
        return {
          color: '#faad14',
          icon: <LoadingOutlined spin={!prefersReducedMotion} />,
          text: 'Pending'
        };
      case 'validating':
        return {
          color: '#1890ff',
          icon: <LoadingOutlined spin={!prefersReducedMotion} />,
          text: 'Validating'
        };
      case 'processing':
        return {
          color: '#1890ff',
          icon: <LoadingOutlined spin={!prefersReducedMotion} />,
          text: 'Processing'
        };
      case 'completed':
        return {
          color: '#52c41a',
          icon: <CheckCircleOutlined />,
          text: 'Completed'
        };
      case 'failed':
        return {
          color: '#ff4d4f',
          icon: <CloseCircleOutlined />,
          text: 'Failed'
        };
      case 'cancelled':
        return {
          color: '#faad14',
          icon: <ExclamationCircleOutlined />,
          text: 'Cancelled'
        };
      default:
        return {
          color: '#d9d9d9',
          icon: null,
          text: 'Unknown'
        };
    }
  };

  const statusConfig = getStatusConfig(status);
  const progressPercent = progress ? (progress.processed_items / progress.total_items) * 100 : 0;

  if (compact) {
    return (
      <div 
        className={`progress-indicator-compact ${animationClass} ${className || ''}`}
        style={style}
      >
        <Space size="small" align="center">
          <div style={{ color: statusConfig.color, fontSize: '14px' }}>
            {statusConfig.icon}
          </div>
          {progress && (
            <Text style={{ fontSize: '12px', color: '#666' }}>
              {progress.processed_items}/{progress.total_items}
            </Text>
          )}
          <Tag color={statusConfig.color} size="small">
            {statusConfig.text}
          </Tag>
        </Space>
      </div>
    );
  }

  return (
    <div 
      className={`progress-indicator ${animationClass} ${className || ''}`}
      style={style}
    >
      <Space direction="vertical" size="small" style={{ width: '100%' }}>
        {/* Status Header */}
        <Space justify="space-between" style={{ width: '100%' }}>
          <Space>
            <div style={{ color: statusConfig.color, fontSize: '16px' }}>
              {statusConfig.icon}
            </div>
            <Text strong>{statusConfig.text}</Text>
          </Space>
          {progress && (
            <Text style={{ fontSize: '12px', color: '#666' }}>
              {Math.round(progressPercent)}%
            </Text>
          )}
        </Space>

        {/* Progress Bar */}
        {progress && ['validating', 'processing'].includes(status) && (
          <Progress
            percent={progressPercent}
            status={status === 'failed' ? 'exception' : 'active'}
            strokeColor={statusConfig.color}
            trailColor="#f0f0f0"
            size="small"
            showInfo={false}
            strokeLinecap="round"
          />
        )}

        {/* Details */}
        {showDetails && progress && (
          <Space direction="vertical" size={2} style={{ width: '100%' }}>
            <Space justify="space-between" style={{ width: '100%' }}>
              <Text style={{ fontSize: '12px', color: '#666' }}>
                Processed: {progress.processed_items} / {progress.total_items}
              </Text>
              {progress.failed_items > 0 && (
                <Text style={{ fontSize: '12px', color: '#ff4d4f' }}>
                  Failed: {progress.failed_items}
                </Text>
              )}
            </Space>

            {progress.current_item && (
              <Text style={{ fontSize: '12px', color: '#666' }} ellipsis>
                Current: {progress.current_item}
              </Text>
            )}

            {progress.estimated_completion && (
              <Text style={{ fontSize: '12px', color: '#666' }}>
                ETA: {new Date(progress.estimated_completion).toLocaleTimeString()}
              </Text>
            )}

            {progress.processing_rate > 0 && (
              <Text style={{ fontSize: '12px', color: '#666' }}>
                Rate: {Math.round(progress.processing_rate)} items/sec
              </Text>
            )}
          </Space>
        )}

        {/* Error Summary */}
        {progress && progress.errors.length > 0 && status !== 'processing' && (
          <div style={{ marginTop: '8px' }}>
            <Text style={{ fontSize: '12px', color: '#ff4d4f' }}>
              {progress.errors.length} error{progress.errors.length !== 1 ? 's' : ''} occurred
            </Text>
          </div>
        )}
      </Space>

      <style jsx>{`
        .progress-indicator {
          padding: 12px;
          border: 1px solid #f0f0f0;
          border-radius: 6px;
          background: #fafafa;
          transition: ${prefersReducedMotion ? 'none' : 'all 0.3s ease-out'};
        }

        .progress-indicator-compact {
          padding: 4px 8px;
          border-radius: 4px;
          background: transparent;
          transition: ${prefersReducedMotion ? 'none' : 'all 0.2s ease-out'};
        }

        .progress-indicator-update {
          transform: ${prefersReducedMotion ? 'none' : 'scale(1.02)'};
          box-shadow: ${prefersReducedMotion ? 'none' : '0 2px 8px rgba(24, 144, 255, 0.2)'};
        }

        @media (prefers-reduced-motion: reduce) {
          .progress-indicator,
          .progress-indicator-compact {
            transition: none;
          }
          
          .progress-indicator-update {
            transform: none;
            box-shadow: none;
          }
        }
      `}</style>
    </div>
  );
};

// Skeleton component for loading states
export const ProgressIndicatorSkeleton: React.FC<{
  className?: string;
  style?: React.CSSProperties;
  compact?: boolean;
}> = ({ className, style, compact = false }) => {
  const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  if (compact) {
    return (
      <div className={className} style={style}>
        <Space size="small" align="center">
          <Spin size="small" />
          <div style={{ 
            width: '40px', 
            height: '14px', 
            background: '#f0f0f0',
            borderRadius: '2px',
            animation: prefersReducedMotion ? 'none' : 'skeleton-pulse 1.5s ease-in-out infinite'
          }} />
        </Space>
      </div>
    );
  }

  return (
    <div 
      className={className}
      style={{
        padding: '12px',
        border: '1px solid #f0f0f0',
        borderRadius: '6px',
        background: '#fafafa',
        ...style
      }}
    >
      <Space direction="vertical" size="small" style={{ width: '100%' }}>
        <Space justify="space-between" style={{ width: '100%' }}>
          <Space>
            <Spin size="small" />
            <div style={{ 
              width: '80px', 
              height: '16px', 
              background: '#f0f0f0',
              borderRadius: '2px',
              animation: prefersReducedMotion ? 'none' : 'skeleton-pulse 1.5s ease-in-out infinite'
            }} />
          </Space>
          <div style={{ 
            width: '30px', 
            height: '14px', 
            background: '#f0f0f0',
            borderRadius: '2px',
            animation: prefersReducedMotion ? 'none' : 'skeleton-pulse 1.5s ease-in-out infinite'
          }} />
        </Space>
        
        <div style={{ 
          width: '100%', 
          height: '6px', 
          background: '#f0f0f0',
          borderRadius: '3px',
          animation: prefersReducedMotion ? 'none' : 'skeleton-pulse 1.5s ease-in-out infinite'
        }} />
        
        <div style={{ 
          width: '60%', 
          height: '12px', 
          background: '#f0f0f0',
          borderRadius: '2px',
          animation: prefersReducedMotion ? 'none' : 'skeleton-pulse 1.5s ease-in-out infinite'
        }} />
      </Space>

      <style jsx>{`
        @keyframes skeleton-pulse {
          0% { opacity: 1; }
          50% { opacity: 0.5; }
          100% { opacity: 1; }
        }

        @media (prefers-reduced-motion: reduce) {
          @keyframes skeleton-pulse {
            0%, 100% { opacity: 1; }
          }
        }
      `}</style>
    </div>
  );
};

export default ProgressIndicator;