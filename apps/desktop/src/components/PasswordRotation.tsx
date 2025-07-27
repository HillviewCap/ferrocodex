import React, { useState, useEffect } from 'react';
import { Modal, Steps, Form, Input, Button, Select, notification, Space, Typography, Alert, Spin } from 'antd';
import { invoke } from '@tauri-apps/api/core';
import { LockOutlined, InfoCircleOutlined, ReloadOutlined, HistoryOutlined } from '@ant-design/icons';
import { VaultSecret, PasswordRotationRequest, PasswordHistory, PasswordStrength, GeneratePasswordRequest, defaultPasswordRequest } from '../types/vault';
import useAuthStore from '../store/auth';

const { Step } = Steps;
const { TextArea } = Input;
const { Text } = Typography;

interface PasswordRotationProps {
  secret: VaultSecret;
  visible: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

const PasswordRotation: React.FC<PasswordRotationProps> = ({ secret, visible, onClose, onSuccess }) => {
  const [currentStep, setCurrentStep] = useState(0);
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);
  const [passwordStrength, setPasswordStrength] = useState<PasswordStrength | null>(null);
  const [passwordHistory, setPasswordHistory] = useState<PasswordHistory[]>([]);
  const [generatedPassword, setGeneratedPassword] = useState('');
  
  const { token, user } = useAuthStore();

  useEffect(() => {
    if (visible && secret) {
      loadPasswordHistory();
    }
  }, [visible, secret]);

  const loadPasswordHistory = async () => {
    if (!token || !secret) return;

    try {
      const history = await invoke<PasswordHistory[]>('get_password_history', {
        token,
        secret_id: secret.id,
      });
      setPasswordHistory(history);
    } catch (error) {
      console.error('Failed to load password history:', error);
    }
  };

  const validatePassword = async (password: string) => {
    if (!token || !password) {
      setPasswordStrength(null);
      return;
    }

    try {
      const strength = await invoke<PasswordStrength>('validate_password_strength', {
        token,
        password,
      });
      setPasswordStrength(strength);

      // Check for password reuse
      const isReused = await invoke<boolean>('check_password_reuse', {
        token,
        password,
        exclude_secret_id: secret.id,
      });

      if (isReused) {
        form.setFields([{
          name: 'new_password',
          errors: ['This password has been used before. Please choose a different password.'],
        }]);
      }
    } catch (error) {
      console.error('Failed to validate password:', error);
    }
  };

  const generatePassword = async () => {
    if (!token) return;

    try {
      const request: GeneratePasswordRequest = {
        ...defaultPasswordRequest,
        length: 20, // Longer for rotation
      };

      const password = await invoke<string>('generate_secure_password', {
        token,
        request,
      });
      
      setGeneratedPassword(password);
      form.setFieldsValue({ new_password: password });
      await validatePassword(password);
    } catch (error) {
      notification.error({
        message: 'Generation Failed',
        description: 'Failed to generate password',
      });
    }
  };

  const handleNext = () => {
    form.validateFields().then(() => {
      setCurrentStep(currentStep + 1);
    });
  };

  const handlePrev = () => {
    setCurrentStep(currentStep - 1);
  };

  const handleRotation = async () => {
    if (!token || !user) return;

    try {
      setLoading(true);
      const values = form.getFieldsValue();

      const request: PasswordRotationRequest = {
        secret_id: secret.id,
        new_password: values.new_password,
        rotation_reason: values.rotation_reason || 'Manual rotation',
        author_id: user.id,
      };

      await invoke('rotate_password', {
        token,
        request,
      });

      notification.success({
        message: 'Rotation Successful',
        description: `Password for "${secret.label}" has been rotated successfully.`,
      });

      onSuccess();
      handleClose();
    } catch (error) {
      notification.error({
        message: 'Rotation Failed',
        description: error instanceof Error ? error.message : 'Failed to rotate password',
      });
    } finally {
      setLoading(false);
    }
  };

  const handleClose = () => {
    form.resetFields();
    setCurrentStep(0);
    setPasswordStrength(null);
    setGeneratedPassword('');
    onClose();
  };

  const getPasswordStrengthColor = () => {
    if (!passwordStrength) return undefined;
    if (passwordStrength.score >= 80) return 'success';
    if (passwordStrength.score >= 60) return 'warning';
    return 'error';
  };

