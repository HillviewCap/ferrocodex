import React, { useState, useEffect } from 'react';
import {
  Modal,
  Steps,
  Form,
  Input,
  Alert,
  Space,
  Typography,
  Divider,
  Button,
  Spin,
  Card,
  List,
  Tag
} from 'antd';
import {
  TrophyOutlined,
  WarningOutlined,
  InfoCircleOutlined,
  CheckCircleOutlined,
  StarOutlined
} from '@ant-design/icons';
import { ConfigurationVersionInfo } from '../types/assets';
import ConfigurationStatusBadge from './ConfigurationStatusBadge';
import { invoke } from '@tauri-apps/api/core';

const { Text, Title, Paragraph } = Typography;
const { TextArea } = Input;
const { Step } = Steps;

interface PromoteToGoldenWizardProps {
  visible: boolean;
  onCancel: () => void;
  onSuccess: () => void;
  version: ConfigurationVersionInfo | null;
  token: string;
}

const PromoteToGoldenWizard: React.FC<PromoteToGoldenWizardProps> = ({
  visible,
  onCancel,
  onSuccess,
  version,
  token
}) => {
  const [form] = Form.useForm();
  const [currentStep, setCurrentStep] = useState(0);
  const [loading, setLoading] = useState(false);
  const [checkingEligibility, setCheckingEligibility] = useState(false);
  const [eligible, setEligible] = useState(false);
  const [existingGolden, setExistingGolden] = useState<ConfigurationVersionInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (visible && version) {
      resetWizard();
      checkPromotionEligibility();
      checkExistingGolden();
    }
  }, [visible, version]);

  const resetWizard = () => {
    setCurrentStep(0);
    setError(null);
    setEligible(false);
    setExistingGolden(null);
    form.resetFields();
  };

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
      // Non-critical error, just log it
      console.warn('Failed to check existing golden version:', err);
    }
  };

  const handleNext = () => {
    if (currentStep < 2) {
      setCurrentStep(currentStep + 1);
    }
  };

  const handlePrev = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1);
    }
  };

  const handlePromote = async () => {
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
    resetWizard();
    onCancel();
  };

  if (!version) return null;

  const steps = [
    {
      title: 'Information',
      icon: <InfoCircleOutlined />,
      content: (
        <Space direction="vertical" size="large" style={{ width: '100%' }}>
          <div>
            <Title level={4} style={{ color: '#1890ff', marginBottom: '16px' }}>
              <TrophyOutlined style={{ marginRight: '8px' }} />
              Promote to Golden Image
            </Title>
            <Paragraph>
              You are about to promote this configuration version to the <strong>Golden Image</strong> status. 
              This designation marks it as the official, trusted master version for disaster recovery purposes.
            </Paragraph>
          </div>

          <Card size="small" style={{ backgroundColor: '#f6ffed', border: '1px solid #b7eb8f' }}>
            <Title level={5} style={{ color: '#52c41a', margin: 0, marginBottom: '8px' }}>
              What is a Golden Image?
            </Title>
            <List size="small" split={false}>
              <List.Item style={{ padding: '4px 0' }}>
                <Text>• The authoritative, trusted version of a configuration</Text>
              </List.Item>
              <List.Item style={{ padding: '4px 0' }}>
                <Text>• Used as the reference for disaster recovery operations</Text>
              </List.Item>
              <List.Item style={{ padding: '4px 0' }}>
                <Text>• Only one Golden version can exist per asset at a time</Text>
              </List.Item>
              <List.Item style={{ padding: '4px 0' }}>
                <Text>• Prominently displayed in the asset's main view</Text>
              </List.Item>
            </List>
          </Card>

          <div>
            <Title level={5} style={{ marginBottom: '8px' }}>Version to Promote</Title>
            <Card size="small">
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
          </div>
        </Space>
      )
    },
    {
      title: 'Impact Assessment',
      icon: <WarningOutlined />,
      content: (
        <Space direction="vertical" size="large" style={{ width: '100%' }}>
          <div>
            <Title level={4} style={{ color: '#fa8c16', marginBottom: '16px' }}>
              <WarningOutlined style={{ marginRight: '8px' }} />
              Impact Assessment
            </Title>
            <Paragraph>
              Please review the impact of this promotion before proceeding.
            </Paragraph>
          </div>

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
            <>
              <Alert
                message="Promotion Eligible"
                description="This version meets the requirements for Golden promotion."
                type="success"
                showIcon
              />

              {existingGolden && (
                <Card 
                  title={
                    <Space>
                      <WarningOutlined style={{ color: '#fa8c16' }} />
                      <Text>Existing Golden Version Will Be Archived</Text>
                    </Space>
                  }
                  size="small"
                  style={{ border: '1px solid #ffd591', backgroundColor: '#fff7e6' }}
                >
                  <div style={{ marginBottom: '12px' }}>
                    <Text>
                      The following version is currently the Golden Image and will be automatically 
                      archived when this promotion completes. <strong>Archived versions can be restored later if needed.</strong>
                    </Text>
                  </div>
                  <Card size="small" style={{ backgroundColor: '#fafafa' }}>
                    <Space>
                      <Text strong>{existingGolden.file_name}</Text>
                      <Tag color="gold">{existingGolden.version_number}</Tag>
                      <ConfigurationStatusBadge status={existingGolden.status} />
                    </Space>
                    <div style={{ marginTop: '8px' }}>
                      <Text type="secondary">
                        Created by {existingGolden.author_username} • {new Date(existingGolden.created_at).toLocaleDateString()}
                      </Text>
                    </div>
                  </Card>
                </Card>
              )}

              <Card title="Promotion Effects" size="small">
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
                        <Text>Previous Golden version will be archived (can be restored later)</Text>
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
            </>
          )}
        </Space>
      )
    },
    {
      title: 'Confirmation',
      icon: <CheckCircleOutlined />,
      content: (
        <Space direction="vertical" size="large" style={{ width: '100%' }}>
          <div>
            <Title level={4} style={{ color: '#52c41a', marginBottom: '16px' }}>
              <StarOutlined style={{ marginRight: '8px' }} />
              Final Confirmation
            </Title>
            <Paragraph>
              Please provide a reason for this promotion and confirm your intent to proceed.
            </Paragraph>
          </div>

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
                rows={4}
                placeholder="Explain why this version should be promoted to Golden Image..."
                showCount
                maxLength={500}
              />
            </Form.Item>
          </Form>

          <Alert
            message="This action cannot be undone"
            description="Once promoted to Golden, this version will become the official master version. The previous Golden version (if any) will be automatically archived but can be restored if needed through the version history."
            type="warning"
            showIcon
          />
        </Space>
      )
    }
  ];

  const getModalTitle = () => {
    switch (currentStep) {
      case 0: return 'Golden Image Promotion - Information';
      case 1: return 'Golden Image Promotion - Impact Assessment';
      case 2: return 'Golden Image Promotion - Confirmation';
      default: return 'Golden Image Promotion';
    }
  };

  const getFooterButtons = () => {
    const buttons = [];
    
    if (currentStep > 0) {
      buttons.push(
        <Button key="prev" onClick={handlePrev} disabled={loading}>
          Previous
        </Button>
      );
    }
    
    buttons.push(
      <Button key="cancel" onClick={handleCancel} disabled={loading}>
        Cancel
      </Button>
    );
    
    if (currentStep < 2) {
      const isNextDisabled = currentStep === 1 && (!eligible || checkingEligibility);
      buttons.push(
        <Button 
          key="next" 
          type="primary" 
          onClick={handleNext}
          disabled={isNextDisabled || loading}
        >
          Next
        </Button>
      );
    } else {
      buttons.push(
        <Button 
          key="promote" 
          type="primary" 
          onClick={handlePromote}
          loading={loading}
          icon={<TrophyOutlined />}
        >
          Promote to Golden
        </Button>
      );
    }
    
    return buttons;
  };

  return (
    <Modal
      title={getModalTitle()}
      open={visible}
      onCancel={handleCancel}
      width={800}
      destroyOnHidden
      footer={getFooterButtons()}
      maskClosable={!loading}
      closable={!loading}
    >
      <Space direction="vertical" size="large" style={{ width: '100%' }}>
        <Steps current={currentStep} size="small">
          {steps.map((step, index) => (
            <Step key={index} title={step.title} icon={step.icon} />
          ))}
        </Steps>
        
        <Divider style={{ margin: '12px 0' }} />
        
        <div style={{ minHeight: '400px' }}>
          {steps[currentStep].content}
        </div>
      </Space>
    </Modal>
  );
};

export default PromoteToGoldenWizard;