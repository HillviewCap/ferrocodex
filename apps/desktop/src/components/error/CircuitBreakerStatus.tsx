import React from 'react';
import { Card, Progress, Space, Typography, Badge, Tooltip, Statistic, Row, Col } from 'antd';
import { ThunderboltOutlined, CheckCircleOutlined, CloseCircleOutlined, ExclamationCircleOutlined } from '@ant-design/icons';

const { Text } = Typography;

export interface CircuitBreakerMetrics {
  state: 'Closed' | 'Open' | 'Half-Open';
  total_calls: number;
  successful_calls: number;
  failed_calls: number;
  rejected_calls: number;
  failure_rate: number; // 0.0 to 1.0
  last_opened_at?: string;
  last_closed_at?: string;
  state_transitions: number;
}

interface CircuitBreakerStatusProps {
  serviceName: string;
  metrics: CircuitBreakerMetrics;
  compact?: boolean;
  showDetails?: boolean;
}

const CircuitBreakerStatus: React.FC<CircuitBreakerStatusProps> = ({
  serviceName,
  metrics,
  compact = false,
  showDetails = false,
}) => {
  const getStateIcon = () => {
    switch (metrics.state) {
      case 'Closed':
        return <CheckCircleOutlined style={{ color: '#52c41a' }} />;
      case 'Open':
        return <CloseCircleOutlined style={{ color: '#ff4d4f' }} />;
      case 'Half-Open':
        return <ExclamationCircleOutlined style={{ color: '#faad14' }} />;
      default:
        return <ThunderboltOutlined style={{ color: '#d9d9d9' }} />;
    }
  };

  const getStateColor = () => {
    switch (metrics.state) {
      case 'Closed': return '#52c41a';
      case 'Open': return '#ff4d4f';
      case 'Half-Open': return '#faad14';
      default: return '#d9d9d9';
    }
  };

  const getStateDescription = () => {
    switch (metrics.state) {
      case 'Closed':
        return 'Service is healthy - requests flow normally';
      case 'Open':
        return 'Service is failing - requests are blocked';
      case 'Half-Open':
        return 'Service recovery testing - limited requests allowed';
      default:
        return 'Unknown circuit state';
    }
  };

  const formatTimestamp = (timestamp?: string) => {
    if (!timestamp) return 'Never';
    return new Date(timestamp).toLocaleString();
  };

  const successRate = metrics.total_calls > 0 
    ? ((metrics.successful_calls / metrics.total_calls) * 100)
    : 100;

  const failurePercent = Math.round(metrics.failure_rate * 100);

  if (compact) {
    return (
      <Space size="small" align="center">
        {getStateIcon()}
        <Badge 
          color={getStateColor()}
          text={
            <Text style={{ fontSize: '12px' }}>
              {serviceName}: {metrics.state}
            </Text>
          }
        />
        {metrics.state === 'Open' && (
          <Text type="danger" style={{ fontSize: '11px' }}>
            {failurePercent}% failure rate
          </Text>
        )}
      </Space>
    );
  }

  return (
    <Card 
      size="small" 
      style={{ marginBottom: 16 }}
      title={
        <Space>
          <ThunderboltOutlined />
          <Text strong>{serviceName} Circuit Breaker</Text>
          <Badge 
            color={getStateColor()}
            text={metrics.state}
          />
        </Space>
      }
    >
      <Space direction="vertical" style={{ width: '100%' }} >
        {/* State Overview */}
        <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
          <Space>
            {getStateIcon()}
            <div>
              <Text strong style={{ color: getStateColor() }}>
                {metrics.state}
              </Text>
              <br />
              <Text type="secondary" style={{ fontSize: '12px' }}>
                {getStateDescription()}
              </Text>
            </div>
          </Space>
          
          {metrics.failure_rate > 0 && (
            <Tooltip title="Current failure rate">
              <Progress
                type="circle"
                size={60}
                percent={failurePercent}
                status={metrics.state === 'Open' ? 'exception' : 'normal'}
                format={(percent) => `${percent}%`}
                strokeColor={metrics.state === 'Open' ? '#ff4d4f' : '#faad14'}
              />
            </Tooltip>
          )}
        </Space>

        {showDetails && (
          <>
            {/* Statistics */}
            <Row gutter={16}>
              <Col span={6}>
                <Statistic
                  title="Total Calls"
                  value={metrics.total_calls}
                  valueStyle={{ fontSize: '16px' }}
                />
              </Col>
              <Col span={6}>
                <Statistic
                  title="Successful"
                  value={metrics.successful_calls}
                  valueStyle={{ fontSize: '16px', color: '#52c41a' }}
                />
              </Col>
              <Col span={6}>
                <Statistic
                  title="Failed"
                  value={metrics.failed_calls}
                  valueStyle={{ fontSize: '16px', color: '#ff4d4f' }}
                />
              </Col>
              <Col span={6}>
                <Statistic
                  title="Rejected"
                  value={metrics.rejected_calls}
                  valueStyle={{ fontSize: '16px', color: '#faad14' }}
                />
              </Col>
            </Row>

            {/* Success Rate Progress */}
            <div>
              <Text strong style={{ marginBottom: 8, display: 'block' }}>
                Success Rate
              </Text>
              <Progress
                percent={Math.round(successRate)}
                status={successRate > 90 ? 'success' : successRate > 70 ? 'normal' : 'exception'}
                strokeColor={successRate > 90 ? '#52c41a' : successRate > 70 ? '#1890ff' : '#ff4d4f'}
              />
            </div>

            {/* State History */}
            <Row gutter={16}>
              <Col span={12}>
                <Text strong style={{ fontSize: '12px' }}>Last Opened:</Text>
                <br />
                <Text type="secondary" style={{ fontSize: '11px' }}>
                  {formatTimestamp(metrics.last_opened_at)}
                </Text>
              </Col>
              <Col span={12}>
                <Text strong style={{ fontSize: '12px' }}>Last Closed:</Text>
                <br />
                <Text type="secondary" style={{ fontSize: '11px' }}>
                  {formatTimestamp(metrics.last_closed_at)}
                </Text>
              </Col>
            </Row>

            <Space>
              <Text type="secondary" style={{ fontSize: '11px' }}>
                State transitions: {metrics.state_transitions}
              </Text>
            </Space>
          </>
        )}
      </Space>
    </Card>
  );
};

export default CircuitBreakerStatus;