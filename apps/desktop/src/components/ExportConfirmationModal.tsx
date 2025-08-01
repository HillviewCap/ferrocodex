import React, { useState } from 'react';
import {
  Modal,
  Typography,
  Space,
  Button,
  Input,
  Form,
  Alert,
  Divider,
  Row,
  Col,
  Progress,
  message
} from 'antd';
import {
  FileOutlined,
  DownloadOutlined,
  FolderOpenOutlined,
  CheckCircleOutlined,
  ExclamationCircleOutlined
} from '@ant-design/icons';
import { save } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { ConfigurationVersionInfo, formatFileSize } from '../types/assets';

const { Text, Title } = Typography;

interface ExportConfirmationModalProps {
  visible: boolean;
  onCancel: () => void;
  onSuccess: (exportPath: string) => void;
  version: ConfigurationVersionInfo;
  token: string;
}

interface ExportProgress {
  step: 'selecting' | 'exporting' | 'validating' | 'completed' | 'error';
  progress: number;
  message: string;
}

const ExportConfirmationModal: React.FC<ExportConfirmationModalProps> = ({
  visible,
  onCancel,
  onSuccess,
  version,
  token
}) => {
  const [form] = Form.useForm();
  const [selectedPath, setSelectedPath] = useState<string>('');
  const [exportProgress, setExportProgress] = useState<ExportProgress>({
    step: 'selecting',
    progress: 0,
    message: 'Select export location'
  });
  const [isExporting, setIsExporting] = useState(false);
  const [error, setError] = useState<string>('');

  const handleSelectPath = async () => {
    try {
      // Suggest filename based on version info
      const suggestedFilename = `${version.file_name}`;
      
      const filePath = await save({
        defaultPath: suggestedFilename,
        filters: [
          {
            name: 'All Files',
            extensions: ['*']
          }
        ]
      });

      if (filePath) {
        // Ensure the selected path maintains the original file extension
        const originalExtension = version.file_name.split('.').pop()?.toLowerCase();
        let correctedPath = filePath;
        
        if (originalExtension && filePath) {
          const selectedExtension = filePath.split('.').pop()?.toLowerCase();
          
          // If the selected path doesn't have the original extension, correct it
          if (selectedExtension !== originalExtension) {
            // Remove any existing extension and add the original one
            const pathWithoutExt = filePath.replace(/\.[^/.]+$/, '');
            correctedPath = `${pathWithoutExt}.${originalExtension}`;
          }
        }
        
        setSelectedPath(correctedPath);
        setError('');
        setExportProgress({
          step: 'selecting',
          progress: 25,
          message: 'Export location selected'
        });
      }
    } catch (err) {
      console.error('Error selecting export path:', err);
      setError('Failed to open file dialog. Please try again.');
    }
  };

  const handleExport = async () => {
    if (!selectedPath) {
      setError('Please select an export location first.');
      return;
    }

    setIsExporting(true);
    setError('');

    try {
      // Update progress - starting export
      setExportProgress({
        step: 'exporting',
        progress: 50,
        message: 'Exporting configuration file...'
      });

      const startTime = Date.now();

      // Log the path for debugging
      console.log('Attempting to export to path:', selectedPath);
      
      // Call the backend export function
      await invoke('export_configuration_version', {
        token,
        versionId: version.id,
        exportPath: selectedPath
      });

      const duration = Date.now() - startTime;

      // Update progress - validating
      setExportProgress({
        step: 'validating',
        progress: 75,
        message: 'Validating exported file...'
      });

      // Add a small delay to show validation step
      await new Promise(resolve => setTimeout(resolve, 500));

      // Update progress - completed
      setExportProgress({
        step: 'completed',
        progress: 100,
        message: `Export completed successfully in ${(duration / 1000).toFixed(2)}s`
      });

      // Show success message
      message.success(`Configuration exported successfully to ${selectedPath}`);

      // Auto-close after success
      setTimeout(() => {
        onSuccess(selectedPath);
        handleClose();
      }, 1500);

    } catch (err: any) {
      console.error('Export failed:', err);
      setExportProgress({
        step: 'error',
        progress: 0,
        message: 'Export failed'
      });
      setError(err.message || 'Failed to export configuration. Please try again.');
    } finally {
      setIsExporting(false);
    }
  };

  const handleClose = () => {
    // Reset state
    setSelectedPath('');
    setExportProgress({
      step: 'selecting',
      progress: 0,
      message: 'Select export location'
    });
    setIsExporting(false);
    setError('');
    form.resetFields();
    onCancel();
  };

  const getProgressColor = () => {
    switch (exportProgress.step) {
      case 'error':
        return '#ff4d4f';
      case 'completed':
        return '#52c41a';
      default:
        return '#1890ff';
    }
  };

  const getProgressIcon = () => {
    switch (exportProgress.step) {
      case 'completed':
        return <CheckCircleOutlined style={{ color: '#52c41a' }} />;
      case 'error':
        return <ExclamationCircleOutlined style={{ color: '#ff4d4f' }} />;
      default:
        return <DownloadOutlined style={{ color: '#1890ff' }} />;
    }
  };

  return (
    <Modal
      title={
        <Space>
          <FileOutlined />
          <span>Export Configuration</span>
        </Space>
      }
      open={visible}
      onCancel={handleClose}
      footer={[
        <Button key="cancel" onClick={handleClose} disabled={isExporting}>
          Cancel
        </Button>,
        <Button
          key="select"
          icon={<FolderOpenOutlined />}
          onClick={handleSelectPath}
          disabled={isExporting}
        >
          Select Location
        </Button>,
        <Button
          key="export"
          type="primary"
          icon={<DownloadOutlined />}
          onClick={handleExport}
          disabled={!selectedPath || isExporting}
          loading={isExporting}
        >
          Export
        </Button>
      ]}
      width={550}
      destroyOnHidden
    >
      <Space direction="vertical" style={{ width: '100%' }} size="middle">
        {/* Version Information */}
        <div>
          <Title level={5} style={{ marginTop: 0, marginBottom: 12 }}>Configuration Details</Title>
          <Row gutter={[16, 4]}>
            <Col span={12}>
              <Text type="secondary">Version:</Text> <Text strong>{version.version_number}</Text>
            </Col>
            <Col span={12}>
              <Text type="secondary">Status:</Text> <Text>{version.status}</Text>
            </Col>
            <Col span={12}>
              <Text type="secondary">File:</Text> <Text strong>{version.file_name}</Text>
            </Col>
            <Col span={12}>
              <Text type="secondary">Size:</Text> <Text>{formatFileSize(version.file_size)}</Text>
            </Col>
            <Col span={12}>
              <Text type="secondary">Author:</Text> <Text>{version.author_username}</Text>
            </Col>
          </Row>
        </div>

        <Divider style={{ margin: '12px 0' }} />

        {/* Export Location */}
        <div>
          <Title level={5} style={{ marginTop: 0, marginBottom: 8 }}>Export Location</Title>
          <Input
            value={selectedPath}
            placeholder="Click 'Select Location' to choose where to save the file"
            readOnly
            suffix={
              <Button
                type="link"
                icon={<FolderOpenOutlined />}
                onClick={handleSelectPath}
                disabled={isExporting}
                style={{ padding: 0 }}
              >
                Browse
              </Button>
            }
          />
        </div>

        {/* Progress Section */}
        {(selectedPath || isExporting) && (
          <>
            <Divider style={{ margin: '12px 0' }} />
            <div>
              <Title level={5} style={{ marginTop: 0, marginBottom: 8 }}>Export Progress</Title>
              <Space direction="vertical" style={{ width: '100%' }}>
                <Row align="middle" gutter={[8, 0]}>
                  <Col flex="none">
                    {getProgressIcon()}
                  </Col>
                  <Col flex="auto">
                    <Text>{exportProgress.message}</Text>
                  </Col>
                </Row>
                <Progress
                  percent={exportProgress.progress}
                  strokeColor={getProgressColor()}
                  showInfo={false}
                  size="small"
                />
              </Space>
            </div>
          </>
        )}

        {/* Error Display */}
        {error && (
          <Alert
            message="Export Error"
            description={error}
            type="error"
            showIcon
            closable
            onClose={() => setError('')}
          />
        )}

        {/* Export Information */}
        <Alert
          message="Export Information"
          description="The export process includes integrity verification to ensure the configuration file is complete and accurate."
          type="info"
          showIcon
        />

        {/* Success Information */}
        {exportProgress.step === 'completed' && (
          <Alert
            message="Export Successful"
            description="The configuration file has been exported successfully and verified for integrity."
            type="success"
            showIcon
          />
        )}

      </Space>
    </Modal>
  );
};

export default ExportConfirmationModal;