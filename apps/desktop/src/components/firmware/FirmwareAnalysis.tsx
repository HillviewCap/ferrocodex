import React, { useEffect, useState } from 'react';
import { Card, Progress, Tag, Descriptions, Alert, Button, Collapse, List, Spin, Space, Typography } from 'antd';
import { 
  SyncOutlined, 
  FileSearchOutlined, 
  WarningOutlined,
  InfoCircleOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  ReloadOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { 
  FirmwareAnalysisResult, 
  AnalysisEvent, 
  SecuritySeverity,
  SecurityFinding 
} from '../../types/firmware';
import useAuthStore from '../../store/auth';

const { Panel } = Collapse;
const { Text } = Typography;

interface FirmwareAnalysisProps {
  firmwareId: number;
}

const FirmwareAnalysis: React.FC<FirmwareAnalysisProps> = ({ firmwareId }) => {
  const { token } = useAuthStore();
  const [analysis, setAnalysis] = useState<FirmwareAnalysisResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [retrying, setRetrying] = useState(false);
  const [analysisProgress, setAnalysisProgress] = useState<AnalysisEvent | null>(null);

  useEffect(() => {
    fetchAnalysis();
    
    // Listen for analysis progress events
    const unlisten = listen<AnalysisEvent>('firmware-analysis-progress', (event) => {
      if (event.payload.firmwareId === firmwareId) {
        setAnalysisProgress(event.payload);
        
        // If analysis completed or failed, refresh the analysis data
        if (event.payload.status === 'completed' || event.payload.status === 'failed') {
          fetchAnalysis();
          setAnalysisProgress(null);
        }
      }
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, [firmwareId, token]);

  const fetchAnalysis = async () => {
    if (!token) return;
    
    setLoading(true);
    try {
      const result = await invoke<FirmwareAnalysisResult | null>('get_firmware_analysis', {
        token,
        firmwareId
      });
      setAnalysis(result);
    } catch (error) {
      console.error('Failed to fetch firmware analysis:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleRetryAnalysis = async () => {
    if (!token) return;
    
    setRetrying(true);
    try {
      await invoke('retry_firmware_analysis', {
        token,
        firmwareId
      });
      // Analysis will be queued, progress will be tracked via events
    } catch (error) {
      console.error('Failed to retry analysis:', error);
    } finally {
      setRetrying(false);
    }
  };

  const getStatusIcon = (status?: string) => {
    switch (status) {
      case 'pending':
        return <CloseCircleOutlined style={{ color: '#d9d9d9' }} />;
      case 'in_progress':
        return <SyncOutlined spin style={{ color: '#1890ff' }} />;
      case 'completed':
        return <CheckCircleOutlined style={{ color: '#52c41a' }} />;
      case 'failed':
        return <CloseCircleOutlined style={{ color: '#ff4d4f' }} />;
      default:
        return null;
    }
  };

  const getStatusColor = (status?: string) => {
    switch (status) {
      case 'pending': return 'default';
      case 'in_progress': return 'processing';
      case 'completed': return 'success';
      case 'failed': return 'error';
      default: return 'default';
    }
  };

  const getSeverityIcon = (severity: SecuritySeverity) => {
    switch (severity) {
      case 'critical':
      case 'high':
        return <WarningOutlined style={{ color: '#ff4d4f' }} />;
      case 'medium':
        return <WarningOutlined style={{ color: '#faad14' }} />;
      case 'low':
        return <InfoCircleOutlined style={{ color: '#1890ff' }} />;
      case 'info':
      default:
        return <InfoCircleOutlined style={{ color: '#d9d9d9' }} />;
    }
  };

  const getSeverityColor = (severity: SecuritySeverity) => {
    switch (severity) {
      case 'critical': return 'error';
      case 'high': return 'error';
      case 'medium': return 'warning';
      case 'low': return 'processing';
      case 'info': return 'default';
      default: return 'default';
    }
  };

  const renderSecurityFindings = (findings: SecurityFinding[]) => {
    if (!findings || findings.length === 0) {
      return (
        <Alert
          message="No security issues found"
          description="The firmware analysis did not detect any security concerns."
          type="success"
          showIcon
        />
      );
    }

    const groupedFindings = findings.reduce((acc, finding) => {
      if (!acc[finding.severity]) {
        acc[finding.severity] = [];
      }
      acc[finding.severity].push(finding);
      return acc;
    }, {} as Record<SecuritySeverity, SecurityFinding[]>);

    const severityOrder: SecuritySeverity[] = ['critical', 'high', 'medium', 'low', 'info'];

    return (
      <List
        dataSource={severityOrder.filter(sev => groupedFindings[sev]?.length > 0)}
        renderItem={(severity) => (
          <List.Item key={severity}>
            <List.Item.Meta
              avatar={getSeverityIcon(severity)}
              title={
                <Space>
                  <Tag color={getSeverityColor(severity)}>
                    {severity.toUpperCase()}
                  </Tag>
                  <Text strong>{groupedFindings[severity].length} finding(s)</Text>
                </Space>
              }
              description={
                <List
                  size="small"
                  dataSource={groupedFindings[severity]}
                  renderItem={(finding, index) => (
                    <List.Item key={index}>
                      <Space direction="vertical" style={{ width: '100%' }}>
                        <Text strong>{finding.findingType}</Text>
                        <Text type="secondary">{finding.description}</Text>
                        {finding.offset !== undefined && (
                          <Text code>Offset: 0x{finding.offset.toString(16)}</Text>
                        )}
                      </Space>
                    </List.Item>
                  )}
                />
              }
            />
          </List.Item>
        )}
      />
    );
  };

  if (loading) {
    return (
      <Card>
        <Spin tip="Loading analysis results..." />
      </Card>
    );
  }

  if (!analysis) {
    return (
      <Card>
        <Alert
          message="No Analysis Available"
          description="This firmware has not been analyzed yet."
          type="info"
          showIcon
          action={
            <Button 
              type="primary" 
              icon={<FileSearchOutlined />}
              onClick={handleRetryAnalysis}
              loading={retrying}
            >
              Start Analysis
            </Button>
          }
        />
      </Card>
    );
  }

  return (
    <Card
      title={
        <Space>
          <FileSearchOutlined />
          <span>Firmware Analysis</span>
        </Space>
      }
      extra={
        analysis.analysisStatus === 'failed' && (
          <Button 
            icon={<ReloadOutlined />} 
            onClick={handleRetryAnalysis}
            loading={retrying}
          >
            Retry Analysis
          </Button>
        )
      }
    >
      {/* Show progress if analysis is running */}
      {analysisProgress && analysisProgress.status === 'in_progress' && (
        <Alert
          message="Analysis in Progress"
          description={
            <Space direction="vertical" style={{ width: '100%' }}>
              <Text>{analysisProgress.message || 'Analyzing firmware...'}</Text>
              {analysisProgress.progress !== undefined && (
                <Progress percent={analysisProgress.progress} />
              )}
            </Space>
          }
          type="info"
          showIcon
          icon={<SyncOutlined spin />}
          style={{ marginBottom: 16 }}
        />
      )}

      {/* Analysis Status */}
      <Descriptions bordered column={2} style={{ marginBottom: 16 }}>
        <Descriptions.Item label="Status" span={2}>
          <Space>
            {getStatusIcon(analysis.analysisStatus)}
            <Tag color={getStatusColor(analysis.analysisStatus)}>
              {analysis.analysisStatus.toUpperCase()}
            </Tag>
          </Space>
        </Descriptions.Item>
        
        {analysis.fileType && (
          <Descriptions.Item label="File Type">
            <Tag>{analysis.fileType}</Tag>
          </Descriptions.Item>
        )}
        
        {analysis.entropyScore !== undefined && analysis.entropyScore !== null && (
          <Descriptions.Item label="Entropy Score">
            <Progress 
              percent={Math.round((analysis.entropyScore / 8) * 100)} 
              size="small"
              format={() => analysis.entropyScore!.toFixed(2)}
            />
          </Descriptions.Item>
        )}
        
        {analysis.startedAt && (
          <Descriptions.Item label="Started At">
            {new Date(analysis.startedAt).toLocaleString()}
          </Descriptions.Item>
        )}
        
        {analysis.completedAt && (
          <Descriptions.Item label="Completed At">
            {new Date(analysis.completedAt).toLocaleString()}
          </Descriptions.Item>
        )}
      </Descriptions>

      {/* Error Message */}
      {analysis.errorMessage && (
        <Alert
          message="Analysis Failed"
          description={analysis.errorMessage}
          type="error"
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}

      {/* Analysis Results */}
      {analysis.analysisStatus === 'completed' && (
        <Collapse defaultActiveKey={['versions', 'security']}>
          {/* Detected Versions */}
          {analysis.detectedVersions && analysis.detectedVersions.length > 0 && (
            <Panel header="Detected Versions" key="versions">
              <List
                size="small"
                dataSource={analysis.detectedVersions}
                renderItem={(version) => (
                  <List.Item>
                    <Tag color="blue">{version}</Tag>
                  </List.Item>
                )}
              />
            </Panel>
          )}

          {/* Security Findings */}
          <Panel 
            header={
              <Space>
                <span>Security Findings</span>
                {analysis.securityFindings && analysis.securityFindings.length > 0 && (
                  <Tag color="orange">{analysis.securityFindings.length}</Tag>
                )}
              </Space>
            } 
            key="security"
          >
            {renderSecurityFindings(analysis.securityFindings || [])}
          </Panel>

          {/* Raw Results (Advanced) */}
          {analysis.rawResults && (
            <Panel header="Raw Analysis Output (Advanced)" key="raw">
              <pre style={{ 
                maxHeight: 400, 
                overflow: 'auto', 
                backgroundColor: '#f5f5f5',
                padding: 8,
                borderRadius: 4
              }}>
                {analysis.rawResults}
              </pre>
            </Panel>
          )}
        </Collapse>
      )}
    </Card>
  );
};

export default FirmwareAnalysis;