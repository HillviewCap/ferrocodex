import React, { useState } from 'react';
import {
  Modal,
  Typography,
  Space,
  Button,
  Input,
  Form,
  Alert,
  Descriptions,
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

const { Text } = Typography;

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
      width={520}
      destroyOnHidden
    >
      <Space orientation="vertical" style={{ width: '100%' }} size="small">
        {/* Version Information */}
        <Descriptions
          title="Configuration Details"
          size="small"
          bordered
          column={2}
          styles={{ label: { width: 80 } }}
          items={[
            { key: 'version', label: 'Version', children: <Text strong>{version.version_number}</Text> },
            { key: 'status', label: 'Status', children: version.status },
            {
              key: 'file',
              label: 'File',
              children: <Text strong ellipsis={{ tooltip: version.file_name }}>{version.file_name}</Text>
            },
            { key: 'size', label: 'Size', children: formatFileSize(version.file_size) },
            { key: 'author', label: 'Author', span: 2, children: version.author_username }
          ]}
        />

        {/* Export Location */}
        <Form form={form} layout="vertical" size="small" component={false}>
          <Form.Item label="Export Location" htmlFor="export-location-input" style={{ marginBottom: 0 }}>
            <Input
              id="export-location-input"
              value={selectedPath}
              placeholder="Click 'Select Location' to choose where to save the file"
              readOnly
              suffix={
                <Button
                  type="link"
                  size="small"
                  icon={<FolderOpenOutlined />}
                  onClick={handleSelectPath}
                  disabled={isExporting}
                  style={{ padding: 0 }}
                >
                  Browse
                </Button>
              }
            />
          </Form.Item>
        </Form>

        {/* Progress Section */}
        {(selectedPath || isExporting) && (
          <div>
            <Row align="middle" gutter={8} wrap={false}>
              <Col flex="none">{getProgressIcon()}</Col>
              <Col flex="none">
                <Text strong>Export Progress</Text>
              </Col>
              <Col flex="auto">
                <Text type="secondary">{exportProgress.message}</Text>
              </Col>
            </Row>
            <Progress
              percent={exportProgress.progress}
              strokeColor={getProgressColor()}
              showInfo={false}
              size="small"
            />
          </div>
        )}

        {/* Error Display */}
        {error && (
          <Alert
            title="Export Error"
            description={error}
            type="error"
            showIcon
            closable={{ onClose: () => setError('') }}
            style={{ padding: '6px 12px' }}
          />
        )}

        {/* Success / general information */}
        {exportProgress.step === 'completed' ? (
          <Alert
            type="success"
            showIcon
            style={{ padding: '6px 12px' }}
            title={
              <>
                <Text strong>Export Successful</Text>
                <Text> - The configuration file has been exported successfully and verified for integrity.</Text>
              </>
            }
          />
        ) : (
          <Alert
            type="info"
            showIcon
            style={{ padding: '6px 12px' }}
            title={
              <>
                <Text strong>Export Information</Text>
                <Text> - The export process includes integrity verification to ensure the configuration file is complete and accurate.</Text>
              </>
            }
          />
        )}
      </Space>
    </Modal>
  );
};

export default ExportConfirmationModal;