import React, { useState, useEffect } from 'react';
import { Card, Progress, Space, Typography, Button, Alert, Tooltip, Badge, Collapse } from 'antd';
import {
  CheckCircleOutlined,
  CloseCircleOutlined,
  LoadingOutlined,
  SafetyCertificateOutlined,
  FileTextOutlined,
  EyeOutlined,
  CopyOutlined,
  ExclamationCircleOutlined
} from '@ant-design/icons';
import { FileIntegrityResult, FileValidationStatus } from '../../types/security';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../../store/auth';

const { Text, Title } = Typography;
const { Panel } = Collapse;

interface FileIntegrityDisplayProps {
  filePath: string;
  expectedHash?: string;
  autoVerify?: boolean;
  showDetails?: boolean;
  onVerificationComplete?: (result: FileIntegrityResult) => void;
  onVerificationError?: (error: string) => void;
}

const FileIntegrityDisplay: React.FC<FileIntegrityDisplayProps> = ({
  filePath,
  expectedHash,
  autoVerify = true,
  showDetails = true,
  onVerificationComplete,
  onVerificationError
}) => {
  const [verificationState, setVerificationState] = useState<{
    status: 'idle' | 'calculating' | 'complete' | 'error';
    result?: FileIntegrityResult;
    progress: number;
    error?: string;
  }>({
    status: 'idle',
    progress: 0
  });

  const { token } = useAuthStore();

  // Start verification process
  const startVerification = async () => {
    setVerificationState({
      status: 'calculating',
      progress: 0
    });

    try {
      // Simulate progress updates during hash calculation
      const progressInterval = setInterval(() => {
        setVerificationState(prev => ({
          ...prev,
          progress: Math.min(prev.progress + Math.random() * 20, 90)
        }));
      }, 200);

      const result = await invoke<FileIntegrityResult>('validate_file_upload', {
        token,
        filePath
      });

      clearInterval(progressInterval);

      // If expected hash is provided, verify integrity
      let integrityVerified = true;
      if (expectedHash) {
        integrityVerified = result.sha256Hash === expectedHash;
      }

      const finalResult = {
        ...result,
        isVerified: integrityVerified
      };

      setVerificationState({
        status: 'complete',
        result: finalResult,
        progress: 100
      });

      onVerificationComplete?.(finalResult);

    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Verification failed';
      
      setVerificationState({
        status: 'error',
        progress: 0,
        error: errorMessage
      });

      onVerificationError?.(errorMessage);
    }
  };

  // Auto-verify on mount if enabled
  useEffect(() => {
    if (autoVerify) {
      startVerification();
    }
  }, [filePath, autoVerify]);

  // Format file size
  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
  };

  // Copy hash to clipboard
  const copyHashToClipboard = async () => {
    if (verificationState.result?.sha256Hash) {
      try {
        await navigator.clipboard.writeText(verificationState.result.sha256Hash);
        // Could add a toast notification here
      } catch (error) {
        console.error('Failed to copy hash:', error);
      }
    }
  };

  // Get status icon and color
  const getStatusIcon = () => {
    switch (verificationState.status) {
      case 'calculating':
        return <LoadingOutlined style={{ color: '#1890ff' }} />;
      case 'complete':
        return verificationState.result?.isVerified && verificationState.result?.securityScanPassed
          ? <CheckCircleOutlined style={{ color: '#52c41a' }} />
          : <ExclamationCircleOutlined style={{ color: '#faad14' }} />;
      case 'error':
        return <CloseCircleOutlined style={{ color: '#ff4d4f' }} />;
      default:
        return <FileTextOutlined style={{ color: '#666' }} />;
    }
  };

  const getStatusText = () => {
    switch (verificationState.status) {
      case 'calculating':
        return 'Calculating hash and verifying integrity...';
      case 'complete':
        if (!verificationState.result) return 'Complete';
        
        if (verificationState.result.isVerified && verificationState.result.securityScanPassed) {
          return 'File integrity verified - All checks passed';
        } else if (!verificationState.result.isVerified) {
          return 'Hash mismatch detected - Integrity compromised';
        } else if (!verificationState.result.securityScanPassed) {
          return 'Security scan detected issues';
        }
        return 'Verification complete with warnings';
      case 'error':
        return verificationState.error || 'Verification failed';
      default:
        return 'Ready for verification';
    }
  };

  const renderProgressSection = () => {
    if (verificationState.status !== 'calculating') return null;

    return (
      <div style={{ marginBottom: 16 }}>
        <Progress
          percent={verificationState.progress}
          status="active"
          strokeColor="#1890ff"
          size="small"
        />
        <Text type="secondary" style={{ fontSize: '12px' }}>
          Computing SHA-256 hash and performing security checks...
        </Text>
      </div>
    );
  };

  const renderHashSection = () => {
    if (!verificationState.result) return null;

    const { sha256Hash, isVerified } = verificationState.result;

    return (
      <div style={{ marginBottom: 16 }}>
        <div style={{ marginBottom: 8 }}>
          <Space>
            <Text strong>SHA-256 Hash:</Text>
            {expectedHash && (
              <Badge 
                status={isVerified ? 'success' : 'error'} 
                text={isVerified ? 'Verified' : 'Mismatch'} 
              />
            )}
          </Space>
        </div>
        
        <div 
          style={{ 
            backgroundColor: '#f5f5f5', 
            padding: '8px 12px', 
            borderRadius: '4px',
            fontFamily: 'monospace',
            fontSize: '12px',
            wordBreak: 'break-all',
            border: expectedHash && !isVerified ? '1px solid #ff4d4f' : '1px solid #d9d9d9'
          }}
        >
          {sha256Hash}
          <Tooltip title="Copy hash">
            <Button
              type="text"
              size="small"
              icon={<CopyOutlined />}
              onClick={copyHashToClipboard}
              style={{ float: 'right', padding: 0 }}
            />
          </Tooltip>
        </div>

        {expectedHash && !isVerified && (
          <Alert
            message="Hash Mismatch"
            description={
              <div>
                <div>Expected: <code>{expectedHash}</code></div>
                <div>Actual: <code>{sha256Hash}</code></div>
                <div style={{ marginTop: 8, color: '#ff4d4f' }}>
                  ⚠️ This file may have been modified or corrupted.
                </div>
              </div>
            }
            type="error"
            showIcon
            style={{ marginTop: 8, fontSize: '12px' }}
          />
        )}
      </div>
    );
  };

  const renderSecuritySection = () => {
    if (!verificationState.result) return null;

    const { securityScanPassed, detectedIssues } = verificationState.result;

    return (
      <div style={{ marginBottom: 16 }}>
        <Space style={{ marginBottom: 8 }}>
          <SafetyCertificateOutlined />
          <Text strong>Security Scan:</Text>
          <Badge 
            status={securityScanPassed ? 'success' : 'error'} 
            text={securityScanPassed ? 'Passed' : 'Issues Detected'} 
          />
        </Space>

        {!securityScanPassed && detectedIssues.length > 0 && (
          <Alert
            message="Security Issues Detected"
            description={
              <ul style={{ margin: '8px 0 0 16px', padding: 0 }}>
                {detectedIssues.map((issue, idx) => (
                  <li key={idx} style={{ marginBottom: 4 }}>{issue}</li>
                ))}
              </ul>
            }
            type="warning"
            showIcon
            style={{ fontSize: '12px' }}
          />
        )}
      </div>
    );
  };

  const renderFileInfoSection = () => {
    if (!verificationState.result) return null;

    const { fileSize } = verificationState.result;

    return (
      <div style={{ marginBottom: 16 }}>
        <Space>
          <Text strong>File Information:</Text>
        </Space>
        <div style={{ marginTop: 8, fontSize: '12px', color: '#666' }}>
          <div>Path: {filePath}</div>
          <div>Size: {formatFileSize(fileSize)}</div>
        </div>
      </div>
    );
  };

  return (
    <Card
      title={
        <Space>
          {getStatusIcon()}
          <Title level={5} style={{ margin: 0 }}>File Integrity Verification</Title>
        </Space>
      }
      extra={
        verificationState.status === 'idle' && (
          <Button 
            type="primary" 
            size="small" 
            onClick={startVerification}
            icon={<EyeOutlined />}
          >
            Verify
          </Button>
        )
      }
      size="small"
    >
      <div>
        <div style={{ marginBottom: 16 }}>
          <Text type={verificationState.status === 'error' ? 'danger' : 'secondary'}>
            {getStatusText()}
          </Text>
        </div>

        {renderProgressSection()}

        {showDetails && verificationState.result && (
          <Collapse size="small" ghost>
            <Panel header="Verification Details" key="details">
              {renderFileInfoSection()}
              {renderHashSection()}
              {renderSecuritySection()}
            </Panel>
          </Collapse>
        )}

        {!showDetails && verificationState.result && (
          <>
            {renderHashSection()}
            {renderSecuritySection()}
          </>
        )}

        {verificationState.status === 'error' && (
          <div style={{ marginTop: 16 }}>
            <Button 
              type="default" 
              size="small" 
              onClick={startVerification}
              icon={<LoadingOutlined />}
            >
              Retry Verification
            </Button>
          </div>
        )}
      </div>
    </Card>
  );
};

export default FileIntegrityDisplay;