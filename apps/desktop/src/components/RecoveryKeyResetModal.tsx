import React, { useState } from 'react';
import { Modal, Form, Input, Alert, Typography } from 'antd';
import { UserOutlined, KeyOutlined, LockOutlined } from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../store/auth';

const { Paragraph } = Typography;

interface RecoveryKeyResetModalProps {
  open: boolean;
  onClose: () => void;
}

interface ResetFormValues {
  username: string;
  recoveryCode: string;
  newPassword: string;
  confirmPassword: string;
}

const RecoveryKeyResetModal: React.FC<RecoveryKeyResetModalProps> = ({ open, onClose }) => {
  const { showRecoveryCode } = useAuthStore();
  const [form] = Form.useForm<ResetFormValues>();
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleClose = () => {
    form.resetFields();
    setError(null);
    onClose();
  };

  const handleFinish = async (values: ResetFormValues) => {
    setSubmitting(true);
    setError(null);
    try {
      const newRecoveryCode = await invoke<string>('reset_password_with_recovery_key', {
        username: values.username,
        recoveryCode: values.recoveryCode,
        newPassword: values.newPassword,
      });
      handleClose();
      showRecoveryCode(newRecoveryCode);
    } catch (err) {
      setError(err as string);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Modal
      open={open}
      title="Reset password with recovery key"
      okText="Reset Password"
      onOk={() => form.submit()}
      onCancel={handleClose}
      confirmLoading={submitting}
      mask={{ closable: !submitting }}
      destroyOnHidden
    >
      <Paragraph type="secondary">
        Enter the recovery key saved when Ferrocodex was set up. This sets a new password
        for the account and issues a new recovery key; the old one stops working.
      </Paragraph>

      {error && (
        <Alert
          title="Reset Failed"
          description={error}
          type="error"
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}

      <Form form={form} layout="vertical" requiredMark={false} onFinish={handleFinish} autoComplete="off">
        <Form.Item
          label="Username"
          name="username"
          rules={[{ required: true, message: 'Please enter the username' }]}
        >
          <Input prefix={<UserOutlined />} placeholder="Username" />
        </Form.Item>

        <Form.Item
          label="Recovery Key"
          name="recoveryCode"
          rules={[{ required: true, message: 'Please enter the recovery key' }]}
        >
          <Input prefix={<KeyOutlined />} placeholder="XXXX-XXXX-XXXX-..." />
        </Form.Item>

        <Form.Item
          label="New Password"
          name="newPassword"
          rules={[
            { required: true, message: 'Please enter a new password' },
            { min: 8, message: 'Password must be at least 8 characters' },
          ]}
        >
          <Input.Password prefix={<LockOutlined />} placeholder="New password" />
        </Form.Item>

        <Form.Item
          label="Confirm New Password"
          name="confirmPassword"
          dependencies={['newPassword']}
          rules={[
            { required: true, message: 'Please confirm the new password' },
            ({ getFieldValue }) => ({
              validator(_, value) {
                if (!value || getFieldValue('newPassword') === value) {
                  return Promise.resolve();
                }
                return Promise.reject(new Error('Passwords do not match'));
              },
            }),
          ]}
        >
          <Input.Password prefix={<LockOutlined />} placeholder="Confirm new password" />
        </Form.Item>
      </Form>
    </Modal>
  );
};

export default RecoveryKeyResetModal;
