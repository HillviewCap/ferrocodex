import React, { useState, useEffect } from 'react';
import { 
  Modal, 
  Form, 
  Input, 
  Button, 
  message, 
  Space,
  Typography,
  Card,
  Alert,
  Tag,
  Avatar,
  Divider
} from 'antd';
import { 
  BranchesOutlined,
  InfoCircleOutlined,
  CheckCircleOutlined,
  FileOutlined,
  CalendarOutlined,
  UserOutlined
} from '@ant-design/icons';
import { ConfigurationVersionInfo } from '../types/assets';
import { BranchInfo, validateBranchName, validateBranchDescription } from '../types/branches';
import useAuthStore from '../store/auth';

const { Text } = Typography;
const { TextArea } = Input;

interface CreateBranchModalProps {
  visible: boolean;
  onCancel: () => void;
  onSuccess: (branch: BranchInfo) => void;
  parentVersion: ConfigurationVersionInfo;
  assetId: number;
}

const CreateBranchModal: React.FC<CreateBranchModalProps> = ({
  visible,
  onCancel,
  onSuccess,
  parentVersion,
  assetId
}) => {
  const { token } = useAuthStore();
  const [form] = Form.useForm();
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    if (visible) {
      form.resetFields();
    }
  }, [visible, form]);

  const handleReset = () => {
    setCreating(false);
    form.resetFields();
  };

  const handleCancel = () => {
    if (!creating) {
      handleReset();
      onCancel();
    }
  };

  const handleSubmit = async () => {
    if (!token) return;

    try {
      const values = await form.validateFields();
      setCreating(true);

      const response = await window.__TAURI__.invoke('create_branch', {
        token,
        name: values.name,
        description: values.description || null,
        asset_id: assetId,
        parent_version_id: parentVersion.id
      });

      message.success('Branch created successfully!');
      onSuccess(response);
      handleReset();
      
    } catch (error) {
      console.error('Branch creation failed:', error);
      message.error(`Failed to create branch: ${error}`);
      setCreating(false);
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

  const getVersionColor = (versionNumber: string) => {
    const versionNum = parseInt(versionNumber.replace('v', ''));
    if (versionNum === 1) return 'green';
    if (versionNum <= 5) return 'blue';
    if (versionNum <= 10) return 'orange';
    return 'purple';
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 Bytes';
    
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };

  return (
    <Modal
      title={
        <Space>
          <BranchesOutlined />
          Create Branch
        </Space>
      }
      open={visible}
      onCancel={handleCancel}
      footer={null}
      width={720}
      closable={!creating}
      maskClosable={!creating}
      aria-labelledby="create-branch-modal-title"
      aria-describedby="create-branch-modal-description"
    >
      <div style={{ marginBottom: '24px' }}>
        <Alert
          message="Create a Branch"
          description="Create a new branch from this configuration version to safely experiment with changes without affecting the main line of development."
          type="info"
          showIcon
          icon={<InfoCircleOutlined />}
          style={{ marginBottom: '16px' }}
        />

        <Card title="Parent Version" size="small" style={{ marginBottom: '16px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
            <Avatar 
              icon={<FileOutlined />} 
              size={40}
              style={{ backgroundColor: '#1890ff' }}
            />
            <div style={{ flex: 1 }}>
              <div style={{ marginBottom: '4px' }}>
                <Space size={8}>
                  <Tag color={getVersionColor(parentVersion.version_number)}>
                    {parentVersion.version_number}
                  </Tag>
                  <Text strong>
                    {parentVersion.file_name}
                  </Text>
                </Space>
              </div>
              <div>
                <Space size={12} wrap>
                  <Space size={4}>
                    <UserOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                    <Text type="secondary" style={{ fontSize: '12px' }}>
                      {parentVersion.author_username}
                    </Text>
                  </Space>
                  <Space size={4}>
                    <CalendarOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                    <Text type="secondary" style={{ fontSize: '12px' }}>
                      {formatDate(parentVersion.created_at)}
                    </Text>
                  </Space>
                  <Space size={4}>
                    <FileOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                    <Text type="secondary" style={{ fontSize: '12px' }}>
                      {formatFileSize(parentVersion.file_size)}
                    </Text>
                  </Space>
                </Space>
              </div>
            </div>
          </div>
          {parentVersion.notes && (
            <>
              <Divider style={{ margin: '12px 0' }} />
              <Text type="secondary" style={{ fontSize: '12px', fontStyle: 'italic' }}>
                {parentVersion.notes}
              </Text>
            </>
          )}
        </Card>
      </div>

      <Form 
        form={form} 
        layout="vertical"
        onFinish={handleSubmit}
      >
        <Form.Item
          name="name"
          label="Branch Name"
          rules={[
            { required: true, message: 'Please enter a branch name' },
            {
              validator: (_, value) => {
                if (!value) return Promise.resolve();
                const error = validateBranchName(value);
                return error ? Promise.reject(error) : Promise.resolve();
              }
            }
          ]}
          tooltip="A unique name for this branch. Use descriptive names like 'test-new-parameters' or 'feature-improvements'"
        >
          <Input 
            placeholder="e.g., test-new-parameters, feature-improvements"
            autoFocus
            addonBefore={<BranchesOutlined />}
          />
        </Form.Item>

        <Form.Item
          name="description"
          label="Description (Optional)"
          rules={[
            {
              validator: (_, value) => {
                if (!value) return Promise.resolve();
                const error = validateBranchDescription(value);
                return error ? Promise.reject(error) : Promise.resolve();
              }
            }
          ]}
          tooltip="Describe the purpose of this branch"
        >
          <TextArea
            rows={3}
            placeholder="Describe the purpose of this branch..."
            showCount
            maxLength={500}
          />
        </Form.Item>

        <Alert
          message="Branch Guidelines"
          description={
            <ul style={{ marginBottom: 0, paddingLeft: '20px' }}>
              <li>Branch names should be descriptive and use kebab-case</li>
              <li>Use branches for experimental changes or feature development</li>
              <li>Each branch maintains its own configuration history</li>
              <li>Branches can be merged back or kept separate as needed</li>
            </ul>
          }
          type="info"
          showIcon
          style={{ marginBottom: '24px' }}
        />

        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px' }}>
          <Button onClick={handleCancel} disabled={creating}>
            Cancel
          </Button>
          <Button 
            type="primary" 
            htmlType="submit" 
            loading={creating}
            icon={<CheckCircleOutlined />}
          >
            Create Branch
          </Button>
        </div>
      </Form>
    </Modal>
  );
};

export default CreateBranchModal;