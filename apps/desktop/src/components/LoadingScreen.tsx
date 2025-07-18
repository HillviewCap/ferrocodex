import React from 'react';
import { Spin, Typography, Space } from 'antd';
import { LockOutlined } from '@ant-design/icons';

const { Title, Text } = Typography;

interface LoadingScreenProps {
  message?: string;
}

const LoadingScreen: React.FC<LoadingScreenProps> = ({ 
  message = "Initializing Ferrocodex..." 
}) => {
  return (
    <div style={{ 
      height: '100vh', 
      display: 'flex', 
      justifyContent: 'center', 
      alignItems: 'center',
      background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)'
    }}>
      <Space direction="vertical" size="large" style={{ textAlign: 'center' }}>
        <LockOutlined style={{ fontSize: '64px', color: '#fff' }} />
        <Title level={2} style={{ color: '#fff', margin: 0 }}>
          Ferrocodex
        </Title>
        <Space direction="vertical" size={16}>
          <Spin size="large" style={{ color: '#fff' }} />
          <Text style={{ color: '#fff', fontSize: '16px' }}>
            {message}
          </Text>
        </Space>
      </Space>
    </div>
  );
};

export default LoadingScreen;