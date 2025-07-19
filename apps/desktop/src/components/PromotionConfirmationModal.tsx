import React, { useState, useEffect } from 'react';
import {
  Modal,
  Form,
  Input,
  Alert,
  Space,
  Typography,
  Card,
  Tag,
  List,
  Spin
} from 'antd';
import {
  TrophyOutlined,
  WarningOutlined,
  CheckCircleOutlined,
  InfoCircleOutlined
} from '@ant-design/icons';
import { ConfigurationVersionInfo } from '../types/assets';
import ConfigurationStatusBadge from './ConfigurationStatusBadge';
import { invoke } from '@tauri-apps/api/core';

const { Text, Paragraph } = Typography;
const { TextArea } = Input;

interface PromotionConfirmationModalProps {
  visible: boolean;
  onCancel: () => void;
  onSuccess: () => void;
  version: ConfigurationVersionInfo | null;
  token: string;
}

const PromotionConfirmationModal: React.FC<PromotionConfirmationModalProps> = ({
  visible,
  onCancel,
  onSuccess,
  version,
  token
}) => {
  const [form] = Form.useForm();
  const [loading, setLoading] = useState(false);
  const [checkingEligibility, setCheckingEligibility] = useState(false);
  const [eligible, setEligible] = useState(false);
  const [existingGolden, setExistingGolden] = useState<ConfigurationVersionInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (visible && version) {
      form.resetFields();
      setError(null);
      checkPromotionEligibility();
      checkExistingGolden();
    }
  }, [visible, version, form]);

  const checkPromotionEligibility = async () => {
    if (!version) return;

    setCheckingEligibility(true);
    try {
      const isEligible = await invoke<boolean>('get_promotion_eligibility', {
        token,
        versionId: version.id
      });
      setEligible(isEligible);
    } catch (err) {
      setError(err as string);
    } finally {
      setCheckingEligibility(false);
    }
  };

  const checkExistingGolden = async () => {
    if (!version) return;

    try {
      const golden = await invoke<ConfigurationVersionInfo | null>('get_golden_version', {
        token,
        assetId: version.asset_id
      });
      setExistingGolden(golden);
    } catch (err) {
      console.warn('Failed to check existing golden version:', err);
    }
  };

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      setLoading(true);
      setError(null);

      await invoke('promote_to_golden', {
        token,
        versionId: version!.id,
        promotionReason: values.reason || null
      });

      form.resetFields();
      onSuccess();
    } catch (err) {
      if (typeof err === 'string') {
        setError(err);
      } else if (err instanceof Error) {
        setError(err.message);
      } else {
        setError('Failed to promote to Golden');
      }
    } finally {
      setLoading(false);
    }
  };

  const handleCancel = () => {
    form.resetFields();
    setError(null);
    onCancel();
  };

  if (!version) return null;

  return (
    <Modal
      title={
        <Space>
          <TrophyOutlined style={{ color: '#fa8c16' }} />
          <Text>Promote to Golden Image</Text>
        </Space>
      }
      open={visible}
      onOk={handleSubmit}
      onCancel={handleCancel}
      confirmLoading={loading}
      width={700}
      destroyOnClose
      okText="Promote to Golden"
      cancelText="Cancel"
      okButtonProps={{
        disabled: checkingEligibility || !eligible,
        icon: <TrophyOutlined />
      }}
    >
      <Space direction="vertical" size="large" style={{ width: '100%' }}>
        {/* Version Information */}
        <Card size="small" title="Version to Promote">
          <Space>
            <Text strong>{version.file_name}</Text>
            <Tag color="blue">{version.version_number}</Tag>
            <ConfigurationStatusBadge status={version.status} />
          </Space>
          <div style={{ marginTop: '8px' }}>
            <Text type="secondary">
              Created by {version.author_username} • {new Date(version.created_at).toLocaleDateString()}
            </Text>
          </div>
          {version.notes && (
            <div style={{ marginTop: '8px' }}>
              <Text style={{ fontStyle: 'italic' }}>{version.notes}</Text>
            </div>
          )}
        </Card>

        {/* Eligibility Check */}
        {checkingEligibility ? (
          <Card>
            <div style={{ textAlign: 'center', padding: '20px' }}>
              <Spin size="large" />
              <div style={{ marginTop: '16px' }}>
                <Text>Checking promotion eligibility...</Text>
              </div>
            </div>
          </Card>
        ) : !eligible ? (
          <Alert
            message="Not Eligible for Promotion"
            description="This version cannot be promoted to Golden. Only versions with 'Approved' status are eligible for Golden promotion."
            type="error"
            showIcon
          />
        ) : (
          <Alert
            message="Ready for Promotion"
            description="This version meets the requirements for Golden promotion."
            type="success"
            showIcon
          />
        )}

        {/* Existing Golden Warning */}
        {eligible && existingGolden && (
          <Alert
            message="Existing Golden Version Will Be Archived"
            description={
              <div>
                <Paragraph style={{ margin: '8px 0' }}>
                  The current Golden version <strong>{existingGolden.file_name} ({existingGolden.version_number})</strong> 
                  will be automatically archived when this promotion completes.
                </Paragraph>
              </div>
            }
            type="warning"
            showIcon
            icon={<WarningOutlined />}
          />
        )}

        {/* Impact Summary */}
        {eligible && (
          <Card 
            title={
              <Space>
                <InfoCircleOutlined style={{ color: '#1890ff' }} />
                <Text>Promotion Effects</Text>
              </Space>
            }
            size="small"
          >
            <List size="small" split={false}>
              <List.Item style={{ padding: '4px 0' }}>
                <Space>
                  <CheckCircleOutlined style={{ color: '#52c41a' }} />
                  <Text>Version status will change to <strong>Golden</strong></Text>
                </Space>
              </List.Item>
              <List.Item style={{ padding: '4px 0' }}>
                <Space>
                  <CheckCircleOutlined style={{ color: '#52c41a' }} />
                  <Text>Version will be prominently displayed in asset view</Text>
                </Space>
              </List.Item>
              {existingGolden && (
                <List.Item style={{ padding: '4px 0' }}>
                  <Space>
                    <InfoCircleOutlined style={{ color: '#1890ff' }} />
                    <Text>Previous Golden version will be archived</Text>
                  </Space>
                </List.Item>
              )}
              <List.Item style={{ padding: '4px 0' }}>
                <Space>
                  <InfoCircleOutlined style={{ color: '#1890ff' }} />
                  <Text>Audit trail will record this promotion</Text>
                </Space>
              </List.Item>
            </List>
          </Card>
        )}

        {/* Error Display */}
        {error && (
          <Alert
            message="Promotion Failed"
            description={error}
            type="error"
            showIcon
            closable
            onClose={() => setError(null)}
          />
        )}

        {/* Promotion Form */}
        {eligible && (
          <Form
            form={form}
            layout="vertical"
            requiredMark={false}
          >
            <Form.Item
              label="Promotion Reason"
              name="reason"
              rules={[
                { required: true, message: 'Please provide a reason for this promotion' },
                { max: 500, message: 'Reason cannot exceed 500 characters' }
              ]}
            >
              <TextArea
                rows={3}
                placeholder="Explain why this version should be promoted to Golden Image..."
                showCount
                maxLength={500}
              />
            </Form.Item>
          </Form>
        )}

        {/* Warning */}
        {eligible && (
          <Alert
            message="This action cannot be undone"
            description="Once promoted to Golden, this version will become the official master version."
            type="info"
            showIcon
          />
        )}
      </Space>
    </Modal>
  );
};

export default PromotionConfirmationModal;