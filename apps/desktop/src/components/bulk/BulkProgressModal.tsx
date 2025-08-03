import React, { useEffect, useState, useCallback } from 'react';
import { Modal, Progress, Typography, Space, Button, List, Alert, Tag, Divider } from 'antd';
import {
  CloseOutlined,
  PauseCircleOutlined,
  StopOutlined,
  ReloadOutlined,
  CheckCircleOutlined,
  ExclamationCircleOutlined,
  ClockCircleOutlined,
} from '@ant-design/icons';
import useBulkOperationsStore from '../../store/bulkOperations';
import { BulkOperationStatus } from '../../types/bulkOperations';
import { formatBulkOperationDuration, getBulkOperationColor, getBulkOperationIcon } from '../../types/bulkOperations';

const { Text, Title } = Typography;

const BulkProgressModal: React.FC = () => {
  const {
    ui,
    operationProgress,
    showProgressModal,
    getOperationProgress,
    cancelOperation,
    error,
    isLoading,
  } = useBulkOperationsStore();

  const [refreshInterval, setRefreshInterval] = useState<NodeJS.Timeout | null>(null);
  
  const currentOperation = ui.currentOperation;
  const currentProgress = currentOperation ? operationProgress.get(currentOperation.id) : null;
  const isVisible = ui.showProgressModal;

  // Get status display properties
  const getStatusDisplay = (status: BulkOperationStatus) => {
    switch (status) {
      case 'pending':
        return { color: 'default', icon: <ClockCircleOutlined />, text: 'Pending' };
      case 'validating':
        return { color: 'processing', icon: <ReloadOutlined spin />, text: 'Validating' };
      case 'processing':
        return { color: 'processing', icon: <ReloadOutlined spin />, text: 'Processing' };
      case 'completed':
        return { color: 'success', icon: <CheckCircleOutlined />, text: 'Completed' };
      case 'failed':
        return { color: 'error', icon: <ExclamationCircleOutlined />, text: 'Failed' };
      case 'cancelled':
        return { color: 'warning', icon: <StopOutlined />, text: 'Cancelled' };
      default:
        return { color: 'default', icon: <ClockCircleOutlined />, text: 'Unknown' };
    }
  };

  // Start progress polling
  useEffect(() => {
    if (isVisible && currentOperation) {
      const pollProgress = async () => {
        try {
          await getOperationProgress(currentOperation.id);
        } catch (err) {
          console.error('Failed to get operation progress:', err);
        }
      };

      // Initial poll
      pollProgress();

      // Set up interval for active operations
      if (currentProgress?.status === 'processing' || currentProgress?.status === 'validating') {
        const interval = setInterval(pollProgress, 1000); // Poll every second
        setRefreshInterval(interval);
        return () => clearInterval(interval);
      }
    }

    return () => {
      if (refreshInterval) {
        clearInterval(refreshInterval);
        setRefreshInterval(null);
      }
    };
  }, [isVisible, currentOperation, currentProgress?.status, getOperationProgress]);

  // Handle modal close
  const handleClose = useCallback(() => {
    showProgressModal(false);
    if (refreshInterval) {
      clearInterval(refreshInterval);
      setRefreshInterval(null);
    }
  }, [showProgressModal, refreshInterval]);

  // Handle operation cancellation
  const handleCancel = useCallback(async () => {
    if (!currentOperation) return;

    try {
      await cancelOperation(currentOperation.id);
    } catch (err) {
      console.error('Failed to cancel operation:', err);
    }
  }, [currentOperation, cancelOperation]);

  // Calculate progress percentage
  const progressPercent = currentProgress 
    ? Math.round((currentProgress.processed_items / currentProgress.total_items) * 100)
    : 0;

  const statusDisplay = currentProgress ? getStatusDisplay(currentProgress.status) : null;
  const isActive = currentProgress?.status === 'processing' || currentProgress?.status === 'validating';
  const isComplete = currentProgress?.status === 'completed';
  const isFailed = currentProgress?.status === 'failed';
  const isCancelled = currentProgress?.status === 'cancelled';

  if (!isVisible || !currentOperation) {
    return null;
  }

  return (
    <Modal
      title={
        <Space align="center">
          <Text strong>Bulk Operation Progress</Text>
          {statusDisplay && (
            <Tag color={statusDisplay.color} icon={statusDisplay.icon}>
              {statusDisplay.text}
            </Tag>
          )}
        </Space>
      }
      open={isVisible}
      onCancel={handleClose}
      footer={[
        <Button
          key="cancel-op"
          danger
          icon={<StopOutlined />}
          onClick={handleCancel}
          disabled={!isActive}
          loading={isLoading}
        >
          Cancel Operation
        </Button>,
        <Button
          key="close"
          type={isComplete ? 'primary' : 'default'}
          icon={<CloseOutlined />}
          onClick={handleClose}
        >
          {isComplete ? 'Done' : 'Close'}
        </Button>,
      ]}
      width={600}
      closable={false}
      maskClosable={false}
    >
      <Space direction="vertical" style={{ width: '100%' }} size="large">
        {/* Operation Details */}
        <div>
          <Text strong>Operation Type: </Text>
          <Text>{currentOperation.operation_type}</Text>
          <br />
          <Text strong>Operation ID: </Text>
          <Text code>{currentOperation.id}</Text>
          <br />
          <Text strong>Assets: </Text>
          <Text>{currentOperation.asset_ids.length} selected</Text>
        </div>

        {/* Progress Bar */}
        {currentProgress && (
          <div>
            <Progress
              percent={progressPercent}
              status={
                isFailed ? 'exception' : 
                isComplete ? 'success' : 
                isActive ? 'active' : 'normal'
              }
              showInfo={true}
              format={(percent) => (
                <span>
                  {percent}% ({currentProgress.processed_items}/{currentProgress.total_items})
                </span>
              )}
            />
            
            {/* Current item being processed */}
            {currentProgress.current_item && isActive && (
              <Text type="secondary" style={{ fontSize: '12px' }}>
                Processing: {currentProgress.current_item}
              </Text>
            )}
          </div>
        )}

        {/* Statistics */}
        {currentProgress && (
          <div>
            <Space size="large">
              <div>
                <Text strong>Processed: </Text>
                <Text>{currentProgress.processed_items}</Text>
              </div>
              <div>
                <Text strong>Failed: </Text>
                <Text type={currentProgress.failed_items > 0 ? 'danger' : 'secondary'}>
                  {currentProgress.failed_items}
                </Text>
              </div>
              <div>
                <Text strong>Rate: </Text>
                <Text>{currentProgress.processing_rate.toFixed(1)} items/sec</Text>
              </div>
            </Space>

            {/* ETA */}
            {currentProgress.estimated_completion && isActive && (
              <div style={{ marginTop: '8px' }}>
                <Text strong>ETA: </Text>
                <Text>{currentProgress.estimated_completion}</Text>
              </div>
            )}
          </div>
        )}

        {/* Errors */}
        {currentProgress && currentProgress.errors.length > 0 && (
          <div>
            <Divider />
            <Alert
              message="Operation Errors"
              type="warning"
              showIcon
              description={
                <List
                  size="small"
                  dataSource={currentProgress.errors.slice(0, 5)} // Show first 5 errors
                  renderItem={(error) => (
                    <List.Item>
                      <Space direction="vertical" size="small" style={{ width: '100%' }}>
                        <Text strong>Asset {error.asset_id} ({error.asset_name})</Text>
                        <Text type="danger" style={{ fontSize: '12px' }}>
                          {error.error_message}
                        </Text>
                      </Space>
                    </List.Item>
                  )}
                />
              }
            />
            {currentProgress.errors.length > 5 && (
              <Text type="secondary" style={{ fontSize: '12px', marginTop: '8px' }}>
                ... and {currentProgress.errors.length - 5} more errors
              </Text>
            )}
          </div>
        )}

        {/* General Error */}
        {error && (
          <Alert
            message="Operation Error"
            description={error}
            type="error"
            showIcon
            closable
          />
        )}

        {/* Completion Status */}
        {isComplete && (
          <Alert
            message="Operation Completed Successfully"
            description={`Processed ${currentProgress?.processed_items || 0} items with ${currentProgress?.failed_items || 0} failures.`}
            type="success"
            showIcon
          />
        )}

        {isFailed && (
          <Alert
            message="Operation Failed"
            description="The bulk operation encountered errors and could not complete successfully."
            type="error"
            showIcon
          />
        )}

        {isCancelled && (
          <Alert
            message="Operation Cancelled"
            description="The bulk operation was cancelled by user request."
            type="warning"
            showIcon
          />
        )}
      </Space>
    </Modal>
  );
};

export default BulkProgressModal;