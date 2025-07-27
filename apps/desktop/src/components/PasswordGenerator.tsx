import React, { useState, useEffect } from 'react';
import {
  Modal,
  Form,
  Input,
  Button,
  Switch,
  Slider,
  Space,
  Typography,
  Alert,
  Card,
  App,
  Tooltip
} from 'antd';
import {
  ReloadOutlined,
  CopyOutlined,
  EyeOutlined,
  EyeInvisibleOutlined,
  SafetyOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { GeneratePasswordRequest, PasswordStrength, defaultPasswordRequest } from '../types/vault';
import PasswordStrengthIndicator from './PasswordStrengthIndicator';
import useAuthStore from '../store/auth';

const { Text } = Typography;

interface PasswordGeneratorProps {
  visible: boolean;
  onCancel: () => void;
  onGenerated: (password: string) => void;
  title?: string;
}

const PasswordGenerator: React.FC<PasswordGeneratorProps> = ({
  visible,
  onCancel,
  onGenerated,
  title = 'Generate Secure Password'
}) => {
  const { token } = useAuthStore();
  const { message } = App.useApp();
  
  const [form] = Form.useForm();
  const [generatedPassword, setGeneratedPassword] = useState<string>('');
  const [passwordStrength, setPasswordStrength] = useState<PasswordStrength | null>(null);
  const [generating, setGenerating] = useState(false);
  const [analyzing, setAnalyzing] = useState(false);
  const [showPassword, setShowPassword] = useState(true);
  const [request, setRequest] = useState<GeneratePasswordRequest>(defaultPasswordRequest);

  useEffect(() => {
    if (visible) {
      form.setFieldsValue(request);
      generatePassword();
    }
  }, [visible]);

  useEffect(() => {
    if (generatedPassword) {
      analyzePassword(generatedPassword);
    }
  }, [generatedPassword]);

  const generatePassword = async () => {
    if (!token) return;

    setGenerating(true);
    try {
      const password = await invoke<string>('generate_secure_password', {
        token,
        request
      });
      setGeneratedPassword(password);
    } catch (err) {
      console.error('Failed to generate password:', err);
      message.error('Failed to generate password');
    } finally {
      setGenerating(false);
    }
  };

  const analyzePassword = async (password: string) => {
    if (!token || !password) return;

    setAnalyzing(true);
    try {
      const strength = await invoke<PasswordStrength>('validate_password_strength', {
        token,
        password
      });
      setPasswordStrength(strength);
    } catch (err) {
      console.error('Failed to analyze password:', err);
      message.error('Failed to analyze password strength');
    } finally {
      setAnalyzing(false);
    }
  };

  const handleFormChange = (changedFields: any) => {
    const newRequest = { ...request };
    Object.keys(changedFields).forEach(key => {
      if (changedFields[key] !== undefined) {
        (newRequest as any)[key] = changedFields[key];
      }
    });
    setRequest(newRequest);
  };

  const handleRegenerate = () => {
    const formValues = form.getFieldsValue();
    setRequest(formValues);
    generatePassword();
  };

  const handleCopyPassword = () => {
    if (generatedPassword) {
      navigator.clipboard.writeText(generatedPassword);
      message.success('Password copied to clipboard');
    }
  };

  const handleUsePassword = () => {
    if (generatedPassword) {
      onGenerated(generatedPassword);
      onCancel();
    }
  };

  const isValidConfiguration = () => {
    return request.include_uppercase || request.include_lowercase || 
           request.include_numbers || request.include_special;
  };

  return (
    <Modal
      title={
        <Space>
          <SafetyOutlined />
          {title}
        </Space>
      }
      open={visible}
      onCancel={onCancel}
      width={600}
      footer={[
        <Button key="cancel" onClick={onCancel}>
          Cancel
        </Button>,
        <Button 
          key="use" 
          type="primary" 
          onClick={handleUsePassword}
          disabled={!generatedPassword || !passwordStrength || passwordStrength.score < 40}
        >
          Use This Password
        </Button>
      ]}
    >
      <Space direction="vertical" style={{ width: '100%' }} size="large">
        {/* Configuration */}
        <Card title="Password Configuration" size="small">
          <Form
            form={form}
            layout="vertical"
            onValuesChange={handleFormChange}
            initialValues={request}
          >
            <Form.Item
              name="length"
              label="Length"
              rules={[
                { required: true, message: 'Please specify password length' },
                { type: 'number', min: 8, max: 128, message: 'Length must be between 8 and 128 characters' }
              ]}
            >
              <Slider
                min={8}
                max={64}
                marks={{
                  8: '8',
                  12: '12',
                  16: '16',
                  24: '24',
                  32: '32',
                  64: '64'
                }}
                tooltip={{ formatter: (value) => `${value} characters` }}
              />
            </Form.Item>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px' }}>
              <Form.Item name="include_uppercase" valuePropName="checked">
                <Space>
                  <Switch size="small" />
                  <Text>Uppercase (A-Z)</Text>
                </Space>
              </Form.Item>

              <Form.Item name="include_lowercase" valuePropName="checked">
                <Space>
                  <Switch size="small" />
                  <Text>Lowercase (a-z)</Text>
                </Space>
              </Form.Item>

              <Form.Item name="include_numbers" valuePropName="checked">
                <Space>
                  <Switch size="small" />
                  <Text>Numbers (0-9)</Text>
                </Space>
              </Form.Item>

              <Form.Item name="include_special" valuePropName="checked">
                <Space>
                  <Switch size="small" />
                  <Text>Special (!@#$...)</Text>
                </Space>
              </Form.Item>
            </div>

            <Form.Item name="exclude_ambiguous" valuePropName="checked">
              <Space align="start">
                <Switch size="small" />
                <div>
                  <Text>Exclude ambiguous characters</Text>
                  <br />
                  <Text type="secondary" style={{ fontSize: '12px' }}>
                    Excludes: 0, O, l, I, 1
                  </Text>
                </div>
              </Space>
            </Form.Item>
          </Form>

          {!isValidConfiguration() && (
            <Alert
              message="At least one character type must be selected"
              type="warning"
              showIcon
              style={{ marginTop: '16px' }}
            />
          )}
        </Card>

        {/* Generated Password */}
        <Card 
          title="Generated Password" 
          size="small"
          extra={
            <Space>
              <Tooltip title={showPassword ? "Hide password" : "Show password"}>
                <Button
                  type="text"
                  icon={showPassword ? <EyeInvisibleOutlined /> : <EyeOutlined />}
                  onClick={() => setShowPassword(!showPassword)}
                />
              </Tooltip>
              <Tooltip title="Copy to clipboard">
                <Button
                  type="text"
                  icon={<CopyOutlined />}
                  onClick={handleCopyPassword}
                  disabled={!generatedPassword}
                />
              </Tooltip>
              <Button
                type="primary"
                icon={<ReloadOutlined />}
                onClick={handleRegenerate}
                loading={generating}
                disabled={!isValidConfiguration()}
              >
                Generate
              </Button>
            </Space>
          }
        >
          <div style={{ marginBottom: '16px' }}>
            <Input.TextArea
              value={showPassword ? generatedPassword : '••••••••••••••••'}
              readOnly
              rows={3}
              style={{ 
                fontFamily: 'monospace',
                fontSize: '14px',
                backgroundColor: '#f5f5f5'
              }}
              placeholder={generating ? 'Generating secure password...' : 'Click Generate to create a password'}
            />
          </div>

          {/* Strength Analysis */}
          {passwordStrength && (
            <div>
              <Text strong style={{ marginBottom: '8px', display: 'block' }}>
                Password Strength Analysis:
              </Text>
              <PasswordStrengthIndicator
                strength={passwordStrength}
                showDetails={true}
              />
            </div>
          )}

          {analyzing && (
            <div style={{ textAlign: 'center', padding: '16px' }}>
              <Text type="secondary">Analyzing password strength...</Text>
            </div>
          )}
        </Card>

        {/* Security Notice */}
        <Alert
          message="Security Notice"
          description="Generated passwords use cryptographically secure random number generation and are never stored in plain text. The password strength analysis ensures compliance with security policies."
          type="info"
          showIcon
        />
      </Space>
    </Modal>
  );
};

export default PasswordGenerator;