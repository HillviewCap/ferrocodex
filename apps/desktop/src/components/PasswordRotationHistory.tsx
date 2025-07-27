import React, { useState, useEffect } from 'react';
import { Modal, Timeline, Empty, Spin, Typography, Tag, Space, Descriptions } from 'antd';
import { HistoryOutlined, UserOutlined, CalendarOutlined, FileTextOutlined } from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { PasswordRotationHistory } from '../types/vault';
import useAuthStore, { UserInfo } from '../store/auth';

const { Text } = Typography;

interface PasswordRotationHistoryViewerProps {
  secretId: number;
  secretLabel: string;
  visible: boolean;
  onClose: () => void;
}

const PasswordRotationHistoryViewer: React.FC<PasswordRotationHistoryViewerProps> = ({
  secretId,
  secretLabel,
  visible,
  onClose,
}) => {
  const [loading, setLoading] = useState(false);
  const [history, setHistory] = useState<PasswordRotationHistory[]>([]);
  const [users, setUsers] = useState<Map<number, UserInfo>>(new Map());
  
  const { token } = useAuthStore();

  useEffect(() => {
    if (visible && secretId) {
      loadRotationHistory();
    }
  }, [visible, secretId]);

  const loadRotationHistory = async () => {
    if (!token) return;

    try {
      setLoading(true);
      const rotationHistory = await invoke<PasswordRotationHistory[]>('get_rotation_history', {
        token,
        secret_id: secretId,
      });

      setHistory(rotationHistory);

      // Load user information for each rotation
      const userIds = [...new Set(rotationHistory.map(h => h.rotated_by))];
      const userMap = new Map<number, UserInfo>();

      for (const userId of userIds) {
        try {
          const userInfo = await invoke<UserInfo>('get_user_info', {
            token,
            user_id: userId,
          });
          userMap.set(userId, userInfo);
        } catch (error) {
          console.error(`Failed to load user info for ${userId}:`, error);
        }
      }

      setUsers(userMap);
    } catch (error) {
      console.error('Failed to load rotation history:', error);
    } finally {
      setLoading(false);
    }
  };

  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    return date.toLocaleString();
  };

  const getTimelineColor = (index: number) => {
    // Most recent is green, older entries fade to gray
    if (index === 0) return 'green';
    if (index === 1) return 'blue';
    return 'gray';
  };

  const getUserName = (userId: number) => {
    const user = users.get(userId);
    return user ? user.username : `User ${userId}`;
  };

  return (
    <Modal
      title={
        <Space>
          <HistoryOutlined />
          Password Rotation History - {secretLabel}
        </Space>
      }
      open={visible}
      onCancel={onClose}
      width={700}
      footer={null}
    >
      <Spin spinning={loading}>
        {history.length === 0 ? (
          <Empty
            description="No rotation history found"
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          />
        ) : (
          <Timeline mode="left" style={{ marginTop: 24 }}>
            {history.map((item, index) => (
              <Timeline.Item
                key={item.rotation_id}
                color={getTimelineColor(index)}
                dot={<HistoryOutlined style={{ fontSize: '16px' }} />}
              >
                <div style={{ marginBottom: 16 }}>
                  <Space direction="vertical" style={{ width: '100%' }}>
                    <Space>
                      <CalendarOutlined />
                      <Text strong>{formatDate(item.rotated_at)}</Text>
                      {index === 0 && <Tag color="green">Latest</Tag>}
                    </Space>
                    
                    <Descriptions size="small" column={1}>
                      <Descriptions.Item 
                        label={
                          <Space>
                            <UserOutlined />
                            Rotated By
                          </Space>
                        }
                      >
                        {getUserName(item.rotated_by)}
                      </Descriptions.Item>
                      
                      <Descriptions.Item 
                        label={
                          <Space>
                            <FileTextOutlined />
                            Reason
                          </Space>
                        }
                      >
                        <Text>{item.rotation_reason}</Text>
                      </Descriptions.Item>
                      
                      {item.batch_id && (
                        <Descriptions.Item label="Batch ID">
                          <Tag>Batch #{item.batch_id}</Tag>
                        </Descriptions.Item>
                      )}
                    </Descriptions>
                  </Space>
                </div>
              </Timeline.Item>
            ))}
          </Timeline>
        )}
      </Spin>
    </Modal>
  );
};

export default PasswordRotationHistoryViewer;