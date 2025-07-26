import React, { useState, useEffect } from 'react';
import {
  Typography,
  Button,
  Card,
  Space,
  Spin,
  Empty,
  App,
  Tabs,
  List,
  Modal,
  Form,
  Input,
  Select,
  Tag,
  Tooltip,
  Popconfirm,
  Alert
} from 'antd';
import {
  SafetyOutlined,
  PlusOutlined,
  EditOutlined,
  DeleteOutlined,
  EyeOutlined,
  EyeInvisibleOutlined,
  LockOutlined,
  GlobalOutlined,
  SecurityScanOutlined,
  FileTextOutlined,
  HistoryOutlined,
  CopyOutlined,
  KeyOutlined,
  ReloadOutlined,
  ExclamationCircleOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { AssetInfo } from '../types/assets';
import {
  VaultInfo,
  VaultSecret,
  VaultVersion,
  SecretType,
  CreateVaultRequest,
  AddSecretRequest,
  secretTypeDisplayNames,
  changeTypeDisplayNames,
  UpdateCredentialPasswordRequest,
  getStrengthColor,
  getStrengthLabel
} from '../types/vault';
import useAuthStore from '../store/auth';
import PasswordGenerator from './PasswordGenerator';
import PasswordInput from './PasswordInput';
import PasswordHistory from './PasswordHistory';

const { Title, Text } = Typography;
const { TextArea } = Input;

interface IdentityVaultProps {
  asset: AssetInfo;
}

const IdentityVault: React.FC<IdentityVaultProps> = ({ asset }) => {
  const { token, user } = useAuthStore();
  const { message } = App.useApp();
  
  const [vaultInfo, setVaultInfo] = useState<VaultInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [createVaultVisible, setCreateVaultVisible] = useState(false);
  const [addSecretVisible, setAddSecretVisible] = useState(false);
  const [vaultHistory, setVaultHistory] = useState<VaultVersion[]>([]);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [visibleSecrets, setVisibleSecrets] = useState<Set<number>>(new Set());
  const [decryptedSecrets, setDecryptedSecrets] = useState<Map<number, string>>(new Map());
  const [activeTab, setActiveTab] = useState('secrets');
  
  // Password management state
  const [passwordGeneratorVisible, setPasswordGeneratorVisible] = useState(false);
  const [passwordHistoryVisible, setPasswordHistoryVisible] = useState(false);
  const [selectedSecret, setSelectedSecret] = useState<VaultSecret | null>(null);
  const [updatePasswordVisible, setUpdatePasswordVisible] = useState(false);
  const [updatePasswordForm] = Form.useForm();
  const [addSecretType, setAddSecretType] = useState<SecretType | null>(null);

  const [createVaultForm] = Form.useForm();
  const [addSecretForm] = Form.useForm();

  useEffect(() => {
    if (token && asset.id) {
      fetchVaultInfo();
    }
  }, [token, asset.id]);

  const fetchVaultInfo = async () => {
    if (!token) return;
    
    setLoading(true);
    try {
      const result = await invoke<VaultInfo | null>('get_vault_by_asset_id', {
        token,
        assetId: asset.id
      });
      setVaultInfo(result);
    } catch (err) {
      console.error('Failed to fetch vault info:', err);
      message.error('Failed to load vault information');
    } finally {
      setLoading(false);
    }
  };

  const fetchVaultHistory = async () => {
    if (!token || !vaultInfo?.vault.id) return;
    
    setHistoryLoading(true);
    try {
      const history = await invoke<VaultVersion[]>('get_vault_history', {
        token,
        vaultId: vaultInfo.vault.id
      });
      setVaultHistory(history);
    } catch (err) {
      console.error('Failed to fetch vault history:', err);
      message.error('Failed to load vault history');
    } finally {
      setHistoryLoading(false);
    }
  };

  const handleCreateVault = async (values: any) => {
    if (!token || !user) return;

    try {
      const request: CreateVaultRequest = {
        asset_id: asset.id,
        name: values.name,
        description: values.description || '',
        created_by: user.id
      };

      await invoke('create_identity_vault', {
        token,
        vaultRequest: request
      });

      message.success('Identity vault created successfully');
      setCreateVaultVisible(false);
      createVaultForm.resetFields();
      fetchVaultInfo();
    } catch (err) {
      console.error('Failed to create vault:', err);
      message.error('Failed to create vault');
    }
  };

  const handleAddSecret = async (values: any) => {
    if (!token || !user || !vaultInfo) return;

    try {
      const request: AddSecretRequest = {
        vault_id: vaultInfo.vault.id,
        secret_type: values.secret_type,
        label: values.label,
        value: values.value,
        author_id: user.id
      };

      await invoke('add_vault_secret', {
        token,
        secretRequest: request
      });

      message.success('Secret added successfully');
      setAddSecretVisible(false);
      addSecretForm.resetFields();
      setAddSecretType(null);
      fetchVaultInfo();
    } catch (err) {
      console.error('Failed to add secret:', err);
      message.error('Failed to add secret');
    }
  };

  const handleRevealSecret = async (secret: VaultSecret) => {
    if (!token || !vaultInfo) return;

    try {
      const decryptedValue = await invoke<string>('decrypt_vault_secret', {
        token,
        secretId: secret.id,
        vaultId: secret.vault_id
      });

      setDecryptedSecrets(prev => new Map(prev).set(secret.id, decryptedValue));
      setVisibleSecrets(prev => new Set(prev).add(secret.id));
    } catch (err) {
      console.error('Failed to decrypt secret:', err);
      message.error('Failed to decrypt secret');
    }
  };

  const handleHideSecret = (secretId: number) => {
    setVisibleSecrets(prev => {
      const newSet = new Set(prev);
      newSet.delete(secretId);
      return newSet;
    });
  };

  const handleCopySecret = (secretId: number) => {
    const decryptedValue = decryptedSecrets.get(secretId);
    if (decryptedValue) {
      navigator.clipboard.writeText(decryptedValue);
      message.success('Secret copied to clipboard');
    }
  };

  const handleUpdatePassword = (secret: VaultSecret) => {
    setSelectedSecret(secret);
    updatePasswordForm.resetFields();
    setUpdatePasswordVisible(true);
  };

  const handlePasswordUpdate = async (values: any) => {
    if (!token || !user || !selectedSecret) return;

    try {
      const request: UpdateCredentialPasswordRequest = {
        secret_id: selectedSecret.id,
        new_password: values.password,
        author_id: user.id,
      };

      await invoke('update_credential_password', {
        token,
        request
      });

      message.success('Password updated successfully');
      setUpdatePasswordVisible(false);
      updatePasswordForm.resetFields();
      setSelectedSecret(null);
      fetchVaultInfo();
    } catch (err) {
      console.error('Failed to update password:', err);
      message.error(typeof err === 'string' ? err : 'Failed to update password');
    }
  };

  const handleViewPasswordHistory = (secret: VaultSecret) => {
    setSelectedSecret(secret);
    setPasswordHistoryVisible(true);
  };

  const handleGeneratePasswordForSecret = (secret: VaultSecret) => {
    setSelectedSecret(secret);
    setPasswordGeneratorVisible(true);
  };

  const handlePasswordGenerated = (password: string) => {
    if (selectedSecret) {
      updatePasswordForm.setFieldsValue({ password });
      setUpdatePasswordVisible(true);
    }
  };

  const getSecretIcon = (secretType: SecretType) => {
    switch (secretType) {
      case 'Password':
        return <LockOutlined />;
      case 'IpAddress':
        return <GlobalOutlined />;
      case 'VpnKey':
        return <SecurityScanOutlined />;
      case 'LicenseFile':
        return <FileTextOutlined />;
      default:
        return <LockOutlined />;
    }
  };

  const formatDate = (dateString: string) => {
    try {
      return new Date(dateString).toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
      });
    } catch {
      return dateString;
    }
  };

  // If no vault exists, show create vault interface
  if (!loading && !vaultInfo) {
    return (
      <div style={{ padding: '24px', textAlign: 'center' }}>
        <Empty
          image={<SafetyOutlined style={{ fontSize: '64px', color: '#d9d9d9' }} />}
          description={
            <div>
              <Title level={4}>No Identity Vault</Title>
              <Text type="secondary">
                Create an identity vault to securely store passwords, IP addresses, VPN keys, and license files for this asset.
              </Text>
            </div>
          }
        >
          <Button type="primary" icon={<PlusOutlined />} onClick={() => setCreateVaultVisible(true)}>
            Create Identity Vault
          </Button>
        </Empty>

        <Modal
          title="Create Identity Vault"
          open={createVaultVisible}
          onCancel={() => {
            setCreateVaultVisible(false);
            createVaultForm.resetFields();
          }}
          footer={null}
        >
          <Form
            form={createVaultForm}
            layout="vertical"
            onFinish={handleCreateVault}
          >
            <Form.Item
              name="name"
              label="Vault Name"
              rules={[
                { required: true, message: 'Please enter a vault name' },
                { min: 2, message: 'Vault name must be at least 2 characters' },
                { max: 100, message: 'Vault name cannot exceed 100 characters' }
              ]}
            >
              <Input placeholder="e.g., PLC-Line5 Identity Vault" />
            </Form.Item>

            <Form.Item
              name="description"
              label="Description"
            >
              <TextArea
                rows={3}
                placeholder="Optional description for this vault"
                maxLength={500}
              />
            </Form.Item>

            <div style={{ textAlign: 'right' }}>
              <Space>
                <Button onClick={() => {
                  setCreateVaultVisible(false);
                  createVaultForm.resetFields();
                }}>
                  Cancel
                </Button>
                <Button type="primary" htmlType="submit">
                  Create Vault
                </Button>
              </Space>
            </div>
          </Form>
        </Modal>
      </div>
    );
  }

  if (loading) {
    return (
      <div style={{ padding: '24px', textAlign: 'center' }}>
        <Spin size="large" />
        <div style={{ marginTop: '16px' }}>
          <Text type="secondary">Loading vault information...</Text>
        </div>
      </div>
    );
  }

  if (!vaultInfo) {
    return (
      <div style={{ padding: '24px', textAlign: 'center' }}>
        <Text type="secondary">No vault information available</Text>
      </div>
    );
  }

  return (
    <div style={{ padding: '24px' }}>
      {/* Vault Header */}
      <Card style={{ marginBottom: '24px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
          <SafetyOutlined style={{ fontSize: '48px', color: '#52c41a' }} />
          <div style={{ flex: 1 }}>
            <Title level={4} style={{ margin: 0 }}>
              {vaultInfo.vault.name}
            </Title>
            <Text type="secondary" style={{ display: 'block', marginBottom: '8px' }}>
              {vaultInfo.vault.description || 'No description'}
            </Text>
            <Space wrap>
              <Tag color="blue">
                {vaultInfo.secret_count} {vaultInfo.secret_count === 1 ? 'secret' : 'secrets'}
              </Tag>
              <Text type="secondary" style={{ fontSize: '12px' }}>
                Created: {formatDate(vaultInfo.vault.created_at)}
              </Text>
            </Space>
          </div>
        </div>
      </Card>

      {/* Security Notice */}
      <Alert
        message="Security Notice"
        description="All secrets are encrypted with AES-256 encryption and can only be decrypted by authorized users. Secret values are masked by default and must be explicitly revealed."
        type="info"
        icon={<SafetyOutlined />}
        style={{ marginBottom: '24px' }}
      />

      {/* Tabs */}
      <Tabs
        activeKey={activeTab}
        onChange={setActiveTab}
        items={[
          {
            key: 'secrets',
            label: (
              <span>
                <LockOutlined />
                Secrets ({vaultInfo.secret_count})
              </span>
            ),
            children: (
              <div>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
                  <Text type="secondary">
                    Securely stored credentials and configuration data for this asset.
                  </Text>
                  <Button
                    type="primary"
                    icon={<PlusOutlined />}
                    onClick={() => setAddSecretVisible(true)}
                  >
                    Add Secret
                  </Button>
                </div>

                {vaultInfo.secrets.length === 0 ? (
                  <Empty
                    description="No secrets stored yet"
                    image={Empty.PRESENTED_IMAGE_SIMPLE}
                  >
                    <Button type="primary" icon={<PlusOutlined />} onClick={() => setAddSecretVisible(true)}>
                      Add First Secret
                    </Button>
                  </Empty>
                ) : (
                  <List
                    dataSource={vaultInfo.secrets}
                    renderItem={(secret) => {
                      const isVisible = visibleSecrets.has(secret.id);
                      const decryptedValue = decryptedSecrets.get(secret.id);

                      return (
                        <List.Item
                          actions={[
                            <Tooltip title={isVisible ? "Hide secret" : "Reveal secret"}>
                              <Button
                                type="text"
                                icon={isVisible ? <EyeInvisibleOutlined /> : <EyeOutlined />}
                                onClick={() => isVisible ? handleHideSecret(secret.id) : handleRevealSecret(secret)}
                              />
                            </Tooltip>,
                            isVisible && decryptedValue && (
                              <Tooltip title="Copy to clipboard">
                                <Button
                                  type="text"
                                  icon={<CopyOutlined />}
                                  onClick={() => handleCopySecret(secret.id)}
                                />
                              </Tooltip>
                            ),
                            secret.secret_type === 'Password' && (
                              <Tooltip title="Update password">
                                <Button
                                  type="text"
                                  icon={<KeyOutlined />}
                                  onClick={() => handleUpdatePassword(secret)}
                                />
                              </Tooltip>
                            ),
                            secret.secret_type === 'Password' && (
                              <Tooltip title="Generate new password">
                                <Button
                                  type="text"
                                  icon={<ReloadOutlined />}
                                  onClick={() => handleGeneratePasswordForSecret(secret)}
                                />
                              </Tooltip>
                            ),
                            secret.secret_type === 'Password' && (
                              <Tooltip title="View password history">
                                <Button
                                  type="text"
                                  icon={<HistoryOutlined />}
                                  onClick={() => handleViewPasswordHistory(secret)}
                                />
                              </Tooltip>
                            ),
                            secret.secret_type !== 'Password' && (
                              <Tooltip title="Edit secret">
                                <Button
                                  type="text"
                                  icon={<EditOutlined />}
                                  disabled
                                />
                              </Tooltip>
                            ),
                            <Popconfirm
                              title="Delete secret"
                              description="Are you sure you want to delete this secret? This action cannot be undone."
                              okText="Delete"
                              okType="danger"
                              cancelText="Cancel"
                              disabled
                            >
                              <Button
                                type="text"
                                danger
                                icon={<DeleteOutlined />}
                                disabled
                              />
                            </Popconfirm>
                          ].filter(Boolean)}
                        >
                          <List.Item.Meta
                            avatar={getSecretIcon(secret.secret_type)}
                            title={
                              <Space>
                                <Text strong>{secret.label}</Text>
                                <Tag color="geekblue">{secretTypeDisplayNames[secret.secret_type]}</Tag>
                                {secret.secret_type === 'Password' && secret.strength_score && (
                                  <Tag color={getStrengthColor(secret.strength_score)}>
                                    {getStrengthLabel(secret.strength_score)} ({secret.strength_score}/100)
                                  </Tag>
                                )}
                                {secret.generation_method === 'generated' && (
                                  <Tag color="cyan" icon={<KeyOutlined />}>
                                    Generated
                                  </Tag>
                                )}
                              </Space>
                            }
                            description={
                              <div>
                                <div style={{ marginBottom: '8px' }}>
                                  {isVisible && decryptedValue ? (
                                    <Text code copyable={{ text: decryptedValue }}>
                                      {secret.secret_type === 'Password' ? '••••••••' : decryptedValue}
                                    </Text>
                                  ) : (
                                    <Text type="secondary">••••••••••••••••</Text>
                                  )}
                                </div>
                                
                                <div style={{ display: 'flex', gap: '16px', fontSize: '12px' }}>
                                  <Text type="secondary">
                                    Added: {formatDate(secret.created_at)}
                                  </Text>
                                  {secret.secret_type === 'Password' && secret.last_changed && (
                                    <Text type="secondary">
                                      Last Changed: {formatDate(secret.last_changed)}
                                    </Text>
                                  )}
                                </div>

                                {secret.secret_type === 'Password' && secret.strength_score && secret.strength_score < 60 && (
                                  <div style={{ marginTop: '4px' }}>
                                    <Space size="small">
                                      <ExclamationCircleOutlined style={{ color: '#faad14' }} />
                                      <Text type="warning" style={{ fontSize: '12px' }}>
                                        Consider updating this password for better security
                                      </Text>
                                    </Space>
                                  </div>
                                )}
                              </div>
                            }
                          />
                        </List.Item>
                      );
                    }}
                  />
                )}
              </div>
            )
          },
          {
            key: 'history',
            label: (
              <span>
                <HistoryOutlined />
                History
              </span>
            ),
            children: (
              <div>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
                  <Text type="secondary">
                    Complete audit trail of all vault and secret changes.
                  </Text>
                  <Button
                    icon={<HistoryOutlined />}
                    onClick={fetchVaultHistory}
                    loading={historyLoading}
                  >
                    Refresh History
                  </Button>
                </div>

                {vaultHistory.length === 0 ? (
                  <Empty
                    description="No history available"
                    image={Empty.PRESENTED_IMAGE_SIMPLE}
                  />
                ) : (
                  <List
                    dataSource={vaultHistory}
                    renderItem={(version) => (
                      <List.Item>
                        <List.Item.Meta
                          avatar={<HistoryOutlined />}
                          title={
                            <Space>
                              <Text strong>{changeTypeDisplayNames[version.change_type]}</Text>
                              <Text type="secondary" style={{ fontSize: '12px' }}>
                                {formatDate(version.timestamp)}
                              </Text>
                            </Space>
                          }
                          description={
                            <div>
                              <Text>{version.notes}</Text>
                              {version.changes_json && (
                                <div style={{ marginTop: '4px' }}>
                                  <Text type="secondary" style={{ fontSize: '12px' }}>
                                    Changes: {JSON.stringify(JSON.parse(version.changes_json))}
                                  </Text>
                                </div>
                              )}
                            </div>
                          }
                        />
                      </List.Item>
                    )}
                  />
                )}
              </div>
            )
          }
        ]}
      />

      {/* Add Secret Modal */}
      <Modal
        title="Add Secret"
        open={addSecretVisible}
        onCancel={() => {
          setAddSecretVisible(false);
          addSecretForm.resetFields();
          setAddSecretType(null);
        }}
        footer={null}
        width={600}
      >
        <Form
          form={addSecretForm}
          layout="vertical"
          onFinish={handleAddSecret}
        >
          <Form.Item
            name="secret_type"
            label="Secret Type"
            rules={[{ required: true, message: 'Please select a secret type' }]}
          >
            <Select 
              placeholder="Select secret type"
              onChange={(value) => setAddSecretType(value as SecretType)}
            >
              <Select.Option value="Password">
                <Space>
                  <LockOutlined />
                  Password
                </Space>
              </Select.Option>
              <Select.Option value="IpAddress">
                <Space>
                  <GlobalOutlined />
                  IP Address
                </Space>
              </Select.Option>
              <Select.Option value="VpnKey">
                <Space>
                  <SecurityScanOutlined />
                  VPN Key
                </Space>
              </Select.Option>
              <Select.Option value="LicenseFile">
                <Space>
                  <FileTextOutlined />
                  License File
                </Space>
              </Select.Option>
            </Select>
          </Form.Item>

          <Form.Item
            name="label"
            label="Label"
            rules={[
              { required: true, message: 'Please enter a label for this secret' },
              { min: 1, message: 'Label cannot be empty' },
              { max: 100, message: 'Label cannot exceed 100 characters' }
            ]}
          >
            <Input placeholder="e.g., Admin Password, PLC IP Address" />
          </Form.Item>

          <Form.Item
            name="value"
            label="Secret Value"
            rules={[
              { required: true, message: 'Please enter the secret value' },
              { min: 1, message: 'Secret value cannot be empty' }
            ]}
          >
            {addSecretType === 'Password' ? (
              <PasswordInput
                placeholder="Enter password (use generator for strong passwords)"
                showStrength={true}
                showGenerator={true}
                checkReuse={true}
                minScore={40}
              />
            ) : (
              <TextArea
                rows={4}
                placeholder="Enter the secret value (will be encrypted when stored)"
              />
            )}
          </Form.Item>

          <Alert
            message="Security Notice"
            description="The secret value will be encrypted with AES-256 encryption before being stored in the database. Only authorized users can decrypt and view the actual value."
            type="info"
            style={{ marginBottom: '16px' }}
          />

          <div style={{ textAlign: 'right' }}>
            <Space>
              <Button onClick={() => {
                setAddSecretVisible(false);
                addSecretForm.resetFields();
                setAddSecretType(null);
              }}>
                Cancel
              </Button>
              <Button type="primary" htmlType="submit">
                Add Secret
              </Button>
            </Space>
          </div>
        </Form>
      </Modal>

      {/* Update Password Modal */}
      <Modal
        title={
          <Space>
            <KeyOutlined />
            Update Password - {selectedSecret?.label}
          </Space>
        }
        open={updatePasswordVisible}
        onCancel={() => {
          setUpdatePasswordVisible(false);
          updatePasswordForm.resetFields();
          setSelectedSecret(null);
        }}
        footer={null}
        width={600}
      >
        <Form
          form={updatePasswordForm}
          layout="vertical"
          onFinish={handlePasswordUpdate}
        >
          <Form.Item
            name="password"
            label="New Password"
            rules={[
              { required: true, message: 'Please enter the new password' },
              { min: 1, message: 'Password cannot be empty' }
            ]}
          >
            <PasswordInput
              placeholder="Enter new password"
              showStrength={true}
              showGenerator={true}
              checkReuse={true}
              secretId={selectedSecret?.id}
              minScore={60}
            />
          </Form.Item>

          <Alert
            message="Password Update Security"
            description="The new password will be validated for strength and checked against previously used passwords to prevent reuse. The old password will be retired and moved to history."
            type="info"
            style={{ marginBottom: '16px' }}
          />

          <div style={{ textAlign: 'right' }}>
            <Space>
              <Button onClick={() => {
                setUpdatePasswordVisible(false);
                updatePasswordForm.resetFields();
                setSelectedSecret(null);
              }}>
                Cancel
              </Button>
              <Button type="primary" htmlType="submit">
                Update Password
              </Button>
            </Space>
          </div>
        </Form>
      </Modal>

      {/* Password Generator Modal */}
      <PasswordGenerator
        visible={passwordGeneratorVisible}
        onCancel={() => {
          setPasswordGeneratorVisible(false);
          setSelectedSecret(null);
        }}
        onGenerated={handlePasswordGenerated}
        title={selectedSecret ? `Generate Password for ${selectedSecret.label}` : 'Generate Password'}
      />

      {/* Password History Modal */}
      {selectedSecret && (
        <PasswordHistory
          visible={passwordHistoryVisible}
          onCancel={() => {
            setPasswordHistoryVisible(false);
            setSelectedSecret(null);
          }}
          secretId={selectedSecret.id}
          secretLabel={selectedSecret.label}
        />
      )}
    </div>
  );
};

export default IdentityVault;