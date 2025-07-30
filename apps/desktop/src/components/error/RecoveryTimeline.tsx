import React, { useState } from 'react';
import { Timeline, Card, Space, Typography, Button, Badge, Tooltip, Collapse, Tag } from 'antd';
import { 
  ReloadOutlined, 
  CheckCircleOutlined, 
  CloseCircleOutlined, 
  ExclamationCircleOutlined,
  ClockCircleOutlined,
  ThunderboltOutlined,
  UserOutlined,
  RobotOutlined,
  HistoryOutlined
} from '@ant-design/icons';

const { Text } = Typography;
const { Panel } = Collapse;

export interface RecoveryStep {
  id: string;
  type: 'retry' | 'circuit_breaker' | 'manual_intervention' | 'fallback' | 'system_action';
  status: 'pending' | 'in_progress' | 'completed' | 'failed' | 'skipped';
  title: string;
  description: string;
  timestamp: string;
  duration?: number; // milliseconds
  details?: {
    attempt_number?: number;
    error_message?: string;
    circuit_state?: string;
    user_action?: string;
    system_response?: string;
  };
  metadata?: Record<string, any>;
}

export interface RecoveryOperation {
  operation_id: string;
  operation_name: string;
  started_at: string;
  completed_at?: string;
  status: 'running' | 'completed' | 'failed' | 'cancelled';
  steps: RecoveryStep[];
  total_duration?: number;
  success_rate?: number;
}

interface RecoveryTimelineProps {
  operation: RecoveryOperation;
  onStepAction?: (stepId: string, action: string) => void;
  showMetadata?: boolean;
  compact?: boolean;
}

