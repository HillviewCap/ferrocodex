import React, { useState, useEffect } from 'react';
import {
  Modal,
  Timeline,
  Typography,
  Space,
  Alert,
  Spin,
  Empty,
  Card,
  Tooltip
} from 'antd';
import {
  UserOutlined,
  CalendarOutlined,
  CommentOutlined,
  ArrowRightOutlined
} from '@ant-design/icons';
import { StatusChangeRecord, ConfigurationVersionInfo } from '../types/assets';
import ConfigurationStatusBadge from './ConfigurationStatusBadge';
import { invoke } from '@tauri-apps/api/core';

const { Text, Title } = Typography;

interface StatusHistoryModalProps {
  visible: boolean;
  onCancel: () => void;
  version: ConfigurationVersionInfo | null;
  token: string;
}

const StatusHistoryModal: React.FC<StatusHistoryModalProps> = ({
  visible,
  onCancel,
  version,
  token
}) => {
  const [loading, setLoading] = useState(false);
  const [history, setHistory] = useState<StatusChangeRecord[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (visible && version) {
      loadStatusHistory();
    }
  }, [visible, version]);

  const loadStatusHistory = async () => {
    if (!version) return;

    setLoading(true);
    setError(null);

    try {
      const historyData = await invoke<StatusChangeRecord[]>('get_configuration_status_history', {
        token,
        versionId: version.id
      });

      setHistory(historyData);
    } catch (err) {
      setError(err as string);
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

  const formatRelativeTime = (dateString: string) => {
    try {
      const date = new Date(dateString);
      const now = new Date();
      const diffInSeconds = Math.floor((now.getTime() - date.getTime()) / 1000);
      
      if (diffInSeconds < 60) return 'just now';
      if (diffInSeconds < 3600) return `${Math.floor(diffInSeconds / 60)} minutes ago`;
      if (diffInSeconds < 86400) return `${Math.floor(diffInSeconds / 3600)} hours ago`;
      if (diffInSeconds < 2592000) return `${Math.floor(diffInSeconds / 86400)} days ago`;
      if (diffInSeconds < 31536000) return `${Math.floor(diffInSeconds / 2592000)} months ago`;
      return `${Math.floor(diffInSeconds / 31536000)} years ago`;
    } catch {
      return dateString;
    }
  };

  const getTimelineColor = (newStatus: string) => {
    switch (newStatus) {
      case 'Draft':
        return 'gray';
      case 'Approved':
        return 'green';
      case 'Golden':
        return 'gold';
      case 'Archived':
        return 'orange';
      default:
        return 'blue';
    }
  };

  if (!version) return null;

  return (
    <Modal
      title="Status Change History"
      open={visible}
      onCancel={onCancel}
      footer={null}
      width={700}
      destroyOnHidden
    >
      <Space direction="vertical" size="large" style={{ width: '100%' }}>
        <div>
          <Title level={5} style={{ margin: 0, marginBottom: '8px' }}>
            Configuration Version
          </Title>
          <Space>
            <Text strong>{version.file_name}</Text>
            <Text type="secondary">({version.version_number})</Text>
            <ConfigurationStatusBadge status={version.status} />
          </Space>
        </div>

        {error && (
          <Alert
            message="Error loading status history"
            description={error}
            type="error"
            showIcon
            closable
            onClose={() => setError(null)}
          />
        )}

        {loading ? (
          <div style={{ textAlign: 'center', padding: '40px' }}>
            <Spin size="large" />
            <div style={{ marginTop: '16px' }}>
              <Text type="secondary">Loading status history...</Text>
            </div>
          </div>
        ) : history.length === 0 ? (
          <Empty
            description="No status changes found"
            style={{ padding: '40px' }}
          />
        ) : (
          <Card style={{ maxHeight: '400px', overflowY: 'auto' }}>
            <Timeline>
              {history.map((record, index) => (
                <Timeline.Item
                  key={record.id}
                  color={getTimelineColor(record.new_status)}
                  style={{ paddingBottom: index === history.length - 1 ? '0' : '24px' }}
                >
                  <div style={{ marginTop: '-6px' }}>
                    <Space direction="vertical" size="small" style={{ width: '100%' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        {record.old_status && (
                          <>
                            <ConfigurationStatusBadge 
                              status={record.old_status as any} 
                              size="small" 
                            />
                            <ArrowRightOutlined style={{ color: '#8c8c8c' }} />
                          </>
                        )}
                        <ConfigurationStatusBadge 
                          status={record.new_status as any} 
                          size="small" 
                        />
                        {!record.old_status && (
                          <Text type="secondary" style={{ fontSize: '12px' }}>
                            (Initial status)
                          </Text>
                        )}
                      </div>

                      <Space size={16} wrap>
                        <Space size={4}>
                          <UserOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                          <Text type="secondary" style={{ fontSize: '12px' }}>
                            {record.changed_by_username}
                          </Text>
                        </Space>
                        <Space size={4}>
                          <CalendarOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                          <Tooltip title={formatDate(record.created_at)}>
                            <Text type="secondary" style={{ fontSize: '12px' }}>
                              {formatRelativeTime(record.created_at)}
                            </Text>
                          </Tooltip>
                        </Space>
                      </Space>

                      {record.change_reason && (
                        <div style={{ 
                          display: 'flex', 
                          alignItems: 'flex-start', 
                          gap: '8px',
                          marginTop: '8px',
                          padding: '8px 12px',
                          backgroundColor: '#fafafa',
                          borderRadius: '4px',
                          border: '1px solid #f0f0f0'
                        }}>
                          <CommentOutlined style={{ 
                            fontSize: '12px', 
                            color: '#8c8c8c', 
                            marginTop: '2px' 
                          }} />
                          <Text 
                            type="secondary" 
                            style={{ 
                              fontSize: '12px', 
                              lineHeight: '1.4',
                              fontStyle: 'italic',
                              flex: 1
                            }}
                          >
                            {record.change_reason}
                          </Text>
                        </div>
                      )}
                    </Space>
                  </div>
                </Timeline.Item>
              ))}
            </Timeline>
          </Card>
        )}
      </Space>
    </Modal>
  );
};

export default StatusHistoryModal;