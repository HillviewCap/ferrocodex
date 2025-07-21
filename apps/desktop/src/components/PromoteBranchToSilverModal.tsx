import React, { useState } from 'react';
import { 
  Modal, 
  Form, 
  Input, 
  Button, 
  Space,
  Typography,
  Alert,
  Tag,
  Card,
  Divider,
  message
} from 'antd';
import { 
  TrophyOutlined,
  InfoCircleOutlined,
  CheckCircleOutlined,
  BranchesOutlined,
  UserOutlined,
  CalendarOutlined,
  ArrowRightOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { BranchInfo } from '../types/branches';
import useAuthStore from '../store/auth';
import { useNavigate } from 'react-router-dom';

const { Text } = Typography;
const { TextArea } = Input;

interface PromoteBranchToSilverModalProps {
  visible: boolean;
  onCancel: () => void;
  onSuccess: () => void;
  branch: BranchInfo;
  assetId: number;
}

const PromoteBranchToSilverModal: React.FC<PromoteBranchToSilverModalProps> = ({
  visible,
  onCancel,
  onSuccess,
  branch,
  assetId
}) => {
  const { token } = useAuthStore();
  const navigate = useNavigate();
  const [form] = Form.useForm();
  const [promoting, setPromoting] = useState(false);

  const handleReset = () => {
    setPromoting(false);
    form.resetFields();
  };

  const handleCancel = () => {
    if (!promoting) {
      handleReset();
      onCancel();
    }
  };

  const handleSubmit = async () => {
    if (!token) return;

    try {
      const values = await form.validateFields();
      setPromoting(true);

      await invoke<number>('promote_branch_to_silver', {
        token,
        branchId: branch.id,
        promotionNotes: values.notes || null
      });

      message.success({
        content: 'Branch promoted to Silver status successfully!',
        duration: 5,
        icon: <TrophyOutlined style={{ color: '#00CED1' }} />
      });

      onSuccess();
      handleReset();
      
      // Navigate to the configuration history to see the new Silver version
      navigate(`/assets/${assetId}/history`);
      
    } catch (error) {
      console.error('Branch promotion failed:', error);
      message.error(`Failed to promote branch: ${error}`);
      setPromoting(false);
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

  return (
    <Modal
      title={
        <Space>
          <TrophyOutlined style={{ color: '#00CED1' }} />
          Promote Branch to Silver
        </Space>
      }
      open={visible}
      onCancel={handleCancel}
      footer={null}
      width={720}
      closable={!promoting}
      maskClosable={!promoting}
    >
      <div style={{ marginBottom: '24px' }}>
        <Alert
          message="Promote to Silver Status"
          description="This will create a new configuration version in the main line with Silver status, marking it as a candidate for review. The branch will remain active for continued development."
          type="info"
          showIcon
          icon={<InfoCircleOutlined />}
          style={{ marginBottom: '16px' }}
        />

        <Card title="Branch Information" size="small" style={{ marginBottom: '16px' }}>
          <div style={{ marginBottom: '12px' }}>
            <Space size={8}>
              <BranchesOutlined style={{ color: branch.is_active ? '#52c41a' : '#ff7875' }} />
              <Text strong style={{ fontSize: '16px' }}>
                {branch.name}
              </Text>
              <Tag color={branch.is_active ? 'green' : 'red'}>
                {branch.is_active ? 'Active' : 'Inactive'}
              </Tag>
            </Space>
          </div>
          
          {branch.description && (
            <>
              <Text type="secondary" style={{ fontSize: '13px', fontStyle: 'italic' }}>
                {branch.description}
              </Text>
              <Divider style={{ margin: '12px 0' }} />
            </>
          )}
          
          <Space size={16} wrap>
            <Space size={4}>
              <UserOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
              <Text type="secondary" style={{ fontSize: '12px' }}>
                Created by: {branch.created_by_username}
              </Text>
            </Space>
            <Space size={4}>
              <CalendarOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
              <Text type="secondary" style={{ fontSize: '12px' }}>
                {formatDate(branch.created_at)}
              </Text>
            </Space>
          </Space>
        </Card>

        <Card size="small" style={{ marginBottom: '16px', backgroundColor: '#f0f9ff' }}>
          <div style={{ textAlign: 'center' }}>
            <Space direction="vertical" size={8}>
              <Text type="secondary">Promotion Path:</Text>
              <Space size={16}>
                <Tag color="blue">Branch</Tag>
                <ArrowRightOutlined />
                <Tag color="cyan" icon={<TrophyOutlined />}>Silver</Tag>
                <ArrowRightOutlined />
                <Tag color="success">Approved</Tag>
                <ArrowRightOutlined />
                <Tag color="gold">Golden</Tag>
              </Space>
            </Space>
          </div>
        </Card>
      </div>

      <Form 
        form={form} 
        layout="vertical"
        onFinish={handleSubmit}
      >
        <Form.Item
          name="notes"
          label="Promotion Notes"
          rules={[
            { required: true, message: 'Please provide promotion notes' },
            { max: 1000, message: 'Notes cannot exceed 1000 characters' }
          ]}
          tooltip="Describe why this branch is ready for review and what changes it contains"
        >
          <TextArea
            rows={4}
            placeholder="Describe the changes in this branch and why it's ready for review..."
            showCount
            maxLength={1000}
          />
        </Form.Item>

        <Alert
          message="What happens next?"
          description={
            <ul style={{ marginBottom: 0, paddingLeft: '20px' }}>
              <li>A new configuration version will be created in the main line</li>
              <li>The version will have Silver status for review</li>
              <li>Administrators can review and approve the Silver version</li>
              <li>Your branch remains active for continued development</li>
            </ul>
          }
          type="info"
          showIcon
          style={{ marginBottom: '24px' }}
        />

        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px' }}>
          <Button onClick={handleCancel} disabled={promoting}>
            Cancel
          </Button>
          <Button 
            type="primary" 
            htmlType="submit" 
            loading={promoting}
            icon={<CheckCircleOutlined />}
            style={{ backgroundColor: '#00CED1', borderColor: '#00CED1' }}
          >
            Promote to Silver
          </Button>
        </div>
      </Form>
    </Modal>
  );
};

export default PromoteBranchToSilverModal;