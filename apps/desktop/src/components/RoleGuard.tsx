import React from 'react';
import { Alert, Button, Space, Typography } from 'antd';
import { ExclamationCircleOutlined, ArrowLeftOutlined } from '@ant-design/icons';
import useAuthStore from '../store/auth';
import type { UserInfo } from '../store/auth';

const { Title, Text } = Typography;

interface RoleGuardProps {
  children: React.ReactNode;
  allowedRoles: ('Administrator' | 'Engineer')[];
  fallback?: React.ReactNode;
  requireActive?: boolean;
}

const RoleGuard: React.FC<RoleGuardProps> = ({
  children,
  allowedRoles,
  fallback,
  requireActive = true,
}) => {
  const { user } = useAuthStore();

  const hasAccess = (currentUser: UserInfo | null): boolean => {
    if (!currentUser) return false;
    if (requireActive && !currentUser.is_active) return false;
    return allowedRoles.includes(currentUser.role);
  };

  if (!hasAccess(user)) {
    if (fallback) {
      return <>{fallback}</>;
    }

    return (
      <div style={{ 
        padding: '48px 24px', 
        textAlign: 'center',
        background: '#fff',
        borderRadius: '8px',
        boxShadow: '0 1px 3px rgba(0,0,0,0.1)',
        margin: '24px'
      }}>
        <Space direction="vertical" size="large" style={{ width: '100%' }}>
          <ExclamationCircleOutlined 
            style={{ 
              fontSize: '48px', 
              color: '#ff4d4f',
              marginBottom: '16px'
            }} 
          />
          
          <Title level={3} style={{ color: '#ff4d4f', marginBottom: '8px' }}>
            Access Denied
          </Title>
          
          <Text type="secondary" style={{ fontSize: '16px', marginBottom: '24px' }}>
            You do not have permission to access this feature.
            {allowedRoles.length === 1 
              ? ` Only ${allowedRoles[0]}s can access this area.`
              : ` Only ${allowedRoles.join(' and ')} users can access this area.`}
          </Text>

          <Alert
            message="Permission Required"
            description={`This feature requires ${allowedRoles.join(' or ')} role access. If you believe this is an error, please contact your administrator.`}
            type="warning"
            showIcon
            style={{ textAlign: 'left', marginBottom: '24px' }}
          />

          <Button 
            type="primary" 
            icon={<ArrowLeftOutlined />}
            onClick={() => window.history.back()}
            size="large"
          >
            Go Back
          </Button>
        </Space>
      </div>
    );
  }

  return <>{children}</>;
};

export default RoleGuard;