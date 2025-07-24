import React, { useState } from 'react';
import { Modal, Form, Select, Input, Typography, Alert, Space } from 'antd';
import { FirmwareStatus } from '../../types/firmware';

const { Text } = Typography;
const { TextArea } = Input;

interface FirmwareStatusDialogProps {
  visible: boolean;
  currentStatus: FirmwareStatus;
  availableTransitions: FirmwareStatus[];
  onConfirm: (newStatus: FirmwareStatus, reason?: string) => Promise<void>;
  onCancel: () => void;
  isPromotingToGolden?: boolean;
}

const FirmwareStatusDialog: React.FC<FirmwareStatusDialogProps> = ({
  visible,
  currentStatus,
  availableTransitions,
  onConfirm,
  onCancel,
  isPromotingToGolden: _isPromotingToGolden = false
}) => {
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);

  const getStatusColor = (status: FirmwareStatus): string => {
    switch (status) {
      case 'Golden':
        return '#faad14';
      case 'Approved':
        return '#52c41a';
      case 'Draft':
        return '#d9d9d9';
      case 'Archived':
        return '#8c8c8c';
      default:
        return '#d9d9d9';
    }
  };

  const getStatusDescription = (status: FirmwareStatus): string => {
    switch (status) {
      case 'Golden':
        return 'The golden standard version - will archive other Golden versions';
      case 'Approved':
        return 'Approved for production use';
      case 'Draft':
        return 'Work in progress version';
      case 'Archived':
        return 'No longer in active use';
      default:
        return '';
    }
  };

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      setLoading(true);
      
      await onConfirm(values.newStatus, values.reason);
      
      // Only reset and close if successful
      form.resetFields();
    } catch (error) {
      console.error('Status update failed:', error);
      // Don't close the dialog on error
    } finally {
      setLoading(false);
    }
  };

  const selectedStatus = Form.useWatch('newStatus', form);
  const isGoldenPromotion = selectedStatus === 'Golden';

  const handleCancel = () => {
    form.resetFields();
    onCancel();
  };

  return (
    <Modal
      title="Change Firmware Status"
      open={visible}
      onOk={handleSubmit}
      onCancel={handleCancel}
      confirmLoading={loading}
      okText="Change Status"
      width={500}
      destroyOnClose={true}
    >
      <Form
        form={form}
        layout="vertical"
        initialValues={{ newStatus: availableTransitions.length > 0 ? availableTransitions[0] : undefined }}
      >
        <Form.Item label="Current Status" style={{ marginBottom: '16px' }}>
          <Space>
            <div
              style={{
                width: '12px',
                height: '12px',
                borderRadius: '50%',
                backgroundColor: getStatusColor(currentStatus),
                display: 'inline-block'
              }}
            />
            <Text strong>{currentStatus}</Text>
          </Space>
        </Form.Item>

        <Form.Item
          name="newStatus"
          label="New Status"
          rules={[{ required: true, message: 'Please select a new status' }]}
        >
          <Select
            placeholder="Select new status"
            style={{ width: '100%' }}
          >
            {availableTransitions.map(status => (
              <Select.Option key={status} value={status}>
                <Space>
                  <div
                    style={{
                      width: '10px',
                      height: '10px',
                      borderRadius: '50%',
                      backgroundColor: getStatusColor(status),
                      display: 'inline-block'
                    }}
                  />
                  <span>{status}</span>
                </Space>
              </Select.Option>
            ))}
          </Select>
        </Form.Item>

        {selectedStatus && (
          <div style={{ marginBottom: '16px' }}>
            <Text type="secondary" style={{ fontSize: '12px' }}>
              {getStatusDescription(selectedStatus)}
            </Text>
          </div>
        )}

        {isGoldenPromotion && (
          <Alert
            message="Golden Promotion Warning"
            description="Promoting this version to Golden will automatically archive any existing Golden versions for this asset. This action cannot be undone."
            type="warning"
            showIcon
            style={{ marginBottom: '16px' }}
          />
        )}

        <Form.Item
          name="reason"
          label="Reason for Change"
          rules={[
            {
              required: isGoldenPromotion,
              message: 'Reason is required for Golden promotion'
            },
            {
              max: 500,
              message: 'Reason cannot exceed 500 characters'
            }
          ]}
        >
          <TextArea
            rows={4}
            placeholder={
              isGoldenPromotion
                ? 'Please provide a reason for promoting this version to Golden status (required)'
                : 'Optional: Provide a reason for this status change'
            }
            showCount
            maxLength={500}
          />
        </Form.Item>
      </Form>
    </Modal>
  );
};

export default FirmwareStatusDialog;