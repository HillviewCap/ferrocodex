import React, { useEffect, useState } from 'react';
import { Timeline, Typography, Tag, Space, Empty, Spin, message, Button } from 'antd';
import { UserOutlined, ClockCircleOutlined, FileTextOutlined } from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { FirmwareStatusHistory, FirmwareStatus } from '../../types/firmware';
import useAuthStore from '../../store/auth';

const { Text, Paragraph } = Typography;

interface FirmwareHistoryTimelineProps {
  firmwareVersionId: number;
  onRefresh?: () => void;
}

const FirmwareHistoryTimeline: React.FC<FirmwareHistoryTimelineProps> = ({ 
  firmwareVersionId,
  onRefresh 
}) => {
  const { token } = useAuthStore();
  const [history, setHistory] = useState<FirmwareStatusHistory[]>([]);
  const [loading, setLoading] = useState(false);
  const [visibleItems, setVisibleItems] = useState(10);

  useEffect(() => {
    if (token && firmwareVersionId) {
      fetchHistory();
    }
  }, [token, firmwareVersionId]);

  const fetchHistory = async () => {
    if (!token) return;

    setLoading(true);
    try {
      const result = await invoke<FirmwareStatusHistory[]>('get_firmware_status_history', {
        token,
        firmwareId: firmwareVersionId
      });
      setHistory(result);
    } catch (error) {
      message.error(`Failed to load firmware history: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const getStatusColor = (status: string): string => {
    switch (status) {
      case 'Golden':
        return 'gold';
      case 'Approved':
        return 'green';
      case 'Draft':
        return 'default';
      case 'Archived':
        return 'default';
      default:
        return 'default';
    }
  };

  const getStatusIcon = (status: string): string => {
    switch (status) {
      case 'Golden':
        return '👑';
      case 'Approved':
        return '✅';
      case 'Draft':
        return '📝';
      case 'Archived':
        return '📦';
      default:
        return '';
    }
  };

  const formatDate = (dateString: string) => {
    try {
      const date = new Date(dateString);
      const now = new Date();
      const diffInMs = now.getTime() - date.getTime();
      const diffInDays = Math.floor(diffInMs / (1000 * 60 * 60 * 24));

      if (diffInDays === 0) {
        const diffInHours = Math.floor(diffInMs / (1000 * 60 * 60));
        if (diffInHours === 0) {
          const diffInMins = Math.floor(diffInMs / (1000 * 60));
          if (diffInMins === 0) {
            return 'Just now';
          }
          return `${diffInMins} minute${diffInMins > 1 ? 's' : ''} ago`;
        }
        return `${diffInHours} hour${diffInHours > 1 ? 's' : ''} ago`;
      } else if (diffInDays === 1) {
        return 'Yesterday';
      } else if (diffInDays < 7) {
        return `${diffInDays} days ago`;
      }

      return date.toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
      });
    } catch {
      return dateString;
    }
  };

  const renderTimelineItem = (item: FirmwareStatusHistory, index: number) => {
    const isLatest = index === 0;
    const color = isLatest ? getStatusColor(item.new_status) : 'gray';

    return (
      <Timeline.Item 
        key={item.id}
        color={color}
        dot={isLatest ? <Tag color={getStatusColor(item.new_status)}>{getStatusIcon(item.new_status)}</Tag> : undefined}
      >
        <Space direction="vertical" size={4} style={{ width: '100%' }}>
          <Space>
            <Text strong>
              {item.old_status} → {item.new_status}
            </Text>
            {isLatest && <Tag color="blue">Current</Tag>}
          </Space>

          <Space size={16}>
            <Space size={4}>
              <UserOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
              <Text type="secondary" style={{ fontSize: '12px' }}>
                {item.changed_by_username || `User ${item.changed_by}`}
              </Text>
            </Space>
            <Space size={4}>
              <ClockCircleOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
              <Text type="secondary" style={{ fontSize: '12px' }}>
                {formatDate(item.changed_at)}
              </Text>
            </Space>
          </Space>

          {item.reason && (
            <div style={{ marginTop: '8px', paddingLeft: '20px' }}>
              <Space size={4} align="start">
                <FileTextOutlined style={{ fontSize: '12px', color: '#8c8c8c', marginTop: '2px' }} />
                <Paragraph 
                  type="secondary" 
                  style={{ fontSize: '12px', margin: 0, fontStyle: 'italic' }}
                  ellipsis={{ rows: 2, expandable: true }}
                >
                  {item.reason}
                </Paragraph>
              </Space>
            </div>
          )}
        </Space>
      </Timeline.Item>
    );
  };

  if (loading) {
    return (
      <div style={{ textAlign: 'center', padding: '32px' }}>
        <Spin />
        <div style={{ marginTop: '16px' }}>
          <Text type="secondary">Loading firmware history...</Text>
        </div>
      </div>
    );
  }

  if (history.length === 0) {
    return (
      <Empty
        image={Empty.PRESENTED_IMAGE_SIMPLE}
        description={
          <Text type="secondary">
            No status history available for this firmware version
          </Text>
        }
      />
    );
  }

  const displayedHistory = history.slice(0, visibleItems);
  const hasMore = history.length > visibleItems;

  return (
    <div>
      <Timeline mode="left">
        {displayedHistory.map((item, index) => renderTimelineItem(item, index))}
      </Timeline>

      {hasMore && (
        <div style={{ textAlign: 'center', marginTop: '16px' }}>
          <Button 
            type="link" 
            onClick={() => setVisibleItems(prev => prev + 10)}
          >
            Show {Math.min(10, history.length - visibleItems)} more items
          </Button>
        </div>
      )}

      <div style={{ textAlign: 'center', marginTop: '16px' }}>
        <Text type="secondary" style={{ fontSize: '12px' }}>
          Showing {displayedHistory.length} of {history.length} history items
        </Text>
      </div>
    </div>
  );
};

export default FirmwareHistoryTimeline;