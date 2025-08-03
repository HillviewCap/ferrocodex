import React, { useState, useEffect } from 'react';
import { 
  Card, 
  Row, 
  Col, 
  Statistic, 
  Progress, 
  Badge, 
  List, 
  Space, 
  Typography, 
  Alert,
  Button,
  Select,
  DatePicker,
  Spin
} from 'antd';
import {
  ShieldCheckOutlined,
  SafetyCertificateOutlined,
  ExclamationCircleOutlined,
  TrophyOutlined,
  BugOutlined,
  EyeOutlined,
  ReloadOutlined,
  DownloadOutlined
} from '@ant-design/icons';
import { 
  SecurityMetrics, 
  SecurityHealthReport, 
  ValidationStats,
  SecurityClassificationLevel,
  SecurityAlert as SecurityAlertType
} from '../../types/security';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../../store/auth';
import useSecurityStore, { useSecurityAlerts, useSecurityHealth } from '../../store/security';
import ComplianceStatusIndicator from './ComplianceStatusIndicator';
import { SecurityAlertPanel } from './SecurityAlert';

const { Title, Text } = Typography;
const { RangePicker } = DatePicker;

interface SecurityMonitoringDashboardProps {
  refreshInterval?: number; // ms
  showDetailedMetrics?: boolean;
  compactView?: boolean;
}

