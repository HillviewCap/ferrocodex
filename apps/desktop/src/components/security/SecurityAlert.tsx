import React, { useState } from 'react';
import { Alert, Badge, Button, Space, Typography, List, Popover, Card, Divider } from 'antd';
import {
  ExclamationCircleOutlined,
  WarningOutlined,
  InfoCircleOutlined,
  CheckCircleOutlined,
  BellOutlined,
  CloseOutlined
} from '@ant-design/icons';
import { SecurityAlert as SecurityAlertType } from '../../types/security';
import { useSecurityAlerts } from '../../store/security';
import useSecurityStore from '../../store/security';

const { Text } = Typography;

interface SecurityAlertProps {
  alert: SecurityAlertType;
  showActions?: boolean;
  compact?: boolean;
  onAcknowledge?: (alertId: string) => void;
  onResolve?: (alertId: string) => void;
}

const SecurityAlert: React.FC<SecurityAlertProps> = ({
  alert,
  showActions = true,
  compact = false,
  onAcknowledge,
  onResolve
}) => {
  const acknowledgeAlert = useSecurityStore(state => state.acknowledgeAlert);

  const getSeverityConfig = (severity: SecurityAlertType['severity']) => {
    switch (severity) {
      case 'critical':
        return {
          type: 'error' as const,
          icon: <ExclamationCircleOutlined />,
          color: '#ff4d4f',
          text: 'Critical'
        };
      case 'warning':
        return {
          type: 'warning' as const,
          icon: <WarningOutlined />,
          color: '#faad14',
          text: 'Warning'
        };
      case 'info':
        return {
          type: 'info' as const,
          icon: <InfoCircleOutlined />,
          color: '#1890ff',
          text: 'Info'
        };
    }
  };

  const config = getSeverityConfig(alert.severity);

  const handleAcknowledge = () => {
    acknowledgeAlert(alert.id);
    onAcknowledge?.(alert.id);
  };

  const formatTimestamp = (timestamp: string) => {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  };

  if (compact) {
    return (
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          padding: '8px 12px',
          backgroundColor: alert.acknowledged ? '#f9f9f9' : '#fff',
          border: `1px solid ${config.color}`,
          borderRadius: '4px',
          marginBottom: '4px',
          opacity: alert.acknowledged ? 0.7 : 1
        }}
      >
        <Space size="small" style={{ flex: 1 }}>
          <Badge status={config.type} />
          <Text strong style={{ fontSize: '12px' }}>
            {alert.title}
          </Text>
          <Text type="secondary" style={{ fontSize: '11px' }}>
            {formatTimestamp(alert.timestamp)}
          </Text>
        </Space>
        
        {showActions && !alert.acknowledged && (
          <Button
            type="text"
            size="small"
            icon={<CheckCircleOutlined />}
            onClick={handleAcknowledge}
            style={{ padding: '0 4px' }}
          />
        )}
      </div>
    );
  }

  return (
    <Alert
      message={
        <Space>
          <Badge status={config.type} text={config.text} />
          <Text strong>{alert.title}</Text>
          {alert.assetName && (
            <Text type="secondary" style={{ fontSize: '12px' }}>
              Asset: {alert.assetName}
            </Text>
          )}
        </Space>
      }
      description={
        <div>
          <Text style={{ fontSize: '13px' }}>{alert.description}</Text>
          
          <div style={{ marginTop: 8, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <Space size="small">
              <Text type="secondary" style={{ fontSize: '11px' }}>
                {formatTimestamp(alert.timestamp)}
              </Text>
              {alert.userName && (
                <Text type="secondary" style={{ fontSize: '11px' }}>
                  by {alert.userName}
                </Text>
              )}
            </Space>
            
            {showActions && (
              <Space size="small">
                {!alert.acknowledged && (
                  <Button
                    type="link"
                    size="small"
                    icon={<CheckCircleOutlined />}
                    onClick={handleAcknowledge}
                  >
                    Acknowledge
                  </Button>
                )}
                {alert.acknowledged && !alert.resolvedAt && onResolve && (
                  <Button
                    type="link"
                    size="small"
                    onClick={() => onResolve(alert.id)}
                  >
                    Resolve
                  </Button>
                )}
              </Space>
            )}
          </div>
        </div>
      }
      type={config.type}
      showIcon
      style={{
        marginBottom: 12,
        opacity: alert.acknowledged ? 0.8 : 1
      }}
    />
  );
};

// Security Alert Panel component for header/sidebar
export const SecurityAlertPanel: React.FC = () => {
  const { alerts, unreadCount, criticalAlerts, warningAlerts } = useSecurityAlerts();
  const [visible, setVisible] = useState(false);

  const content = (
    <Card
      title={
        <Space>
          <BellOutlined />
          <Text strong>Security Alerts</Text>
          {unreadCount > 0 && (
            <Badge count={unreadCount} />
          )}
        </Space>
      }
      style={{ width: 400, maxHeight: 500, overflow: 'auto' }}
      size="small"
      extra={
        <Button
          type="text"
          size="small"
          icon={<CloseOutlined />}
          onClick={() => setVisible(false)}
        />
      }
    >
      {alerts.length === 0 ? (
        <Text type="secondary">No security alerts</Text>
      ) : (
        <div>
          {/* Critical alerts first */}
          {criticalAlerts.length > 0 && (
            <>
              <Text strong style={{ color: '#ff4d4f', fontSize: '12px' }}>
                CRITICAL ({criticalAlerts.length})
              </Text>
              <List
                dataSource={criticalAlerts}
                renderItem={(alert) => (
                  <List.Item style={{ padding: '4px 0' }}>
                    <SecurityAlert alert={alert} compact />
                  </List.Item>
                )}
                style={{ marginBottom: 12 }}
              />
              <Divider style={{ margin: '8px 0' }} />
            </>
          )}

          {/* Warning alerts */}
          {warningAlerts.length > 0 && (
            <>
              <Text strong style={{ color: '#faad14', fontSize: '12px' }}>
                WARNINGS ({warningAlerts.length})
              </Text>
              <List
                dataSource={warningAlerts.slice(0, 5)} // Show max 5
                renderItem={(alert) => (
                  <List.Item style={{ padding: '4px 0' }}>
                    <SecurityAlert alert={alert} compact />
                  </List.Item>
                )}
                style={{ marginBottom: 12 }}
              />
            </>
          )}

          {/* Show "View All" if there are more alerts */}
          {alerts.length > 10 && (
            <div style={{ textAlign: 'center', marginTop: 8 }}>
              <Button type="link" size="small">
                View All Alerts ({alerts.length})
              </Button>
            </div>
          )}
        </div>
      )}
    </Card>
  );

  return (
    <Popover
      content={content}
      trigger="click"
      placement="bottomRight"
      visible={visible}
      onVisibleChange={setVisible}
    >
      <Button
        type="text"
        icon={<BellOutlined />}
        style={{ position: 'relative' }}
      >
        {unreadCount > 0 && (
          <Badge
            count={unreadCount}
            size="small"
            style={{
              position: 'absolute',
              top: -2,
              right: -2
            }}
          />
        )}
      </Button>
    </Popover>
  );
};

export default SecurityAlert;