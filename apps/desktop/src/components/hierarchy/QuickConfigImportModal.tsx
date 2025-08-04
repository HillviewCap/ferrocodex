import React, { useState, useEffect } from 'react';
import { 
  Modal, 
  Form, 
  Input, 
  Button, 
  message, 
  Space,
  Typography,
  Steps,
  Card,
  Progress,
  Select
} from 'antd';
import { 
  InboxOutlined,
  FileOutlined,
  CheckCircleOutlined,
  DatabaseOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import * as dialog from '@tauri-apps/plugin-dialog';
import { validateConfigurationNotes, formatFileSize, validateFileSize, validateFileExtension } from '../../types/assets';
import useAuthStore from '../../store/auth';

const { Title, Text } = Typography;
const { TextArea } = Input;
const { Option } = Select;

interface QuickConfigImportModalProps {
  visible: boolean;
  onCancel: () => void;
  onSuccess: () => void;
  assetId: number;
  assetName?: string;
}

interface FileInfo {
  name: string;
  size: number;
  path: string;
  type: string;
}

const QuickConfigImportModal: React.FC<QuickConfigImportModalProps> = ({
  visible,
  onCancel,
  onSuccess,
  assetId,
  assetName
}) => {
  const { token } = useAuthStore();
  const [form] = Form.useForm();
  const [currentStep, setCurrentStep] = useState(0);
  const [selectedFile, setSelectedFile] = useState<FileInfo | null>(null);
  const [importing, setImporting] = useState(false);
  const [importProgress, setImportProgress] = useState(0);

  // Reset form when modal opens/closes
  useEffect(() => {
    if (visible) {
      setCurrentStep(0);
      setSelectedFile(null);
      setImporting(false);
      setImportProgress(0);
      form.resetFields();
      
      // Pre-fill asset if provided
      if (assetName) {
        form.setFieldsValue({
          asset_name: assetName
        });
      }
    }
  }, [visible, form, assetName]);

  const handleFileSelect = async () => {
    try {
      const selected = await dialog.open({
        multiple: false,
        filters: [
          {
            name: 'All Files',
            extensions: ['*']
          }
        ]
      });

      if (selected && typeof selected === 'string') {
        // Get file info
        const fileInfo = await invoke<FileInfo>('get_file_info', { path: selected });
        
        // Validate file
        const sizeValidation = validateFileSize(fileInfo.size);
        if (!sizeValidation.isValid) {
          message.error(sizeValidation.message);
          return;
        }

        const extensionValidation = validateFileExtension(fileInfo.name);
        if (!extensionValidation.isValid) {
          message.error(extensionValidation.message);
          return;
        }

        setSelectedFile(fileInfo);
        setCurrentStep(1);
      }
    } catch (error) {
      console.error('File selection failed:', error);
      message.error('Failed to select file');
    }
  };

  const handleImport = async () => {
    if (!selectedFile || !token) return;

    try {
      const values = await form.validateFields();
      setImporting(true);
      setImportProgress(0);

      // Simulate progress for better UX
      const progressInterval = setInterval(() => {
        setImportProgress(prev => Math.min(prev + 10, 90));
      }, 100);

      const importData = {
        assetId: assetId,
        filePath: selectedFile.path,
        versionNotes: values.version_notes || '',
        classification: values.classification || 'Internal'
      };

      await invoke('import_configuration_for_asset', {
        token,
        ...importData
      });

      clearInterval(progressInterval);
      setImportProgress(100);
      
      setTimeout(() => {
        setCurrentStep(2);
        setImporting(false);
      }, 500);

    } catch (error) {
      setImporting(false);
      setImportProgress(0);
      console.error('Import failed:', error);
      message.error(typeof error === 'string' ? error : 'Configuration import failed');
    }
  };

  const handleComplete = () => {
    onSuccess();
    onCancel(); // Close modal
  };

  const steps = [
    {
      title: 'Select File',
      description: 'Choose configuration file'
    },
    {
      title: 'Configure Import',
      description: 'Add details and notes'
    },
    {
      title: 'Complete',
      description: 'Import successful'
    }
  ];

  const renderStepContent = () => {
    switch (currentStep) {
      case 0:
        return (
          <Card>
            <div style={{ textAlign: 'center', padding: '40px 20px' }}>
              <InboxOutlined style={{ fontSize: '48px', color: '#1890ff', marginBottom: '16px' }} />
              <Title level={4}>Select Configuration File</Title>
              <Text type="secondary">
                Choose a configuration file to import for <strong>{assetName || `Asset #${assetId}`}</strong>
              </Text>
              <div style={{ marginTop: '24px' }}>
                <Button 
                  type="primary" 
                  size="large"
                  onClick={handleFileSelect}
                >
                  Browse Files
                </Button>
              </div>
              <div style={{ marginTop: '16px' }}>
                <Text type="secondary" style={{ fontSize: '12px' }}>
                  All file formats supported
                </Text>
              </div>
            </div>
          </Card>
        );

      case 1:
        return (
          <div>
            {selectedFile && (
              <Card style={{ marginBottom: '16px' }}>
                <Space>
                  <FileOutlined style={{ fontSize: '24px', color: '#52c41a' }} />
                  <div>
                    <div><strong>{selectedFile.name}</strong></div>
                    <Text type="secondary">{formatFileSize(selectedFile.size)}</Text>
                  </div>
                </Space>
              </Card>
            )}

            <Form form={form} layout="vertical">
              <Form.Item
                name="asset_name"
                label="Target Asset"
              >
                <Input 
                  disabled 
                  value={assetName || `Asset #${assetId}`}
                  prefix={<DatabaseOutlined />}
                />
              </Form.Item>

              <Form.Item
                name="version_notes"
                label="Version Notes"
                rules={[
                  { 
                    validator: (_, value) => {
                      const error = validateConfigurationNotes(value || '');
                      return error ? Promise.reject(error) : Promise.resolve();
                    }
                  }
                ]}
              >
                <TextArea 
                  rows={3}
                  placeholder="Describe this configuration version..."
                  maxLength={500}
                  showCount
                />
              </Form.Item>

              <Form.Item
                name="classification"
                label="Security Classification"
                initialValue="Internal"
              >
                <Select>
                  <Option value="Public">Public</Option>
                  <Option value="Internal">Internal</Option>
                  <Option value="Confidential">Confidential</Option>
                  <Option value="Restricted">Restricted</Option>
                </Select>
              </Form.Item>
            </Form>

            {importing && (
              <div style={{ marginTop: '16px' }}>
                <Progress percent={importProgress} status="active" />
                <Text type="secondary">Importing configuration...</Text>
              </div>
            )}
          </div>
        );

      case 2:
        return (
          <div style={{ textAlign: 'center', padding: '40px 20px' }}>
            <CheckCircleOutlined style={{ fontSize: '48px', color: '#52c41a', marginBottom: '16px' }} />
            <Title level={4}>Import Successful!</Title>
            <Text type="secondary">
              Configuration has been imported for <strong>{assetName || `Asset #${assetId}`}</strong>
            </Text>
          </div>
        );

      default:
        return null;
    }
  };

  const renderFooter = () => {
    switch (currentStep) {
      case 0:
        return [
          <Button key="cancel" onClick={onCancel}>
            Cancel
          </Button>
        ];

      case 1:
        return [
          <Button key="back" onClick={() => setCurrentStep(0)}>
            Back
          </Button>,
          <Button 
            key="import" 
            type="primary" 
            onClick={handleImport}
            loading={importing}
            disabled={!selectedFile}
          >
            Import Configuration
          </Button>
        ];

      case 2:
        return [
          <Button key="complete" type="primary" onClick={handleComplete}>
            Complete
          </Button>
        ];

      default:
        return [];
    }
  };

  return (
    <Modal
      title={
        <Space>
          <DatabaseOutlined />
          <span>Quick Configuration Import</span>
        </Space>
      }
      open={visible}
      onCancel={onCancel}
      footer={renderFooter()}
      width={600}
      destroyOnClose
    >
      <Steps current={currentStep} style={{ marginBottom: '24px' }}>
        {steps.map(step => (
          <Steps.Step
            key={step.title}
            title={step.title}
            description={step.description}
          />
        ))}
      </Steps>

      {renderStepContent()}
    </Modal>
  );
};

export default QuickConfigImportModal;