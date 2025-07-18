import React, { useEffect, useState } from 'react';
import { 
  Typography, 
  Button, 
  Table, 
  Space, 
  Modal, 
  Form, 
  Input, 
  message, 
  Popconfirm,
  Tag,
  Card,
  Row,
  Col,
  Alert,
  Spin
} from 'antd';
import { 
  PlusOutlined, 
  UserOutlined, 
  StopOutlined,
  PlayCircleOutlined,
  ExclamationCircleOutlined
} from '@ant-design/icons';
import useAuthStore from '../store/auth';
import useUserManagementStore from '../store/userManagement';
import RoleGuard from './RoleGuard';
import type { UserInfo } from '../store/auth';

const { Title, Text } = Typography;

interface CreateUserFormData {
  username: string;
  password: string;
  confirmPassword: string;
}

const UserManagement: React.FC = () => {
  const { token } = useAuthStore();
  const { 
    users, 
    isLoading, 
    error, 
    isCreatingUser, 
    createUserError,
    fetchUsers, 
    createEngineerUser, 
    deactivateUser, 
    reactivateUser,
    clearError,
    clearCreateUserError
  } = useUserManagementStore();
  
  const [isCreateModalVisible, setIsCreateModalVisible] = useState(false);
  const [createUserForm] = Form.useForm<CreateUserFormData>();

  useEffect(() => {
    if (token) {
      fetchUsers(token).catch(console.error);
    }
  }, [token, fetchUsers]);

  const handleCreateUser = async (values: CreateUserFormData) => {
    if (!token) {
      message.error('Authentication required');
      return;
    }

    try {
      await createEngineerUser(token, values.username, values.password);
      message.success('Engineer account created successfully');
      setIsCreateModalVisible(false);
      createUserForm.resetFields();
    } catch (error) {
      message.error('Failed to create engineer account');
    }
  };

  const handleDeactivateUser = async (userId: number, username: string) => {
    if (!token) {
      message.error('Authentication required');
      return;
    }

    try {
      await deactivateUser(token, userId);
      message.success(`User ${username} deactivated successfully`);
    } catch (error) {
      message.error('Failed to deactivate user');
    }
  };

  const handleReactivateUser = async (userId: number, username: string) => {
    if (!token) {
      message.error('Authentication required');
      return;
    }

    try {
      await reactivateUser(token, userId);
      message.success(`User ${username} reactivated successfully`);
    } catch (error) {
      message.error('Failed to reactivate user');
    }
  };

  const handleRefresh = () => {
    if (token) {
      fetchUsers(token).catch(console.error);
    }
  };

  const columns = [
    {
      title: 'Username',
      dataIndex: 'username',
      key: 'username',
      render: (text: string) => (
        <Space>
          <UserOutlined />
          <Text strong>{text}</Text>
        </Space>
      ),
    },
    {
      title: 'Role',
      dataIndex: 'role',
      key: 'role',
      render: (role: string) => (
        <Tag color={role === 'Administrator' ? 'red' : 'blue'}>
          {role}
        </Tag>
      ),
    },
    {
      title: 'Status',
      dataIndex: 'is_active',
      key: 'is_active',
      render: (isActive: boolean) => (
        <Tag color={isActive ? 'green' : 'gray'}>
          {isActive ? 'Active' : 'Inactive'}
        </Tag>
      ),
    },
    {
      title: 'Created',
      dataIndex: 'created_at',
      key: 'created_at',
      render: (date: string) => {
        if (!date) return 'N/A';
        return new Date(date).toLocaleDateString();
      },
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_: any, record: UserInfo) => (
        <Space size="middle">
          {record.is_active ? (
            <Popconfirm
              title="Deactivate User"
              description={`Are you sure you want to deactivate ${record.username}?`}
              onConfirm={() => handleDeactivateUser(record.id, record.username)}
              okText="Yes"
              cancelText="No"
              icon={<ExclamationCircleOutlined style={{ color: 'red' }} />}
            >
              <Button 
                type="text" 
                danger 
                size="small"
                icon={<StopOutlined />}
                disabled={record.role === 'Administrator'}
              >
                Deactivate
              </Button>
            </Popconfirm>
          ) : (
            <Button 
              type="text" 
              size="small"
              icon={<PlayCircleOutlined />}
              onClick={() => handleReactivateUser(record.id, record.username)}
              style={{ color: '#52c41a' }}
            >
              Reactivate
            </Button>
          )}
        </Space>
      ),
    },
  ];

  const activeUsers = users.filter(user => user.is_active);
  const engineerUsers = users.filter(user => user.role === 'Engineer');
  const adminUsers = users.filter(user => user.role === 'Administrator');

  return (
    <RoleGuard allowedRoles={['Administrator']}>
      <div style={{ padding: '24px' }}>
        <div style={{ marginBottom: '24px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
          <Title level={2} style={{ margin: 0 }}>User Management</Title>
          <Space>
            <Button 
              type="default" 
              onClick={handleRefresh}
              loading={isLoading}
            >
              Refresh
            </Button>
            <Button 
              type="primary" 
              icon={<PlusOutlined />}
              onClick={() => setIsCreateModalVisible(true)}
            >
              Create Engineer Account
            </Button>
          </Space>
        </div>
        
        <Text type="secondary">
          Manage engineer accounts, including creating new accounts and controlling user access.
        </Text>
      </div>

      {error && (
        <Alert
          message="Error"
          description={error}
          type="error"
          showIcon
          closable
          onClose={clearError}
          style={{ marginBottom: '24px' }}
        />
      )}

      {/* Quick Stats */}
      <Row gutter={[16, 16]} style={{ marginBottom: '24px' }}>
        <Col xs={24} sm={12} md={6}>
          <Card>
            <div style={{ textAlign: 'center' }}>
              <Title level={2} style={{ color: '#1890ff', margin: 0 }}>
                {users.length}
              </Title>
              <Text type="secondary">Total Users</Text>
            </div>
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card>
            <div style={{ textAlign: 'center' }}>
              <Title level={2} style={{ color: '#52c41a', margin: 0 }}>
                {activeUsers.length}
              </Title>
              <Text type="secondary">Active Users</Text>
            </div>
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card>
            <div style={{ textAlign: 'center' }}>
              <Title level={2} style={{ color: '#fa8c16', margin: 0 }}>
                {engineerUsers.length}
              </Title>
              <Text type="secondary">Engineers</Text>
            </div>
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6}>
          <Card>
            <div style={{ textAlign: 'center' }}>
              <Title level={2} style={{ color: '#f5222d', margin: 0 }}>
                {adminUsers.length}
              </Title>
              <Text type="secondary">Administrators</Text>
            </div>
          </Card>
        </Col>
      </Row>

      {/* Users Table */}
      <Card>
        <Spin spinning={isLoading}>
          <Table
            columns={columns}
            dataSource={users}
            rowKey="id"
            pagination={{
              pageSize: 10,
              showSizeChanger: true,
              showQuickJumper: true,
              showTotal: (total, range) => 
                `${range[0]}-${range[1]} of ${total} users`,
            }}
            locale={{ emptyText: 'No users found' }}
          />
        </Spin>
      </Card>

      {/* Create User Modal */}
      <Modal
        title="Create Engineer Account"
        open={isCreateModalVisible}
        onCancel={() => {
          setIsCreateModalVisible(false);
          createUserForm.resetFields();
          clearCreateUserError();
        }}
        footer={null}
        width={500}
      >
        <Form
          form={createUserForm}
          layout="vertical"
          onFinish={handleCreateUser}
          autoComplete="off"
        >
          {createUserError && (
            <Alert
              message="Error"
              description={createUserError}
              type="error"
              showIcon
              closable
              onClose={clearCreateUserError}
              style={{ marginBottom: '16px' }}
            />
          )}

          <Form.Item
            label="Username"
            name="username"
            rules={[
              { required: true, message: 'Please input username' },
              { min: 3, message: 'Username must be at least 3 characters' },
              { max: 50, message: 'Username cannot exceed 50 characters' },
              { pattern: /^[a-zA-Z0-9_-]+$/, message: 'Username can only contain letters, numbers, hyphens, and underscores' },
            ]}
          >
            <Input 
              placeholder="Enter username"
              prefix={<UserOutlined />}
            />
          </Form.Item>

          <Form.Item
            label="Initial Password"
            name="password"
            rules={[
              { required: true, message: 'Please input password' },
              { min: 8, message: 'Password must be at least 8 characters' },
            ]}
          >
            <Input.Password 
              placeholder="Enter initial password"
              autoComplete="new-password"
            />
          </Form.Item>

          <Form.Item
            label="Confirm Password"
            name="confirmPassword"
            dependencies={['password']}
            rules={[
              { required: true, message: 'Please confirm password' },
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
              placeholder="Confirm password"
              autoComplete="new-password"
            />
          </Form.Item>

          <Form.Item style={{ marginBottom: 0 }}>
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px' }}>
              <Button 
                onClick={() => {
                  setIsCreateModalVisible(false);
                  createUserForm.resetFields();
                  clearCreateUserError();
                }}
              >
                Cancel
              </Button>
              <Button 
                type="primary" 
                htmlType="submit"
                loading={isCreatingUser}
              >
                Create Account
              </Button>
            </div>
          </Form.Item>
        </Form>
      </Modal>
      </div>
    </RoleGuard>
  );
};

export default UserManagement;