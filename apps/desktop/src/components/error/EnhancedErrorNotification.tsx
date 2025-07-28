import React, { useState } from 'react';
import { 
  notification, 
  Button, 
  Space, 
  Tag, 
  Progress,
  Typography
} from 'antd';
import { 
  ExclamationCircleOutlined, 
  WarningOutlined, 
  InfoCircleOutlined, 
  CheckCircleOutlined,
  ReloadOutlined,
  EyeOutlined,
  CloseOutlined
} from '@ant-design/icons';
import { 
  ProcessedErrorInfo,
  ErrorSeverity,
  getRecoveryActions,
  executeRecoveryAction
} from '../../utils/errorHandling';
import EnhancedErrorDisplay from './EnhancedErrorDisplay';

const { Text } = Typography;

interface EnhancedErrorNotificationProps {
  errorInfo: ProcessedErrorInfo;
  duration?: number;
  onRecoveryComplete?: (success: boolean) => void;
}

/**
 * Get notification type based on error severity
 */
const getNotificationType = (severity: ErrorSeverity): 'error' | 'warning' | 'info' | 'success' => {
  switch (severity) {
    case 'Critical':
    case 'High':
      return 'error';
    case 'Medium':
      return 'warning';
    case 'Low':
      return 'info';
    default:
      return 'info';
  }
};

/**
 * Get severity icon
 */
const getSeverityIcon = (severity: ErrorSeverity) => {
  switch (severity) {
    case 'Critical':
      return <ExclamationCircleOutlined style={{ color: '#ff4d4f' }} />;
    case 'High':
      return <WarningOutlined style={{ color: '#fa8c16' }} />;
    case 'Medium':
      return <InfoCircleOutlined style={{ color: '#1890ff' }} />;
    case 'Low':
      return <CheckCircleOutlined style={{ color: '#52c41a' }} />;
    default:
      return <InfoCircleOutlined style={{ color: '#1890ff' }} />;
  }
};

/**
 * Enhanced Error Notification Component
 */
