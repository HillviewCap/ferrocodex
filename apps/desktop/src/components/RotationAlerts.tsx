import React, { useState, useEffect } from 'react';
import { Card, List, Tag, Space, Typography, Button, Empty, Badge, Alert, Progress, Statistic, Row, Col } from 'antd';
import { WarningOutlined, CalendarOutlined, LockOutlined, CloseCircleOutlined, ExclamationCircleOutlined } from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { RotationAlert, RotationComplianceMetrics } from '../types/vault';
import useAuthStore from '../store/auth';

const { Text, Title } = Typography;

interface RotationAlertsProps {
  daysAhead?: number;
  onRotateClick?: (alert: RotationAlert) => void;
}

const RotationAlerts: React.FC<RotationAlertsProps> = ({ daysAhead = 30, onRotateClick }) => {
  const [loading, setLoading] = useState(false);
  const [alerts, setAlerts] = useState<RotationAlert[]>([]);
  const [metrics, setMetrics] = useState<RotationComplianceMetrics | null>(null);
  
  const { token } = useAuthStore();

  useEffect(() => {
    loadAlerts();
    loadMetrics();
  }, [daysAhead]);

  const loadAlerts = async () => {
    if (!token) return;

    try {
      setLoading(true);
      const rotationAlerts = await invoke<RotationAlert[]>('get_rotation_alerts', {
        token,
        days_ahead: daysAhead,
      });
      setAlerts(rotationAlerts);
    } catch (error) {
      console.error('Failed to load rotation alerts:', error);
    } finally {
      setLoading(false);
    }
  };

  const loadMetrics = async () => {
    if (!token) return;

    try {
      const complianceMetrics = await invoke<RotationComplianceMetrics>('get_rotation_compliance_metrics', {
        token,
      });
      setMetrics(complianceMetrics);
    } catch (error) {
      console.error('Failed to load compliance metrics:', error);
    }
  };

  const getAlertIcon = (daysUntilRotation: number) => {
    if (daysUntilRotation < 0) return <CloseCircleOutlined />;
    if (daysUntilRotation <= 7) return <ExclamationCircleOutlined />;
    return <CalendarOutlined />;
  };

  const formatDaysUntil = (days: number) => {
    if (days < 0) return `${Math.abs(days)} days overdue`;
    if (days === 0) return 'Due today';
    if (days === 1) return '1 day';
    return `${days} days`;
  };

  const overdueAlerts = alerts.filter(a => a.days_until_rotation < 0);
  const urgentAlerts = alerts.filter(a => a.days_until_rotation >= 0 && a.days_until_rotation <= 7);
  const upcomingAlerts = alerts.filter(a => a.days_until_rotation > 7);

  return (
    <div>
      {metrics && (
        <Row gutter={16} style={{ marginBottom: 24 }}>
          <Col span={6}>
            <Card>
              <Statistic
                title="Total Passwords"
                value={metrics.total_passwords}
                prefix={<LockOutlined />}
              />
            </Card>
          </Col>
          <Col span={6}>
            <Card>
              <Statistic
                title="Overdue"
                value={metrics.overdue_passwords}
                valueStyle={{ color: '#cf1322' }}
                prefix={<WarningOutlined />}
              />
            </Card>
          </Col>
          <Col span={6}>
            <Card>
              <Statistic
                title="Due in 7 Days"
                value={metrics.due_within_7_days}
                valueStyle={{ color: '#faad14' }}
                prefix={<CalendarOutlined />}
              />
            </Card>
          </Col>
          <Col span={6}>
            <Card>
              <Statistic
                title="Compliance"
                value={metrics.compliance_percentage}
                precision={1}
                valueStyle={{ color: metrics.compliance_percentage >= 80 ? '#3f8600' : '#cf1322' }}
                suffix="%"
              />
              <Progress
                percent={metrics.compliance_percentage}
                status={metrics.compliance_percentage >= 80 ? 'success' : 'exception'}
                showInfo={false}
                strokeWidth={4}
                style={{ marginTop: 8 }}
              />
            </Card>
          </Col>
        </Row>
      )}

      <Card
        title={
          <Space>
            <WarningOutlined />
            Password Rotation Alerts
            <Badge count={alerts.length} />
          </Space>
        }
        loading={loading}
      >
        {alerts.length === 0 ? (
          <Empty
            description="No passwords require rotation"
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          />
        ) : (
          <Space direction="vertical" style={{ width: '100%' }} size="large">
            {overdueAlerts.length > 0 && (
              <div>
                <Alert
                  message={`${overdueAlerts.length} Overdue Passwords`}
                  description="These passwords are past their rotation date and should be updated immediately."
                  type="error"
                  showIcon
                  style={{ marginBottom: 16 }}
                />
                <List
                  dataSource={overdueAlerts}
                  renderItem={(alert) => (
                    <List.Item
                      actions={[
                        <Button
                          type="primary"
                          danger
                          size="small"
                          icon={<LockOutlined />}
                          onClick={() => onRotateClick?.(alert)}
                        >
                          Rotate Now
                        </Button>
                      ]}
                    >
                      <List.Item.Meta
                        avatar={getAlertIcon(alert.days_until_rotation)}
                        title={
                          <Space>
                            <Text strong>{alert.secret_label}</Text>
                            <Tag color="error">{formatDaysUntil(alert.days_until_rotation)}</Tag>
                          </Space>
                        }
                        description={
                          <Space direction="vertical" size="small">
                            <Text type="secondary">Asset: {alert.asset_name}</Text>
                            <Text type="secondary">
                              Last Rotated: {alert.last_rotated ? new Date(alert.last_rotated).toLocaleDateString() : 'Never'}
                            </Text>
                          </Space>
                        }
                      />
                    </List.Item>
                  )}
                />
              </div>
            )}

            {urgentAlerts.length > 0 && (
              <div>
                <Title level={5}>
                  <ExclamationCircleOutlined /> Urgent (Due within 7 days)
                </Title>
                <List
                  dataSource={urgentAlerts}
                  renderItem={(alert) => (
                    <List.Item
                      actions={[
                        <Button
                          type="primary"
                          size="small"
                          icon={<LockOutlined />}
                          onClick={() => onRotateClick?.(alert)}
                        >
                          Rotate
                        </Button>
                      ]}
                    >
                      <List.Item.Meta
                        avatar={getAlertIcon(alert.days_until_rotation)}
                        title={
                          <Space>
                            <Text>{alert.secret_label}</Text>
                            <Tag color="warning">{formatDaysUntil(alert.days_until_rotation)}</Tag>
                          </Space>
                        }
                        description={
                          <Space direction="vertical" size="small">
                            <Text type="secondary">Asset: {alert.asset_name}</Text>
                            <Text type="secondary">Due: {new Date(alert.next_rotation_due).toLocaleDateString()}</Text>
                          </Space>
                        }
                      />
                    </List.Item>
                  )}
                />
              </div>
            )}

            {upcomingAlerts.length > 0 && (
              <div>
                <Title level={5}>
                  <CalendarOutlined /> Upcoming
                </Title>
                <List
                  dataSource={upcomingAlerts}
                  renderItem={(alert) => (
                    <List.Item
                      actions={[
                        <Button
                          size="small"
                          onClick={() => onRotateClick?.(alert)}
                        >
                          Rotate Early
                        </Button>
                      ]}
                    >
                      <List.Item.Meta
                        avatar={getAlertIcon(alert.days_until_rotation)}
                        title={
                          <Space>
                            <Text>{alert.secret_label}</Text>
                            <Tag>{formatDaysUntil(alert.days_until_rotation)}</Tag>
                          </Space>
                        }
                        description={
                          <Text type="secondary">
                            Asset: {alert.asset_name} | Due: {new Date(alert.next_rotation_due).toLocaleDateString()}
                          </Text>
                        }
                      />
                    </List.Item>
                  )}
                />
              </div>
            )}
          </Space>
        )}
      </Card>
    </div>
  );
};

export default RotationAlerts;