  const steps = [
    {
      title: 'Current Password',
      content: (
        <div>
          <Alert
            message="Password Rotation"
            description={`You are about to rotate the password for "${secret.label}". This action cannot be undone.`}
            type="info"
            showIcon
            icon={<LockOutlined />}
            style={{ marginBottom: 16 }}
          />
          
          <Form.Item label="Current Password Info">
            <Space direction="vertical" style={{ width: '100%' }}>
              <Text>Last Changed: {secret.last_changed || 'Unknown'}</Text>
              <Text>Strength Score: {secret.strength_score || 'Not evaluated'}/100</Text>
              <Text>History Entries: {passwordHistory.length}</Text>
            </Space>
          </Form.Item>

          <Form.Item
            name="rotation_reason"
            label="Rotation Reason"
            rules={[{ required: true, message: 'Please provide a reason for rotation' }]}
          >
            <TextArea
              rows={3}
              placeholder="e.g., Scheduled rotation, Security incident, Policy compliance, etc."
            />
          </Form.Item>
        </div>
      ),
    },
    {
      title: 'New Password',
      content: (
        <div>
          <Form.Item
            name="new_password"
            label="New Password"
            rules={[
              { required: true, message: 'Please enter a new password' },
              { min: 12, message: 'Password must be at least 12 characters long' },
            ]}
          >
            <Input.Password
              size="large"
              placeholder="Enter new password"
              onChange={(e) => validatePassword(e.target.value)}
              addonAfter={
                <Button
                  icon={<ReloadOutlined />}
                  onClick={generatePassword}
                  size="small"
                >
                  Generate
                </Button>
              }
            />
          </Form.Item>

          {passwordStrength && (
            <Alert
              message={`Password Strength: ${passwordStrength.score}/100`}
              description={
                <Space direction="vertical">
                  {passwordStrength.feedback.map((msg, idx) => (
                    <Text key={idx}>{msg}</Text>
                  ))}
                </Space>
              }
              type={getPasswordStrengthColor()}
              showIcon
              style={{ marginTop: 16 }}
            />
          )}

          {generatedPassword && (
            <Alert
              message="Generated Password"
              description="A secure password has been generated and filled in the password field."
              type="success"
              showIcon
              closable
              style={{ marginTop: 16 }}
            />
          )}
        </div>
      ),
    },
    {
      title: 'Confirm',
      content: (
        <div>
          <Alert
            message="Rotation Summary"
            description="Please review the rotation details before confirming."
            type="warning"
            showIcon
            style={{ marginBottom: 16 }}
          />

          <Form.Item label="Secret">
            <Text strong>{secret.label}</Text>
          </Form.Item>

          <Form.Item label="Rotation Reason">
            <Text>{form.getFieldValue('rotation_reason')}</Text>
          </Form.Item>

          <Form.Item label="New Password Strength">
            <Text type={passwordStrength && passwordStrength.score >= 60 ? 'success' : 'warning'}>
              {passwordStrength ? `${passwordStrength.score}/100` : 'Not evaluated'}
            </Text>
          </Form.Item>

          <Alert
            message="Important"
            description="After rotation, you will need to update this password on the actual device or system."
            type="info"
            showIcon
            icon={<InfoCircleOutlined />}
            style={{ marginTop: 16 }}
          />
        </div>
      ),
    },
  ];

  return (
    <Modal
      title={
        <Space>
          <LockOutlined />
          Password Rotation Wizard
        </Space>
      }
      open={visible}
      onCancel={handleClose}
      width={700}
      footer={null}
    >
      <Spin spinning={loading}>
        <Steps current={currentStep} style={{ marginBottom: 24 }}>
          {steps.map(item => (
            <Step key={item.title} title={item.title} />
          ))}
        </Steps>

        <Form
          form={form}
          layout="vertical"
          style={{ minHeight: 300 }}
        >
          {steps[currentStep].content}
        </Form>

        <div style={{ marginTop: 24, textAlign: 'right' }}>
          {currentStep > 0 && (
            <Button style={{ marginRight: 8 }} onClick={handlePrev}>
              Previous
            </Button>
          )}
          {currentStep < steps.length - 1 && (
            <Button type="primary" onClick={handleNext}>
              Next
            </Button>
          )}
          {currentStep === steps.length - 1 && (
            <Button
              type="primary"
              onClick={handleRotation}
              loading={loading}
              icon={<LockOutlined />}
            >
              Rotate Password
            </Button>
          )}
        </div>
      </Spin>
    </Modal>
  );
};

export default PasswordRotation;