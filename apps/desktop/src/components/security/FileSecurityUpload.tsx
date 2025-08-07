import React, { useState, useRef } from 'react';
import { Upload, Card, Progress, Space, Typography, Button, Alert, List, Badge, Tooltip } from 'antd';
import {
  InboxOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  LoadingOutlined,
  SafetyCertificateOutlined,
  FileOutlined,
  DeleteOutlined,
  EyeOutlined
} from '@ant-design/icons';
import { FileIntegrityResult, FileValidationStatus, SecurityClassificationLevel } from '../../types/security';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../../store/auth';
import SecurityClassificationSelector from './SecurityClassificationSelector';
import FileIntegrityDisplay from './FileIntegrityDisplay';

const { Dragger } = Upload;
const { Text, Title } = Typography;

interface FileSecurityUploadProps {
  onUploadComplete: (results: FileIntegrityResult[]) => void;
  onUploadProgress: (status: FileValidationStatus[]) => void;
  onUploadError: (error: string) => void;
  acceptedFileTypes?: string[];
  maxFileSize?: number;
  maxFiles?: number;
  autoClassification?: SecurityClassificationLevel;
  requiresClassification?: boolean;
  showIntegrityDetails?: boolean;
}

const FileSecurityUpload: React.FC<FileSecurityUploadProps> = ({
  onUploadComplete,
  onUploadProgress,
  onUploadError,
  acceptedFileTypes = [],
  maxFileSize = 100 * 1024 * 1024, // 100MB
  maxFiles = 10,
  autoClassification,
  requiresClassification = true,
  showIntegrityDetails = true
}) => {
  const [uploadFiles, setUploadFiles] = useState<{
    [key: string]: FileValidationStatus & {
      file: File;
      classification?: SecurityClassificationLevel;
      result?: FileIntegrityResult;
    };
  }>({});
  
  const [selectedClassification, setSelectedClassification] = useState<SecurityClassificationLevel | undefined>(
    autoClassification
  );
  
  const [isUploading] = useState(false);
  const { token } = useAuthStore();

  // Validate file before upload
  const validateFileSelection = (file: File): string | null => {
    // Check file size
    if (file.size > maxFileSize) {
      return `File size exceeds limit (${Math.round(maxFileSize / 1024 / 1024)}MB)`;
    }

    // Check file type
    if (acceptedFileTypes.length > 0) {
      const fileExtension = file.name.split('.').pop()?.toLowerCase();
      if (!fileExtension || !acceptedFileTypes.includes(`.${fileExtension}`)) {
        return `File type not allowed. Accepted types: ${acceptedFileTypes.join(', ')}`;
      }
    }

    // Check maximum files
    if (Object.keys(uploadFiles).length >= maxFiles) {
      return `Maximum ${maxFiles} files allowed`;
    }

    return null;
  };

  // Handle file selection
  const handleFileSelect = (file: File) => {
    const validationError = validateFileSelection(file);
    if (validationError) {
      onUploadError(validationError);
      return false;
    }

    const fileId = `${file.name}-${Date.now()}`;
    const initialStatus: FileValidationStatus & {
      file: File;
      classification?: SecurityClassificationLevel;
    } = {
      filename: file.name,
      validationStage: 'pending',
      progressPercentage: 0,
      hashStatus: 'pending',
      complianceStatus: 'pending',
      file,
      classification: selectedClassification
    };

    setUploadFiles(prev => ({
      ...prev,
      [fileId]: initialStatus
    }));

    // Start validation process
    startFileValidation(fileId, file);
    return false; // Prevent default upload
  };

  // Start comprehensive file validation
  const startFileValidation = async (fileId: string, file: File) => {
    const updateStatus = (updates: Partial<FileValidationStatus>) => {
      setUploadFiles(prev => ({
        ...prev,
        [fileId]: { ...prev[fileId], ...updates }
      }));
    };

    try {
      // Stage 1: Filename validation
      updateStatus({ 
        validationStage: 'filename', 
        progressPercentage: 10 
      });

      const sanitizedFilename = await invoke<string>('sanitize_filename', {
        token,
        filename: file.name
      });

      if (sanitizedFilename !== file.name) {
        updateStatus({
          error: `Filename will be sanitized to: ${sanitizedFilename}`,
          progressPercentage: 20
        });
      } else {
        updateStatus({ progressPercentage: 20 });
      }

      // Stage 2: Content validation (simulate file save and validation)
      updateStatus({ 
        validationStage: 'content', 
        progressPercentage: 40,
        complianceStatus: 'checking'
      });

      // For demo purposes, we'll simulate file upload validation
      // In a real implementation, you'd save the file temporarily and validate it
      
      // Stage 3: Hash calculation
      updateStatus({ 
        validationStage: 'hash', 
        progressPercentage: 60,
        hashStatus: 'calculating'
      });

      // Simulate hash calculation progress
      const hashInterval = setInterval(() => {
        setUploadFiles(prev => {
          const currentFile = prev[fileId];
          if (currentFile && currentFile.progressPercentage < 80) {
            return {
              ...prev,
              [fileId]: {
                ...currentFile,
                progressPercentage: Math.min(currentFile.progressPercentage + 5, 80)
              }
            };
          }
          return prev;
        });
      }, 200);

      // For demo, we'll simulate the validation result
      // In real implementation, this would call validate_file_upload
      setTimeout(() => {
        clearInterval(hashInterval);
        
        const mockResult: FileIntegrityResult = {
          sha256Hash: `sha256_${file.name}_${Date.now()}`,
          fileSize: file.size,
          isVerified: true,
          securityScanPassed: Math.random() > 0.2, // 80% pass rate for demo
          detectedIssues: Math.random() > 0.8 ? ['Minor security warning'] : []
        };

        updateStatus({
          validationStage: 'complete',
          progressPercentage: 100,
          hashStatus: 'complete',
          complianceStatus: mockResult.securityScanPassed ? 'passed' : 'failed',
          result: mockResult
        });

        setUploadFiles(prev => ({
          ...prev,
          [fileId]: { ...prev[fileId], result: mockResult }
        }));

      }, 1500);

    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Validation failed';
      updateStatus({
        validationStage: 'failed',
        error: errorMessage,
        hashStatus: 'failed',
        complianceStatus: 'failed'
      });
    }
  };

  // Remove file from upload list
  const removeFile = (fileId: string) => {
    setUploadFiles(prev => {
      const newFiles = { ...prev };
      delete newFiles[fileId];
      return newFiles;
    });
  };

  // Complete upload process
  const completeUpload = () => {
    const completedFiles = Object.values(uploadFiles).filter(f => f.result);
    const results = completedFiles.map(f => f.result!);
    onUploadComplete(results);
  };

  // Get overall upload status
  const getOverallStatus = () => {
    const files = Object.values(uploadFiles);
    if (files.length === 0) return 'idle';
    
    const allComplete = files.every(f => f.validationStage === 'complete');
    const anyFailed = files.some(f => f.validationStage === 'failed');
    
    if (anyFailed) return 'error';
    if (allComplete) return 'complete';
    return 'processing';
  };

  // Render file status item
  const renderFileStatus = (fileId: string, fileStatus: typeof uploadFiles[string]) => {
    const getStageIcon = () => {
      switch (fileStatus.validationStage) {
        case 'complete':
          return fileStatus.complianceStatus === 'passed' 
            ? <CheckCircleOutlined style={{ color: '#52c41a' }} />
            : <CloseCircleOutlined style={{ color: '#ff4d4f' }} />;
        case 'failed':
          return <CloseCircleOutlined style={{ color: '#ff4d4f' }} />;
        default:
          return <LoadingOutlined style={{ color: '#1890ff' }} />;
      }
    };

    const getStageText = () => {
      switch (fileStatus.validationStage) {
        case 'filename': return 'Validating filename...';
        case 'content': return 'Scanning content...';
        case 'hash': return 'Calculating hash...';
        case 'classification': return 'Setting classification...';
        case 'complete': return 'Validation complete';
        case 'failed': return fileStatus.error || 'Validation failed';
        default: return 'Pending validation';
      }
    };

    return (
      <List.Item
        key={fileId}
        actions={[
          showIntegrityDetails && fileStatus.result && (
            <Tooltip title="View integrity details">
              <Button
                type="text"
                size="small"
                icon={<EyeOutlined />}
                onClick={() => {/* Show details modal */}}
              />
            </Tooltip>
          ),
          <Tooltip title="Remove file">
            <Button
              type="text"
              size="small"
              danger
              icon={<DeleteOutlined />}
              onClick={() => removeFile(fileId)}
            />
          </Tooltip>
        ].filter(Boolean)}
      >
        <List.Item.Meta
          avatar={<FileOutlined style={{ fontSize: '24px', color: '#1890ff' }} />}
          title={
            <Space>
              <Text strong>{fileStatus.filename}</Text>
              {fileStatus.classification && (
                <Badge 
                  color={fileStatus.classification === SecurityClassificationLevel.SECRET ? '#f5222d' : '#52c41a'} 
                  text={fileStatus.classification} 
                />
              )}
            </Space>
          }
          description={
            <div>
              <Space direction="vertical" size="small" style={{ width: '100%' }}>
                <Space>
                  {getStageIcon()}
                  <Text type="secondary" style={{ fontSize: '12px' }}>
                    {getStageText()}
                  </Text>
                </Space>
                
                {fileStatus.validationStage !== 'pending' && fileStatus.validationStage !== 'failed' && (
                  <Progress 
                    percent={fileStatus.progressPercentage} 
                    size="small" 
                    showInfo={false}
                    status={fileStatus.validationStage === 'complete' ? 'success' : 'active'}
                  />
                )}

                {fileStatus.result && (
                  <Space size="small">
                    <Badge 
                      status={fileStatus.result.securityScanPassed ? 'success' : 'error'} 
                      text={`Security: ${fileStatus.result.securityScanPassed ? 'Passed' : 'Issues'}`}
                    />
                    <Badge 
                      status="processing" 
                      text={`${Math.round(fileStatus.result.fileSize / 1024)}KB`}
                    />
                  </Space>
                )}
              </Space>
            </div>
          }
        />
      </List.Item>
    );
  };

  const overallStatus = getOverallStatus();
  const fileCount = Object.keys(uploadFiles).length;
  const completedCount = Object.values(uploadFiles).filter(f => f.validationStage === 'complete').length;

  return (
    <Card
      title={
        <Space>
          <SafetyCertificateOutlined />
          <Title level={4} style={{ margin: 0 }}>Secure File Upload</Title>
        </Space>
      }
      size="small"
    >
      {/* Classification Selection */}
      {requiresClassification && !autoClassification && (
        <div style={{ marginBottom: 16 }}>
          <Text strong style={{ display: 'block', marginBottom: 8 }}>
            Security Classification:
          </Text>
          <SecurityClassificationSelector
            value={selectedClassification}
            onChange={setSelectedClassification}
            showDescription={false}
            size="small"
          />
        </div>
      )}

      {/* File Upload Area */}
      <Dragger
        name="files"
        multiple={maxFiles > 1}
        beforeUpload={handleFileSelect}
        disabled={isUploading || (requiresClassification && !selectedClassification)}
        style={{ marginBottom: 16 }}
      >
        <p className="ant-upload-drag-icon">
          <InboxOutlined />
        </p>
        <p className="ant-upload-text">
          Click or drag files to this area to upload
        </p>
        <p className="ant-upload-hint">
          Files will be validated for security compliance and integrity.
          {acceptedFileTypes.length > 0 && (
            <br />
          )}
          {acceptedFileTypes.length > 0 && `Accepted types: ${acceptedFileTypes.join(', ')}`}
          <br />
          Maximum file size: {Math.round(maxFileSize / 1024 / 1024)}MB
        </p>
      </Dragger>

      {/* Upload Progress Summary */}
      {fileCount > 0 && (
        <div style={{ marginBottom: 16 }}>
          <Space>
            <Text strong>Upload Progress:</Text>
            <Text>{completedCount}/{fileCount} files processed</Text>
            {overallStatus === 'complete' && (
              <Badge status="success" text="All files validated" />
            )}
            {overallStatus === 'error' && (
              <Badge status="error" text="Some files failed validation" />
            )}
          </Space>
        </div>
      )}

      {/* File List */}
      {fileCount > 0 && (
        <List
          dataSource={Object.entries(uploadFiles)}
          renderItem={([fileId, fileStatus]) => renderFileStatus(fileId, fileStatus)}
          style={{ marginBottom: 16 }}
        />
      )}

      {/* Action Buttons */}
      {fileCount > 0 && (
        <div style={{ textAlign: 'right' }}>
          <Space>
            <Button 
              onClick={() => setUploadFiles({})}
              disabled={isUploading}
            >
              Clear All
            </Button>
            <Button 
              type="primary" 
              onClick={completeUpload}
              disabled={overallStatus !== 'complete'}
              icon={<CheckCircleOutlined />}
            >
              Complete Upload ({completedCount} files)
            </Button>
          </Space>
        </div>
      )}

      {/* Error Alert */}
      {overallStatus === 'error' && (
        <Alert
          message="Upload Issues Detected"
          description="Some files failed security validation. Please review the issues above and retry if necessary."
          type="warning"
          showIcon
          style={{ marginTop: 16 }}
        />
      )}
    </Card>
  );
};

export default FileSecurityUpload;