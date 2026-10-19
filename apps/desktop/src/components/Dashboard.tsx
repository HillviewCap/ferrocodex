import React, { useState, useEffect } from 'react';
import { Layout, Menu, Avatar, Dropdown, Button, Typography, Space, Card, Row, Col, Alert, message } from 'antd';
import { 
  UserOutlined, 
  LogoutOutlined, 
  DashboardOutlined,
  SettingOutlined,
  LockOutlined,
  TeamOutlined,
  DatabaseOutlined,
  ImportOutlined,
  QuestionCircleOutlined,
  SafetyCertificateOutlined,
  AuditOutlined,
  SearchOutlined
} from '@ant-design/icons';
import useAuthStore from '../store/auth';
import useAssetStore from '../store/assets';
import { invoke } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';
import { DashboardStats } from '../types/dashboard';
import { AssetInfo } from '../types/assets';
import useKeyRotationStore from '../store/keyRotation';
import KeyRotationModal from './KeyRotationModal';
import { KEY_ROTATION_GATE_MESSAGE } from '../utils/keyRotationErrors';
import UserManagement from './UserManagement';
import AssetManagement from './AssetManagement';
import StandaloneCredentials from './StandaloneCredentials';
import RetryPreferences from './RetryPreferences';
import MetadataManagement from './metadata/MetadataManagement';
import MetadataSearchPage from './search/MetadataSearchPage';
import { canAccessUserManagement } from '../utils/roleUtils';
import SecurityMonitoringDashboard from './security/SecurityMonitoringDashboard';
import SecurityAuditLog from './security/SecurityAuditLog';
import BulkManagement from './bulk/BulkManagement';

const { Header, Sider, Content } = Layout;
const { Title, Text } = Typography;

