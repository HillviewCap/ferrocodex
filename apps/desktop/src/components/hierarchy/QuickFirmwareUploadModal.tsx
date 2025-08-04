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
  FileOutlined,
  CheckCircleOutlined,
  CloudUploadOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import * as dialog from '@tauri-apps/plugin-dialog';
import { 
  validateFirmwareVendor, 
  validateFirmwareModel, 
  validateFirmwareVersion,
  validateFirmwareNotes,
  validateFirmwareFileSize,
  validateFirmwareFileExtension,
  formatFirmwareFileSize
} from '../../types/firmware';
import useAuthStore from '../../store/auth';

const { Title, Text } = Typography;
const { TextArea } = Input;
const { Option } = Select;

interface QuickFirmwareUploadModalProps {
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

const QuickFirmwareUploadModal: React.FC<QuickFirmwareUploadModalProps> = ({
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
  const [uploading, setUploading] = useState(false);
  const [uploadProgress, setUploadProgress] = useState(0);

  // Reset form when modal opens/closes
  useEffect(() => {
    if (visible) {
      setCurrentStep(0);
      setSelectedFile(null);
      setUploading(false);
      setUploadProgress(0);
      form.resetFields();
    }
  }, [visible, form]);

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
        
        // Validate file size
        const sizeValidation = validateFirmwareFileSize(fileInfo.size);
        if (!sizeValidation.isValid) {
          message.error(sizeValidation.message);
          return;
        }

        // Validate file extension
        const extensionValidation = validateFirmwareFileExtension(fileInfo.name);
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

  const handleUpload = async () => {
    if (!selectedFile || !token) return;

    try {
      const values = await form.validateFields();
      setUploading(true);
      setUploadProgress(0);

      // Simulate progress for better UX
      const progressInterval = setInterval(() => {
        setUploadProgress(prev => Math.min(prev + 10, 90));
      }, 100);

      const uploadData = {
        assetId: assetId,
        filePath: selectedFile.path,
        vendor: values.vendor || null,
        model: values.model || null,
        version: values.version || '1.0.0',
        notes: values.release_notes || null
      };

      await invoke('upload_firmware', {
        token,
        ...uploadData
      });

      clearInterval(progressInterval);
      setUploadProgress(100);
      
      setTimeout(() => {
        setCurrentStep(2);
        setUploading(false);
      }, 500);

    } catch (error) {
      setUploading(false);
      setUploadProgress(0);
      console.error('Upload failed:', error);
      message.error(typeof error === 'string' ? error : 'Firmware upload failed');
    }
  };

  const handleComplete = () => {
    onSuccess();
    onCancel(); // Close modal
  };

  const steps = [
    {
      title: 'Select File',
      description: 'Choose firmware file'
    },
    {
      title: 'Configure Upload',
      description: 'Add firmware details'
    },
    {
      title: 'Complete',
      description: 'Upload successful'
    }
  ];

  const renderStepContent = () => {
    switch (currentStep) {
      case 0:
        return (
          <Card>
            <div style={{ textAlign: 'center', padding: '40px 20px' }}>
              <CloudUploadOutlined style={{ fontSize: '48px', color: '#1890ff', marginBottom: '16px' }} />
              <Title level={4}>Select Firmware File</Title>
              <Text type="secondary">
                Choose a firmware file to upload for <strong>{assetName || `Asset #${assetId}`}</strong>
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
                    <Text type="secondary">{formatFirmwareFileSize(selectedFile.size)}</Text>
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
                  prefix={<CloudUploadOutlined />}
                />
              </Form.Item>

              <Form.Item
                name="vendor"
                label="Vendor"
                rules={[
                  { required: true, message: 'Please enter the vendor' },
                  { 
                    validator: (_, value) => {
                      const result = validateFirmwareVendor(value || '');
                      return result.isValid ? Promise.resolve() : Promise.reject(result.message);
                    }
                  }
                ]}
              >
                <Input placeholder="e.g., Siemens, Allen-Bradley, Schneider" />
              </Form.Item>

              <Form.Item
                name="model"
                label="Model"
                rules={[
                  { required: true, message: 'Please enter the model' },
                  { 
                    validator: (_, value) => {
                      const result = validateFirmwareModel(value || '');
                      return result.isValid ? Promise.resolve() : Promise.reject(result.message);
                    }
                  }
                ]}
              >
                <Input placeholder="e.g., S7-1200, CompactLogix, Modicon M580" />
              </Form.Item>

              <Form.Item
                name="version"
                label="Firmware Version"
                rules={[
                  { required: true, message: 'Please enter the version' },
                  { 
                    validator: (_, value) => {
                      const result = validateFirmwareVersion(value || '');
                      return result.isValid ? Promise.resolve() : Promise.reject(result.message);
                    }
                  }
                ]}
              >
                <Input placeholder="e.g., 4.2.3, v1.0.1" />
              </Form.Item>

              <Form.Item
                name="release_notes"
                label="Release Notes"
                rules={[
                  { 
                    validator: (_, value) => {
                      const result = validateFirmwareNotes(value || '');
                      return result.isValid ? Promise.resolve() : Promise.reject(result.message);
                    }
                  }
                ]}
              >
                <TextArea 
                  rows={3}
                  placeholder="Describe this firmware version..."
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

            {uploading && (
              <div style={{ marginTop: '16px' }}>
                <Progress percent={uploadProgress} status="active" />
                <Text type="secondary">Uploading firmware...</Text>
              </div>
            )}
          </div>
        );

      case 2:
        return (
          <div style={{ textAlign: 'center', padding: '40px 20px' }}>
            <CheckCircleOutlined style={{ fontSize: '48px', color: '#52c41a', marginBottom: '16px' }} />
            <Title level={4}>Upload Successful!</Title>
            <Text type="secondary">
              Firmware has been uploaded for <strong>{assetName || `Asset #${assetId}`}</strong>
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
            key="upload" 
            type="primary" 
            onClick={handleUpload}
            loading={uploading}
            disabled={!selectedFile}
          >
            Upload Firmware
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
          <CloudUploadOutlined />
          <span>Quick Firmware Upload</span>
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

export default QuickFirmwareUploadModal;