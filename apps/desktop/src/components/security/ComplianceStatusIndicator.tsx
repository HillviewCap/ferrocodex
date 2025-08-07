import React from 'react';
import { Badge, Tooltip, Space, Typography, Progress } from 'antd';
import {
  CheckCircleOutlined,
  ExclamationCircleOutlined,
  CloseCircleOutlined,
  InfoCircleOutlined,
  TrophyOutlined,
  WarningOutlined
} from '@ant-design/icons';
import { ComplianceStatus } from '../../types/security';

const { Text } = Typography;

interface ComplianceStatusIndicatorProps {
  status: ComplianceStatus;
  showScore?: boolean;
  showProgress?: boolean;
  showDetails?: boolean;
  size?: 'small' | 'default' | 'large';
  inline?: boolean;
}

const ComplianceStatusIndicator: React.FC<ComplianceStatusIndicatorProps> = ({
  status,
  showScore = true,
  showProgress = false,
  showDetails = false,
  size = 'default',
  inline = false
}) => {
  const getStatusConfig = (level: ComplianceStatus['level']) => {
    switch (level) {
      case 'excellent':
        return {
          color: '#52c41a',
          icon: <TrophyOutlined />,
          text: 'Excellent',
          badgeStatus: 'success' as const,
          description: 'All security requirements met with best practices'
        };
      case 'good':
        return {
          color: '#1890ff',
          icon: <CheckCircleOutlined />,
          text: 'Good',
          badgeStatus: 'processing' as const,
          description: 'Security requirements met with minor recommendations'
        };
      case 'fair':
        return {
          color: '#faad14',
          icon: <ExclamationCircleOutlined />,
          text: 'Fair',
          badgeStatus: 'warning' as const,
          description: 'Some security requirements need attention'
        };
      case 'needs-attention':
        return {
          color: '#ff4d4f',
          icon: <CloseCircleOutlined />,
          text: 'Needs Attention',
          badgeStatus: 'error' as const,
          description: 'Critical security requirements not met'
        };
      default:
        return {
          color: '#d9d9d9',
          icon: <InfoCircleOutlined />,
          text: 'Unknown',
          badgeStatus: 'default' as const,
          description: 'Compliance status unknown'
        };
    }
  };

  const config = getStatusConfig(status.level);

  const renderBadge = () => (
    <Badge
      status={config.badgeStatus}
      text={
        <Space size="small">
          {config.icon}
          <Text strong style={{ color: config.color }}>
            {config.text}
          </Text>
          {showScore && (
            <Text type="secondary" style={{ fontSize: '12px' }}>
              ({status.score}%)
            </Text>
          )}
        </Space>
      }
    />
  );

  const renderProgress = () => {
    if (!showProgress) return null;

    return (
      <Progress
        percent={status.score}
        size="small"
        strokeColor={config.color}
        showInfo={false}
        style={{ marginTop: 4 }}
      />
    );
  };

  const renderDetails = () => {
    if (!showDetails) return null;

    return (
      <div style={{ marginTop: 8, fontSize: '12px' }}>
        {status.issues.length > 0 && (
          <div style={{ marginBottom: 8 }}>
            <Text strong style={{ color: '#ff4d4f', fontSize: '12px' }}>
              <WarningOutlined /> Issues ({status.issues.length}):
            </Text>
            <ul style={{ margin: '4px 0 0 16px', padding: 0 }}>
              {status.issues.map((issue, idx) => (
                <li key={idx} style={{ marginBottom: 2, color: '#666' }}>
                  {issue}
                </li>
              ))}
            </ul>
          </div>
        )}

        {status.recommendations.length > 0 && (
          <div>
            <Text strong style={{ color: '#1890ff', fontSize: '12px' }}>
              <InfoCircleOutlined /> Recommendations ({status.recommendations.length}):
            </Text>
            <ul style={{ margin: '4px 0 0 16px', padding: 0 }}>
              {status.recommendations.map((rec, idx) => (
                <li key={idx} style={{ marginBottom: 2, color: '#666' }}>
                  {rec}
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    );
  };

  const content = (
    <div style={{ display: inline ? 'inline-block' : 'block' }}>
      {renderBadge()}
      {renderProgress()}
      {renderDetails()}
    </div>
  );

  // Wrap in tooltip if not showing details inline
  if (!showDetails && (status.issues.length > 0 || status.recommendations.length > 0)) {
    return (
      <Tooltip
        title={
          <div style={{ maxWidth: 300 }}>
            <div style={{ marginBottom: 8, fontWeight: 'bold' }}>
              {config.description}
            </div>
            
            {status.issues.length > 0 && (
              <div style={{ marginBottom: 8 }}>
                <div style={{ color: '#ff7875', fontWeight: 'bold', marginBottom: 4 }}>
                  Issues:
                </div>
                <ul style={{ margin: 0, paddingLeft: 16 }}>
                  {status.issues.map((issue, idx) => (
                    <li key={idx} style={{ marginBottom: 2 }}>{issue}</li>
                  ))}
                </ul>
              </div>
            )}

            {status.recommendations.length > 0 && (
              <div>
                <div style={{ color: '#91d5ff', fontWeight: 'bold', marginBottom: 4 }}>
                  Recommendations:
                </div>
                <ul style={{ margin: 0, paddingLeft: 16 }}>
                  {status.recommendations.map((rec, idx) => (
                    <li key={idx} style={{ marginBottom: 2 }}>{rec}</li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        }
        placement="top"
        overlayStyle={{ maxWidth: '400px' }}
      >
        {content}
      </Tooltip>
    );
  }

  return content;
};

export default ComplianceStatusIndicator;