const Dashboard: React.FC = () => {
  const { user, logout, token, keyRotationInProgress } = useAuthStore();
  const rotationStatus = useKeyRotationStore((state) => state.status);
  const fetchRotationStatus = useKeyRotationStore((state) => state.fetchStatus);
  const [rotationModalOpen, setRotationModalOpen] = useState(false);
  const navigateToHistory = useAssetStore((state) => state.navigateToHistory);
  const [selectedMenuItem, setSelectedMenuItem] = useState('dashboard');
  const [dashboardStats, setDashboardStats] = useState<DashboardStats | null>(null);
  const [statsLoading, setStatsLoading] = useState(true);

  useEffect(() => {
    if (token) {
      fetchRotationStatus(token);
    }
  }, [token, fetchRotationStatus]);

  // The fetched status is authoritative; the login flag covers the moment before it arrives
  const rotationActive = rotationStatus ? rotationStatus.in_progress : keyRotationInProgress === true;

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

  // Open an asset picked from metadata search results in the Assets screen (history view)
  const openAssetFromSearch = async (assetId: number) => {
    if (!token) return;

    try {
      const asset = await invoke<AssetInfo>('get_asset_details', { token, assetId });
      navigateToHistory(asset);
      setSelectedMenuItem('assets');
    } catch (error) {
      console.error('Failed to open asset from search:', error);
      message.error('Failed to open the selected asset');
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

  const userMenuItems = React.useMemo(() => [
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
  ], []);

  const sidebarMenuItems = React.useMemo(() => {
    const items: any[] = [
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
        key: 'standalone-credentials',
        icon: <LockOutlined />,
        label: 'Vault',
      },
    ];

    // Add Bulk Operations menu item
    items.push({
      key: 'bulk-operations',
      icon: <ImportOutlined />,
      label: 'Bulk Operations',
    });

    // Metadata search is available to every role (read-only lookup of assets by metadata)
    items.push({
      key: 'metadata-search',
      icon: <SearchOutlined />,
      label: 'Metadata Search',
    });

    // Add Metadata menu for administrators only
    // TEMPORARILY HIDDEN: Metadata manager hidden until field testing or final decision
    // if (canAccessUserManagement(user)) {
    //   items.push({
    //     key: 'metadata',
    //     icon: <FileTextOutlined />,
    //     label: 'Metadata',
    //     children: [
    //       {
    //         key: 'schema-designer',
    //         icon: <AppstoreOutlined />,
    //         label: 'Schema Designer',
    //       },
    //       {
    //         key: 'field-templates',
    //         icon: <FileTextOutlined />,
    //         label: 'Field Templates',
    //       },
    //       {
    //         key: 'metadata-search',
    //         icon: <SearchOutlined />,
    //         label: 'Metadata Search',
    //       },
    //     ],
    //   });
    // }

    // Add Security menu for administrators only
    if (canAccessUserManagement(user)) {
      items.push({
        key: 'security',
        icon: <SafetyCertificateOutlined />,
        label: 'Security',
        children: [
          {
            key: 'security-dashboard',
            icon: <SafetyCertificateOutlined />,
            label: 'Security Dashboard',
          },
          {
            key: 'audit-log',
            icon: <AuditOutlined />,
            label: 'Audit Log',
          },
        ],
      });
    }

    // Add User Management for administrators only
    if (canAccessUserManagement(user)) {
      items.push({
        key: 'user-management',
        icon: <TeamOutlined />,
        label: 'User Management',
      });
    }

    // Add divider and help
    items.push(
      {
        type: 'divider' as const,
      },
      {
        key: 'help',
        icon: <QuestionCircleOutlined />,
        label: 'Help',
        onClick: handleOpenDocumentation,
      }
    );

    return items;
  }, [user]);

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
            menu={{ 
              items: userMenuItems,
              onClick: ({ key }) => {
                if (key === 'settings') {
                  setSelectedMenuItem('settings');
                } else if (key === 'profile') {
                  setSelectedMenuItem('profile');
                }
              }
            }}
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
          {rotationActive && (
            <Alert
              type="warning"
              showIcon
              style={{ marginBottom: 16 }}
              title="Encryption key rotation in progress"
              description={
                user?.role === 'Administrator'
                  ? 'Encrypted data is unavailable to other users until you finish the rotation.'
                  : KEY_ROTATION_GATE_MESSAGE
              }
              action={
                user?.role === 'Administrator' ? (
                  <Button size="small" type="primary" onClick={() => setRotationModalOpen(true)}>
                    Resume
                  </Button>
                ) : undefined
              }
            />
          )}
          <Content
            style={{
              background: '#fff',
              padding: (selectedMenuItem === 'user-management' || selectedMenuItem === 'assets' || selectedMenuItem === 'standalone-credentials' || selectedMenuItem === 'settings' || selectedMenuItem === 'schema-designer' || selectedMenuItem === 'field-templates' || selectedMenuItem === 'security-dashboard' || selectedMenuItem === 'audit-log' || selectedMenuItem === 'bulk-operations') ? '0' : '24px',
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
            ) : selectedMenuItem === 'standalone-credentials' ? (
              <StandaloneCredentials />
            ) : selectedMenuItem === 'settings' ? (
              <RetryPreferences />
            ) : selectedMenuItem === 'metadata-search' ? (
              <MetadataSearchPage embedded onAssetSelect={openAssetFromSearch} />
            ) : (selectedMenuItem === 'schema-designer' || selectedMenuItem === 'field-templates') ? (
              <MetadataManagement />
            ) : selectedMenuItem === 'security-dashboard' && canAccessUserManagement(user) ? (
              <SecurityMonitoringDashboard />
            ) : selectedMenuItem === 'audit-log' && canAccessUserManagement(user) ? (
              <SecurityAuditLog />
            ) : selectedMenuItem === 'bulk-operations' ? (
              <BulkManagement />
            ) : selectedMenuItem === 'profile' ? (
              <div style={{ textAlign: 'center', padding: '48px' }}>
                <Title level={3}>User Profile</Title>
                <Text type="secondary">Profile management is under development</Text>
              </div>
            ) : selectedMenuItem === 'dashboard' ? (
              <Space orientation="vertical" size="large" style={{ width: '100%' }}>
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
                      styles={{ body: { 
                        display: 'flex', 
                        flexDirection: 'column', 
                        justifyContent: 'center',
                        height: '100%'
                      } }}
                      onClick={() => setSelectedMenuItem('assets')}
                    >
                      <DatabaseOutlined style={{ fontSize: '32px', color: '#52c41a', marginBottom: '12px' }} />
                      <Title level={4} style={{ margin: '0 0 8px 0' }}>Assets</Title>
                      <Text type="secondary">Manage configuration assets</Text>
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
      {user?.role === 'Administrator' && (
        <KeyRotationModal
          open={rotationModalOpen}
          onClose={() => {
            setRotationModalOpen(false);
            if (token) fetchRotationStatus(token);
          }}
        />
      )}
    </Layout>
  );
};

export default Dashboard;