import React, { useState, useEffect } from 'react';
import { 
  Modal, 
  Form, 
  Input, 
  Button, 
  Space,
  Typography,
  Card,
  Alert,
  Tag,
  Avatar,
  Divider,
  App
} from 'antd';
import { 
  UploadOutlined,
  InfoCircleOutlined,
  CheckCircleOutlined,
  FileOutlined,
  CalendarOutlined,
  UserOutlined,
  BranchesOutlined,
  FolderOpenOutlined
} from '@ant-design/icons';
import { BranchInfo } from '../types/branches';
import useAuthStore from '../store/auth';
import useBranchStore from '../store/branches';
import * as dialog from '@tauri-apps/plugin-dialog';

const { Text } = Typography;
const { TextArea } = Input;

interface ImportVersionToBranchModalProps {
  visible: boolean;
  onCancel: () => void;
  onSuccess: () => void;
  branch: BranchInfo;
}

const ImportVersionToBranchModal: React.FC<ImportVersionToBranchModalProps> = ({
  visible,
  onCancel,
  onSuccess,
  branch
}) => {
  const { token } = useAuthStore();
  const { importVersionToBranch, isImportingVersion } = useBranchStore();
  const { message } = App.useApp();
  const [form] = Form.useForm();
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [fileName, setFileName] = useState<string>('');

  useEffect(() => {
    if (visible) {
      form.resetFields();
      setSelectedFile(null);
      setFileName('');
    }
  }, [visible, form]);

  const handleReset = () => {
    form.resetFields();
    setSelectedFile(null);
    setFileName('');
  };

  const handleCancel = () => {
    if (!isImportingVersion) {
      handleReset();
      onCancel();
    }
  };

  const handleFileSelect = async () => {
    try {
      const selected = await dialog.open({
        multiple: false,
        filters: [
          {
            name: 'Configuration Files',
            extensions: ['json', 'xml', 'yaml', 'yml', 'txt', 'cfg', 'conf', 'ini', 'csv', 'log', 'properties', 'config', 'settings', 'toml', 'bin', 'dat', 'hex', 'raw', 'dump', 'vio']
          },
          {
            name: 'All Files',
            extensions: ['*']
          }
        ]
      });
      
      if (selected && typeof selected === 'string') {
        // Get file name from path - handle both Windows and Unix paths
        const fileName = selected.split(/[\\/]/).pop() || 'unknown';
        
        setSelectedFile(selected);
        setFileName(fileName);
        form.setFieldValue('file_path', selected);
      }
    } catch (error) {
      console.error('File selection failed:', error);
      message.error('Failed to select file');
    }
  };

  const handleSubmit = async () => {
    if (!token || !selectedFile) return;

    try {
      const values = await form.validateFields();

      await importVersionToBranch(token, {
        branch_id: branch.id,
        file_path: selectedFile,
        notes: values.notes || ''
      });

      message.success('Version imported to branch successfully!');
      handleReset();
      onSuccess();
      
    } catch (error) {
      console.error('Version import failed:', error);
      message.error(`Failed to import version: ${error}`);
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
          <UploadOutlined />
          Import Version to Branch
        </Space>
      }
      open={visible}
      onCancel={handleCancel}
      footer={null}
      width={720}
      closable={!isImportingVersion}
      maskClosable={!isImportingVersion}
      aria-labelledby="import-version-modal-title"
      aria-describedby="import-version-modal-description"
    >
      <div style={{ marginBottom: '24px' }}>
        <Alert
          message="Import New Version"
          description="Import a new configuration file version to this branch. This will create a new branch-specific version without affecting the main asset versions."
          type="info"
          showIcon
          icon={<InfoCircleOutlined />}
          style={{ marginBottom: '16px' }}
        />

        <Card title="Target Branch" size="small" style={{ marginBottom: '16px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
            <Avatar 
              icon={<BranchesOutlined />} 
              size={40}
              style={{ backgroundColor: '#52c41a' }}
            />
            <div style={{ flex: 1 }}>
              <div style={{ marginBottom: '4px' }}>
                <Space size={8}>
                  <Tag color="green">Branch</Tag>
                  <Text strong>{branch.name}</Text>
                </Space>
              </div>
              <div>
                <Space size={12} wrap>
                  <Space size={4}>
                    <Text type="secondary" style={{ fontSize: '12px' }}>
                      Parent: {branch.parent_version_number}
                    </Text>
                  </Space>
                  <Space size={4}>
                    <UserOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                    <Text type="secondary" style={{ fontSize: '12px' }}>
                      {branch.created_by_username}
                    </Text>
                  </Space>
                  <Space size={4}>
                    <CalendarOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                    <Text type="secondary" style={{ fontSize: '12px' }}>
                      {formatDate(branch.created_at)}
                    </Text>
                  </Space>
                </Space>
              </div>
            </div>
          </div>
          {branch.description && (
            <>
              <Divider style={{ margin: '12px 0' }} />
              <Text type="secondary" style={{ fontSize: '12px', fontStyle: 'italic' }}>
                {branch.description}
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
          name="file_path"
          label="Configuration File"
          rules={[
            { required: true, message: 'Please select a configuration file' }
          ]}
          tooltip="Select the configuration file to import into this branch"
        >
          <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
            <Button 
              icon={<FolderOpenOutlined />} 
              onClick={handleFileSelect}
              style={{ flex: 1 }}
            >
              {selectedFile ? 'Change File' : 'Select Configuration File'}
            </Button>
          </div>
        </Form.Item>

        {selectedFile && (
          <Card size="small" style={{ marginBottom: '16px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
              <FileOutlined style={{ fontSize: '20px', color: '#1890ff' }} />
              <div>
                <Text strong>{fileName}</Text>
                <br />
                <Text type="secondary" style={{ fontSize: '12px' }}>
                  {selectedFile}
                </Text>
              </div>
            </div>
          </Card>
        )}

        <Form.Item
          name="notes"
          label="Version Notes"
          rules={[
            { required: true, message: 'Please provide notes for this version' },
            { max: 1000, message: 'Notes cannot exceed 1000 characters' }
          ]}
          tooltip="Describe the changes or purpose of this version"
        >
          <TextArea
            rows={4}
            placeholder="Describe what changes this version contains..."
            showCount
            maxLength={1000}
          />
        </Form.Item>

        <Alert
          message="Version Import Guidelines"
          description={
            <ul style={{ marginBottom: 0, paddingLeft: '20px' }}>
              <li>Each imported version gets a unique branch version number (branch-v1, branch-v2, etc.)</li>
              <li>Branch versions are independent from main asset versions</li>
              <li>The imported file will be encrypted and stored securely</li>
              <li>Version notes help track changes and purposes</li>
            </ul>
          }
          type="info"
          showIcon
          style={{ marginBottom: '24px' }}
        />

        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px' }}>
          <Button onClick={handleCancel} disabled={isImportingVersion}>
            Cancel
          </Button>
          <Button 
            type="primary" 
            htmlType="submit" 
            loading={isImportingVersion}
            icon={<CheckCircleOutlined />}
            disabled={!selectedFile}
          >
            Import Version
          </Button>
        </div>
      </Form>
    </Modal>
  );
};

export default ImportVersionToBranchModal;