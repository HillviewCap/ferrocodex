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
  Checkbox,
  Tooltip
} from 'antd';
import { 
  BranchesOutlined,
  InfoCircleOutlined,
  CheckCircleOutlined,
  FileOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { ConfigurationVersionInfo } from '../types/assets';
import { BranchInfo, validateBranchName, validateBranchDescription } from '../types/branches';
import useAuthStore from '../store/auth';
import ExportConfirmationModal from './ExportConfirmationModal';

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
  const [downloadAfterCreate, setDownloadAfterCreate] = useState(false);
  const [showExportModal, setShowExportModal] = useState(false);
  const [createdBranch, setCreatedBranch] = useState<BranchInfo | null>(null);

  useEffect(() => {
    if (visible) {
      form.resetFields();
      setDownloadAfterCreate(false);
      setCreatedBranch(null);
    }
  }, [visible, form]);

  const handleReset = () => {
    setCreating(false);
    form.resetFields();
    setDownloadAfterCreate(false);
    setCreatedBranch(null);
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

      const response = await invoke<BranchInfo>('create_branch', {
        token,
        name: values.name,
        description: values.description || null,
        assetId: assetId,
        parentVersionId: parentVersion.id
      });

      message.success('Branch created successfully!');
      setCreatedBranch(response);
      
      if (downloadAfterCreate) {
        // Show export modal for the created branch
        setShowExportModal(true);
        setCreating(false);
      } else {
        onSuccess(response);
        handleReset();
      }
      
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
      width={600}
      closable={!creating}
      maskClosable={!creating}
      aria-labelledby="create-branch-modal-title"
      aria-describedby="create-branch-modal-description"
    >
      <div style={{ marginBottom: '24px' }}>
        <Alert
          message="Create a Branch"
          description="Create a new branch from this version to safely experiment with changes."
          type="info"
          showIcon
          style={{ marginBottom: '16px' }}
        />

        <Card size="small" style={{ marginBottom: '16px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <Avatar 
              icon={<FileOutlined />} 
              size={32}
              style={{ backgroundColor: '#1890ff' }}
            />
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ marginBottom: '2px' }}>
                <Space size={6}>
                  <Tag color={getVersionColor(parentVersion.version_number)}>
                    {parentVersion.version_number}
                  </Tag>
                  <Text strong style={{ fontSize: '13px' }}>
                    {parentVersion.file_name}
                  </Text>
                </Space>
              </div>
              <div>
                <Text type="secondary" style={{ fontSize: '11px' }}>
                  {parentVersion.author_username} · {formatDate(parentVersion.created_at)} · {formatFileSize(parentVersion.file_size)}
                </Text>
              </div>
            </div>
          </div>
        </Card>
      </div>

      <Form 
        form={form} 
        layout="vertical"
        onFinish={handleSubmit}
      >
        <Form.Item
          name="name"
          label={
            <Space>
              Branch Name
              <Tooltip title="A unique name for this branch. Use descriptive names like 'test-new-parameters' or 'feature-improvements'">
                <InfoCircleOutlined style={{ color: '#8c8c8c' }} />
              </Tooltip>
            </Space>
          }
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
        >
          <Input 
            placeholder="e.g., test-new-parameters, feature-improvements"
            autoFocus
            addonBefore={<BranchesOutlined />}
          />
        </Form.Item>

        <Form.Item
          name="description"
          label={
            <Space>
              Description (Optional)
              <Tooltip title="Describe the purpose of this branch">
                <InfoCircleOutlined style={{ color: '#8c8c8c' }} />
              </Tooltip>
            </Space>
          }
          rules={[
            {
              validator: (_, value) => {
                if (!value) return Promise.resolve();
                const error = validateBranchDescription(value);
                return error ? Promise.reject(error) : Promise.resolve();
              }
            }
          ]}
        >
          <TextArea
            rows={2}
            placeholder="Describe the purpose of this branch..."
            showCount
            maxLength={500}
          />
        </Form.Item>

        <Form.Item>
          <Checkbox 
            checked={downloadAfterCreate} 
            onChange={(e) => setDownloadAfterCreate(e.target.checked)}
          >
            Download configuration after creating branch
          </Checkbox>
        </Form.Item>

        <Alert
          message="Guidelines"
          description="Use descriptive kebab-case names for experimental changes. Each branch maintains its own history."
          type="info"
          showIcon
          style={{ marginBottom: '16px' }}
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

      {/* Export Modal for downloading after branch creation */}
      {showExportModal && createdBranch && token && (
        <ExportConfirmationModal
          visible={showExportModal}
          onCancel={() => {
            setShowExportModal(false);
            onSuccess(createdBranch);
            handleReset();
          }}
          onSuccess={(exportPath) => {
            message.success(`Configuration exported to ${exportPath}`);
            setShowExportModal(false);
            onSuccess(createdBranch);
            handleReset();
          }}
          version={{
            id: parentVersion.id,
            version_number: parentVersion.version_number,
            file_name: parentVersion.file_name,
            file_size: parentVersion.file_size,
            created_at: parentVersion.created_at,
            author_username: parentVersion.author_username,
            status: parentVersion.status,
            notes: `Branch: ${createdBranch.name}`,
            content_hash: parentVersion.content_hash,
            asset_id: parentVersion.asset_id,
            author: parentVersion.author,
            status_changed_by: parentVersion.status_changed_by,
            status_changed_at: parentVersion.status_changed_at
          }}
          token={token}
        />
      )}
    </Modal>
  );
};

export default CreateBranchModal;