export const showEnhancedErrorNotification = ({
  errorInfo,
  duration = 4500,
  onRecoveryComplete
}: EnhancedErrorNotificationProps) => {
  const [modalVisible, setModalVisible] = useState(false);
  const [quickRecoveryProgress, setQuickRecoveryProgress] = useState<number | null>(null);
  
  const notificationType = getNotificationType(errorInfo.severity);
  const recoveryActions = getRecoveryActions(errorInfo.error, errorInfo.context);
  const quickRecoveryAction = recoveryActions.find(action => 
    action.type === 'user_guided' && action.priority === 'high' && action.handler
  );

  const handleQuickRecovery = async () => {
    if (!quickRecoveryAction) return;

    setQuickRecoveryProgress(0);
    
    try {
      const result = await executeRecoveryAction(quickRecoveryAction.id, recoveryActions);
      
      // Simulate progress
      const progressInterval = setInterval(() => {
        setQuickRecoveryProgress(prev => {
          if (prev === null) return null;
          if (prev >= 100) {
            clearInterval(progressInterval);
            setQuickRecoveryProgress(null);
            
            if (result.success) {
              notification.success({
                message: 'Recovery Successful',
                description: result.message,
                duration: 3
              });
              onRecoveryComplete?.(true);
            } else {
              notification.error({
                message: 'Recovery Failed',
                description: result.message || result.error,
                duration: 4
              });
            }
            return 100;
          }
          return prev + 25;
        });
      }, 500);
    } catch (error) {
      setQuickRecoveryProgress(null);
      notification.error({
        message: 'Recovery Error',
        description: `Failed to execute recovery: ${error}`,
        duration: 4
      });
    }
  };

  const handleViewDetails = () => {
    setModalVisible(true);
  };

  const handleModalClose = () => {
    setModalVisible(false);
  };

  const renderNotificationActions = () => {
    const actions: React.ReactNode[] = [];

    // Quick recovery action
    if (quickRecoveryAction && quickRecoveryProgress === null) {
      actions.push(
        <Button
          key="quick-recovery"
          type="primary"
          size="small"
          icon={<ReloadOutlined />}
          onClick={handleQuickRecovery}
          style={{ backgroundColor: '#52c41a', border: 'none' }}
        >
          Quick Fix
        </Button>
      );
    }

    // View details action
    actions.push(
      <Button
        key="view-details"
        size="small"
        icon={<EyeOutlined />}
        onClick={handleViewDetails}
      >
        View Details
      </Button>
    );

    return actions;
  };

  const renderNotificationDescription = () => {
    return (
      <Space direction="vertical" style={{ width: '100%' }} size="small">
        <Text>{errorInfo.userMessage}</Text>
        
        {errorInfo.context.operation && (
          <Space size="small">
            <Text type="secondary" style={{ fontSize: '12px' }}>
              Operation: {errorInfo.context.operation.name}
            </Text>
            {errorInfo.context.operation.component && (
              <Tag color="blue">
                {errorInfo.context.operation.component}
              </Tag>
            )}
          </Space>
        )}
        
        {quickRecoveryProgress !== null && (
          <div style={{ marginTop: 8 }}>
            <Text style={{ fontSize: '12px' }}>Applying quick fix...</Text>
            <Progress 
              percent={quickRecoveryProgress} 
              size="small" 
              style={{ marginTop: 4 }}
              strokeColor="#52c41a"
            />
          </div>
        )}
        
        {recoveryActions.length > 0 && quickRecoveryProgress === null && (
          <div style={{ marginTop: 8 }}>
            <Space wrap>
              {renderNotificationActions()}
            </Space>
          </div>
        )}
      </Space>
    );
  };

  // Determine notification duration based on severity
  const notificationDuration = errorInfo.severity === 'Critical' ? 0 : duration;

  // Show the notification
  const notificationKey = `error-${errorInfo.correlationId}`;
  
  notification[notificationType]({
    key: notificationKey,
    message: (
      <Space>
        {getSeverityIcon(errorInfo.severity)}
        <span>{errorInfo.severity} Error</span>
        <Tag color={errorInfo.domain === 'Auth' ? 'red' : 
                   errorInfo.domain === 'Data' ? 'blue' : 
                   errorInfo.domain === 'Assets' ? 'green' : 
                   errorInfo.domain === 'System' ? 'orange' : 'purple'}>
          {errorInfo.domain}
        </Tag>
      </Space>
    ),
    description: renderNotificationDescription(),
    duration: notificationDuration,
    placement: 'topRight',
    style: { width: '400px' },
    closeIcon: <CloseOutlined />,
    onClose: () => {
      // Clean up any ongoing operations
      setQuickRecoveryProgress(null);
    }
  });

  // Return modal component for detailed view
  return (
    <EnhancedErrorDisplay
      errorInfo={errorInfo}
      visible={modalVisible}
      onClose={handleModalClose}
      onRecoveryComplete={(success) => {
        onRecoveryComplete?.(success);
        handleModalClose();
      }}
    />
  );
};

/**
 * Simplified API for showing enhanced error notifications
 */
export const showErrorNotification = (
  error: any,
  context?: any,
  options?: {
    duration?: number;
    onRecoveryComplete?: (success: boolean) => void;
  }
) => {
  // Import the context-aware error processor
  import('../../utils/errorHandling').then(({ processErrorWithContext, setErrorContext }) => {
    if (context) {
      setErrorContext(context);
    }
    
    const errorInfo = processErrorWithContext(error);
    
    showEnhancedErrorNotification({
      errorInfo,
      duration: options?.duration,
      onRecoveryComplete: options?.onRecoveryComplete
    });
  });
};

export default {
  showEnhancedErrorNotification,
  showErrorNotification
};