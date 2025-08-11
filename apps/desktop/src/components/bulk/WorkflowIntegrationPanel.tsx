import React, { useState, useEffect } from 'react';
import {
  Card,
  Row,
  Col,
  Button,
  Space,
  Typography,
  Badge,
  Progress,
  Alert,
  Statistic,
  Timeline,
  Tag,
  Tooltip,
  Switch,
  notification,
  Divider,
} from 'antd';
import {
  SyncOutlined,
  CheckCircleOutlined,
  ExclamationCircleOutlined,
  ClockCircleOutlined,
  BugOutlined,
  ApiOutlined,
  DatabaseOutlined,
  FolderOpenOutlined,
} from '@ant-design/icons';
import { 
 
  formatProcessingRate,
  formatEstimatedTime,
} from '../../types/bulk';
import useBulkImportStore from '../../store/bulk';
import { useWorkflowStore } from '../../store/workflow';

const { Text } = Typography;

interface WorkflowIntegrationPanelProps {
  sessionId?: number;
  showControls?: boolean;
  compact?: boolean;
}

interface WorkflowHealthStatus {
  bulkImport: 'healthy' | 'warning' | 'error';
  assetCreation: 'healthy' | 'warning' | 'error';
  database: 'healthy' | 'warning' | 'error';
  validation: 'healthy' | 'warning' | 'error';
}

