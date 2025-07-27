import React, { useState, useEffect } from 'react';
import { Tag, Tooltip, Space } from 'antd';
import {
  LockOutlined,
  UnlockOutlined,
  EyeOutlined,
  EditOutlined,
  ExportOutlined,
  ShareAltOutlined,
  ClockCircleOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../store/auth';
import { VaultAccessInfo, VaultPermission, PermissionType } from '../types/vault';
import dayjs from 'dayjs';

interface VaultAccessIndicatorProps {
  vaultId: number;
  compact?: boolean;
  showDetails?: boolean;
  onAccessChange?: (hasAccess: boolean) => void;
}

const VaultAccessIndicator: React.FC<VaultAccessIndicatorProps> = ({
  vaultId,
  compact = false,
  showDetails = false,
  onAccessChange
}) => {
  const { token, user } = useAuthStore();
  const [accessInfo, setAccessInfo] = useState<VaultAccessInfo | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    checkAccess();
  }, [vaultId, token, user]);

  const checkAccess = async () => {
    if (!token || !user) {
      setLoading(false);
      return;
    }

    try {
      const info = await invoke<VaultAccessInfo>('check_vault_access', {
        token,
        vaultId: vaultId,
        permissionType: 'Read'
      });
      
      setAccessInfo(info);
      onAccessChange?.(info.has_access);
    } catch (error) {
      console.error('Failed to check vault access:', error);
      setAccessInfo(null);
      onAccessChange?.(false);
    } finally {
      setLoading(false);
    }
  };

  const getPermissionIcon = (type: PermissionType) => {
    switch (type) {
      case 'Read':
        return <EyeOutlined />;
      case 'Write':
        return <EditOutlined />;
      case 'Export':
        return <ExportOutlined />;
      case 'Share':
        return <ShareAltOutlined />;
      default:
        return <LockOutlined />;
    }
  };

  const isPermissionExpired = (permission: VaultPermission): boolean => {
    if (!permission.expires_at) return false;
    return dayjs(permission.expires_at).isBefore(dayjs());
  };

  const isPermissionExpiringSoon = (permission: VaultPermission): boolean => {
    if (!permission.expires_at) return false;
    const expiryDate = dayjs(permission.expires_at);
    const now = dayjs();
    const daysUntilExpiry = expiryDate.diff(now, 'days');
    return daysUntilExpiry <= 7 && daysUntilExpiry > 0;
  };

  if (loading) {
    return null;
  }

  // Administrator indicator
  if (accessInfo?.is_administrator) {
    return (
      <Tooltip title="Full access as Administrator">
        <Tag icon={<UnlockOutlined />} color="gold">
          Admin Access
        </Tag>
      </Tooltip>
    );
  }

  // No access indicator
  if (!accessInfo?.has_access) {
    if (compact) {
      return (
        <Tooltip title="No access to this vault">
          <LockOutlined style={{ color: '#ff4d4f' }} />
        </Tooltip>
      );
    }
    return (
      <Tag icon={<LockOutlined />} color="error">
        No Access
      </Tag>
    );
  }

  // Has access - show permissions
  if (compact) {
    const activePermissions = accessInfo.permissions.filter(p => p.is_active && !isPermissionExpired(p));
    
    return (
      <Tooltip 
        title={
          <div>
            <div style={{ marginBottom: '8px' }}>Vault Permissions:</div>
            {activePermissions.map(p => (
              <div key={p.permission_id} style={{ marginBottom: '4px' }}>
                {getPermissionIcon(p.permission_type)} {p.permission_type}
                {p.expires_at && (
                  <span style={{ marginLeft: '8px', fontSize: '11px' }}>
                    (expires {dayjs(p.expires_at).format('MMM D, YYYY')})
                  </span>
                )}
              </div>
            ))}
          </div>
        }
      >
        <UnlockOutlined style={{ color: '#52c41a' }} />
      </Tooltip>
    );
  }

  if (!showDetails) {
    return (
      <Tag icon={<UnlockOutlined />} color="success">
        Has Access
      </Tag>
    );
  }

  // Detailed permission display
  const activePermissions = accessInfo.permissions.filter(p => p.is_active);
  
  return (
    <Space wrap size="small">
      {activePermissions.map(permission => {
        const expired = isPermissionExpired(permission);
        const expiringSoon = isPermissionExpiringSoon(permission);
        
        return (
          <Tooltip
            key={permission.permission_id}
            title={
              <div>
                <div>Permission: {permission.permission_type}</div>
                <div>Granted by: User {permission.granted_by}</div>
                <div>Granted at: {dayjs(permission.granted_at).format('YYYY-MM-DD HH:mm')}</div>
                {permission.expires_at && (
                  <div>
                    {expired ? 'Expired' : 'Expires'}: {dayjs(permission.expires_at).format('YYYY-MM-DD HH:mm')}
                  </div>
                )}
              </div>
            }
          >
            <Tag
              icon={getPermissionIcon(permission.permission_type)}
              color={expired ? 'default' : expiringSoon ? 'warning' : 'blue'}
              style={{ 
                textDecoration: expired ? 'line-through' : 'none',
                opacity: expired ? 0.6 : 1
              }}
            >
              {permission.permission_type}
              {permission.expires_at && (
                <ClockCircleOutlined style={{ marginLeft: '4px', fontSize: '11px' }} />
              )}
            </Tag>
          </Tooltip>
        );
      })}
    </Space>
  );
};

export default VaultAccessIndicator;