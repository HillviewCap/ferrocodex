import React, { useState } from 'react';
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
  Alert,
  Descriptions
} from 'antd';
import { 
  InboxOutlined,
  FileOutlined,
  CheckCircleOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import * as dialog from '@tauri-apps/plugin-dialog';
import { AssetInfo, validateAssetName, validateConfigurationNotes, formatFileSize, validateFileSize, validateFileExtension } from '../types/assets';
import useAuthStore from '../store/auth';

const { Title, Text } = Typography;
const { TextArea } = Input;

interface ImportConfigurationModalProps {
  visible: boolean;
  onCancel: () => void;
  onSuccess: (asset: AssetInfo) => void;
}

interface FileInfo {
  name: string;
  size: number;
  path: string;
  type: string;
}

const ImportConfigurationModal: React.FC<ImportConfigurationModalProps> = ({
  visible,
  onCancel,
  onSuccess
}) => {
  const { token } = useAuthStore();
  const [form] = Form.useForm();
  const [currentStep, setCurrentStep] = useState(0);
  const [selectedFile, setSelectedFile] = useState<FileInfo | null>(null);
  const [importing, setImporting] = useState(false);
  const [importProgress, setImportProgress] = useState(0);

  const handleReset = () => {
    setCurrentStep(0);
    setSelectedFile(null);
    setImporting(false);
    setImportProgress(0);
    form.resetFields();
  };

  const handleCancel = () => {
    if (!importing) {
      handleReset();
      onCancel();
    }
  };

  const handleFileSelect = async () => {
    try {
      const selected = await dialog.open({
        multiple: false,
        filters: [{
          name: 'Configuration Files',
          extensions: ['json', 'xml', 'yaml', 'yml', 'txt', 'cfg', 'conf', 'ini', 'csv', 'log', 'properties', 'config', 'settings', 'toml', 'bin', 'dat', 'hex', 'raw', 'dump', 'vio']
        }]
      });

      if (selected && typeof selected === 'string') {
        // Get file metadata from backend
        try {
          const metadata = await invoke<{name: string, size: number, content_type: string, hash: string}>('get_file_metadata', {
            filePath: selected
          });

          // Validate file extension
          const extensionError = validateFileExtension(metadata.name);
          if (extensionError) {
            message.error(extensionError);
            return;
          }

          // Validate file size
          const sizeError = validateFileSize(metadata.size);
          if (sizeError) {
            message.error(sizeError);
            return;
          }

          const fileInfo: FileInfo = {
            name: metadata.name,
            size: metadata.size,
            path: selected,
            type: metadata.content_type
          };

          setSelectedFile(fileInfo);
          setCurrentStep(1);
        } catch (metadataError) {
          console.error('Failed to get file metadata:', metadataError);
          message.error(`Failed to read file information: ${metadataError instanceof Error ? metadataError.message : String(metadataError)}`);
        }
      }
    } catch (error) {
      console.error('File selection error:', error);
      message.error(`Failed to select file: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const handleDraggerClick = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    handleFileSelect();
  };

  const handleImport = async () => {
    if (!token || !selectedFile) return;

    try {
      const values = await form.validateFields();
      setImporting(true);
      setImportProgress(0);

      // Simulate progress for better UX
      const progressInterval = setInterval(() => {
        setImportProgress(prev => {
          if (prev >= 90) {
            clearInterval(progressInterval);
            return prev;
          }
          return prev + 10;
        });
      }, 100);

      const response = await invoke<AssetInfo>('import_configuration', {
        token,
        assetName: values.assetName,
        filePath: selectedFile.path,
        notes: values.notes || ''
      });

      clearInterval(progressInterval);
      setImportProgress(100);

      // Wait a moment to show completion
      setTimeout(() => {
        setCurrentStep(2);
        onSuccess(response);
        handleReset();
      }, 500);

    } catch (error) {
      console.error('Import failed:', error);
      message.error(`Import failed: ${error}`);
      setImporting(false);
      setImportProgress(0);
    }
  };

  const steps = [
    {
      title: 'Select File',
      icon: <FileOutlined />,
      description: 'Choose a configuration file'
    },
    {
      title: 'Asset Details',
      icon: <InboxOutlined />,
      description: 'Enter asset information'
    },
    {
      title: 'Complete',
      icon: <CheckCircleOutlined />,
      description: 'Import successful'
    }
  ];

  return (
    <Modal
      title="Import Configuration"
      visible={visible}
      onCancel={handleCancel}
      footer={null}
      width={720}
      closable={!importing}
      maskClosable={!importing}
    >
      <Steps current={currentStep} items={steps} style={{ marginBottom: '24px' }} />

      {currentStep === 0 && (
        <div>
          <Title level={4}>Select Configuration File</Title>
          <Text type="secondary" style={{ display: 'block', marginBottom: '16px' }}>
            Choose a configuration file to import. Supported formats include JSON, XML, YAML, and binary files.
          </Text>
          
          <div 
            onClick={handleDraggerClick}
            style={{ 
              border: '1px dashed #d9d9d9',
              borderRadius: '8px',
              padding: '40px',
              textAlign: 'center',
              cursor: 'pointer',
              marginBottom: '16px',
              backgroundColor: '#fafafa',
              transition: 'border-color 0.3s',
            }}
            onMouseEnter={(e) => e.currentTarget.style.borderColor = '#40a9ff'}
            onMouseLeave={(e) => e.currentTarget.style.borderColor = '#d9d9d9'}
          >
            <p style={{ fontSize: '48px', color: '#40a9ff', marginBottom: '8px' }}>
              <InboxOutlined />
            </p>
            <p style={{ margin: '0 0 4px', fontSize: '16px' }}>Click to select configuration file</p>
            <p style={{ margin: 0, color: '#8c8c8c', fontSize: '14px' }}>
              Support for configuration files up to 100MB
            </p>
          </div>

          <Alert
            message="Supported File Types"
            description="JSON, XML, YAML, TXT, CFG, CONF, INI, CSV, LOG, PROPERTIES, CONFIG, SETTINGS, TOML, BIN, DAT, HEX, RAW, DUMP"
            type="info"
            showIcon
          />
        </div>
      )}

      {currentStep === 1 && selectedFile && (
        <div>
          <Title level={4}>Asset Information</Title>
          
          <Card style={{ marginBottom: '16px' }}>
            <Descriptions title="Selected File" bordered size="small">
              <Descriptions.Item label="Name">{selectedFile.name}</Descriptions.Item>
              <Descriptions.Item label="Size">{formatFileSize(selectedFile.size)}</Descriptions.Item>
              <Descriptions.Item label="Type">{selectedFile.type}</Descriptions.Item>
            </Descriptions>
          </Card>

          <Form 
            form={form} 
            layout="vertical"
            onFinish={handleImport}
          >
            <Form.Item
              name="assetName"
              label="Asset Name"
              rules={[
                { required: true, message: 'Please enter an asset name' },
                {
                  validator: (_, value) => {
                    if (!value) return Promise.resolve();
                    const error = validateAssetName(value);
                    return error ? Promise.reject(error) : Promise.resolve();
                  }
                }
              ]}
              tooltip="A unique name for this configuration asset"
            >
              <Input 
                placeholder="e.g., PLC-Line5, RouterConfig-Main"
                autoFocus
              />
            </Form.Item>

            <Form.Item
              name="notes"
              label="Notes (Optional)"
              rules={[
                {
                  validator: (_, value) => {
                    if (!value) return Promise.resolve();
                    const error = validateConfigurationNotes(value);
                    return error ? Promise.reject(error) : Promise.resolve();
                  }
                }
              ]}
              tooltip="Initial version notes"
            >
              <TextArea
                rows={3}
                placeholder="Enter any notes about this configuration..."
                showCount
                maxLength={1000}
              />
            </Form.Item>

            <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: '24px' }}>
              <Button onClick={() => setCurrentStep(0)}>
                Back
              </Button>
              <Space>
                <Button onClick={handleCancel} disabled={importing}>
                  Cancel
                </Button>
                <Button type="primary" htmlType="submit" loading={importing}>
                  Import Configuration
                </Button>
              </Space>
            </div>
          </Form>

          {importing && (
            <div style={{ marginTop: '16px' }}>
              <Progress 
                percent={importProgress} 
                status={importProgress === 100 ? "success" : "active"}
                showInfo={false}
              />
              <Text type="secondary" style={{ fontSize: '12px' }}>
                {importProgress < 50 ? 'Reading file...' : 
                 importProgress < 80 ? 'Validating content...' : 
                 importProgress < 100 ? 'Storing configuration...' : 'Complete!'}
              </Text>
            </div>
          )}
        </div>
      )}

      {currentStep === 2 && (
        <div style={{ textAlign: 'center', padding: '24px' }}>
          <CheckCircleOutlined style={{ fontSize: '48px', color: '#52c41a', marginBottom: '16px' }} />
          <Title level={3}>Import Successful!</Title>
          <Text type="secondary">
            Your configuration has been imported and versioned successfully.
          </Text>
        </div>
      )}
    </Modal>
  );
};

export default ImportConfigurationModal;