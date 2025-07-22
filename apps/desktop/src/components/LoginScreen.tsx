import React from 'react';
import { Form, Input, Button, Card, Typography, Alert, Space, Checkbox } from 'antd';
import { UserOutlined, LockOutlined } from '@ant-design/icons';
import useAuthStore from '../store/auth';
import ferrocodexLogo from '../assets/ferrocodex-logo.png';

const { Title, Text } = Typography;

interface LoginFormValues {
  username: string;
  password: string;
  remember: boolean;
}

const LoginScreen: React.FC = () => {
  const { login, isLoading, error, clearError } = useAuthStore();
  const [form] = Form.useForm();

  const onFinish = async (values: LoginFormValues) => {
    try {
      clearError();
      await login(values.username, values.password);
    } catch (error) {
      console.error('Login failed:', error);
    }
  };

  return (
    <div style={{ 
      height: '100vh', 
      display: 'flex', 
      justifyContent: 'center', 
      alignItems: 'center',
      background: 'linear-gradient(135deg, #003049 0%, #669bbc 100%)'
    }}>
      <Card 
        style={{ 
          width: 400, 
          boxShadow: '0 4px 20px rgba(0,0,0,0.1)',
          backgroundColor: '#fdf0d5'
        }}
      >
        <Space direction="vertical" size="large" style={{ width: '100%' }}>
          <div style={{ textAlign: 'center' }}>
            <img 
              src={ferrocodexLogo} 
              alt="Ferrocodex Logo" 
              style={{ 
                height: 'auto', 
                maxWidth: '80%',
                width: 'auto', 
                marginBottom: 0,
                display: 'block',
                margin: '0 auto 0px'
              }} 
            />
            <Text type="secondary">
              Sign in to your account
            </Text>
          </div>

          {error && (
            <Alert
              message="Login Failed"
              description={error}
              type="error"
              showIcon
              closable
              onClose={clearError}
            />
          )}

          <Form
            form={form}
            name="login"
            onFinish={onFinish}
            autoComplete="off"
            layout="vertical"
            requiredMark={false}
          >
            <Form.Item
              label="Username"
              name="username"
              rules={[
                { required: true, message: 'Please enter your username' },
              ]}
            >
              <Input 
                prefix={<UserOutlined />} 
                placeholder="Username"
                size="large"
                autoFocus
              />
            </Form.Item>

            <Form.Item
              label="Password"
              name="password"
              rules={[
                { required: true, message: 'Please enter your password' },
              ]}
            >
              <Input.Password 
                prefix={<LockOutlined />} 
                placeholder="Password"
                size="large"
              />
            </Form.Item>

            <Form.Item>
              <Form.Item name="remember" valuePropName="checked" noStyle>
                <Checkbox>Remember me</Checkbox>
              </Form.Item>
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
                  backgroundColor: '#003049',
                  border: 'none'
                }}
              >
                Sign In
              </Button>
            </Form.Item>
          </Form>

          <div style={{ textAlign: 'center' }}>
            <Text type="secondary" style={{ fontSize: '12px' }}>
              Secure password management and encrypted storage
            </Text>
          </div>
        </Space>
      </Card>
    </div>
  );
};

export default LoginScreen;