const WorkflowIntegrationPanel: React.FC<WorkflowIntegrationPanelProps> = ({
  sessionId,
  showControls = true,
  compact = false,
}) => {
  const [autoSync, setAutoSync] = useState(true);
  const [lastSyncTime, setLastSyncTime] = useState<Date | null>(null);
  const [healthStatus, setHealthStatus] = useState<WorkflowHealthStatus>({
    bulkImport: 'healthy',
    assetCreation: 'healthy',
    database: 'healthy',
    validation: 'healthy',
  });

  const {
    currentProgress,
    isLoading: bulkLoading,
    error: bulkError,
    getProgress,
  } = useBulkImportStore();

  const {
    isLoading: workflowLoading,
    error: workflowError,
  } = useWorkflowStore();

  useEffect(() => {
    if (sessionId && autoSync) {
      const interval = setInterval(() => {
        syncWorkflowStatus();
      }, 5000); // Sync every 5 seconds

      // Initial sync
      syncWorkflowStatus();

      return () => clearInterval(interval);
    }
  }, [sessionId, autoSync]);

  useEffect(() => {
    // Update health status based on current state
    updateHealthStatus();
  }, [currentProgress, bulkError, workflowError]);

  const syncWorkflowStatus = async () => {
    if (!sessionId) return;

    try {
      await Promise.all([
        getProgress(sessionId),
        // getActiveWorkflows?.(), // Not implemented
      ]);
      setLastSyncTime(new Date());
    } catch (error) {
      console.error('Failed to sync workflow status:', error);
    }
  };

  const updateHealthStatus = () => {
    const newStatus: WorkflowHealthStatus = {
      bulkImport: 'healthy',
      assetCreation: 'healthy',
      database: 'healthy',
      validation: 'healthy',
    };

    // Check bulk import health
    if (bulkError) {
      newStatus.bulkImport = 'error';
    } else if (currentProgress?.status === 'Failed') {
      newStatus.bulkImport = 'error';
    } else if (currentProgress?.status === 'Paused') {
      newStatus.bulkImport = 'warning';
    }

    // Check workflow health
    if (workflowError) {
      newStatus.assetCreation = 'error';
    } else if (false) { // activeWorkflows not available
      newStatus.assetCreation = 'error';
    } else if (false) { // activeWorkflows not available
      newStatus.assetCreation = 'warning';
    }

    // Simulate database and validation health (in real implementation, these would be checked)
    newStatus.database = 'healthy';
    newStatus.validation = 'healthy';

    setHealthStatus(newStatus);
  };

  const getHealthColor = (status: 'healthy' | 'warning' | 'error') => {
    switch (status) {
      case 'healthy': return '#52c41a';
      case 'warning': return '#faad14';
      case 'error': return '#ff4d4f';
      default: return '#d9d9d9';
    }
  };

  const getHealthIcon = (status: 'healthy' | 'warning' | 'error') => {
    switch (status) {
      case 'healthy': return <CheckCircleOutlined />;
      case 'warning': return <ExclamationCircleOutlined />;
      case 'error': return <BugOutlined />;
      default: return <ClockCircleOutlined />;
    }
  };

  const renderWorkflowHealth = () => (
    <Card size="small" title="Workflow Health">
      <Row gutter={[8, 8]}>
        <Col span={12}>
          <Space>
            <Badge color={getHealthColor(healthStatus.bulkImport)} />
            <Text>Bulk Import</Text>
            {getHealthIcon(healthStatus.bulkImport)}
          </Space>
        </Col>
        <Col span={12}>
          <Space>
            <Badge color={getHealthColor(healthStatus.assetCreation)} />
            <Text>Asset Creation</Text>
            {getHealthIcon(healthStatus.assetCreation)}
          </Space>
        </Col>
        <Col span={12}>
          <Space>
            <Badge color={getHealthColor(healthStatus.database)} />
            <Text>Database</Text>
            {getHealthIcon(healthStatus.database)}
          </Space>
        </Col>
        <Col span={12}>
          <Space>
            <Badge color={getHealthColor(healthStatus.validation)} />
            <Text>Validation</Text>
            {getHealthIcon(healthStatus.validation)}
          </Space>
        </Col>
      </Row>
    </Card>
  );

  const renderIntegrationStatus = () => (
    <Card size="small" title="Integration Status">
      <Timeline>
        <Timeline.Item 
          color={currentProgress ? 'blue' : 'gray'}
          dot={<DatabaseOutlined />}
        >
          <Space direction="vertical" size={0}>
            <Text strong>Bulk Import Session</Text>
            <Text type="secondary">
              {currentProgress ? `${currentProgress.status} - ${currentProgress.processed_items}/${currentProgress.total_items}` : 'No active session'}
            </Text>
          </Space>
        </Timeline.Item>
        
        <Timeline.Item 
          color='gray' // activeWorkflows not available
          dot={<ApiOutlined />}
        >
          <Space direction="vertical" size={0}>
            <Text strong>Asset Creation Workflows</Text>
            <Text type="secondary">
              {'No active workflows'} // activeWorkflows not available
            </Text>
          </Space>
        </Timeline.Item>

        <Timeline.Item 
          color={lastSyncTime ? 'green' : 'gray'}
          dot={<SyncOutlined />}
        >
          <Space direction="vertical" size={0}>
            <Text strong>Workflow Coordination</Text>
            <Text type="secondary">
              {lastSyncTime ? `Last sync: ${lastSyncTime.toLocaleTimeString()}` : 'Not synchronized'}
            </Text>
          </Space>
        </Timeline.Item>
      </Timeline>
    </Card>
  );

  const renderProgressSummary = () => {
    if (!currentProgress) return null;

    const progressPercentage = currentProgress.total_items > 0 
      ? Math.round((currentProgress.processed_items / currentProgress.total_items) * 100)
      : 0;

    return (
      <Card size="small" title="Progress Summary">
        <Space direction="vertical" style={{ width: '100%' }}>
          <Progress
            percent={progressPercentage}
            size="small"
            status={
              currentProgress.status === 'Failed' ? 'exception' :
              currentProgress.status === 'Completed' ? 'success' :
              currentProgress.status === 'Processing' ? 'active' : 'normal'
            }
          />
          
          <Row gutter={16}>
            <Col span={8}>
              <Statistic 
                title="Total" 
                value={currentProgress.total_items} 
                valueStyle={{ fontSize: '14px' }}
              />
            </Col>
            <Col span={8}>
              <Statistic 
                title="Processed" 
                value={currentProgress.processed_items} 
                valueStyle={{ fontSize: '14px', color: '#52c41a' }}
              />
            </Col>
            <Col span={8}>
              <Statistic 
                title="Failed" 
                value={currentProgress.failed_items} 
                valueStyle={{ fontSize: '14px', color: '#ff4d4f' }}
              />
            </Col>
          </Row>

          {currentProgress.status === 'Processing' && (
            <Space>
              <Text type="secondary">Rate:</Text>
              <Tag>{formatProcessingRate(currentProgress.processing_rate)}</Tag>
              {currentProgress.estimated_completion && (
                <>
                  <Text type="secondary">ETA:</Text>
                  <Tag>{formatEstimatedTime(parseInt(currentProgress.estimated_completion))}</Tag>
                </>
              )}
            </Space>
          )}
        </Space>
      </Card>
    );
  };

  const renderControls = () => {
    if (!showControls) return null;

    return (
      <Card size="small" title="Integration Controls">
        <Space direction="vertical" style={{ width: '100%' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <Text>Auto-sync</Text>
            <Switch
              checked={autoSync}
              onChange={setAutoSync}
              checkedChildren="ON"
              unCheckedChildren="OFF"
            />
          </div>

          <Button
            type="primary"
            icon={<SyncOutlined />}
            onClick={syncWorkflowStatus}
            loading={bulkLoading || workflowLoading}
            block
          >
            Sync Now
          </Button>

          {currentProgress && ['Processing', 'Paused'].includes(currentProgress.status) && (
            <Button
              icon={<FolderOpenOutlined />}
              onClick={() => {
                notification.info({
                  message: 'Workflow Dashboard',
                  description: 'Opening detailed workflow dashboard...',
                });
              }}
              block
            >
              Open Dashboard
            </Button>
          )}
        </Space>
      </Card>
    );
  };

  if (compact) {
    return (
      <Card size="small" title="Workflow Integration" style={{ width: '100%' }}>
        <Space direction="vertical" style={{ width: '100%' }} size="small">
          {/* Compact health indicators */}
          <Row gutter={8}>
            <Col span={6}>
              <Tooltip title="Bulk Import">
                <Badge color={getHealthColor(healthStatus.bulkImport)} text="Import" />
              </Tooltip>
            </Col>
            <Col span={6}>
              <Tooltip title="Asset Creation">
                <Badge color={getHealthColor(healthStatus.assetCreation)} text="Assets" />
              </Tooltip>
            </Col>
            <Col span={6}>
              <Tooltip title="Database">
                <Badge color={getHealthColor(healthStatus.database)} text="DB" />
              </Tooltip>
            </Col>
            <Col span={6}>
              <Tooltip title="Validation">
                <Badge color={getHealthColor(healthStatus.validation)} text="Valid" />
              </Tooltip>
            </Col>
          </Row>

          {/* Compact progress */}
          {currentProgress && (
            <div>
              <Progress
                percent={Math.round((currentProgress.processed_items / currentProgress.total_items) * 100)}
                size="small"
                showInfo={false}
              />
              <Text type="secondary" style={{ fontSize: '12px' }}>
                {currentProgress.processed_items}/{currentProgress.total_items} items
              </Text>
            </div>
          )}

          {/* Auto-sync indicator */}
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <Text type="secondary" style={{ fontSize: '12px' }}>
              {autoSync ? 'Auto-sync ON' : 'Auto-sync OFF'}
            </Text>
            {lastSyncTime && (
              <Text type="secondary" style={{ fontSize: '12px' }}>
                {lastSyncTime.toLocaleTimeString()}
              </Text>
            )}
          </div>
        </Space>
      </Card>
    );
  }

  return (
    <Card title="Workflow Integration" extra={
      <Space>
        <Tag color={autoSync ? 'green' : 'default'}>
          {autoSync ? 'Auto-sync ON' : 'Auto-sync OFF'}
        </Tag>
        {lastSyncTime && (
          <Text type="secondary" style={{ fontSize: '12px' }}>
            Last sync: {lastSyncTime.toLocaleTimeString()}
          </Text>
        )}
      </Space>
    }>
      <Space direction="vertical" style={{ width: '100%' }} size={16}>
        {/* Alert for any critical issues */}
        {(bulkError || workflowError) && (
          <Alert
            message="Integration Issues Detected"
            description={bulkError || workflowError}
            type="error"
            showIcon
            closable
            action={
              <Button size="small" onClick={syncWorkflowStatus}>
                Retry
              </Button>
            }
          />
        )}

        <Row gutter={[16, 16]}>
          <Col xs={24} lg={8}>
            {renderWorkflowHealth()}
          </Col>
          <Col xs={24} lg={8}>
            {renderIntegrationStatus()}
          </Col>
          <Col xs={24} lg={8}>
            {renderControls()}
          </Col>
        </Row>

        {currentProgress && (
          <>
            <Divider />
            {renderProgressSummary()}
          </>
        )}
      </Space>
    </Card>
  );
};

export default WorkflowIntegrationPanel;