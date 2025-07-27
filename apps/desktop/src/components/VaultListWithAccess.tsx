import React, { useState, useEffect } from 'react';
import { 
  List, 
  Card, 
  Tag, 
  Space, 
  Button, 
  Empty, 
  Spin, 
  Typography,
  Tooltip,
  notification
} from 'antd';
import {
  LockOutlined,
  UnlockOutlined,
  SafetyOutlined,
  FileProtectOutlined,
  KeyOutlined,
  GlobalOutlined,
  SecurityScanOutlined,
  FileTextOutlined,
  QuestionCircleOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../store/auth';
import { VaultInfo, PermissionRequest, RequestStatus } from '../types/vault';
import VaultAccessIndicator from './VaultAccessIndicator';

const { Text, Title } = Typography;

interface VaultListWithAccessProps {
  assetId: number;
  onVaultSelect?: (vault: VaultInfo) => void;
  showRequestButton?: boolean;
}

interface VaultWithAccess {
  vault: VaultInfo;
  hasAccess: boolean;
  pendingRequest?: PermissionRequest;
}

const VaultListWithAccess: React.FC<VaultListWithAccessProps> = ({
  assetId,
  onVaultSelect,
  showRequestButton = true
}) => {
  const { token, user } = useAuthStore();
  const [vaults, setVaults] = useState<VaultWithAccess[]>([]);
  const [loading, setLoading] = useState(true);
  const [requestingAccess, setRequestingAccess] = useState<number | null>(null);

  useEffect(() => {
    if (token && assetId) {
      loadVaults();
    }
  }, [token, assetId]);

  const loadVaults = async () => {
    if (!token || !user) return;

    setLoading(true);
    try {
      // For now, we'll get the vault by asset ID
      // In a real implementation, there might be multiple vaults per asset
      const vaultInfo = await invoke<VaultInfo | null>('get_vault_by_asset_id', {
        token,
        assetId
      });

      if (vaultInfo) {
        // Check access for this vault
        const accessInfo = await invoke<{
          has_access: boolean;
          permissions: any[];
          is_administrator: boolean;
        }>('check_vault_access', {
          token,
          request: {
            user_id: user.id,
            vault_id: vaultInfo.vault.id,
            permission_type: 'Read'
          }
        });

        // Check for pending requests
        const permissions = await invoke<PermissionRequest[]>('get_user_vault_permissions', {
          token,
          userId: user.id
        });

        const pendingRequest = permissions.find(
          p => p.vault_id === vaultInfo.vault.id && 
               p.status === 'Pending' as RequestStatus
        );

        setVaults([{
          vault: vaultInfo,
          hasAccess: accessInfo.has_access || accessInfo.is_administrator,
          pendingRequest
        }]);
      } else {
        setVaults([]);
      }
    } catch (error) {
      console.error('Failed to load vaults:', error);
      notification.error({
        message: 'Error',
        description: 'Failed to load vault information'
      });
    } finally {
      setLoading(false);
    }
  };

  const handleRequestAccess = async (vaultId: number) => {
    if (!token || !user) return;

    setRequestingAccess(vaultId);
    try {
      await invoke('create_permission_request', {
        token,
        request: {
          vault_id: vaultId,
          requested_permission: 'Read',
          requested_by: user.id
        }
      });

      notification.success({
        message: 'Access Requested',
        description: 'Your access request has been submitted and is pending approval.'
      });

      // Reload to show pending status
      await loadVaults();
    } catch (error) {
      console.error('Failed to request access:', error);
      notification.error({
        message: 'Error',
        description: 'Failed to submit access request'
      });
    } finally {
      setRequestingAccess(null);
    }
  };

  const getSecretTypeIcon = (type: string) => {
    switch (type) {
      case 'Password':
        return <KeyOutlined />;
      case 'IpAddress':
        return <GlobalOutlined />;
      case 'VpnKey':
        return <SecurityScanOutlined />;
      case 'LicenseFile':
        return <FileTextOutlined />;
      default:
        return <FileProtectOutlined />;
    }
  };

  const renderSecretSummary = (vault: VaultInfo) => {
    const secretTypes = vault.secrets.reduce((acc, secret) => {
      acc[secret.secret_type] = (acc[secret.secret_type] || 0) + 1;
      return acc;
    }, {} as Record<string, number>);

    return (
      <Space size="small" wrap>
        {Object.entries(secretTypes).map(([type, count]) => (
          <Tooltip key={type} title={`${count} ${type} ${count === 1 ? 'secret' : 'secrets'}`}>
            <Tag icon={getSecretTypeIcon(type)}>
              {count} {type}
            </Tag>
          </Tooltip>
        ))}
      </Space>
    );
  };

  if (loading) {
    return (
      <div style={{ textAlign: 'center', padding: '24px' }}>
        <Spin size="large" />
        <div style={{ marginTop: '16px' }}>
          <Text type="secondary">Loading vault information...</Text>
        </div>
      </div>
    );
  }

  if (vaults.length === 0) {
    return (
      <Empty
        image={<SafetyOutlined style={{ fontSize: '64px', color: '#d9d9d9' }} />}
        description={
          <div>
            <Title level={5}>No Identity Vault</Title>
            <Text type="secondary">
              No identity vault has been created for this asset yet.
            </Text>
          </div>
        }
      />
    );
  }

  return (
    <List
      dataSource={vaults}
      renderItem={({ vault, hasAccess, pendingRequest }) => (
        <Card
          hoverable={hasAccess}
          onClick={() => hasAccess && onVaultSelect?.(vault)}
          style={{ 
            marginBottom: '16px',
            opacity: hasAccess ? 1 : 0.8,
            cursor: hasAccess ? 'pointer' : 'default'
          }}
          extra={
            <VaultAccessIndicator 
              vaultId={vault.vault.id} 
              showDetails={true}
              onAccessChange={(access) => {
                if (!access && hasAccess) {
                  // Access was revoked, reload
                  loadVaults();
                }
              }}
            />
          }
        >
          <Card.Meta
            avatar={<SafetyOutlined style={{ fontSize: '32px', color: hasAccess ? '#52c41a' : '#8c8c8c' }} />}
            title={
              <Space>
                <Text strong>{vault.vault.name}</Text>
                {!hasAccess && <LockOutlined style={{ color: '#ff4d4f' }} />}
              </Space>
            }
            description={
              <div>
                <Text type="secondary" style={{ display: 'block', marginBottom: '8px' }}>
                  {vault.vault.description || 'No description'}
                </Text>
                
                {hasAccess ? (
                  <div>
                    <div style={{ marginBottom: '8px' }}>
                      {renderSecretSummary(vault)}
                    </div>
                    <Text type="secondary" style={{ fontSize: '12px' }}>
                      Total: {vault.secret_count} {vault.secret_count === 1 ? 'secret' : 'secrets'}
                    </Text>
                  </div>
                ) : (
                  <div>
                    {pendingRequest ? (
                      <Tag color="orange" icon={<QuestionCircleOutlined />}>
                        Access Request Pending
                      </Tag>
                    ) : (
                      showRequestButton && user?.role === 'Engineer' && (
                        <Button
                          size="small"
                          icon={<UnlockOutlined />}
                          onClick={(e) => {
                            e.stopPropagation();
                            handleRequestAccess(vault.vault.id);
                          }}
                          loading={requestingAccess === vault.vault.id}
                        >
                          Request Access
                        </Button>
                      )
                    )}
                  </div>
                )}
              </div>
            }
          />
        </Card>
      )}
    />
  );
};

export default VaultListWithAccess;