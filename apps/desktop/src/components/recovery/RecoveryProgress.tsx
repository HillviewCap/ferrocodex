import React from 'react';
import { Progress, Card, Typography, Space, Button, Statistic, Row, Col } from 'antd';
import { CloseOutlined, CheckCircleOutlined, ExclamationCircleOutlined } from '@ant-design/icons';
import { RecoveryProgressProps, ExportStep } from '../../types/recovery';

const { Text, Title } = Typography;

export const RecoveryProgress: React.FC<RecoveryProgressProps> = ({
  progress,
  onCancel,
}) => {
  const getStepLabel = (step: ExportStep): string => {
    switch (step) {
      case 'selecting':
        return 'Preparing Export';
      case 'exporting_config':
        return 'Exporting Configuration';
      case 'exporting_firmware':
        return 'Exporting Firmware';
      case 'creating_manifest':
        return 'Creating Manifest';
      case 'completed':
        return 'Export Complete';
      case 'error':
        return 'Export Failed';
      default:
        return 'Processing';
    }
  };

  const getProgressColor = (step: ExportStep): string => {
    switch (step) {
      case 'completed':
        return '#52c41a';
      case 'error':
        return '#ff4d4f';
      default:
        return '#1890ff';
    }
  };

  const getIcon = (step: ExportStep) => {
    switch (step) {
      case 'completed':
        return <CheckCircleOutlined style={{ color: '#52c41a', fontSize: 24 }} />;
      case 'error':
        return <ExclamationCircleOutlined style={{ color: '#ff4d4f', fontSize: 24 }} />;
      default:
        return null;
    }
  };

  const formatTime = (ms?: number): string => {
    if (!ms) return 'N/A';
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  const showCancel = progress.step !== 'completed' && progress.step !== 'error' && onCancel;

  return (
    <Card
      title={
        <Space>
          {getIcon(progress.step)}
          <Title level={4} style={{ margin: 0 }}>
            {getStepLabel(progress.step)}
          </Title>
        </Space>
      }
      extra={
        showCancel && (
          <Button 
            type="text" 
            icon={<CloseOutlined />} 
            onClick={onCancel}
            danger
          >
            Cancel
          </Button>
        )
      }
    >
      <Space direction="vertical" style={{ width: '100%' }} size="large">
        <Progress
          percent={progress.progress}
          status={progress.step === 'error' ? 'exception' : 'active'}
          strokeColor={getProgressColor(progress.step)}
          showInfo={true}
        />

        <div>
          <Text>{progress.message}</Text>
        </div>

        {progress.timing && (
          <Row gutter={16}>
            <Col span={8}>
              <Statistic
                title="Config Export"
                value={formatTime(progress.timing.config_export_ms)}
                valueStyle={{ fontSize: 14 }}
              />
            </Col>
            <Col span={8}>
              <Statistic
                title="Firmware Export"
                value={formatTime(progress.timing.firmware_export_ms)}
                valueStyle={{ fontSize: 14 }}
              />
            </Col>
            <Col span={8}>
              <Statistic
                title="Total Time"
                value={formatTime(progress.timing.total_ms)}
                valueStyle={{ fontSize: 14 }}
              />
            </Col>
          </Row>
        )}

        {progress.step === 'completed' && (
          <div style={{ textAlign: 'center', padding: '16px 0' }}>
            <CheckCircleOutlined style={{ color: '#52c41a', fontSize: 48 }} />
            <div style={{ marginTop: 8 }}>
              <Text strong>Recovery package exported successfully!</Text>
            </div>
          </div>
        )}

        {progress.step === 'error' && (
          <div style={{ textAlign: 'center', padding: '16px 0' }}>
            <ExclamationCircleOutlined style={{ color: '#ff4d4f', fontSize: 48 }} />
            <div style={{ marginTop: 8 }}>
              <Text type="danger" strong>Export failed. Please try again.</Text>
            </div>
          </div>
        )}
      </Space>
    </Card>
  );
};