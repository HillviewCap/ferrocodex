import React, { useState } from 'react';
import { 
  Alert, 
  Button, 
  Card, 
  Collapse, 
  Descriptions, 
  Modal, 
  Progress, 
  Space, 
  Tag, 
  Typography,
  Divider
} from 'antd';
import { 
  ExclamationCircleOutlined, 
  WarningOutlined, 
  InfoCircleOutlined, 
  CheckCircleOutlined,
  BugOutlined,
  ReloadOutlined,
  ContactsOutlined
} from '@ant-design/icons';
import { 
  ProcessedErrorInfo,
  RecoveryAction,
  ErrorSeverity,
  ErrorDomain,
  getRecoveryActions,
  executeRecoveryAction,
  startRecoveryWorkflow
} from '../../utils/errorHandling';

const { Text, Paragraph } = Typography;

interface EnhancedErrorDisplayProps {
  errorInfo: ProcessedErrorInfo;
  visible: boolean;
  onClose: () => void;
  onRecoveryComplete?: (success: boolean) => void;
}

/**
 * Get severity icon and color
 */
const getSeverityDisplay = (severity: ErrorSeverity) => {
  switch (severity) {
    case 'Critical':
      return { icon: <ExclamationCircleOutlined />, color: '#ff4d4f', status: 'error' as const };
    case 'High':
      return { icon: <WarningOutlined />, color: '#fa8c16', status: 'warning' as const };
    case 'Medium':
      return { icon: <InfoCircleOutlined />, color: '#1890ff', status: 'info' as const };
    case 'Low':
      return { icon: <CheckCircleOutlined />, color: '#52c41a', status: 'success' as const };
    default:
      return { icon: <InfoCircleOutlined />, color: '#1890ff', status: 'info' as const };
  }
};

/**
 * Get domain color for tags
 */
const getDomainColor = (domain: ErrorDomain): string => {
  switch (domain) {
    case 'Auth': return 'red';
    case 'Data': return 'blue';
    case 'Assets': return 'green';
    case 'System': return 'orange';
    case 'UI': return 'purple';
    default: return 'default';
  }
};

/**
 * Enhanced Error Display Modal Component
 */