const SecurityMonitoringDashboard: React.FC<SecurityMonitoringDashboardProps> = ({
  refreshInterval = 30000, // 30 seconds
  showDetailedMetrics = true,
  compactView = false
}) => {
  const { token, user } = useAuthStore();
  const { 
    metrics, 
    healthReport, 
    validationStats,
    setMetrics, 
    setHealthReport, 
    setValidationStats 
  } = useSecurityStore();
  
  const { alerts, unreadCount, criticalAlerts } = useSecurityAlerts();
  const { overallStatus } = useSecurityHealth();
  
  const [loading, setLoading] = useState(false);
  const [lastRefresh, setLastRefresh] = useState<Date>(new Date());
  const [selectedPeriod, setSelectedPeriod] = useState<'24h' | '7d' | '30d'>('24h');

  // Auto-refresh data
  useEffect(() => {
    const fetchData = async () => {
      if (user?.role !== 'Administrator') return; // Only admins can view detailed metrics
      
      try {
        setLoading(true);
        
        // Fetch all security data in parallel
        const [metricsResult, healthResult, statsResult] = await Promise.all([
          invoke<SecurityMetrics>('get_security_metrics', { token, period: selectedPeriod }),
          invoke<SecurityHealthReport>('perform_security_health_check', { token }),
          invoke<ValidationStats>('get_validation_statistics', { token })
        ]);

        setMetrics(metricsResult);
        setHealthReport(healthResult);
        setValidationStats(statsResult);
        setLastRefresh(new Date());
        
      } catch (error) {
        console.error('Failed to fetch security data:', error);
      } finally {
        setLoading(false);
      }
    };

    // Initial fetch
    fetchData();

    // Set up auto-refresh
    const interval = setInterval(fetchData, refreshInterval);
    return () => clearInterval(interval);
  }, [token, user, selectedPeriod, refreshInterval, setMetrics, setHealthReport, setValidationStats]);

  const handleRefresh = () => {
    // Trigger immediate refresh by updating lastRefresh
    setLastRefresh(new Date());
  };

  const handleExportReport = async () => {
    try {
      await invoke('export_security_report', { 
        token, 
        startDate: new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString(),
        endDate: new Date().toISOString()
      });
    } catch (error) {
      console.error('Failed to export report:', error);
    }
  };

  // Get overall system health color
  const getHealthColor = () => {
    switch (overallStatus) {
      case 'excellent': return '#52c41a';
      case 'good': return '#1890ff';
      case 'warning': return '#faad14';
      case 'critical': return '#ff4d4f';
      default: return '#d9d9d9';
    }
  };

  if (user?.role !== 'Administrator') {
    return (
      <Alert
        message="Administrator Access Required"
        description="Security monitoring dashboard is only available to administrators."
        type="warning"
        showIcon
      />
    );
  }

  if (compactView) {
    return (
      <Card size="small">
        <Row gutter={16}>
          <Col span={8}>
            <Statistic
              title="Security Status"
              value={overallStatus}
              valueStyle={{ color: getHealthColor(), fontSize: '16px' }}
              prefix={<ShieldCheckOutlined />}
            />
          </Col>
          <Col span={8}>
            <Statistic
              title="Active Alerts"
              value={unreadCount}
              valueStyle={{ color: unreadCount > 0 ? '#ff4d4f' : '#52c41a' }}
              prefix={<ExclamationCircleOutlined />}
            />
          </Col>
          <Col span={8}>
            <Statistic
              title="Success Rate"
              value={healthReport?.validationSuccessRate || 0}
              suffix="%"
              precision={1}
              valueStyle={{ color: (healthReport?.validationSuccessRate || 0) > 90 ? '#52c41a' : '#faad14' }}
              prefix={<TrophyOutlined />}
            />
          </Col>
        </Row>
      </Card>
    );
  }

  return (
    <div>
      {/* Header */}
      <Row justify="space-between" align="middle" style={{ marginBottom: 24 }}>
        <Col>
          <Title level={3}>
            <Space>
              <SafetyCertificateOutlined />
              Security Monitoring Dashboard
            </Space>
          </Title>
        </Col>
        <Col>
          <Space>
            <Select
              value={selectedPeriod}
              onChange={setSelectedPeriod}
              style={{ width: 120 }}
            >
              <Select.Option value="24h">Last 24h</Select.Option>
              <Select.Option value="7d">Last 7 days</Select.Option>
              <Select.Option value="30d">Last 30 days</Select.Option>
            </Select>
            <Button 
              icon={<ReloadOutlined />} 
              onClick={handleRefresh}
              loading={loading}
            >
              Refresh
            </Button>
            <Button 
              icon={<DownloadOutlined />} 
              onClick={handleExportReport}
            >
              Export Report
            </Button>
            <SecurityAlertPanel />
          </Space>
        </Col>
      </Row>

      {/* Critical Alerts Banner */}
      {criticalAlerts.length > 0 && (
        <Alert
          message={`${criticalAlerts.length} Critical Security Alert${criticalAlerts.length > 1 ? 's' : ''}`}
          description="Immediate attention required. Click the alerts button to review."
          type="error"
          showIcon
          style={{ marginBottom: 24 }}
          action={
            <Button size="small" danger>
              View Alerts
            </Button>
          }
        />
      )}

      <Spin spinning={loading}>
        {/* Key Metrics Row */}
        <Row gutter={[16, 16]} style={{ marginBottom: 24 }}>
          <Col xs={24} sm={12} lg={6}>
            <Card>
              <Statistic
                title="Overall Security Status"
                value={overallStatus.toUpperCase()}
                valueStyle={{ color: getHealthColor() }}
                prefix={<ShieldCheckOutlined />}
              />
              <Progress
                percent={healthReport?.validationSuccessRate || 0}
                strokeColor={getHealthColor()}
                size="small"
                showInfo={false}
                style={{ marginTop: 8 }}
              />
            </Card>
          </Col>

          <Col xs={24} sm={12} lg={6}>
            <Card>
              <Statistic
                title="Total Assets"
                value={metrics?.totalAssets || 0}
                prefix={<EyeOutlined />}
              />
              <Text type="secondary" style={{ fontSize: '12px' }}>
                Under security management
              </Text>
            </Card>
          </Col>

          <Col xs={24} sm={12} lg={6}>
            <Card>
              <Statistic
                title="Validation Success Rate"
                value={healthReport?.validationSuccessRate || 0}
                suffix="%"
                precision={1}
                valueStyle={{ color: (healthReport?.validationSuccessRate || 0) > 90 ? '#52c41a' : '#faad14' }}
                prefix={<TrophyOutlined />}
              />
            </Card>
          </Col>

          <Col xs={24} sm={12} lg={6}>
            <Card>
              <Statistic
                title="Security Alerts"
                value={unreadCount}
                valueStyle={{ color: unreadCount > 0 ? '#ff4d4f' : '#52c41a' }}
                prefix={<ExclamationCircleOutlined />}
              />
              <Badge status={criticalAlerts.length > 0 ? 'error' : 'success'} 
                     text={`${criticalAlerts.length} critical`} 
                     style={{ fontSize: '12px' }} />
            </Card>
          </Col>
        </Row>

        {/* Detailed Metrics Row */}
        {showDetailedMetrics && (
          <Row gutter={[16, 16]} style={{ marginBottom: 24 }}>
            {/* Classification Breakdown */}
            <Col xs={24} lg={12}>
              <Card title="Security Classification Distribution" size="small">
                {metrics?.classificationBreakdown ? (
                  <div>
                    {Object.entries(metrics.classificationBreakdown).map(([level, count]) => (
                      <div key={level} style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 8 }}>
                        <Badge 
                          color={level === SecurityClassificationLevel.SECRET ? '#f5222d' : 
                                level === SecurityClassificationLevel.RESTRICTED ? '#ff7a45' :
                                level === SecurityClassificationLevel.CONFIDENTIAL ? '#faad14' :
                                level === SecurityClassificationLevel.INTERNAL ? '#1890ff' : '#52c41a'} 
                          text={level} 
                        />
                        <Text strong>{count}</Text>
                      </div>
                    ))}
                  </div>
                ) : (
                  <Text type="secondary">No classification data available</Text>
                )}
              </Card>
            </Col>

            {/* Validation Statistics */}
            <Col xs={24} lg={12}>
              <Card title="Validation Statistics" size="small">
                {validationStats ? (
                  <Space direction="vertical" style={{ width: '100%' }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                      <Text>Total Validations:</Text>
                      <Text strong>{validationStats.totalValidations}</Text>
                    </div>
                    <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                      <Text>Successful:</Text>
                      <Text strong style={{ color: '#52c41a' }}>{validationStats.successfulValidations}</Text>
                    </div>
                    <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                      <Text>Failed:</Text>
                      <Text strong style={{ color: '#ff4d4f' }}>{validationStats.failedValidations}</Text>
                    </div>
                    <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                      <Text>Blocked:</Text>
                      <Text strong style={{ color: '#faad14' }}>{validationStats.blockedAttempts}</Text>
                    </div>
                  </Space>
                ) : (
                  <Text type="secondary">No validation statistics available</Text>
                )}
              </Card>
            </Col>
        </Row>
        )}

        {/* Health Report and Recommendations */}
        <Row gutter={[16, 16]}>
          <Col xs={24} lg={16}>
            <Card title="Security Health Report" size="small">
              {healthReport ? (
                <Space direction="vertical" style={{ width: '100%' }} size="middle">
                  <ComplianceStatusIndicator
                    status={{
                      level: healthReport.systemSecurityLevel === 'High' ? 'excellent' :
                            healthReport.systemSecurityLevel === 'Medium' ? 'good' : 'needs-attention',
                      score: healthReport.validationSuccessRate,
                      issues: healthReport.validationSuccessRate < 90 ? ['Validation success rate below 90%'] : [],
                      recommendations: healthReport.recommendations
                    }}
                    showScore={true}
                    showProgress={true}
                    showDetails={true}
                  />

                  {healthReport.recommendations.length > 0 && (
                    <div>
                      <Title level={5}>Recommendations</Title>
                      <List
                        dataSource={healthReport.recommendations}
                        renderItem={(item) => (
                          <List.Item>
                            <Text>{item}</Text>
                          </List.Item>
                        )}
                        size="small"
                      />
                    </div>
                  )}
                </Space>
              ) : (
                <Text type="secondary">Health report not available</Text>
              )}
            </Card>
          </Col>

          <Col xs={24} lg={8}>
            <Card title="Recent Security Events" size="small">
              <List
                dataSource={alerts.slice(0, 5)}
                renderItem={(alert) => (
                  <List.Item>
                    <Space>
                      <Badge status={alert.severity === 'critical' ? 'error' : 
                                   alert.severity === 'warning' ? 'warning' : 'processing'} />
                      <div>
                        <Text strong style={{ fontSize: '12px' }}>{alert.title}</Text>
                        <br />
                        <Text type="secondary" style={{ fontSize: '11px' }}>
                          {new Date(alert.timestamp).toLocaleString()}
                        </Text>
                      </div>
                    </Space>
                  </List.Item>
                )}
                size="small"
              />
              {alerts.length === 0 && (
                <Text type="secondary">No recent security events</Text>
              )}
            </Card>
          </Col>
        </Row>
      </Spin>

      <div style={{ textAlign: 'right', marginTop: 16, fontSize: '12px', color: '#666' }}>
        Last updated: {lastRefresh.toLocaleString()}
      </div>
    </div>
  );
};

export default SecurityMonitoringDashboard;