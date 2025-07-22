import React, { useState, useEffect } from 'react';
import { Layout, Menu, Avatar, Dropdown, Button, Typography, Space, Card, Row, Col, message } from 'antd';
import { 
  UserOutlined, 
  LogoutOutlined, 
  DashboardOutlined,
  SettingOutlined,
  KeyOutlined,
  LockOutlined,
  SafetyOutlined,
  TeamOutlined,
  DatabaseOutlined,
  ImportOutlined,
  QuestionCircleOutlined
} from '@ant-design/icons';
import useAuthStore from '../store/auth';
import { invoke } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';
import { DashboardStats } from '../types/dashboard';
import UserManagement from './UserManagement';
import AssetManagement from './AssetManagement';
import { canAccessUserManagement } from '../utils/roleUtils';

const { Header, Sider, Content } = Layout;
const { Title, Text } = Typography;

const Dashboard: React.FC = () => {
  const { user, logout, token } = useAuthStore();
  const [selectedMenuItem, setSelectedMenuItem] = useState('dashboard');
  const [dashboardStats, setDashboardStats] = useState<DashboardStats | null>(null);
  const [statsLoading, setStatsLoading] = useState(true);

  useEffect(() => {
    if (selectedMenuItem === 'dashboard' && token) {
      fetchDashboardStats();
    }
  }, [selectedMenuItem, token]);

  const fetchDashboardStats = async () => {
    if (!token) return;
    
    setStatsLoading(true);
    try {
      const stats = await invoke<DashboardStats>('get_dashboard_stats', {
        token: token
      });
      setDashboardStats(stats);
    } catch (error) {
      console.error('Failed to fetch dashboard stats:', error);
    } finally {
      setStatsLoading(false);
    }
  };

  const handleLogout = async () => {
    try {
      await logout();
    } catch (error) {
      console.error('Logout failed:', error);
    }
  };

  const handleOpenDocumentation = async () => {
    try {
      await openUrl('https://ferrocodex.readthedocs.io');
    } catch (error) {
      console.error('Failed to open documentation:', error);
      message.error('Failed to open documentation. Please visit https://ferrocodex.readthedocs.io manually.');
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
      key: 'assets',
      icon: <DatabaseOutlined />,
      label: 'Assets',
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
    {
      type: 'divider' as const,
    },
    {
      key: 'help',
      icon: <QuestionCircleOutlined />,
      label: 'Help',
      onClick: handleOpenDocumentation,
    },
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
          <LockOutlined style={{ fontSize: '24px', color: '#003049', marginRight: '12px' }} />
          <Title level={4} style={{ margin: 0, color: '#003049' }}>
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
                style={{ backgroundColor: '#003049' }}
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
            onClick={({ key }) => {
              // Don't change selected menu item for help
              if (key !== 'help') {
                setSelectedMenuItem(key);
              }
            }}
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
              padding: (selectedMenuItem === 'user-management' || selectedMenuItem === 'assets') ? '0' : '24px',
              margin: 0,
              minHeight: 280,
              borderRadius: '8px',
              boxShadow: '0 1px 3px rgba(0,0,0,0.1)'
            }}
          >
            {selectedMenuItem === 'user-management' && canAccessUserManagement(user) ? (
              <UserManagement />
            ) : selectedMenuItem === 'assets' ? (
              <AssetManagement />
            ) : selectedMenuItem === 'dashboard' ? (
              <Space direction="vertical" size="large" style={{ width: '100%' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <div>
                    <Title level={2} style={{ marginBottom: '8px' }}>
                      Welcome to Ferrocodex
                    </Title>
                    <Text type="secondary">
                      Your secure configuration management and versioning solution
                    </Text>
                  </div>
                  <Button 
                    type="primary" 
                    icon={<ImportOutlined />}
                    onClick={() => setSelectedMenuItem('assets')}
                    size="large"
                  >
                    Import Configuration
                  </Button>
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
                      onClick={() => setSelectedMenuItem('assets')}
                    >
                      <DatabaseOutlined style={{ fontSize: '32px', color: '#52c41a', marginBottom: '12px' }} />
                      <Title level={4} style={{ margin: '0 0 8px 0' }}>Assets</Title>
                      <Text type="secondary">Manage configuration assets</Text>
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
                      <KeyOutlined style={{ fontSize: '32px', color: '#1890ff', marginBottom: '12px' }} />
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
                      <SafetyOutlined style={{ fontSize: '32px', color: '#fa8c16', marginBottom: '12px' }} />
                      <Title level={4} style={{ margin: '0 0 8px 0' }}>Security</Title>
                      <Text type="secondary">Security settings</Text>
                    </Card>
                  </Col>
                </Row>

                <Card style={{ marginTop: '24px' }} loading={statsLoading}>
                  <Title level={4}>Quick Stats</Title>
                  <Row gutter={[32, 16]}>
                    <Col span={8}>
                      <div style={{ textAlign: 'center' }}>
                        <Title level={2} style={{ color: '#52c41a', margin: 0 }}>
                          {dashboardStats?.total_assets ?? 0}
                        </Title>
                        <Text type="secondary">Configuration Assets</Text>
                      </div>
                    </Col>
                    <Col span={8}>
                      <div style={{ textAlign: 'center' }}>
                        <Title level={2} style={{ color: '#1890ff', margin: 0 }}>
                          {dashboardStats?.total_versions ?? 0}
                        </Title>
                        <Text type="secondary">Total Versions</Text>
                      </div>
                    </Col>
                    <Col span={8}>
                      <div style={{ textAlign: 'center' }}>
                        <Title level={2} style={{ color: '#fa8c16', margin: 0 }}>
                          {dashboardStats?.encryption_type ?? 'AES-256'}
                        </Title>
                        <Text type="secondary">Encryption</Text>
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