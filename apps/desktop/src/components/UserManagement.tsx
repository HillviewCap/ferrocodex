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
  Spin,
  Tooltip
} from 'antd';
import { 
  PlusOutlined, 
  UserOutlined, 
  StopOutlined,
  PlayCircleOutlined,
  ExclamationCircleOutlined,
  KeyOutlined,
  LockOutlined,
  SyncOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../store/auth';
import useUserManagementStore from '../store/userManagement';
import useKeyRotationStore from '../store/keyRotation';
import RoleGuard from './RoleGuard';
import KeyRotationModal from './KeyRotationModal';
import type { UserInfo } from '../store/auth';

const { Title, Text } = Typography;

interface TemporaryPasswordFormData {
  password: string;
  confirmPassword: string;
}

type TemporaryPasswordTarget = {
  mode: 'reactivate' | 'reset';
  user: UserInfo;
};

interface CreateUserFormData {
  username: string;
  password: string;
  confirmPassword: string;
}

const UserManagement: React.FC = () => {
  const { token, user: currentUser, showRecoveryCode } = useAuthStore();
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
    resetUserPassword,
    clearError,
    clearCreateUserError
  } = useUserManagementStore();
  
  const [isCreateModalVisible, setIsCreateModalVisible] = useState(false);
  const [createUserForm] = Form.useForm<CreateUserFormData>();
  const [passwordTarget, setPasswordTarget] = useState<TemporaryPasswordTarget | null>(null);
  const [isSubmittingPassword, setIsSubmittingPassword] = useState(false);
  const [passwordForm] = Form.useForm<TemporaryPasswordFormData>();
  const [isRotationModalOpen, setIsRotationModalOpen] = useState(false);
  const rotationStatus = useKeyRotationStore((state) => state.status);
  const fetchRotationStatus = useKeyRotationStore((state) => state.fetchStatus);

  const usersWithoutEncryptionAccess = new Set<number>([
    ...(rotationStatus?.details?.users_without_access ?? []).map((u) => u.id),
    ...(rotationStatus?.details?.users_without_keypair ?? []).map((u) => u.id),
  ]);

  useEffect(() => {
    if (token) {
      fetchUsers(token).catch(console.error);
      fetchRotationStatus(token);
    }
  }, [token, fetchUsers, fetchRotationStatus]);

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

  const handleReplaceRecoveryKey = async () => {
    if (!token) {
      message.error('Authentication required');
      return;
    }

    try {
      const recoveryCode = await invoke<string>('replace_recovery_key', { token });
      showRecoveryCode(recoveryCode);
    } catch (error) {
      message.error(`Failed to replace recovery key: ${error}`);
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

  const closePasswordModal = () => {
    setPasswordTarget(null);
    passwordForm.resetFields();
  };

  const handleSubmitTemporaryPassword = async (values: TemporaryPasswordFormData) => {
    if (!token || !passwordTarget) {
      message.error('Authentication required');
      return;
    }

    const { mode, user } = passwordTarget;
    setIsSubmittingPassword(true);
    try {
      if (mode === 'reactivate') {
        await reactivateUser(token, user.id, values.password);
        message.success(`User ${user.username} reactivated with a temporary password`);
      } else {
        await resetUserPassword(token, user.id, values.password);
        message.success(`Temporary password set for ${user.username}`);
      }
      closePasswordModal();
    } catch (error) {
      message.error(mode === 'reactivate' ? 'Failed to reactivate user' : 'Failed to reset password');
    } finally {
      setIsSubmittingPassword(false);
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
      title: 'Encryption access',
      key: 'encryption_access',
      render: (_: any, record: UserInfo) =>
        record.is_active && usersWithoutEncryptionAccess.has(record.id) ? (
          <Tooltip title="This user cannot decrypt data. Use Reset password to restore their access.">
            <Tag color="orange">No access - reset password</Tag>
          </Tooltip>
        ) : record.is_active ? (
          <Tag color="green">Has access</Tag>
        ) : (
          <Text type="secondary">-</Text>
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
              description={`Deactivate ${record.username}? Their sessions end immediately and their access to encrypted data is revoked. Reactivating them later requires setting a temporary password.`}
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
              onClick={() => setPasswordTarget({ mode: 'reactivate', user: record })}
              style={{ color: '#52c41a' }}
            >
              Reactivate
            </Button>
          )}
          {record.is_active && record.id !== currentUser?.id && (
            <Button
              type="text"
              size="small"
              icon={<LockOutlined />}
              onClick={() => setPasswordTarget({ mode: 'reset', user: record })}
            >
              Reset password
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
            <Popconfirm
              title="Replace Recovery Key"
              description="Issue a new recovery key? The current one stops working immediately."
              onConfirm={handleReplaceRecoveryKey}
              okText="Replace"
              cancelText="Cancel"
            >
              <Button icon={<KeyOutlined />}>
                Replace Recovery Key
              </Button>
            </Popconfirm>
            <Button
              icon={<SyncOutlined />}
              onClick={() => setIsRotationModalOpen(true)}
            >
              Rotate Encryption Key
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
          title="Error"
          description={error}
          type="error"
          showIcon
          closable={{ onClose: clearError }}
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

      <KeyRotationModal
        open={isRotationModalOpen}
        onClose={() => {
          setIsRotationModalOpen(false);
          if (token) {
            fetchRotationStatus(token);
            fetchUsers(token).catch(console.error);
          }
        }}
      />

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
              title="Error"
              description={createUserError}
              type="error"
              showIcon
              closable={{ onClose: clearCreateUserError }}
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

      {/* Temporary Password Modal (Reactivate / Reset password) */}
      <Modal
        title={passwordTarget?.mode === 'reactivate' ? 'Reactivate User' : 'Reset Password'}
        open={passwordTarget !== null}
        onCancel={closePasswordModal}
        footer={null}
        width={500}
        destroyOnHidden
      >
        <Form
          form={passwordForm}
          layout="vertical"
          onFinish={handleSubmitTemporaryPassword}
          autoComplete="off"
        >
          <Alert
            type="info"
            showIcon
            style={{ marginBottom: '16px' }}
            title={`Set a temporary password for ${passwordTarget?.user.username ?? ''}`}
            description="Share it with the user through a secure channel. It replaces their current password and restores their access to encrypted data."
          />

          <Form.Item
            label="Temporary Password"
            name="password"
            rules={[
              { required: true, message: 'Please input a temporary password' },
              { min: 8, message: 'Password must be at least 8 characters' },
            ]}
          >
            <Input.Password
              placeholder="Enter temporary password"
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
              placeholder="Confirm temporary password"
              autoComplete="new-password"
            />
          </Form.Item>

          <Form.Item style={{ marginBottom: 0 }}>
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px' }}>
              <Button onClick={closePasswordModal}>Cancel</Button>
              <Button type="primary" htmlType="submit" loading={isSubmittingPassword}>
                {passwordTarget?.mode === 'reactivate' ? 'Reactivate' : 'Set Password'}
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