const RecoveryTimeline: React.FC<RecoveryTimelineProps> = ({
  operation,
  onStepAction,
  showMetadata = false,
  compact = false,
}) => {
  const [expandedSteps, setExpandedSteps] = useState<Set<string>>(new Set());

  const getTypeIcon = (type: string) => {
    switch (type) {
      case 'retry':
        return <ReloadOutlined style={{ color: '#1890ff' }} />;
      case 'circuit_breaker':
        return <ThunderboltOutlined style={{ color: '#faad14' }} />;
      case 'manual_intervention':
        return <UserOutlined style={{ color: '#722ed1' }} />;
      case 'fallback':
        return <ExclamationCircleOutlined style={{ color: '#fa8c16' }} />;
      case 'system_action':
        return <RobotOutlined style={{ color: '#52c41a' }} />;
      default:
        return <ClockCircleOutlined style={{ color: '#d9d9d9' }} />;
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'completed':
        return <CheckCircleOutlined style={{ color: '#52c41a' }} />;
      case 'failed':
        return <CloseCircleOutlined style={{ color: '#ff4d4f' }} />;
      case 'in_progress':
        return <ReloadOutlined spin style={{ color: '#1890ff' }} />;
      case 'skipped':
        return <ExclamationCircleOutlined style={{ color: '#d9d9d9' }} />;
      case 'pending':
      default:
        return <ClockCircleOutlined style={{ color: '#faad14' }} />;
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'completed': return 'green';
      case 'failed': return 'red';
      case 'in_progress': return 'blue';
      case 'skipped': return 'default';
      case 'pending': return 'orange';
      default: return 'default';
    }
  };

  const formatDuration = (duration?: number) => {
    if (!duration) return '';
    if (duration < 1000) return `${duration}ms`;
    return `${(duration / 1000).toFixed(2)}s`;
  };

  const formatTimestamp = (timestamp: string) => {
    return new Date(timestamp).toLocaleTimeString();
  };

  const toggleStepExpansion = (stepId: string) => {
    const newExpanded = new Set(expandedSteps);
    if (newExpanded.has(stepId)) {
      newExpanded.delete(stepId);
    } else {
      newExpanded.add(stepId);
    }
    setExpandedSteps(newExpanded);
  };

  const renderStepDetails = (step: RecoveryStep) => {
    if (!step.details && !step.metadata) return null;

    return (
      <div style={{ marginTop: 8, padding: 8, backgroundColor: '#fafafa', borderRadius: 4 }}>
        {step.details && (
          <Space direction="vertical" size="small" style={{ width: '100%' }}>
            {step.details.attempt_number && (
              <Text type="secondary" style={{ fontSize: '11px' }}>
                Attempt: {step.details.attempt_number}
              </Text>
            )}
            {step.details.error_message && (
              <Text type="secondary" style={{ fontSize: '11px' }}>
                Error: {step.details.error_message}
              </Text>
            )}
            {step.details.circuit_state && (
              <Text type="secondary" style={{ fontSize: '11px' }}>
                Circuit State: {step.details.circuit_state}
              </Text>
            )}
            {step.details.user_action && (
              <Text type="secondary" style={{ fontSize: '11px' }}>
                User Action: {step.details.user_action}
              </Text>
            )}
            {step.details.system_response && (
              <Text type="secondary" style={{ fontSize: '11px' }}>
                System Response: {step.details.system_response}
              </Text>
            )}
          </Space>
        )}
        
        {showMetadata && step.metadata && Object.keys(step.metadata).length > 0 && (
          <Collapse size="small" ghost style={{ marginTop: 8 }}>
            <Panel header="Metadata" key="metadata">
              <pre style={{ fontSize: '10px', margin: 0, backgroundColor: '#f5f5f5', padding: 4 }}>
                {JSON.stringify(step.metadata, null, 2)}
              </pre>
            </Panel>
          </Collapse>
        )}
      </div>
    );
  };

  const getOperationStatusColor = () => {
    switch (operation.status) {
      case 'completed': return '#52c41a';
      case 'failed': return '#ff4d4f';
      case 'running': return '#1890ff';
      case 'cancelled': return '#faad14';
      default: return '#d9d9d9';
    }
  };

  return (
    <Card 
      size="small"
      title={
        <Space>
          <HistoryOutlined />
          <Text strong>{operation.operation_name}</Text>
          <Badge 
            color={getOperationStatusColor()}
            text={operation.status}
          />
          {operation.success_rate !== undefined && (
            <Tag color={operation.success_rate > 0.8 ? 'green' : operation.success_rate > 0.5 ? 'orange' : 'red'}>
              {Math.round(operation.success_rate * 100)}% success
            </Tag>
          )}
        </Space>
      }
      extra={
        <Space>
          <Text type="secondary" style={{ fontSize: '12px' }}>
            Started: {formatTimestamp(operation.started_at)}
          </Text>
          {operation.total_duration && (
            <Text type="secondary" style={{ fontSize: '12px' }}>
              Duration: {formatDuration(operation.total_duration)}
            </Text>
          )}
        </Space>
      }
    >
      <Timeline mode={compact ? undefined : 'left'}>
        {operation.steps.map((step) => (
          <Timeline.Item
            key={step.id}
            color={getStatusColor(step.status)}
            dot={
              <Tooltip title={`${step.type} - ${step.status}`}>
                <Space>
                  {getTypeIcon(step.type)}
                  {getStatusIcon(step.status)}
                </Space>
              </Tooltip>
            }
          >
            <div>
              <Space direction="vertical" style={{ width: '100%' }} size="small">
                <Space style={{ width: '100%', justifyContent: 'space-between' }}>
                  <div>
                    <Text strong style={{ fontSize: compact ? '12px' : '14px' }}>
                      {step.title}
                    </Text>
                    <Badge 
                      count={step.type.replace('_', ' ')}
                      style={{ 
                        marginLeft: 8,
                        fontSize: '10px',
                        backgroundColor: getStatusColor(step.status) === 'green' ? '#52c41a' : 
                                          getStatusColor(step.status) === 'red' ? '#ff4d4f' :
                                          getStatusColor(step.status) === 'blue' ? '#1890ff' : '#faad14'
                      }}
                    />
                  </div>
                  
                  <Space>
                    <Text type="secondary" style={{ fontSize: '11px' }}>
                      {formatTimestamp(step.timestamp)}
                    </Text>
                    {step.duration && (
                      <Text type="secondary" style={{ fontSize: '11px' }}>
                        {formatDuration(step.duration)}
                      </Text>
                    )}
                  </Space>
                </Space>

                <Text type="secondary" style={{ fontSize: compact ? '11px' : '12px' }}>
                  {step.description}
                </Text>

                {!compact && (step.details || step.metadata) && (
                  <Button
                    type="text"
                    size="small"
                    onClick={() => toggleStepExpansion(step.id)}
                    style={{ padding: '0 4px', fontSize: '11px' }}
                  >
                    {expandedSteps.has(step.id) ? 'Hide Details' : 'Show Details'}
                  </Button>
                )}

                {!compact && expandedSteps.has(step.id) && renderStepDetails(step)}

                {step.status === 'failed' && onStepAction && (
                  <Space>
                    <Button
                      type="link"
                      size="small"
                      icon={<ReloadOutlined />}
                      onClick={() => onStepAction(step.id, 'retry')}
                    >
                      Retry
                    </Button>
                    <Button
                      type="link"
                      size="small"
                      onClick={() => onStepAction(step.id, 'skip')}
                    >
                      Skip
                    </Button>
                  </Space>
                )}
              </Space>
            </div>
          </Timeline.Item>
        ))}
      </Timeline>
    </Card>
  );
};

export default RecoveryTimeline;