import React, { useState } from 'react';
import { Form, Input, Button, Card, Typography, Alert, Space } from 'antd';
import { UserOutlined, LockOutlined } from '@ant-design/icons';
import useAuthStore from '../store/auth';

const { Title, Text } = Typography;

interface AdminSetupFormValues {
  username: string;
  password: string;
  confirmPassword: string;
}

const AdminSetup: React.FC = () => {
  const { createAdminAccount, isLoading, error, clearError } = useAuthStore();
  const [form] = Form.useForm();
  const [passwordStrength, setPasswordStrength] = useState<{
    score: number;
    message: string;
    color: string;
  }>({ score: 0, message: '', color: '' });

  const validatePassword = (password: string) => {
    let score = 0;
    let message = 'Very Weak';
    let color = '#ff4d4f';

    if (password.length >= 8) score++;
    if (/[a-z]/.test(password)) score++;
    if (/[A-Z]/.test(password)) score++;
    if (/[0-9]/.test(password)) score++;
    if (/[^A-Za-z0-9]/.test(password)) score++;

    switch (score) {
      case 0:
      case 1:
        message = 'Very Weak';
        color = '#ff4d4f';
        break;
      case 2:
        message = 'Weak';
        color = '#fa8c16';
        break;
      case 3:
        message = 'Fair';
        color = '#fadb14';
        break;
      case 4:
        message = 'Good';
        color = '#52c41a';
        break;
      case 5:
        message = 'Strong';
        color = '#389e0d';
        break;
    }

    return { score, message, color };
  };

  const handlePasswordChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const password = e.target.value;
    setPasswordStrength(validatePassword(password));
  };

  const onFinish = async (values: AdminSetupFormValues) => {
    try {
      clearError();
      await createAdminAccount(values.username, values.password);
    } catch (error) {
      console.error('Admin account creation failed:', error);
    }
  };

  return (
    <div style={{ 
      height: '100vh', 
      display: 'flex', 
      justifyContent: 'center', 
      alignItems: 'center',
      background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)'
    }}>
      <Card 
        style={{ 
          width: 500, 
          boxShadow: '0 4px 20px rgba(0,0,0,0.1)' 
        }}
      >
        <Space direction="vertical" size="large" style={{ width: '100%' }}>
          <div style={{ textAlign: 'center' }}>
            <Title level={2} style={{ marginBottom: 8 }}>
              Welcome to Ferrocodex
            </Title>
            <Text type="secondary">
              Create your administrator account to get started
            </Text>
          </div>

          {error && (
            <Alert
              message="Setup Failed"
              description={error}
              type="error"
              showIcon
              closable
              onClose={clearError}
            />
          )}

          <Form
            form={form}
            name="admin-setup"
            onFinish={onFinish}
            autoComplete="off"
            layout="vertical"
            requiredMark={false}
          >
            <Form.Item
              label="Username"
              name="username"
              rules={[
                { required: true, message: 'Please enter a username' },
                { min: 3, message: 'Username must be at least 3 characters' },
                { max: 50, message: 'Username cannot exceed 50 characters' },
                { 
                  pattern: /^[a-zA-Z0-9_-]+$/, 
                  message: 'Username can only contain letters, numbers, hyphens, and underscores' 
                },
              ]}
            >
              <Input 
                prefix={<UserOutlined />} 
                placeholder="Enter admin username"
                size="large"
              />
            </Form.Item>

            <Form.Item
              label="Password"
              name="password"
              rules={[
                { required: true, message: 'Please enter a password' },
                { min: 8, message: 'Password must be at least 8 characters' },
                { 
                  validator: (_, value) => {
                    const strength = validatePassword(value || '');
                    if (strength.score < 3) {
                      return Promise.reject(new Error('Password is too weak. Use a mix of letters, numbers, and symbols.'));
                    }
                    return Promise.resolve();
                  }
                },
              ]}
            >
              <Input.Password 
                prefix={<LockOutlined />} 
                placeholder="Enter secure password"
                size="large"
                onChange={handlePasswordChange}
              />
            </Form.Item>

            {passwordStrength.score > 0 && (
              <div style={{ marginTop: -16, marginBottom: 16 }}>
                <Text style={{ color: passwordStrength.color, fontSize: '12px' }}>
                  Password Strength: {passwordStrength.message}
                </Text>
              </div>
            )}

            <Form.Item
              label="Confirm Password"
              name="confirmPassword"
              dependencies={['password']}
              rules={[
                { required: true, message: 'Please confirm your password' },
                ({ getFieldValue }) => ({
                  validator(_, value) {
                    if (!value || getFieldValue('password') === value) {
                      return Promise.resolve();
                    }
                    return Promise.reject(new Error('Passwords do not match'));
                  },
                }),
              ]}
            >
              <Input.Password 
                prefix={<LockOutlined />} 
                placeholder="Confirm your password"
                size="large"
              />
            </Form.Item>

            <Form.Item style={{ marginBottom: 0 }}>
              <Button 
                type="primary" 
                htmlType="submit" 
                size="large"
                loading={isLoading}
                block
                style={{ 
                  height: 48,
                  background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
                  border: 'none'
                }}
              >
                Create Admin Account
              </Button>
            </Form.Item>
          </Form>

          <div style={{ textAlign: 'center' }}>
            <Text type="secondary" style={{ fontSize: '12px' }}>
              This will be the primary administrator account for this application.
              Make sure to use a strong, unique password.
            </Text>
          </div>
        </Space>
      </Card>
    </div>
  );
};

export default AdminSetup;