import React, { useState, useEffect } from 'react';
import {
  Card,
  Progress,
  Space,
  Button,
  Statistic,
  Typography,
  Row,
  Col,
  Alert,
  Timeline,
  Tag,
  Tooltip,
  notification,
} from 'antd';
import {
  PlayCircleOutlined,
  PauseCircleOutlined,
  StopOutlined,
  ReloadOutlined,
  CheckCircleOutlined,
  ExclamationCircleOutlined,
  CloseCircleOutlined,
  ClockCircleOutlined,
} from '@ant-design/icons';
import { 
  ProgressStatus, 
  BulkImportStatus, 
  formatProcessingRate, 
  formatEstimatedTime, 
  calculateSuccessRate,
} from '../../types/bulk';
import useBulkImportStore from '../../store/bulk';

const { Title, Text } = Typography;

interface BulkProgressTrackerProps {
  sessionId: number;
  autoRefresh?: boolean;
  refreshInterval?: number;
  onStatusChange?: (status: BulkImportStatus) => void;
}

const BulkProgressTracker: React.FC<BulkProgressTrackerProps> = ({
  sessionId,
  autoRefresh = true,
  refreshInterval = 2000,
  onStatusChange,
}) => {
  const [isManualRefreshing, setIsManualRefreshing] = useState(false);
  
  const {
    currentProgress,
    currentSession,
    error,
    getProgress,
    pauseProcessing,
    resumeProcessing,
    cancelProcessing,
    clearError,
  } = useBulkImportStore();

  useEffect(() => {
    let interval: NodeJS.Timeout;
    
    if (autoRefresh && sessionId) {
      // Initial load
      refreshProgress();
      
      // Set up auto-refresh
      interval = setInterval(() => {
        refreshProgress();
      }, refreshInterval);
    }

    return () => {
      if (interval) {
        clearInterval(interval);
      }
    };
  }, [sessionId, autoRefresh, refreshInterval]);

  useEffect(() => {
    if (currentProgress && onStatusChange) {
      onStatusChange(currentProgress.status);
    }
  }, [currentProgress?.status, onStatusChange]);

  useEffect(() => {
    if (error) {
      notification.error({
        message: 'Progress Update Failed',
        description: error,
        duration: 4,
      });
    }
  }, [error]);

  const refreshProgress = async () => {
    try {
      await getProgress(sessionId);
    } catch (error) {
      // Error handled by store
    }
  };

  const handleManualRefresh = async () => {
    setIsManualRefreshing(true);
    try {
      await refreshProgress();
    } finally {
      setIsManualRefreshing(false);
    }
  };

  const handlePause = async () => {
    try {
      await pauseProcessing(sessionId);
      notification.success({
        message: 'Import Paused',
        description: 'Bulk import processing has been paused',
      });
    } catch (error) {
      // Error handled by store
    }
  };

  const handleResume = async () => {
    try {
      await resumeProcessing(sessionId);
      notification.success({
        message: 'Import Resumed',
        description: 'Bulk import processing has been resumed',
      });
    } catch (error) {
      // Error handled by store
    }
  };

  const handleCancel = async () => {
    try {
      await cancelProcessing(sessionId);
      notification.warning({
        message: 'Import Cancelled',
        description: 'Bulk import processing has been cancelled',
      });
    } catch (error) {
      // Error handled by store
    }
  };

  if (!currentProgress) {
    return (
      <Card>
        <Space direction="vertical" style={{ width: '100%' }} align="center">
          <Text type="secondary">No progress data available</Text>
          <Button onClick={handleManualRefresh} loading={isManualRefreshing}>
            Load Progress
          </Button>
        </Space>
      </Card>
    );
  }

  const progressPercentage = currentProgress.total_items > 0 
    ? Math.round((currentProgress.processed_items / currentProgress.total_items) * 100)
    : 0;

  const successRate = calculateSuccessRate(
    currentProgress.processed_items, 
    currentProgress.failed_items
  );

  const getStatusColor = (status: BulkImportStatus) => {
    switch (status) {
      case 'Created': return 'default';
      case 'Validating': return 'processing';
      case 'Processing': return 'processing';
      case 'Paused': return 'warning';
      case 'Completed': return 'success';
      case 'Failed': return 'error';
      case 'Cancelled': return 'default';
      default: return 'default';
    }
  };

  const getStatusIcon = (status: BulkImportStatus) => {
    switch (status) {
      case 'Processing': return <PlayCircleOutlined />;
      case 'Paused': return <PauseCircleOutlined />;
      case 'Completed': return <CheckCircleOutlined />;
      case 'Failed': return <ExclamationCircleOutlined />;
      case 'Cancelled': return <StopOutlined />;
      default: return <ClockCircleOutlined />;
    }
  };

  const canPause = currentProgress.status === 'Processing';
  const canResume = currentProgress.status === 'Paused';
  const canCancel = ['Created', 'Validating', 'Processing', 'Paused'].includes(currentProgress.status);

  return (
    <Card 
      title={
        <Space>
          <Title level={4} style={{ margin: 0 }}>
            Import Progress
          </Title>
          <Tag 
            color={getStatusColor(currentProgress.status)} 
            icon={getStatusIcon(currentProgress.status)}
          >
            {currentProgress.status}
          </Tag>
        </Space>
      }
      extra={
        <Space>
          <Tooltip title="Refresh Progress">
            <Button
              icon={<ReloadOutlined />}
              onClick={handleManualRefresh}
              loading={isManualRefreshing}
              size="small"
            />
          </Tooltip>
          {canPause && (
            <Button
              icon={<PauseCircleOutlined />}
              onClick={handlePause}
              size="small"
              type="default"
            >
              Pause
            </Button>
          )}
          {canResume && (
            <Button
              icon={<PlayCircleOutlined />}
              onClick={handleResume}
              size="small"
              type="primary"
            >
              Resume
            </Button>
          )}
          {canCancel && (
            <Button
              icon={<StopOutlined />}
              onClick={handleCancel}
              size="small"
              danger
            >
              Cancel
            </Button>
          )}
        </Space>
      }
    >
      <Space direction="vertical" style={{ width: '100%' }} size="large">
        {/* Main Progress Bar */}
        <div>
          <Progress
            percent={progressPercentage}
            status={
              currentProgress.status === 'Failed' ? 'exception' :
              currentProgress.status === 'Completed' ? 'success' :
              currentProgress.status === 'Processing' ? 'active' : 'normal'
            }
            strokeWidth={12}
            format={(percent) => (
              <span style={{ fontSize: '16px', fontWeight: 'bold' }}>
                {percent}%
              </span>
            )}
          />
          <Text type="secondary" style={{ fontSize: '12px' }}>
            {currentProgress.processed_items} of {currentProgress.total_items} items processed
          </Text>
        </div>

        {/* Statistics */}
        <Row gutter={[16, 16]}>
          <Col xs={12} sm={6}>
            <Statistic
              title="Total Items"
              value={currentProgress.total_items}
              prefix={<CheckCircleOutlined style={{ color: '#1890ff' }} />}
            />
          </Col>
          <Col xs={12} sm={6}>
            <Statistic
              title="Processed"
              value={currentProgress.processed_items}
              prefix={<CheckCircleOutlined style={{ color: '#52c41a' }} />}
            />
          </Col>
          <Col xs={12} sm={6}>
            <Statistic
              title="Failed"
              value={currentProgress.failed_items}
              prefix={<ExclamationCircleOutlined style={{ color: '#ff4d4f' }} />}
            />
          </Col>
          <Col xs={12} sm={6}>
            <Statistic
              title="Success Rate"
              value={successRate}
              suffix="%"
              prefix={<CheckCircleOutlined style={{ color: '#52c41a' }} />}
            />
          </Col>
        </Row>

        {/* Processing Details */}
        {currentProgress.status === 'Processing' && (
          <Row gutter={[16, 16]}>
            <Col xs={24} sm={12}>
              <Card size="small">
                <Statistic
                  title="Processing Rate"
                  value={formatProcessingRate(currentProgress.processing_rate)}
                  prefix={<ClockCircleOutlined />}
                />
              </Card>
            </Col>
            {currentProgress.estimated_completion && (
              <Col xs={24} sm={12}>
                <Card size="small">
                  <Statistic
                    title="Estimated Completion"
                    value={formatEstimatedTime(parseInt(currentProgress.estimated_completion))}
                    prefix={<ClockCircleOutlined />}
                  />
                </Card>
              </Col>
            )}
          </Row>
        )}

        {/* Current Item */}
        {currentProgress.current_item && (
          <Alert
            message="Currently Processing"
            description={currentProgress.current_item}
            type="info"
            showIcon
            icon={<ClockCircleOutlined />}
          />
        )}

        {/* Error Summary */}
        {currentProgress.failed_items > 0 && (
          <Alert
            message={`${currentProgress.failed_items} items failed to process`}
            description="Review the error details to understand what went wrong"
            type="warning"
            showIcon
            action={
              <Button size="small" onClick={() => {
                // TODO: Show error details modal
                notification.info({
                  message: 'Error Details',
                  description: 'Error details view will be implemented in the next phase',
                });
              }}>
                View Errors
              </Button>
            }
          />
        )}

        {/* Completion Status */}
        {currentProgress.status === 'Completed' && (
          <Alert
            message="Import Completed Successfully"
            description={`Successfully processed ${currentProgress.processed_items - currentProgress.failed_items} items${
              currentProgress.failed_items > 0 ? ` with ${currentProgress.failed_items} failures` : ''
            }`}
            type="success"
            showIcon
          />
        )}

        {currentProgress.status === 'Failed' && (
          <Alert
            message="Import Failed"
            description="The bulk import operation encountered critical errors and could not continue"
            type="error"
            showIcon
          />
        )}

        {currentProgress.status === 'Cancelled' && (
          <Alert
            message="Import Cancelled"
            description="The bulk import operation was cancelled by user request"
            type="warning"
            showIcon
          />
        )}

        {/* Timeline for completed operations */}
        {['Completed', 'Failed', 'Cancelled'].includes(currentProgress.status) && currentSession && (
          <Card size="small" title="Import Timeline">
            <Timeline size="small">
              <Timeline.Item 
                color="blue" 
                dot={<CheckCircleOutlined />}
              >
                Session Created - {new Date(currentSession.session.created_at).toLocaleString()}
              </Timeline.Item>
              {currentSession.session.updated_at !== currentSession.session.created_at && (
                <Timeline.Item 
                  color="orange"
                  dot={<ClockCircleOutlined />}
                >
                  Processing Started - {new Date(currentSession.session.updated_at).toLocaleString()}
                </Timeline.Item>
              )}
              {currentSession.session.completed_at && (
                <Timeline.Item 
                  color={currentProgress.status === 'Completed' ? 'green' : 'red'}
                  dot={currentProgress.status === 'Completed' ? <CheckCircleOutlined /> : <CloseCircleOutlined />}
                >
                  {currentProgress.status === 'Completed' ? 'Completed' : 'Failed'} - {new Date(currentSession.session.completed_at).toLocaleString()}
                </Timeline.Item>
              )}
            </Timeline>
          </Card>
        )}
      </Space>
    </Card>
  );
};

export default BulkProgressTracker;