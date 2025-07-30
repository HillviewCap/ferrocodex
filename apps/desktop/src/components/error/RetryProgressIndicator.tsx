import React, { useState, useEffect } from 'react';
import { Progress, Card, Button, Typography, Space, Divider, Badge, Timeline, Tooltip } from 'antd';
import { ReloadOutlined, CloseCircleOutlined, CheckCircleOutlined, ExclamationCircleOutlined, PauseCircleOutlined } from '@ant-design/icons';

const { Text, Title } = Typography;

export interface RetryAttempt {
  attempt: number;
  error: {
    id: string;
    message: string;
    severity: 'Critical' | 'High' | 'Medium' | 'Low';
    domain: string;
    timestamp: string;
  };
  timestamp: string;
  delay: number; // milliseconds
  is_final: boolean;
}

export interface RetryProgress {
  execution_id: string;
  operation_name: string;
  current_attempt: number;
  max_attempts: number;
  attempts: RetryAttempt[];
  status: 'running' | 'success' | 'failed' | 'cancelled';
  total_duration: number; // milliseconds
  retry_limit_exceeded: boolean;
  next_retry_in?: number; // milliseconds until next retry
}

interface RetryProgressIndicatorProps {
  progress: RetryProgress;
  onCancel?: () => void;
  onRetry?: () => void;
  showDetails?: boolean;
  compact?: boolean;
}

const RetryProgressIndicator: React.FC<RetryProgressIndicatorProps> = ({
  progress,
  onCancel,
  onRetry,
  showDetails = false,
  compact = false,
}) => {
  const [countdown, setCountdown] = useState<number>(progress.next_retry_in || 0);

  useEffect(() => {
    if (progress.status === 'running' && progress.next_retry_in && progress.next_retry_in > 0) {
      setCountdown(progress.next_retry_in);
      const interval = setInterval(() => {
        setCountdown((prev) => {
          if (prev <= 100) {
            clearInterval(interval);
            return 0;
          }
          return prev - 100;
        });
      }, 100);

      return () => clearInterval(interval);
    }
  }, [progress.next_retry_in, progress.status]);

  const getStatusIcon = () => {
    switch (progress.status) {
      case 'running':
        return <ReloadOutlined spin style={{ color: '#1890ff' }} />;
      case 'success':
        return <CheckCircleOutlined style={{ color: '#52c41a' }} />;
      case 'failed':
        return <CloseCircleOutlined style={{ color: '#ff4d4f' }} />;
      case 'cancelled':
        return <PauseCircleOutlined style={{ color: '#faad14' }} />;
      default:
        return <ExclamationCircleOutlined style={{ color: '#faad14' }} />;
    }
  };

  const getStatusText = () => {
    switch (progress.status) {
      case 'running':
        return countdown > 0 
          ? `Retrying in ${(countdown / 1000).toFixed(1)}s...`
          : `Attempting retry ${progress.current_attempt} of ${progress.max_attempts}`;
      case 'success':
        return 'Operation completed successfully';
      case 'failed':
        return progress.retry_limit_exceeded ? 'Maximum retry attempts exceeded' : 'Operation failed';
      case 'cancelled':
        return 'Operation cancelled by user';
      default:
        return 'Unknown status';
    }
  };

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'Critical': return '#ff4d4f';
      case 'High': return '#ff7a45';
      case 'Medium': return '#faad14';
      case 'Low': return '#52c41a';
      default: return '#d9d9d9';
    }
  };

  const progressPercent = progress.max_attempts > 0 
    ? (progress.current_attempt / progress.max_attempts) * 100 
    : 0;

  if (compact) {
    return (
      <Space size="small" align="center">
        {getStatusIcon()}
        <Text type="secondary" style={{ fontSize: '12px' }}>
          {getStatusText()}
        </Text>
        {progress.status === 'running' && onCancel && (
          <Button 
            type="text" 
            size="small" 
            icon={<CloseCircleOutlined />}
            onClick={onCancel}
            style={{ padding: '0 4px' }}
          />
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
          {getStatusIcon()}
          <Text strong>{progress.operation_name}</Text>
          <Badge 
            count={`${progress.current_attempt}/${progress.max_attempts}`}
            style={{ backgroundColor: progress.status === 'running' ? '#1890ff' : '#d9d9d9' }}
          />
        </Space>
      }
      extra={
        progress.status === 'running' && onCancel && (
          <Button 
            type="text" 
            size="small" 
            icon={<CloseCircleOutlined />}
            onClick={onCancel}
            danger
          >
            Cancel
          </Button>
        )
      }
    >
      <Space direction="vertical" style={{ width: '100%' }} size="small">
        <Progress 
          percent={progressPercent}
          status={progress.status === 'failed' ? 'exception' : progress.status === 'success' ? 'success' : 'active'}
          showInfo={false}
          strokeColor={progress.status === 'running' ? '#1890ff' : undefined}
        />
        
        <Text type="secondary">{getStatusText()}</Text>
        
        {progress.total_duration > 0 && (
          <Text type="secondary" style={{ fontSize: '12px' }}>
            Total duration: {(progress.total_duration / 1000).toFixed(2)}s
          </Text>
        )}

        {showDetails && progress.attempts.length > 0 && (
          <>
            <Divider style={{ margin: '8px 0' }} />
            <Title level={5} style={{ margin: 0 }}>Retry History</Title>
            <Timeline style={{ marginTop: 8 }}>
              {progress.attempts.map((attempt) => (
                <Timeline.Item
                  key={attempt.attempt}
                  color={getSeverityColor(attempt.error.severity)}
                  dot={
                    <Tooltip title={`Severity: ${attempt.error.severity}`}>
                      <div 
                        style={{ 
                          width: 8, 
                          height: 8, 
                          borderRadius: '50%', 
                          backgroundColor: getSeverityColor(attempt.error.severity) 
                        }} 
                      />
                    </Tooltip>
                  }
                >
                  <Space direction="vertical" size={2}>
                    <Text strong style={{ fontSize: '12px' }}>
                      Attempt {attempt.attempt}
                      {attempt.is_final && <Badge count="Final" style={{ marginLeft: 4, fontSize: '10px' }} />}
                    </Text>
                    <Text style={{ fontSize: '11px', color: '#666' }}>
                      {attempt.error.message}
                    </Text>
                    <Text type="secondary" style={{ fontSize: '10px' }}>
                      {new Date(attempt.timestamp).toLocaleTimeString()}
                      {attempt.delay > 0 && ` • Delayed ${(attempt.delay / 1000).toFixed(1)}s`}
                    </Text>
                  </Space>
                </Timeline.Item>
              ))}
            </Timeline>
          </>
        )}

        {progress.status === 'failed' && !progress.retry_limit_exceeded && onRetry && (
          <Button 
            type="primary" 
            size="small" 
            icon={<ReloadOutlined />}
            onClick={onRetry}
            style={{ marginTop: 8 }}
          >
            Retry Operation
          </Button>
        )}
      </Space>
    </Card>
  );
};

export default RetryProgressIndicator;