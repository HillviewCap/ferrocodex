import React, { useState, useEffect } from 'react';
import { Table, Button, Modal, Select, DatePicker, Tag, Space, notification, Tooltip, Switch, Card } from 'antd';
import { UserOutlined, LockOutlined, UnlockOutlined, EyeOutlined, EditOutlined, ExportOutlined, ShareAltOutlined, CalendarOutlined } from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../store/auth';
import { VaultPermission, PermissionType, GrantVaultAccessRequest, RevokeVaultAccessRequest, permissionTypeDisplayNames } from '../types/vault';
import { UserInfo } from '../store/auth';
import dayjs from 'dayjs';

interface VaultPermissionManagerProps {
  vaultId: number;
  vaultName: string;
  onClose?: () => void;
}

const VaultPermissionManager: React.FC<VaultPermissionManagerProps> = ({ vaultId, vaultName, onClose }) => {
  const { token } = useAuthStore();
  const [permissions, setPermissions] = useState<VaultPermission[]>([]);
  const [loading, setLoading] = useState(false);
  const [users, setUsers] = useState<UserInfo[]>([]);
  const [grantModalVisible, setGrantModalVisible] = useState(false);
  const [selectedUser, setSelectedUser] = useState<number | undefined>();
  const [selectedPermission, setSelectedPermission] = useState<PermissionType>('Read');
  const [expiresAt, setExpiresAt] = useState<dayjs.Dayjs | null>(null);
  const [hasExpiry, setHasExpiry] = useState(false);

  useEffect(() => {
    loadPermissions();
    loadUsers();
  }, [vaultId]);

  const loadPermissions = async () => {
    if (!token) return;
    
    setLoading(true);
    try {
      const result = await invoke<VaultPermission[]>('get_vault_permissions', {
        token,
        vaultId: vaultId
      });
      setPermissions(result);
    } catch (error) {
      notification.error({
        message: 'Error Loading Permissions',
        description: error as string
      });
    } finally {
      setLoading(false);
    }
  };

  const loadUsers = async () => {
    if (!token) return;
    
    try {
      const result = await invoke<UserInfo[]>('list_users', { token });
      setUsers(result.filter(u => u.is_active));
    } catch (error) {
      console.error('Error loading users:', error);
    }
  };

  const handleGrantAccess = async () => {
    if (!token || !selectedUser) return;

    setLoading(true);
    try {
      const request: GrantVaultAccessRequest = {
        user_id: selectedUser,
        vault_id: vaultId,
        permission_type: selectedPermission,
        granted_by: 0, // Will be set by backend
        expires_at: hasExpiry && expiresAt ? expiresAt.toISOString() : undefined
      };

      await invoke('grant_vault_access', { token, request });
      
      notification.success({
        message: 'Access Granted',
        description: `${selectedPermission} access granted successfully`
      });

      setGrantModalVisible(false);
      setSelectedUser(undefined);
      setSelectedPermission('Read');
      setExpiresAt(null);
      setHasExpiry(false);
      
      loadPermissions();
    } catch (error) {
      notification.error({
        message: 'Error Granting Access',
        description: error as string
      });
    } finally {
      setLoading(false);
    }
  };

  const handleRevokeAccess = async (userId: number, permissionType?: PermissionType) => {
    if (!token) return;

    Modal.confirm({
      title: 'Revoke Access',
      content: `Are you sure you want to revoke ${permissionType || 'all'} access for this user?`,
      okText: 'Revoke',
      okType: 'danger',
      onOk: async () => {
        setLoading(true);
        try {
          const request: RevokeVaultAccessRequest = {
            user_id: userId,
            vault_id: vaultId,
            permission_type: permissionType,
            revoked_by: 0 // Will be set by backend
          };

          await invoke('revoke_vault_access', { token, request });
          
          notification.success({
            message: 'Access Revoked',
            description: 'Access revoked successfully'
          });
          
          loadPermissions();
        } catch (error) {
          notification.error({
            message: 'Error Revoking Access',
            description: error as string
          });
        } finally {
          setLoading(false);
        }
      }
    });
  };

  const getUserPermissions = (userId: number): VaultPermission[] => {
    return permissions.filter(p => p.user_id === userId && p.is_active);
  };

  const getUniqueUsers = (): number[] => {
    const userIds = new Set(permissions.filter(p => p.is_active).map(p => p.user_id));
    return Array.from(userIds);
  };

  const getPermissionIcon = (type: PermissionType) => {
    const icons = {
      Read: <EyeOutlined />,
      Write: <EditOutlined />,
      Export: <ExportOutlined />,
      Share: <ShareAltOutlined />
    };
    return icons[type];
  };

  const isPermissionExpired = (permission: VaultPermission): boolean => {
    if (!permission.expires_at) return false;
    return dayjs(permission.expires_at).isBefore(dayjs());
  };

  const columns = [
    {
      title: 'User',
      key: 'user',
      render: (_: any, record: number) => {
        const user = users.find(u => u.id === record);
        return (
          <Space>
            <UserOutlined />
            {user?.username || `User ${record}`}
            {user?.role === 'Administrator' && <Tag color="gold">Admin</Tag>}
          </Space>
        );
      }
    },
    {
      title: 'Permissions',
      key: 'permissions',
      render: (_: any, userId: number) => {
        const userPerms = getUserPermissions(userId);
        return (
          <Space wrap>
            {userPerms.map(perm => (
              <Tooltip 
                key={perm.permission_id}
                title={
                  <div>
                    <div>Granted by: User {perm.granted_by}</div>
                    <div>Granted at: {dayjs(perm.granted_at).format('YYYY-MM-DD HH:mm')}</div>
                    {perm.expires_at && (
                      <div>Expires: {dayjs(perm.expires_at).format('YYYY-MM-DD HH:mm')}</div>
                    )}
                  </div>
                }
              >
                <Tag 
                  icon={getPermissionIcon(perm.permission_type)}
                  color={isPermissionExpired(perm) ? 'default' : 'blue'}
                  style={{ textDecoration: isPermissionExpired(perm) ? 'line-through' : 'none' }}
                >
                  {permissionTypeDisplayNames[perm.permission_type]}
                  {perm.expires_at && (
                    <CalendarOutlined style={{ marginLeft: 4 }} />
                  )}
                </Tag>
              </Tooltip>
            ))}
          </Space>
        );
      }
    },
    {
      title: 'Actions',
      key: 'actions',
      width: 200,
      render: (_: any, userId: number) => {
        const userPerms = getUserPermissions(userId);
        return (
          <Space>
            {userPerms.map(perm => (
              <Button
                key={perm.permission_id}
                size="small"
                danger
                icon={<UnlockOutlined />}
                onClick={() => handleRevokeAccess(userId, perm.permission_type)}
                disabled={isPermissionExpired(perm)}
              >
                Revoke {perm.permission_type}
              </Button>
            ))}
          </Space>
        );
      }
    }
  ];

  return (
    <Card 
      title={
        <Space>
          <LockOutlined />
          <span>Vault Permissions - {vaultName}</span>
        </Space>
      }
      extra={
        <Space>
          <Button
            type="primary"
            icon={<LockOutlined />}
            onClick={() => setGrantModalVisible(true)}
          >
            Grant Access
          </Button>
          {onClose && <Button onClick={onClose}>Close</Button>}
        </Space>
      }
    >
      <Table
        dataSource={getUniqueUsers()}
        columns={columns}
        rowKey={userId => userId}
        loading={loading}
        pagination={false}
      />

      <Modal
        title="Grant Vault Access"
        open={grantModalVisible}
        onOk={handleGrantAccess}
        onCancel={() => {
          setGrantModalVisible(false);
          setSelectedUser(undefined);
          setSelectedPermission('Read');
          setExpiresAt(null);
          setHasExpiry(false);
        }}
        confirmLoading={loading}
        okText="Grant Access"
        okButtonProps={{ disabled: !selectedUser }}
      >
        <Space direction="vertical" style={{ width: '100%' }} size="large">
          <div>
            <label>Select User:</label>
            <Select
              style={{ width: '100%', marginTop: 8 }}
              placeholder="Select a user"
              value={selectedUser}
              onChange={setSelectedUser}
              showSearch
              optionFilterProp="children"
            >
              {users
                .filter(user => {
                  // Filter out users who already have this permission
                  const existingPerms = permissions.filter(
                    p => p.user_id === user.id && 
                         p.permission_type === selectedPermission && 
                         p.is_active &&
                         !isPermissionExpired(p)
                  );
                  return existingPerms.length === 0;
                })
                .map(user => (
                  <Select.Option key={user.id} value={user.id}>
                    {user.username} {user.role === 'Administrator' && '(Admin)'}
                  </Select.Option>
                ))}
            </Select>
          </div>

          <div>
            <label>Permission Type:</label>
            <Select
              style={{ width: '100%', marginTop: 8 }}
              value={selectedPermission}
              onChange={setSelectedPermission}
            >
              {Object.entries(permissionTypeDisplayNames).map(([key, value]) => (
                <Select.Option key={key} value={key}>
                  {getPermissionIcon(key as PermissionType)} {value}
                </Select.Option>
              ))}
            </Select>
          </div>

          <div>
            <Space>
              <Switch
                checked={hasExpiry}
                onChange={setHasExpiry}
              />
              <label>Set Expiration Date</label>
            </Space>
          </div>

          {hasExpiry && (
            <div>
              <label>Expires At:</label>
              <DatePicker
                style={{ width: '100%', marginTop: 8 }}
                showTime
                value={expiresAt}
                onChange={setExpiresAt}
                disabledDate={(current) => current && current < dayjs().endOf('day')}
                format="YYYY-MM-DD HH:mm"
              />
            </div>
          )}
        </Space>
      </Modal>
    </Card>
  );
};

export default VaultPermissionManager;