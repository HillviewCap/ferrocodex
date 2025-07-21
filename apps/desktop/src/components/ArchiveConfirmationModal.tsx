import React, { useState } from 'react';
import { Modal, Form, Input, Alert, Typography, Space, Tag } from 'antd';
import { ExclamationCircleOutlined, InboxOutlined } from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { message } from 'antd';
import { ConfigurationVersionInfo } from '../types/assets';
import ConfigurationStatusBadge from './ConfigurationStatusBadge';

const { TextArea } = Input;
const { Text, Title } = Typography;

interface ArchiveConfirmationModalProps {
  visible: boolean;
  onCancel: () => void;
  onSuccess: () => void;
  version: ConfigurationVersionInfo;
  token: string;
}

const ArchiveConfirmationModal: React.FC<ArchiveConfirmationModalProps> = ({
  visible,
  onCancel,
  onSuccess,
  version,
  token
}) => {
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);

  const handleArchive = async (values: { reason?: string }) => {
    if (!token) {
      message.error('No authentication token available');
      return;
    }

    setLoading(true);
    try {
      await invoke('archive_version', {
        token,
        versionId: version.id,
        archiveReason: values.reason || null
      });
      
      message.success(`Version ${version.version_number} has been archived`);
      form.resetFields();
      onSuccess();
    } catch (error) {
      console.error('Failed to archive version:', error);
      message.error(`Failed to archive version: ${error}`);
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
          <InboxOutlined />
          <span>Archive Configuration Version</span>
        </Space>
      }
      open={visible}
      onCancel={handleCancel}
      okText="Archive Version"
      cancelText="Cancel"
      onOk={form.submit}
      okButtonProps={{ 
        loading, 
        danger: true,
        icon: <InboxOutlined />
      }}
      width={600}
      destroyOnClose
    >
      <Form form={form} onFinish={handleArchive} layout="vertical">
        <Alert
          message="Archive Version"
          description={
            <div>
              <p>You are about to archive this configuration version. Archived versions:</p>
              <ul style={{ margin: '8px 0', paddingLeft: '20px' }}>
                <li>Will be hidden from the default version history view</li>
                <li>Cannot be modified or have their status changed</li>
                <li>Can be restored later if needed</li>
                <li>Will maintain their complete audit trail</li>
              </ul>
            </div>
          }
          type="warning"
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
          label="Archive Reason (Optional)"
          help="Provide a reason for archiving this version (up to 500 characters)"
        >
          <TextArea
            rows={4}
            placeholder="e.g., Superseded by newer version, No longer needed, etc."
            maxLength={500}
            showCount
          />
        </Form.Item>
      </Form>
    </Modal>
  );
};

export default ArchiveConfirmationModal;