const EnhancedErrorDisplay: React.FC<EnhancedErrorDisplayProps> = ({
  errorInfo,
  visible,
  onClose,
  onRecoveryComplete
}) => {
  const [recoveryActions, setRecoveryActions] = useState<RecoveryAction[]>([]);
  const [activeRecovery, setActiveRecovery] = useState<string | null>(null);
  const [recoveryProgress, setRecoveryProgress] = useState(0);
  const [showTechnicalDetails, setShowTechnicalDetails] = useState(false);

  React.useEffect(() => {
    if (visible && errorInfo) {
      const actions = getRecoveryActions(errorInfo.error, errorInfo.context);
      setRecoveryActions(actions);
    }
  }, [visible, errorInfo]);

  const severityDisplay = getSeverityDisplay(errorInfo.severity);
  const isAdminUser = errorInfo.context.user?.role === 'Admin';

  const handleRecoveryAction = async (action: RecoveryAction) => {
    if (action.type === 'automatic' && action.handler) {
      try {
        const result = await executeRecoveryAction(action.id, recoveryActions);
        if (result.success) {
          Modal.success({
            title: 'Recovery Successful',
            content: result.message,
            onOk: () => onRecoveryComplete?.(true)
          });
        } else {
          Modal.error({
            title: 'Recovery Failed',
            content: result.message || result.error
          });
        }
      } catch (error) {
        Modal.error({
          title: 'Recovery Error',
          content: `Recovery action failed: ${error}`
        });
      }
    } else if (action.type === 'user_guided') {
      // Start guided recovery workflow
      const workflowId = startRecoveryWorkflow(errorInfo.correlationId, [action]);
      setActiveRecovery(workflowId);
      setRecoveryProgress(0);
      
      // Simulate progress for guided actions
      const progressInterval = setInterval(() => {
        setRecoveryProgress(prev => {
          if (prev >= 100) {
            clearInterval(progressInterval);
            setActiveRecovery(null);
            onRecoveryComplete?.(true);
            onClose();
            return 100;
          }
          return prev + 20;
        });
      }, 1000);
    }
  };

  const renderRecoveryActions = () => {
    if (recoveryActions.length === 0) return null;

    return (
      <Card 
        title={
          <Space>
            <ReloadOutlined />
            <span>Suggested Recovery Actions</span>
          </Space>
        }
        size="small"
        style={{ marginTop: 16 }}
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          {recoveryActions.map((action) => (
            <Card key={action.id} size="small" style={{ backgroundColor: '#fafafa' }}>
              <Space direction="vertical" style={{ width: '100%' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <Text strong>{action.label}</Text>
                  <Space>
                    <Tag color={action.priority === 'high' ? 'red' : action.priority === 'medium' ? 'orange' : 'green'}>
                      {action.priority}
                    </Tag>
                    <Tag color={action.riskLevel === 'high' ? 'red' : action.riskLevel === 'medium' ? 'orange' : 'green'}>
                      Risk: {action.riskLevel}
                    </Tag>
                  </Space>
                </div>
                <Text type="secondary">{action.description}</Text>
                {action.estimatedTime && (
                  <Text type="secondary" style={{ fontSize: '12px' }}>
                    Estimated time: {action.estimatedTime}
                  </Text>
                )}
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <Button
                    type={action.priority === 'high' ? 'primary' : 'default'}
                    size="small"
                    icon={action.type === 'manual' ? <ContactsOutlined /> : <ReloadOutlined />}
                    onClick={() => handleRecoveryAction(action)}
                    disabled={!!activeRecovery}
                  >
                    {action.type === 'manual' ? 'Get Help' : 'Start Recovery'}
                  </Button>
                  {action.type === 'manual' && (
                    <Text type="secondary" style={{ fontSize: '11px' }}>
                      Manual action required
                    </Text>
                  )}
                </div>
              </Space>
            </Card>
          ))}
        </Space>
        
        {activeRecovery && (
          <div style={{ marginTop: 16 }}>
            <Text strong>Recovery in Progress...</Text>
            <Progress percent={recoveryProgress} size="small" style={{ marginTop: 8 }} />
          </div>
        )}
      </Card>
    );
  };

  const renderTechnicalDetails = () => {
    if (!showTechnicalDetails && !isAdminUser) return null;

    return (
      <Collapse 
        ghost 
        style={{ marginTop: 16 }}
        items={[
          {
            key: '1',
            label: (
              <Space>
                <BugOutlined />
                <span>Technical Details</span>
                {isAdminUser && <Tag color="blue">Admin</Tag>}
              </Space>
            ),
            children: (
              <Card size="small" style={{ backgroundColor: '#f6f6f6' }}>
                <Descriptions column={1} size="small" bordered>
                  <Descriptions.Item label="Error ID">{errorInfo.correlationId}</Descriptions.Item>
                  <Descriptions.Item label="Timestamp">{errorInfo.timestamp}</Descriptions.Item>
                  <Descriptions.Item label="Severity">{errorInfo.severity}</Descriptions.Item>
                  <Descriptions.Item label="Domain">{errorInfo.domain}</Descriptions.Item>
                  <Descriptions.Item label="Recoverable">{errorInfo.isRecoverable ? 'Yes' : 'No'}</Descriptions.Item>
                  {errorInfo.context.user && (
                    <Descriptions.Item label="User">
                      {errorInfo.context.user.id} ({errorInfo.context.user.role})
                    </Descriptions.Item>
                  )}
                  {errorInfo.context.operation && (
                    <Descriptions.Item label="Operation">
                      {errorInfo.context.operation.name} in {errorInfo.context.operation.component}
                    </Descriptions.Item>
                  )}
                  {errorInfo.context.session?.sessionId && (
                    <Descriptions.Item label="Session">{errorInfo.context.session.sessionId}</Descriptions.Item>
                  )}
                </Descriptions>
                
                {isAdminUser && (
                  <>
                    <Divider orientation="left" style={{ margin: '16px 0 8px 0' }}>Debug Information</Divider>
                    <Paragraph 
                      code 
                      style={{ 
                        backgroundColor: '#000', 
                        color: '#00ff00', 
                        padding: '12px',
                        fontSize: '11px',
                        maxHeight: '200px',
                        overflowY: 'auto'
                      }}
                    >
                      {errorInfo.debugMessage}
                    </Paragraph>
                  </>
                )}
              </Card>
            )
          }
        ]}
      />
    );
  };

  return (
    <Modal
      title={
        <Space>
          {severityDisplay.icon}
          <span style={{ color: severityDisplay.color }}>
            {errorInfo.severity} Error Detected
          </span>
          <Tag color={getDomainColor(errorInfo.domain)}>{errorInfo.domain}</Tag>
        </Space>
      }
      open={visible}
      onCancel={onClose}
      width={600}
      footer={[
        <Button key="close" onClick={onClose}>
          Close
        </Button>,
        ...(isAdminUser ? [
          <Button 
            key="details" 
            type="dashed" 
            icon={<BugOutlined />}
            onClick={() => setShowTechnicalDetails(!showTechnicalDetails)}
          >
            {showTechnicalDetails ? 'Hide' : 'Show'} Technical Details
          </Button>
        ] : [])
      ]}
    >
      <Space direction="vertical" style={{ width: '100%' }} size="large">
        {/* Main Error Message */}
        <Alert
          message={errorInfo.userMessage}
          type={severityDisplay.status}
          showIcon
          icon={severityDisplay.icon}
          style={{ marginBottom: 0 }}
        />

        {/* Error Context Information */}
        {(errorInfo.context.operation || errorInfo.context.user) && (
          <Card size="small" title="Context Information">
            <Descriptions column={2} size="small">
              {errorInfo.context.operation && (
                <Descriptions.Item label="Operation" span={2}>
                  {errorInfo.context.operation.name}
                </Descriptions.Item>
              )}
              {errorInfo.context.operation?.component && (
                <Descriptions.Item label="Component" span={2}>
                  {errorInfo.context.operation.component}
                </Descriptions.Item>
              )}
              {errorInfo.context.user && (
                <Descriptions.Item label="User Role" span={2}>
                  {errorInfo.context.user.role}
                </Descriptions.Item>
              )}
            </Descriptions>
          </Card>
        )}

        {/* Recovery Actions */}
        {renderRecoveryActions()}

        {/* Technical Details */}
        {renderTechnicalDetails()}
      </Space>
    </Modal>
  );
};

export default EnhancedErrorDisplay;