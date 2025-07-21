import React, { useState } from 'react';
import { Modal, Form, Input, Alert, Typography, Space, Tag } from 'antd';
import { ExclamationCircleOutlined, UndoOutlined } from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { message } from 'antd';
import { ConfigurationVersionInfo } from '../types/assets';
import ConfigurationStatusBadge from './ConfigurationStatusBadge';

const { TextArea } = Input;
const { Text, Title } = Typography;

interface RestoreConfirmationModalProps {
  visible: boolean;
  onCancel: () => void;
  onSuccess: () => void;
  version: ConfigurationVersionInfo;
  token: string;
}

const RestoreConfirmationModal: React.FC<RestoreConfirmationModalProps> = ({
  visible,
  onCancel,
  onSuccess,
  version,
  token
}) => {
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);

  const handleRestore = async (values: { reason?: string }) => {
    if (!token) {
      message.error('No authentication token available');
      return;
    }

    setLoading(true);
    try {
      await invoke('restore_version', {
        token,
        versionId: version.id,
        restoreReason: values.reason || null
      });
      
      message.success(`Version ${version.version_number} has been restored`);
      form.resetFields();
      onSuccess();
    } catch (error) {
      console.error('Failed to restore version:', error);
      message.error(`Failed to restore version: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleCancel = () => {
    form.resetFields();
    onCancel();
  };

  return (
    <Modal
      title={
        <Space>
          <UndoOutlined />
          <span>Restore Configuration Version</span>
        </Space>
      }
      open={visible}
      onCancel={handleCancel}
      okText="Restore Version"
      cancelText="Cancel"
      onOk={form.submit}
      okButtonProps={{ 
        loading, 
        type: 'primary',
        icon: <UndoOutlined />
      }}
      width={600}
      destroyOnClose
    >
      <Form form={form} onFinish={handleRestore} layout="vertical">
        <Alert
          message="Restore Archived Version"
          description={
            <div>
              <p>You are about to restore this archived configuration version. Restoring will:</p>
              <ul style={{ margin: '8px 0', paddingLeft: '20px' }}>
                <li>Return the version to its previous active status</li>
                <li>Make it visible in the default version history view</li>
                <li>Allow status changes and modifications again</li>
                <li>Maintain the complete audit trail of the archive/restore</li>
              </ul>
            </div>
          }
          type="info"
          icon={<ExclamationCircleOutlined />}
          style={{ marginBottom: 16 }}
        />

        <div style={{ 
          padding: '16px', 
          backgroundColor: '#f5f5f5', 
          borderRadius: '8px',
          marginBottom: '16px'
        }}>
          <Title level={5} style={{ margin: 0, marginBottom: '8px' }}>
            Version Details
          </Title>
          <Space size={[8, 8]} wrap>
            <Tag color="blue">{version.version_number}</Tag>
            <ConfigurationStatusBadge status={version.status} />
          </Space>
          <div style={{ marginTop: '8px' }}>
            <Text strong>{version.file_name}</Text>
            <br />
            <Text type="secondary">
              Created by {version.author_username} on{' '}
              {new Date(version.created_at).toLocaleDateString('en-US', {
                year: 'numeric',
                month: 'long',
                day: 'numeric',
                hour: '2-digit',
                minute: '2-digit'
              })}
            </Text>
            {version.notes && (
              <>
                <br />
                <Text type="secondary" italic>
                  "{version.notes}"
                </Text>
              </>
            )}
          </div>
        </div>

        <Form.Item
          name="reason"
          label="Restore Reason (Optional)"
          help="Provide a reason for restoring this version (up to 500 characters)"
        >
          <TextArea
            rows={4}
            placeholder="e.g., Need to revert changes, Version needed again, etc."
            maxLength={500}
            showCount
          />
        </Form.Item>
      </Form>
    </Modal>
  );
};

export default RestoreConfirmationModal;