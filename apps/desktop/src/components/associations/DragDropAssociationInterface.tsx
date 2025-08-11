import React, { useState, useCallback } from 'react';
import { 
  Card, 
  Typography, 
  Space, 
  Button, 
  Alert, 
  List,
  Tag,
  Progress,
  message,
  Tooltip,
  Upload
} from 'antd';
import { 
  InboxOutlined,
  FileOutlined,
  DeleteOutlined,
  LinkOutlined,
  CheckCircleOutlined,
  ExclamationCircleOutlined,
  DragOutlined,
} from '@ant-design/icons';
import { 
  AssetInfo, 
  AssociationType,
  CreateAssociationRequest 
} from '../../types/associations';
import useAssociationStore from '../../store/associations';

const { Dragger } = Upload;
const { Title, Text } = Typography;

interface DragDropAssociationInterfaceProps {
  targetAsset: AssetInfo;
  allowedFileTypes?: AssociationType[];
  onAssociationCreated?: (association: any) => void;
  disabled?: boolean;
}

interface DraggedFile {
  id: string;
  file: File;
  fileType: AssociationType;
  status: 'pending' | 'uploading' | 'associating' | 'completed' | 'failed';
  progress: number;
  error?: string;
  associationId?: number;
}

const DragDropAssociationInterface: React.FC<DragDropAssociationInterfaceProps> = ({
  targetAsset,
  allowedFileTypes,
  onAssociationCreated,
  disabled = false
}) => {
  const [draggedFiles, setDraggedFiles] = useState<DraggedFile[]>([]);
  const [isDragOver] = useState(false);
  const { createAssociation, loading } = useAssociationStore();

  const detectFileType = (fileName: string): AssociationType => {
    const extension = fileName.split('.').pop()?.toLowerCase();
    const firmwareExtensions = ['bin', 'hex', 'srec', 'elf', 'img', 'rom', 'fw'];
    
    return firmwareExtensions.includes(extension || '') ? 'Firmware' : 'Configuration';
  };

  const validateFile = (file: File): { valid: boolean; error?: string } => {
    const fileType = detectFileType(file.name);
    
    // Check if file type is allowed
    if (allowedFileTypes && !allowedFileTypes.includes(fileType)) {
      return {
        valid: false,
        error: `File type ${fileType} is not allowed. Allowed types: ${allowedFileTypes.join(', ')}`
      };
    }

    // Size validation
    const maxSize = fileType === 'Firmware' ? 2 * 1024 * 1024 * 1024 : 100 * 1024 * 1024;
    if (file.size > maxSize) {
      const maxSizeStr = fileType === 'Firmware' ? '2GB' : '100MB';
      return {
        valid: false,
        error: `File size exceeds maximum limit of ${maxSizeStr}`
      };
    }

    // Check for duplicates
    const isDuplicate = draggedFiles.some(f => 
      f.file.name === file.name && f.file.size === file.size
    );
    
    if (isDuplicate) {
      return {
        valid: false,
        error: 'This file has already been added'
      };
    }

    return { valid: true };
  };

  const simulateFileUpload = async (fileId: string): Promise<{ fileId: number; error?: string }> => {
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
          setDraggedFiles(prev => 
            prev.map(f => 
              f.id === fileId 
                ? { ...f, progress: Math.min(progress, 100) }
                : f
            )
          );
        }
      }, 100);
    });
  };

  const processFileAssociation = async (draggedFile: DraggedFile) => {
    try {
      // Step 1: Upload file (simulated)
      setDraggedFiles(prev => 
        prev.map(f => 
          f.id === draggedFile.id 
            ? { ...f, status: 'uploading' as const }
            : f
        )
      );

      const uploadResult = await simulateFileUpload(draggedFile.id);
      
      if (uploadResult.error) {
        throw new Error(uploadResult.error);
      }

      // Step 2: Create association
      setDraggedFiles(prev => 
        prev.map(f => 
          f.id === draggedFile.id 
            ? { ...f, status: 'associating' as const, progress: 100 }
            : f
        )
      );

      const associationRequest: CreateAssociationRequest = {
        assetId: targetAsset.id,
        fileId: uploadResult.fileId,
        fileType: draggedFile.fileType,
        metadata: JSON.stringify({
          originalFileName: draggedFile.file.name,
          fileSize: draggedFile.file.size,
          uploadDate: new Date().toISOString(),
          dragDropImport: true
        }),
        createdBy: 1, // This would come from auth context
      };

      const association = await createAssociation(associationRequest);

      // Success
      setDraggedFiles(prev => 
        prev.map(f => 
          f.id === draggedFile.id 
            ? { 
                ...f, 
                status: 'completed' as const,
                associationId: association.id
              }
            : f
        )
      );

      message.success(`Successfully associated ${draggedFile.file.name} with ${targetAsset.name}`);
      
      if (onAssociationCreated) {
        onAssociationCreated(association);
      }

    } catch (error) {
      // Error
      setDraggedFiles(prev => 
        prev.map(f => 
          f.id === draggedFile.id 
            ? { 
                ...f, 
                status: 'failed' as const,
                error: error instanceof Error ? error.message : 'Association failed'
              }
            : f
        )
      );

      message.error(`Failed to associate ${draggedFile.file.name}: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  };

  const handleDrop = useCallback((files: File[]) => {
    const newFiles: DraggedFile[] = [];

    files.forEach(file => {
      const validation = validateFile(file);
      
      if (!validation.valid) {
        message.error(validation.error);
        return;
      }

      const draggedFile: DraggedFile = {
        id: `${file.name}-${Date.now()}-${Math.random()}`,
        file,
        fileType: detectFileType(file.name),
        status: 'pending',
        progress: 0
      };

      newFiles.push(draggedFile);
    });

    if (newFiles.length > 0) {
      setDraggedFiles(prev => [...prev, ...newFiles]);
      
      // Start processing files
      newFiles.forEach(file => {
        processFileAssociation(file);
      });
    }
  }, [draggedFiles, targetAsset, allowedFileTypes, createAssociation, onAssociationCreated]);

  const handleFileSelect = useCallback((options: any) => {
    const { file } = options;
    handleDrop([file]);
    return false; // Prevent default upload behavior
  }, [handleDrop]);

  const handleRemoveFile = (fileId: string) => {
    setDraggedFiles(prev => prev.filter(f => f.id !== fileId));
  };

  const handleRetryFile = async (fileId: string) => {
    const file = draggedFiles.find(f => f.id === fileId);
    if (file) {
      // Reset status and retry
      setDraggedFiles(prev => 
        prev.map(f => 
          f.id === fileId 
            ? { ...f, status: 'pending' as const, progress: 0, error: undefined }
            : f
        )
      );
      
      await processFileAssociation(file);
    }
  };

  const getStatusIcon = (status: DraggedFile['status']) => {
    switch (status) {
      case 'completed':
        return <CheckCircleOutlined style={{ color: '#52c41a' }} />;
      case 'failed':
        return <ExclamationCircleOutlined style={{ color: '#ff4d4f' }} />;
      case 'uploading':
      case 'associating':
        return <LinkOutlined style={{ color: '#1890ff' }} />;
      default:
        return <FileOutlined />;
    }
  };

  const getStatusText = (file: DraggedFile) => {
    switch (file.status) {
      case 'pending':
        return 'Pending';
      case 'uploading':
        return 'Uploading...';
      case 'associating':
        return 'Creating association...';
      case 'completed':
        return 'Associated';
      case 'failed':
        return `Failed: ${file.error}`;
      default:
        return 'Unknown';
    }
  };

  const getStatusColor = (status: DraggedFile['status']) => {
    switch (status) {
      case 'completed':
        return 'success';
      case 'failed':
        return 'error';
      case 'uploading':
      case 'associating':
        return 'processing';
      default:
        return 'default';
    }
  };

  const pendingFiles = draggedFiles.filter(f => f.status === 'pending').length;
  const processingFiles = draggedFiles.filter(f => ['uploading', 'associating'].includes(f.status)).length;
  const completedFiles = draggedFiles.filter(f => f.status === 'completed').length;
  const failedFiles = draggedFiles.filter(f => f.status === 'failed').length;

  return (
    <Card 
      title={
        <Space>
          <DragOutlined />
          <Title level={4} style={{ margin: 0 }}>
            Drag & Drop File Association
          </Title>
        </Space>
      }
      style={{ width: '100%' }}
    >
      <Space direction="vertical" style={{ width: '100%' }} size="large">
        {/* Target Asset Info */}
        <Alert
          message="Target Asset"
          description={
            <Space>
              <Text strong>{targetAsset.name}</Text>
              <Text type="secondary">- {targetAsset.description}</Text>
              {allowedFileTypes && (
                <Text type="secondary">
                  | Allowed types: {allowedFileTypes.join(', ')}
                </Text>
              )}
            </Space>
          }
          type="info"
          showIcon
        />

        {/* Drop Zone */}
        <Dragger
          multiple
          showUploadList={false}
          customRequest={handleFileSelect}
          disabled={disabled || loading}
          style={{
            backgroundColor: isDragOver ? '#f0f8ff' : undefined,
            borderColor: isDragOver ? '#1890ff' : undefined,
          }}
        >
          <p className="ant-upload-drag-icon">
            <InboxOutlined style={{ color: isDragOver ? '#1890ff' : undefined }} />
          </p>
          <p className="ant-upload-text">
            Drop files here to associate with <strong>{targetAsset.name}</strong>
          </p>
          <p className="ant-upload-hint">
            Files will be automatically uploaded and associated with the selected asset.
            <br />
            Supported file types: Configuration files (.json, .xml, .yaml, etc.) and Firmware files (.bin, .hex, etc.)
          </p>
        </Dragger>

        {/* Processing Summary */}
        {draggedFiles.length > 0 && (
          <Card size="small" style={{ backgroundColor: '#fafafa' }}>
            <Space wrap>
              <Text strong>Status:</Text>
              {pendingFiles > 0 && <Tag color="default">{pendingFiles} Pending</Tag>}
              {processingFiles > 0 && <Tag color="blue">{processingFiles} Processing</Tag>}
              {completedFiles > 0 && <Tag color="green">{completedFiles} Completed</Tag>}
              {failedFiles > 0 && <Tag color="red">{failedFiles} Failed</Tag>}
            </Space>
          </Card>
        )}

        {/* File List */}
        {draggedFiles.length > 0 && (
          <List
            header={<Text strong>File Association Queue</Text>}
            bordered
            dataSource={draggedFiles}
            renderItem={(item) => (
              <List.Item
                actions={[
                  item.status === 'failed' && (
                    <Tooltip title="Retry association">
                      <Button
                        type="text"
                        icon={<LinkOutlined />}
                        onClick={() => handleRetryFile(item.id)}
                        size="small"
                      />
                    </Tooltip>
                  ),
                  <Tooltip title="Remove from queue">
                    <Button
                      type="text"
                      icon={<DeleteOutlined />}
                      onClick={() => handleRemoveFile(item.id)}
                      size="small"
                      danger
                      disabled={['uploading', 'associating'].includes(item.status)}
                    />
                  </Tooltip>
                ].filter(Boolean)}
              >
                <List.Item.Meta
                  avatar={getStatusIcon(item.status)}
                  title={
                    <Space>
                      <Text>{item.file.name}</Text>
                      <Tag color={item.fileType === 'Configuration' ? 'green' : 'blue'}>
                        {item.fileType}
                      </Tag>
                      <Tag color={getStatusColor(item.status)}>
                        {item.status.toUpperCase()}
                      </Tag>
                    </Space>
                  }
                  description={
                    <Space direction="vertical" style={{ width: '100%' }}>
                      <Text type="secondary">
                        Size: {(item.file.size / 1024 / 1024).toFixed(2)} MB
                      </Text>
                      <Text type="secondary">{getStatusText(item)}</Text>
                      {['uploading', 'associating'].includes(item.status) && (
                        <Progress 
                          percent={Math.round(item.progress)} 
                          size="small"
                          status="active"
                        />
                      )}
                    </Space>
                  }
                />
              </List.Item>
            )}
          />
        )}

        {/* Instructions */}
        <Alert
          message="How to Use"
          description={
            <div>
              <p>• Drag files from your file explorer directly onto the drop zone</p>
              <p>• Files will be automatically uploaded and associated with <strong>{targetAsset.name}</strong></p>
              <p>• File type (Configuration/Firmware) is detected automatically based on file extension</p>
              <p>• You can retry failed associations or remove files from the queue</p>
              <p>• Multiple files can be processed simultaneously</p>
            </div>
          }
          type="info"
          style={{ marginTop: 16 }}
        />
      </Space>
    </Card>
  );
};

export default DragDropAssociationInterface;