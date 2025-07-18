import React, { useState } from 'react';
import { Layout, Menu, Avatar, Dropdown, Button, Typography, Space, Card, Row, Col } from 'antd';
import { 
  UserOutlined, 
  LogoutOutlined, 
  DashboardOutlined,
  SettingOutlined,
  KeyOutlined,
  LockOutlined,
  SafetyOutlined,
  TeamOutlined
} from '@ant-design/icons';
import useAuthStore from '../store/auth';
import UserManagement from './UserManagement';
import { canAccessUserManagement } from '../utils/roleUtils';

const { Header, Sider, Content } = Layout;
const { Title, Text } = Typography;

const Dashboard: React.FC = () => {
  const { user, logout } = useAuthStore();
  const [selectedMenuItem, setSelectedMenuItem] = useState('dashboard');

  const handleLogout = async () => {
    try {
      await logout();
    } catch (error) {
      console.error('Logout failed:', error);
    }
  };

  const userMenuItems = [
    {
      key: 'profile',
      icon: <UserOutlined />,
      label: 'Profile',
    },
    {
      key: 'settings',
      icon: <SettingOutlined />,
      label: 'Settings',
    },
    {
      type: 'divider' as const,
    },
    {
      key: 'logout',
      icon: <LogoutOutlined />,
      label: 'Logout',
      onClick: handleLogout,
    },
  ];

  const sidebarMenuItems = [
    {
      key: 'dashboard',
      icon: <DashboardOutlined />,
      label: 'Dashboard',
    },
    {
      key: 'passwords',
      icon: <KeyOutlined />,
      label: 'Passwords',
    },
    {
      key: 'secure-notes',
      icon: <LockOutlined />,
      label: 'Secure Notes',
    },
    {
      key: 'security',
      icon: <SafetyOutlined />,
      label: 'Security',
    },
    // Add User Management for administrators only
    ...(canAccessUserManagement(user) ? [{
      key: 'user-management',
      icon: <TeamOutlined />,
      label: 'User Management',
    }] : []),
  ];

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Header 
        style={{ 
          background: '#fff', 
          padding: '0 24px',
          borderBottom: '1px solid #f0f0f0',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center'
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center' }}>
          <LockOutlined style={{ fontSize: '24px', color: '#667eea', marginRight: '12px' }} />
          <Title level={4} style={{ margin: 0, color: '#667eea' }}>
            Ferrocodex
          </Title>
        </div>
        
        <Space>
          <Text>Welcome, {user?.username}</Text>
          <Dropdown 
            menu={{ items: userMenuItems }}
            placement="bottomRight"
            arrow
          >
            <Button type="text" style={{ padding: '4px 8px' }}>
              <Avatar 
                size="small" 
                icon={<UserOutlined />} 
                style={{ backgroundColor: '#667eea' }}
              />
              <span style={{ marginLeft: '8px' }}>{user?.role}</span>
            </Button>
          </Dropdown>
        </Space>
      </Header>

      <Layout>
        <Sider 
          width={250} 
          style={{ 
            background: '#fff',
            borderRight: '1px solid #f0f0f0'
          }}
        >
          <Menu
            mode="inline"
            selectedKeys={[selectedMenuItem]}
            items={sidebarMenuItems}
            onClick={({ key }) => setSelectedMenuItem(key)}
            style={{ 
              height: '100%', 
              borderRight: 0,
              paddingTop: '16px'
            }}
          />
        </Sider>

        <Layout style={{ padding: '24px' }}>
          <Content
            style={{
              background: '#fff',
              padding: selectedMenuItem === 'user-management' ? '0' : '24px',
              margin: 0,
              minHeight: 280,
              borderRadius: '8px',
              boxShadow: '0 1px 3px rgba(0,0,0,0.1)'
            }}
          >
            {selectedMenuItem === 'user-management' && canAccessUserManagement(user) ? (
              <UserManagement />
            ) : selectedMenuItem === 'dashboard' ? (
              <Space direction="vertical" size="large" style={{ width: '100%' }}>
                <div>
                  <Title level={2} style={{ marginBottom: '8px' }}>
                    Welcome to Ferrocodex
                  </Title>
                  <Text type="secondary">
                    Your secure password management and encrypted storage solution
                  </Text>
                </div>

                <Row gutter={[16, 16]}>
                  <Col xs={24} sm={12} lg={8}>
                    <Card 
                      hoverable
                      style={{ textAlign: 'center', height: '160px' }}
                      bodyStyle={{ 
                        display: 'flex', 
                        flexDirection: 'column', 
                        justifyContent: 'center',
                        height: '100%'
                      }}
                    >
                      <KeyOutlined style={{ fontSize: '32px', color: '#52c41a', marginBottom: '12px' }} />
                      <Title level={4} style={{ margin: '0 0 8px 0' }}>Passwords</Title>
                      <Text type="secondary">Manage your passwords</Text>
                    </Card>
                  </Col>

                  <Col xs={24} sm={12} lg={8}>
                    <Card 
                      hoverable
                      style={{ textAlign: 'center', height: '160px' }}
                      bodyStyle={{ 
                        display: 'flex', 
                        flexDirection: 'column', 
                        justifyContent: 'center',
                        height: '100%'
                      }}
                    >
                      <LockOutlined style={{ fontSize: '32px', color: '#1890ff', marginBottom: '12px' }} />
                      <Title level={4} style={{ margin: '0 0 8px 0' }}>Secure Notes</Title>
                      <Text type="secondary">Store encrypted notes</Text>
                    </Card>
                  </Col>

                  <Col xs={24} sm={12} lg={8}>
                    <Card 
                      hoverable
                      style={{ textAlign: 'center', height: '160px' }}
                      bodyStyle={{ 
                        display: 'flex', 
                        flexDirection: 'column', 
                        justifyContent: 'center',
                        height: '100%'
                      }}
                    >
                      <SafetyOutlined style={{ fontSize: '32px', color: '#fa8c16', marginBottom: '12px' }} />
                      <Title level={4} style={{ margin: '0 0 8px 0' }}>Security</Title>
                      <Text type="secondary">Security settings</Text>
                    </Card>
                  </Col>
                </Row>

                <Card style={{ marginTop: '24px' }}>
                  <Title level={4}>Quick Stats</Title>
                  <Row gutter={[32, 16]}>
                    <Col span={8}>
                      <div style={{ textAlign: 'center' }}>
                        <Title level={2} style={{ color: '#52c41a', margin: 0 }}>0</Title>
                        <Text type="secondary">Stored Passwords</Text>
                      </div>
                    </Col>
                    <Col span={8}>
                      <div style={{ textAlign: 'center' }}>
                        <Title level={2} style={{ color: '#1890ff', margin: 0 }}>0</Title>
                        <Text type="secondary">Secure Notes</Text>
                      </div>
                    </Col>
                    <Col span={8}>
                      <div style={{ textAlign: 'center' }}>
                        <Title level={2} style={{ color: '#fa8c16', margin: 0 }}>Strong</Title>
                        <Text type="secondary">Security Level</Text>
                      </div>
                    </Col>
                  </Row>
                </Card>
              </Space>
            ) : (
              <div style={{ textAlign: 'center', padding: '48px' }}>
                <Title level={3}>Coming Soon</Title>
                <Text type="secondary">This feature is under development</Text>
              </div>
            )}
          </Content>
        </Layout>
      </Layout>
    </Layout>
  );
};

export default Dashboard;