import React, { useState, useEffect } from 'react';
import {
  Modal,
  Form,
  Select,
  Input,
  Alert,
  Space,
  Typography,
  Divider,
  Spin
} from 'antd';
import { ConfigurationStatus, ConfigurationVersionInfo } from '../types/assets';
import ConfigurationStatusBadge from './ConfigurationStatusBadge';
import { invoke } from '@tauri-apps/api/core';

const { Text, Title } = Typography;
const { TextArea } = Input;
const { Option } = Select;

interface ChangeStatusModalProps {
  visible: boolean;
  onCancel: () => void;
  onSuccess: () => void;
  version: ConfigurationVersionInfo | null;
  token: string;
}

const ChangeStatusModal: React.FC<ChangeStatusModalProps> = ({
  visible,
  onCancel,
  onSuccess,
  version,
  token
}) => {
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);
  const [loadingTransitions, setLoadingTransitions] = useState(false);
  const [availableTransitions, setAvailableTransitions] = useState<ConfigurationStatus[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (visible && version) {
      loadAvailableTransitions();
      form.resetFields();
    }
  }, [visible, version, form]);

  const loadAvailableTransitions = async () => {
    if (!version) return;

    setLoadingTransitions(true);
    setError(null);

    try {
      const transitions = await invoke<string[]>('get_available_status_transitions', {
        token,
        versionId: version.id
      });

      setAvailableTransitions(transitions as ConfigurationStatus[]);
    } catch (err) {
      setError(err as string);
    } finally {
      setLoadingTransitions(false);
    }
  };

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      setLoading(true);
      setError(null);

      await invoke('update_configuration_status', {
        token,
        versionId: version!.id,
        newStatus: values.status,
        changeReason: values.reason || null
      });

      form.resetFields();
      onSuccess();
    } catch (err) {
      if (typeof err === 'string') {
        setError(err);
      } else if (err instanceof Error) {
        setError(err.message);
      } else {
        setError('Failed to update status');
      }
    } finally {
      setLoading(false);
    }
  };

  const handleCancel = () => {
    form.resetFields();
    setError(null);
    onCancel();
  };

  if (!version) return null;

  return (
    <Modal
      title="Change Configuration Status"
      open={visible}
      onOk={handleSubmit}
      onCancel={handleCancel}
      confirmLoading={loading}
      width={600}
      destroyOnClose
      okText="Update Status"
      cancelText="Cancel"
      okButtonProps={{
        disabled: loadingTransitions || availableTransitions.length === 0
      }}
    >
      <Space direction="vertical" size="large" style={{ width: '100%' }}>
        <div>
          <Title level={5} style={{ margin: 0, marginBottom: '8px' }}>
            Configuration Version
          </Title>
          <Space>
            <Text strong>{version.file_name}</Text>
            <Text type="secondary">({version.version_number})</Text>
          </Space>
        </div>

        <Divider style={{ margin: '12px 0' }} />

        <div>
          <Title level={5} style={{ margin: 0, marginBottom: '8px' }}>
            Current Status
          </Title>
          <ConfigurationStatusBadge status={version.status} />
        </div>

        {error && (
          <Alert
            message="Error"
            description={error}
            type="error"
            showIcon
            closable
            onClose={() => setError(null)}
          />
        )}

        <Form
          form={form}
          layout="vertical"
          requiredMark={false}
        >
          <Form.Item
            label="New Status"
            name="status"
            rules={[
              { required: true, message: 'Please select a new status' }
            ]}
          >
            {loadingTransitions ? (
              <div style={{ textAlign: 'center', padding: '20px' }}>
                <Spin size="small" />
                <Text type="secondary" style={{ marginLeft: '8px' }}>
                  Loading available status transitions...
                </Text>
              </div>
            ) : availableTransitions.length === 0 ? (
              <Alert
                message="No status changes available"
                description="You do not have permission to change the status of this configuration, or no transitions are available from the current status."
                type="info"
                showIcon
              />
            ) : (
              <Select
                placeholder="Select new status"
                style={{ width: '100%' }}
              >
                {availableTransitions.map((status) => (
                  <Option key={status} value={status}>
                    <Space>
                      <ConfigurationStatusBadge status={status} size="small" />
                      <Text>{status}</Text>
                    </Space>
                  </Option>
                ))}
              </Select>
            )}
          </Form.Item>

          <Form.Item
            label="Change Reason (Optional)"
            name="reason"
            rules={[
              { max: 500, message: 'Reason cannot exceed 500 characters' }
            ]}
          >
            <TextArea
              rows={3}
              placeholder="Provide a reason for this status change..."
              showCount
              maxLength={500}
            />
          </Form.Item>
        </Form>

        <Alert
          message="Status Change Information"
          description="Status changes are tracked and cannot be undone. Make sure you have the appropriate permissions before proceeding."
          type="info"
          showIcon
        />
      </Space>
    </Modal>
  );
};

export default ChangeStatusModal;