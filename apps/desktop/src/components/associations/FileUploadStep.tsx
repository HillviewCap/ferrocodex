import React, { useState, useCallback } from 'react';
import { 
  Card, 
  Upload, 
  Typography, 
  Space, 
  Progress, 
  List, 
  Tag, 
  Button,
  Alert,
  Tooltip,
  message
} from 'antd';
import { 
  InboxOutlined, 
  FileOutlined, 
  CheckCircleOutlined,
  ExclamationCircleOutlined,
  DeleteOutlined,
  ReloadOutlined
} from '@ant-design/icons';
import { AssetInfo, AssociationType, FileUploadProgress } from '../../types/associations';

const { Dragger } = Upload;
const { Title, Text } = Typography;

interface FileUploadStepProps {
  selectedAsset: AssetInfo;
  fileType?: AssociationType;
  onFilesSelected: (files: File[]) => void;
  onUploadProgress: (uploadedFiles: Array<{
    file: File;
    fileId?: number;
    progress: number;
    status: 'pending' | 'uploading' | 'completed' | 'failed';
    error?: string;
  }>) => void;
}

interface UploadedFile {
  file: File;
  fileId?: number;
  progress: number;
  status: 'pending' | 'uploading' | 'completed' | 'failed';
  error?: string;
}

const FileUploadStep: React.FC<FileUploadStepProps> = ({
  selectedAsset,
  fileType,
  onFilesSelected,
  onUploadProgress
}) => {
  const [uploadedFiles, setUploadedFiles] = useState<UploadedFile[]>([]);
  const [uploading, setUploading] = useState(false);

  const getAcceptedFileTypes = () => {
    if (fileType === 'Configuration') {
      return '.json,.xml,.yaml,.yml,.cfg,.conf,.ini,.txt';
    } else if (fileType === 'Firmware') {
      return '.bin,.hex,.srec,.elf,.img,.rom,.fw';
    }
    return '.json,.xml,.yaml,.yml,.cfg,.conf,.ini,.txt,.bin,.hex,.srec,.elf,.img,.rom,.fw';
  };

  const validateFile = (file: File): string | null => {
    // Size validation (100MB for config, 2GB for firmware)
    const maxSize = fileType === 'Firmware' ? 2 * 1024 * 1024 * 1024 : 100 * 1024 * 1024;
    if (file.size > maxSize) {
      const maxSizeStr = fileType === 'Firmware' ? '2GB' : '100MB';
      return `File size exceeds maximum limit of ${maxSizeStr}`;
    }

    // File type validation
    const acceptedExtensions = getAcceptedFileTypes().split(',').map(ext => ext.trim());
    const fileExtension = '.' + file.name.split('.').pop()?.toLowerCase();
    
    if (acceptedExtensions.length > 0 && !acceptedExtensions.includes(fileExtension)) {
      return `File type ${fileExtension} is not supported. Accepted types: ${acceptedExtensions.join(', ')}`;
    }

    return null;
  };

  const simulateUpload = async (file: File): Promise<{ fileId: number; error?: string }> => {
    // Simulate file upload with progress
    return new Promise((resolve) => {
      let progress = 0;
      const interval = setInterval(() => {
        progress += Math.random() * 30;
        if (progress >= 100) {
          clearInterval(interval);
          // Simulate random success/failure for demo
          const success = Math.random() > 0.1; // 90% success rate
          resolve({
            fileId: success ? Math.floor(Math.random() * 10000) : 0,
            error: success ? undefined : 'Upload failed due to network error'
          });
        } else {
          // Update progress
          setUploadedFiles(prev => 
            prev.map(f => 
              f.file === file 
                ? { ...f, progress: Math.min(progress, 100) }
                : f
            )
          );
        }
      }, 200);
    });
  };

  const handleFileSelect = useCallback(async (options: any) => {
    const { file } = options;
    
    // Validate file
    const validationError = validateFile(file);
    if (validationError) {
      message.error(validationError);
      return false;
    }

    // Check for duplicates
    const isDuplicate = uploadedFiles.some(f => 
      f.file.name === file.name && f.file.size === file.size
    );
    
    if (isDuplicate) {
      message.warning('This file has already been selected');
      return false;
    }

    // Add file to upload list
    const newUploadedFile: UploadedFile = {
      file,
      progress: 0,
      status: 'pending'
    };

    setUploadedFiles(prev => [...prev, newUploadedFile]);
    
    // Start upload
    try {
      setUploading(true);
      
      // Update status to uploading
      setUploadedFiles(prev => 
        prev.map(f => 
          f.file === file 
            ? { ...f, status: 'uploading' as const }
            : f
        )
      );

      const result = await simulateUpload(file);
      
      // Update with result
      setUploadedFiles(prev => {
        const updated = prev.map(f => 
          f.file === file 
            ? { 
                ...f, 
                fileId: result.fileId || undefined,
                progress: 100,
                status: result.error ? 'failed' as const : 'completed' as const,
                error: result.error
              }
            : f
        );
        
        // Notify parent of update
        onUploadProgress(updated);
        
        // Update selected files for parent
        const validFiles = updated
          .filter(f => f.status === 'completed')
          .map(f => f.file);
        onFilesSelected(validFiles);
        
        return updated;
      });
      
    } catch (error) {
      // Handle upload error
      setUploadedFiles(prev => 
        prev.map(f => 
          f.file === file 
            ? { 
                ...f, 
                status: 'failed' as const,
                error: error instanceof Error ? error.message : 'Upload failed'
              }
            : f
        )
      );
    } finally {
      setUploading(false);
    }

    return false; // Prevent default upload behavior
  }, [uploadedFiles, fileType, onFilesSelected, onUploadProgress]);

  const handleRemoveFile = (file: File) => {
    setUploadedFiles(prev => {
      const updated = prev.filter(f => f.file !== file);
      
      // Update parent
      onUploadProgress(updated);
      const validFiles = updated
        .filter(f => f.status === 'completed')
        .map(f => f.file);
      onFilesSelected(validFiles);
      
      return updated;
    });
  };

  const handleRetryUpload = async (file: File) => {
    // Reset file status and retry
    setUploadedFiles(prev => 
      prev.map(f => 
        f.file === file 
          ? { ...f, status: 'uploading' as const, progress: 0, error: undefined }
          : f
      )
    );

    try {
      const result = await simulateUpload(file);
      
      setUploadedFiles(prev => {
        const updated = prev.map(f => 
          f.file === file 
            ? { 
                ...f, 
                fileId: result.fileId || undefined,
                progress: 100,
                status: result.error ? 'failed' as const : 'completed' as const,
                error: result.error
              }
            : f
        );
        
        onUploadProgress(updated);
        const validFiles = updated
          .filter(f => f.status === 'completed')
          .map(f => f.file);
        onFilesSelected(validFiles);
        
        return updated;
      });
      
    } catch (error) {
      setUploadedFiles(prev => 
        prev.map(f => 
          f.file === file 
            ? { 
                ...f, 
                status: 'failed' as const,
                error: error instanceof Error ? error.message : 'Upload failed'
              }
            : f
        )
      );
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'completed':
        return <CheckCircleOutlined style={{ color: '#52c41a' }} />;
      case 'failed':
        return <ExclamationCircleOutlined style={{ color: '#ff4d4f' }} />;
      default:
        return <FileOutlined />;
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'completed':
        return '#52c41a';
      case 'failed':
        return '#ff4d4f';
      case 'uploading':
        return '#1890ff';
      default:
        return '#d9d9d9';
    }
  };

  const completedFiles = uploadedFiles.filter(f => f.status === 'completed').length;
  const failedFiles = uploadedFiles.filter(f => f.status === 'failed').length;
  const totalFiles = uploadedFiles.length;

  return (
    <Card 
      title={
        <Space>
          <InboxOutlined />
          <Title level={4} style={{ margin: 0 }}>
            Upload Files for {selectedAsset.name}
          </Title>
        </Space>
      }
    >
      <Space direction="vertical" style={{ width: '100%' }} size="large">
        {/* Instructions */}
        <Alert
          message="File Upload Instructions"
          description={
            <div>
              <p>Upload {fileType ? fileType.toLowerCase() : 'configuration or firmware'} files to associate with <strong>{selectedAsset.name}</strong>.</p>
              <p>
                <strong>Accepted file types:</strong> {getAcceptedFileTypes()}<br/>
                <strong>Maximum file size:</strong> {fileType === 'Firmware' ? '2GB' : '100MB'}
              </p>
            </div>
          }
          type="info"
          showIcon
        />

        {/* Upload Area */}
        <Dragger
          multiple
          showUploadList={false}
          customRequest={handleFileSelect}
          accept={getAcceptedFileTypes()}
          disabled={uploading}
        >
          <p className="ant-upload-drag-icon">
            <InboxOutlined />
          </p>
          <p className="ant-upload-text">
            Click or drag files to this area to upload
          </p>
          <p className="ant-upload-hint">
            Support for single or bulk upload. Select multiple files to upload them all at once.
          </p>
        </Dragger>

        {/* Upload Progress Summary */}
        {totalFiles > 0 && (
          <Card size="small" style={{ backgroundColor: '#fafafa' }}>
            <Space>
              <Text strong>Upload Progress:</Text>
              <Tag color="blue">{totalFiles} Total</Tag>
              <Tag color="green">{completedFiles} Completed</Tag>
              {failedFiles > 0 && <Tag color="red">{failedFiles} Failed</Tag>}
            </Space>
          </Card>
        )}

        {/* File List */}
        {uploadedFiles.length > 0 && (
          <List
            header={<Text strong>Uploaded Files</Text>}
            bordered
            dataSource={uploadedFiles}
            renderItem={(item) => (
              <List.Item
                actions={[
                  item.status === 'failed' && (
                    <Tooltip title="Retry upload">
                      <Button
                        type="text"
                        icon={<ReloadOutlined />}
                        onClick={() => handleRetryUpload(item.file)}
                        size="small"
                      />
                    </Tooltip>
                  ),
                  <Tooltip title="Remove file">
                    <Button
                      type="text"
                      icon={<DeleteOutlined />}
                      onClick={() => handleRemoveFile(item.file)}
                      size="small"
                      danger
                    />
                  </Tooltip>
                ].filter(Boolean)}
              >
                <List.Item.Meta
                  avatar={getStatusIcon(item.status)}
                  title={
                    <Space>
                      <Text>{item.file.name}</Text>
                      <Tag color={getStatusColor(item.status)} size="small">
                        {item.status.toUpperCase()}
                      </Tag>
                    </Space>
                  }
                  description={
                    <Space direction="vertical" style={{ width: '100%' }}>
                      <Text type="secondary">
                        Size: {(item.file.size / 1024 / 1024).toFixed(2)} MB
                      </Text>
                      {item.status === 'uploading' && (
                        <Progress 
                          percent={Math.round(item.progress)} 
                          size="small"
                          status="active"
                        />
                      )}
                      {item.error && (
                        <Text type="danger" style={{ fontSize: '12px' }}>
                          {item.error}
                        </Text>
                      )}
                    </Space>
                  }
                />
              </List.Item>
            )}
          />
        )}
      </Space>
    </Card>
  );
};

export default FileUploadStep;