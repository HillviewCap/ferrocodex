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
  Alert
} from 'antd';
import { 
  InboxOutlined,
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
} from '../types/firmware';
import useFirmwareStore from '../store/firmware';

const { Title, Text } = Typography;
const { TextArea } = Input;

interface FirmwareUploadModalProps {
  visible: boolean;
  onCancel: () => void;
  onSuccess: () => void;
  assetId: number;
}

interface FileInfo {
  name: string;
  size: number;
  path: string;
  type: string;
}

const FirmwareUploadModal: React.FC<FirmwareUploadModalProps> = ({
  visible,
  onCancel,
  onSuccess,
  assetId
}) => {
  const [form] = Form.useForm();
  const [currentStep, setCurrentStep] = useState(0);
  const [selectedFile, setSelectedFile] = useState<FileInfo | null>(null);
  const { uploadFirmware, uploadProgress, resetUploadProgress } = useFirmwareStore();

  const handleReset = () => {
    setCurrentStep(0);
    setSelectedFile(null);
    form.resetFields();
    resetUploadProgress();
  };

  const handleCancel = () => {
    if (uploadProgress?.status !== 'uploading') {
      handleReset();
      onCancel();
    }
  };

  const handleFileSelect = async () => {
    try {
      const selected = await dialog.open({
        multiple: false,
        filters: [{
          name: 'Firmware Files',
          extensions: ['bin', 'hex', 'img', 'rom', 'fw', 'elf', 'dfu', 'upd', 
                      'dat', 'firmware', 'update', 'pkg', 'ipk', 'tar', 'gz',
                      'bz2', 'xz', 'zip', 'rar', '7z', 'cab', 'iso', 'dmg']
        }]
      });

      if (selected && typeof selected === 'string') {
        // Get file metadata from backend
        try {
          const metadata = await invoke<{name: string, size: number, content_type: string, hash: string}>('get_file_metadata', {
            filePath: selected
          });

          // Validate file extension
          const extensionError = validateFirmwareFileExtension(metadata.name);
          if (extensionError) {
            message.error(extensionError);
            return;
          }

          // Validate file size (2GB limit)
          const sizeError = validateFirmwareFileSize(metadata.size);
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

  const handleUpload = async () => {
    if (!selectedFile) return;

    try {
      const values = await form.validateFields();
      
      await uploadFirmware({
        asset_id: assetId,
        vendor: values.vendor || null,
        model: values.model || null,
        version: values.version,
        notes: values.notes || null,
        file_path: selectedFile.path
      });

      setCurrentStep(2);
      message.success('Firmware uploaded successfully!');
      
      // Wait a moment before closing
      setTimeout(() => {
        handleReset();
        onSuccess();
      }, 1500);
    } catch (error) {
      console.error('Upload error:', error);
      message.error(`Failed to upload firmware: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const renderFileSelection = () => (
    <div>
      <div 
        onClick={handleDraggerClick}
        style={{ 
          padding: '40px',
          textAlign: 'center',
          background: '#fafafa',
          border: '1px dashed #d9d9d9',
          borderRadius: '8px',
          cursor: 'pointer',
          transition: 'all 0.3s',
          marginBottom: '16px'
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.borderColor = '#1890ff';
          e.currentTarget.style.background = '#f0f8ff';
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.borderColor = '#d9d9d9';
          e.currentTarget.style.background = '#fafafa';
        }}
      >
        <p style={{ fontSize: '48px', marginBottom: '16px' }}>
          <InboxOutlined style={{ color: '#1890ff' }} />
        </p>
        <p style={{ marginBottom: '8px', fontSize: '16px' }}>
          Click to select firmware file
        </p>
        <p style={{ color: '#8c8c8c', fontSize: '14px' }}>
          Support for binary firmware files (max 2GB)
        </p>
      </div>
      
      <Alert
        message="Firmware files are stored encrypted"
        description="All firmware files are encrypted before storage to ensure security. Only authorized users can access them."
        type="info"
        showIcon
        style={{ marginTop: '16px' }}
      />
    </div>
  );

  const renderFileDetails = () => (
    <div>
      <Card style={{ marginBottom: '24px' }}>
        <Space direction="vertical" style={{ width: '100%' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
            <FileOutlined style={{ fontSize: '32px', color: '#1890ff' }} />
            <div style={{ flex: 1 }}>
              <Text strong style={{ fontSize: '16px', display: 'block' }}>
                {selectedFile?.name}
              </Text>
              <Text type="secondary">
                {formatFirmwareFileSize(selectedFile?.size || 0)}
              </Text>
            </div>
          </div>
        </Space>
      </Card>

      <Form
        form={form}
        layout="vertical"
        initialValues={{
          vendor: '',
          model: '',
          version: '',
          notes: ''
        }}
      >
        <Form.Item
          label="Vendor (Optional)"
          name="vendor"
          rules={[
            { validator: async (_, value) => {
              if (value) {
                const error = validateFirmwareVendor(value);
                if (error) throw new Error(error);
              }
            }}
          ]}
        >
          <Input placeholder="e.g., Siemens, Allen-Bradley" />
        </Form.Item>

        <Form.Item
          label="Model (Optional)"
          name="model"
          rules={[
            { validator: async (_, value) => {
              if (value) {
                const error = validateFirmwareModel(value);
                if (error) throw new Error(error);
              }
            }}
          ]}
        >
          <Input placeholder="e.g., S7-1500, CompactLogix" />
        </Form.Item>

        <Form.Item
          label="Version"
          name="version"
          rules={[
            { required: true, message: 'Please enter firmware version' },
            { validator: async (_, value) => {
              if (value) {
                const error = validateFirmwareVersion(value);
                if (error) throw new Error(error);
              }
            }}
          ]}
        >
          <Input placeholder="e.g., 2.8.1, V4.5.0" />
        </Form.Item>

        <Form.Item
          label="Notes (Optional)"
          name="notes"
          rules={[
            { validator: async (_, value) => {
              if (value) {
                const error = validateFirmwareNotes(value);
                if (error) throw new Error(error);
              }
            }}
          ]}
        >
          <TextArea 
            rows={3} 
            placeholder="Additional information about this firmware version"
          />
        </Form.Item>
      </Form>
    </div>
  );

  const renderUploadProgress = () => (
    <div style={{ textAlign: 'center', padding: '40px' }}>
      {uploadProgress?.status === 'complete' ? (
        <>
          <CheckCircleOutlined style={{ fontSize: '64px', color: '#52c41a', marginBottom: '24px' }} />
          <Title level={4}>Upload Complete!</Title>
          <Text type="secondary">Firmware has been uploaded and encrypted successfully</Text>
        </>
      ) : uploadProgress?.status === 'error' ? (
        <>
          <Alert
            message="Upload Failed"
            description={uploadProgress.message || 'An error occurred during upload'}
            type="error"
            showIcon
            style={{ marginBottom: '24px' }}
          />
        </>
      ) : (
        <>
          <CloudUploadOutlined style={{ fontSize: '64px', color: '#1890ff', marginBottom: '24px' }} />
          <Title level={4}>
            {uploadProgress?.status === 'preparing' && 'Preparing Upload...'}
            {uploadProgress?.status === 'uploading' && 'Uploading Firmware...'}
            {uploadProgress?.status === 'processing' && 'Processing Firmware...'}
          </Title>
          <Progress 
            percent={uploadProgress?.progress || 0} 
            style={{ marginTop: '24px' }}
            strokeColor={{
              '0%': '#108ee9',
              '100%': '#87d068',
            }}
          />
        </>
      )}
    </div>
  );

  const steps = [
    {
      title: 'Select File',
      icon: <InboxOutlined />
    },
    {
      title: 'Firmware Details',
      icon: <FileOutlined />
    },
    {
      title: 'Upload',
      icon: <CloudUploadOutlined />
    }
  ];

  return (
    <Modal
      title="Upload Firmware"
      open={visible}
      onCancel={handleCancel}
      width={600}
      footer={
        currentStep === 0 ? (
          <Button onClick={handleCancel}>Cancel</Button>
        ) : currentStep === 1 ? (
          <Space>
            <Button onClick={() => setCurrentStep(0)}>Back</Button>
            <Button onClick={handleCancel}>Cancel</Button>
            <Button type="primary" onClick={handleUpload}>
              Upload Firmware
            </Button>
          </Space>
        ) : currentStep === 2 ? null : (
          <Button onClick={handleCancel}>Cancel</Button>
        )
      }
    >
      <Steps
        current={currentStep}
        items={steps}
        style={{ marginBottom: '24px' }}
      />

      {currentStep === 0 && renderFileSelection()}
      {currentStep === 1 && renderFileDetails()}
      {currentStep === 2 && renderUploadProgress()}
    </Modal>
  );
};

export default FirmwareUploadModal;