import React, { useState, useEffect } from 'react';
import {
  Card,
  Button,
  Modal,
  Form,
  Select,
  Input,
  Alert,
  Space,
  Tag,
  Typography,
  List,
  Empty,
  Spin,
  notification,
  Badge
} from 'antd';
import {
  LockOutlined,
  UnlockOutlined,
  QuestionCircleOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  ClockCircleOutlined,
  TeamOutlined,
  EyeOutlined,
  EditOutlined,
  ExportOutlined,
  ShareAltOutlined,
  HistoryOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../store/auth';
import {
  PermissionRequest,
  PermissionType,
  RequestStatus,
  VaultInfo,
  permissionTypeDisplayNames,
  requestStatusColors
} from '../types/vault';
import dayjs from 'dayjs';

const { Text, Title } = Typography;
const { TextArea } = Input;

interface VaultPermissionRequestProps {
  vaultId?: number;
  vaultName?: string;
  onRequestSubmitted?: () => void;
}

const VaultPermissionRequest: React.FC<VaultPermissionRequestProps> = ({
  vaultId,
  vaultName,
  onRequestSubmitted
}) => {
  const { token, user } = useAuthStore();
  const [requests, setRequests] = useState<PermissionRequest[]>([]);
  const [loading, setLoading] = useState(false);
  const [requestModalVisible, setRequestModalVisible] = useState(false);
  const [form] = Form.useForm();
  const [vaults] = useState<VaultInfo[]>([]);

  useEffect(() => {
    if (token) {
      loadRequests();
      if (!vaultId) {
        loadVaults();
      }
    }
  }, [token, vaultId]);

  const loadRequests = async () => {
    if (!token || !user) return;

    setLoading(true);
    try {
      const allRequests = await invoke<PermissionRequest[]>('get_user_vault_permissions', {
        token,
        userId: user.id
      });

      // Filter to show only requests created by this user
      const myRequests = allRequests.filter(r => r.requested_by === user.id);
      
      // If vaultId is provided, filter to that vault
      const filteredRequests = vaultId 
        ? myRequests.filter(r => r.vault_id === vaultId)
        : myRequests;

      setRequests(filteredRequests);
    } catch (error) {
      console.error('Failed to load permission requests:', error);
    } finally {
      setLoading(false);
    }
  };

  const loadVaults = async () => {
    if (!token) return;

    try {
      // In a real implementation, this would load all vaults the user knows about
      // For now, we'll use a placeholder
      // const vaultList = await invoke<VaultInfo[]>('list_all_vaults', { token });
      // setVaults(vaultList); // Would uncomment when API is available
    } catch (error) {
      console.error('Failed to load vaults:', error);
    }
  };

  const handleSubmitRequest = async (values: any) => {
    if (!token || !user) return;

    try {
      await invoke('create_permission_request', {
        token,
        request: {
          vault_id: values.vault_id || vaultId,
          requested_permission: values.permission_type,
          requested_by: user.id
        }
      });

      notification.success({
        message: 'Request Submitted',
        description: 'Your permission request has been submitted for approval.'
      });

      setRequestModalVisible(false);
      form.resetFields();
      loadRequests();
      onRequestSubmitted?.();
    } catch (error) {
      console.error('Failed to submit request:', error);
      notification.error({
        message: 'Error',
        description: 'Failed to submit permission request'
      });
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

  const getStatusIcon = (status: RequestStatus) => {
    switch (status) {
      case 'Pending':
        return <ClockCircleOutlined />;
      case 'Approved':
        return <CheckCircleOutlined />;
      case 'Denied':
        return <CloseCircleOutlined />;
      case 'Expired':
        return <ClockCircleOutlined />;
      default:
        return <QuestionCircleOutlined />;
    }
  };

  const formatDate = (dateString: string) => {
    return dayjs(dateString).format('MMM D, YYYY HH:mm');
  };

  const pendingRequests = requests.filter(r => r.status === 'Pending');
  const processedRequests = requests.filter(r => r.status !== 'Pending');

  return (
    <div>
      <Card
        title={
          <Space>
            <TeamOutlined />
            <span>Permission Requests</span>
            {vaultName && <Tag>{vaultName}</Tag>}
          </Space>
        }
        extra={
          <Badge count={pendingRequests.length} offset={[-5, 5]}>
            <Button
              type="primary"
              icon={<UnlockOutlined />}
              onClick={() => setRequestModalVisible(true)}
            >
              Request Access
            </Button>
          </Badge>
        }
      >
        {loading ? (
          <div style={{ textAlign: 'center', padding: '24px' }}>
            <Spin />
          </div>
        ) : requests.length === 0 ? (
          <Empty
            description="No permission requests"
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          >
            <Button
              type="primary"
              icon={<UnlockOutlined />}
              onClick={() => setRequestModalVisible(true)}
            >
              Request Your First Access
            </Button>
          </Empty>
        ) : (
          <div>
            {pendingRequests.length > 0 && (
              <div style={{ marginBottom: '24px' }}>
                <Title level={5} style={{ marginBottom: '16px' }}>
                  <ClockCircleOutlined /> Pending Requests
                </Title>
                <List
                  dataSource={pendingRequests}
                  renderItem={(request) => (
                    <List.Item>
                      <List.Item.Meta
                        avatar={getStatusIcon(request.status)}
                        title={
                          <Space>
                            <Text strong>Vault #{request.vault_id}</Text>
                            <Tag icon={getPermissionIcon(request.requested_permission)}>
                              {permissionTypeDisplayNames[request.requested_permission]}
                            </Tag>
                          </Space>
                        }
                        description={
                          <div>
                            <Text type="secondary">
                              Requested on {formatDate(request.created_at)}
                            </Text>
                            <Alert
                              message="Awaiting administrator approval"
                              type="info"
                              showIcon
                              style={{ marginTop: '8px' }}
                            />
                          </div>
                        }
                      />
                    </List.Item>
                  )}
                />
              </div>
            )}

            {processedRequests.length > 0 && (
              <div>
                <Title level={5} style={{ marginBottom: '16px' }}>
                  <HistoryOutlined /> Request History
                </Title>
                <List
                  dataSource={processedRequests}
                  renderItem={(request) => (
                    <List.Item>
                      <List.Item.Meta
                        avatar={getStatusIcon(request.status)}
                        title={
                          <Space>
                            <Text strong>Vault #{request.vault_id}</Text>
                            <Tag icon={getPermissionIcon(request.requested_permission)}>
                              {permissionTypeDisplayNames[request.requested_permission]}
                            </Tag>
                            <Tag color={requestStatusColors[request.status]}>
                              {request.status}
                            </Tag>
                          </Space>
                        }
                        description={
                          <div>
                            <Text type="secondary">
                              Requested on {formatDate(request.created_at)}
                            </Text>
                            {request.updated_at !== request.created_at && (
                              <div>
                                <Text type="secondary">
                                  {request.status} on {formatDate(request.updated_at)}
                                  {request.approved_by && ` by User #${request.approved_by}`}
                                </Text>
                              </div>
                            )}
                            {request.approval_notes && (
                              <div style={{ marginTop: '4px' }}>
                                <Text type="secondary" style={{ fontStyle: 'italic' }}>
                                  Note: {request.approval_notes}
                                </Text>
                              </div>
                            )}
                          </div>
                        }
                      />
                    </List.Item>
                  )}
                />
              </div>
            )}
          </div>
        )}
      </Card>

      <Modal
        title="Request Vault Access"
        open={requestModalVisible}
        onCancel={() => {
          setRequestModalVisible(false);
          form.resetFields();
        }}
        footer={null}
        width={600}
      >
        <Form
          form={form}
          layout="vertical"
          onFinish={handleSubmitRequest}
          initialValues={{
            vault_id: vaultId,
            permission_type: 'Read'
          }}
        >
          {!vaultId && (
            <Form.Item
              name="vault_id"
              label="Select Vault"
              rules={[{ required: true, message: 'Please select a vault' }]}
            >
              <Select
                placeholder="Select a vault to request access"
                showSearch
                optionFilterProp="children"
              >
                {vaults.map(vault => (
                  <Select.Option key={vault.vault.id} value={vault.vault.id}>
                    {vault.vault.name}
                  </Select.Option>
                ))}
              </Select>
            </Form.Item>
          )}

          <Form.Item
            name="permission_type"
            label="Permission Type"
            rules={[{ required: true, message: 'Please select permission type' }]}
          >
            <Select>
              {Object.entries(permissionTypeDisplayNames).map(([key, value]) => (
                <Select.Option key={key} value={key}>
                  <Space>
                    {getPermissionIcon(key as PermissionType)}
                    {value}
                  </Space>
                </Select.Option>
              ))}
            </Select>
          </Form.Item>

          <Form.Item
            name="reason"
            label="Reason for Request (Optional)"
          >
            <TextArea
              rows={3}
              placeholder="Provide a reason for your access request"
              maxLength={500}
            />
          </Form.Item>

          <Alert
            message="Access Request Process"
            description="Your request will be reviewed by an administrator. You will be notified once a decision has been made."
            type="info"
            showIcon
            style={{ marginBottom: '16px' }}
          />

          <div style={{ textAlign: 'right' }}>
            <Space>
              <Button onClick={() => {
                setRequestModalVisible(false);
                form.resetFields();
              }}>
                Cancel
              </Button>
              <Button type="primary" htmlType="submit">
                Submit Request
              </Button>
            </Space>
          </div>
        </Form>
      </Modal>
    </div>
  );
};

export default VaultPermissionRequest;