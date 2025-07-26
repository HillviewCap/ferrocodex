import React, { useState, useEffect } from 'react';
import {
  Modal,
  Typography,
  Tag,
  Space,
  Empty,
  Spin,
  Alert,
  Timeline,
  Card
} from 'antd';
import {
  HistoryOutlined,
  SafetyOutlined,
  ClockCircleOutlined,
  CheckCircleOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { PasswordHistory } from '../types/vault';
import useAuthStore from '../store/auth';

const { Title, Text } = Typography;

interface PasswordHistoryProps {
  visible: boolean;
  onCancel: () => void;
  secretId: number;
  secretLabel: string;
}

const PasswordHistoryComponent: React.FC<PasswordHistoryProps> = ({
  visible,
  onCancel,
  secretId,
  secretLabel
}) => {
  const { token } = useAuthStore();
  
  const [history, setHistory] = useState<PasswordHistory[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (visible && token && secretId) {
      fetchPasswordHistory();
    }
  }, [visible, token, secretId]);

  const fetchPasswordHistory = async () => {
    if (!token) return;

    setLoading(true);
    try {
      const result = await invoke<PasswordHistory[]>('get_password_history', {
        token,
        secretId
      });
      setHistory(result);
    } catch (err) {
      console.error('Failed to fetch password history:', err);
      setHistory([]);
    } finally {
      setLoading(false);
    }
  };

  const formatDate = (dateString: string) => {
    try {
      return new Date(dateString).toLocaleDateString('en-US', {
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

  const getRelativeTime = (dateString: string) => {
    try {
      const date = new Date(dateString);
      const now = new Date();
      const diffInMs = now.getTime() - date.getTime();
      const diffInDays = Math.floor(diffInMs / (1000 * 60 * 60 * 24));
      
      if (diffInDays === 0) return 'Today';
      if (diffInDays === 1) return 'Yesterday';
      if (diffInDays < 7) return `${diffInDays} days ago`;
      if (diffInDays < 30) return `${Math.floor(diffInDays / 7)} weeks ago`;
      if (diffInDays < 365) return `${Math.floor(diffInDays / 30)} months ago`;
      return `${Math.floor(diffInDays / 365)} years ago`;
    } catch {
      return 'Unknown';
    }
  };

  const isCurrentPassword = (item: PasswordHistory) => {
    return !item.retired_at;
  };

  const timelineItems = history.map((item, index) => ({
    color: isCurrentPassword(item) ? '#52c41a' : '#d9d9d9',
    dot: isCurrentPassword(item) ? 
      <CheckCircleOutlined style={{ fontSize: '16px', color: '#52c41a' }} /> :
      <ClockCircleOutlined style={{ fontSize: '16px', color: '#d9d9d9' }} />,
    children: (
      <Card 
        size="small" 
        style={{ 
          marginBottom: '8px',
          border: isCurrentPassword(item) ? '1px solid #52c41a' : '1px solid #f0f0f0'
        }}
      >
        <Space direction="vertical" size="small" style={{ width: '100%' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <Space>
              <Text strong>
                {isCurrentPassword(item) ? 'Current Password' : `Password #${history.length - index}`}
              </Text>
              {isCurrentPassword(item) && (
                <Tag color="green" icon={<CheckCircleOutlined />}>
                  Active
                </Tag>
              )}
            </Space>
            <Text type="secondary" style={{ fontSize: '12px' }}>
              {getRelativeTime(item.created_at)}
            </Text>
          </div>
          
          <div style={{ display: 'flex', justifyContent: 'space-between' }}>
            <div>
              <Text type="secondary" style={{ fontSize: '12px' }}>
                Created: {formatDate(item.created_at)}
              </Text>
            </div>
            {item.retired_at && (
              <div>
                <Text type="secondary" style={{ fontSize: '12px' }}>
                  Retired: {formatDate(item.retired_at)}
                </Text>
              </div>
            )}
          </div>

          <div style={{ fontSize: '11px', color: '#8c8c8c', fontFamily: 'monospace' }}>
            Hash: {item.password_hash.substring(0, 24)}...
          </div>
        </Space>
      </Card>
    )
  }));

  return (
    <Modal
      title={
        <Space>
          <HistoryOutlined />
          Password History - {secretLabel}
        </Space>
      }
      open={visible}
      onCancel={onCancel}
      width={700}
      footer={null}
    >
      <Space direction="vertical" style={{ width: '100%' }} size="large">
        {/* Security Notice */}
        <Alert
          message="Password History Security"
          description="Password hashes are stored using bcrypt for reuse detection. Actual passwords are never stored in plain text and cannot be recovered from these hashes."
          type="info"
          showIcon
          icon={<SafetyOutlined />}
        />

        {/* Loading State */}
        {loading && (
          <div style={{ textAlign: 'center', padding: '40px' }}>
            <Spin size="large" />
            <div style={{ marginTop: '16px' }}>
              <Text type="secondary">Loading password history...</Text>
            </div>
          </div>
        )}

        {/* Empty State */}
        {!loading && history.length === 0 && (
          <Empty
            image={<HistoryOutlined style={{ fontSize: '64px', color: '#d9d9d9' }} />}
            description={
              <div>
                <Title level={4} style={{ color: '#8c8c8c' }}>No Password History</Title>
                <Text type="secondary">
                  No password changes have been recorded for this credential yet.
                </Text>
              </div>
            }
          />
        )}

        {/* History Timeline */}
        {!loading && history.length > 0 && (
          <div>
            <div style={{ marginBottom: '16px', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <Title level={5} style={{ margin: 0 }}>
                Password Change History ({history.length} entries)
              </Title>
              <Text type="secondary" style={{ fontSize: '12px' }}>
                Showing last 5 password changes
              </Text>
            </div>

            <Timeline
              mode="left"
              items={timelineItems}
            />

            {/* Summary Stats */}
            <Card title="Security Summary" size="small" style={{ marginTop: '16px' }}>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '16px', textAlign: 'center' }}>
                <div>
                  <div style={{ fontSize: '24px', fontWeight: 'bold', color: '#1890ff' }}>
                    {history.length}
                  </div>
                  <Text type="secondary" style={{ fontSize: '12px' }}>
                    Total Changes
                  </Text>
                </div>
                
                <div>
                  <div style={{ fontSize: '24px', fontWeight: 'bold', color: '#52c41a' }}>
                    {history.filter(h => !h.retired_at).length}
                  </div>
                  <Text type="secondary" style={{ fontSize: '12px' }}>
                    Active Password
                  </Text>
                </div>
                
                <div>
                  <div style={{ fontSize: '24px', fontWeight: 'bold', color: '#faad14' }}>
                    {history.length > 0 ? getRelativeTime(history[0].created_at) : 'Never'}
                  </div>
                  <Text type="secondary" style={{ fontSize: '12px' }}>
                    Last Changed
                  </Text>
                </div>
              </div>
            </Card>
          </div>
        )}
      </Space>
    </Modal>
  );
};

export default PasswordHistoryComponent;