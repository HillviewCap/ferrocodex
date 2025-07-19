import React from 'react';
import {
  Card,
  Typography,
  Space,
  Tag,
  Tooltip,
  Row,
  Col,
  Button,
  Badge
} from 'antd';
import {
  TrophyOutlined,
  StarFilled,
  UserOutlined,
  CalendarOutlined,
  FileOutlined,
  HistoryOutlined,
  EyeOutlined
} from '@ant-design/icons';
import { ConfigurationVersionInfo, formatVersion, formatFileSize } from '../types/assets';

const { Text, Title } = Typography;

interface GoldenVersionIndicatorProps {
  goldenVersion: ConfigurationVersionInfo;
  onViewDetails?: (version: ConfigurationVersionInfo) => void;
  onViewHistory?: (version: ConfigurationVersionInfo) => void;
  className?: string;
  style?: React.CSSProperties;
}

const GoldenVersionIndicator: React.FC<GoldenVersionIndicatorProps> = ({
  goldenVersion,
  onViewDetails,
  onViewHistory,
  className,
  style
}) => {
  const formatDate = (dateString: string) => {
    try {
      return new Date(dateString).toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
      });
    } catch {
      return dateString;
    }
  };

  const formatRelativeTime = (dateString: string) => {
    try {
      const date = new Date(dateString);
      const now = new Date();
      const diffInSeconds = Math.floor((now.getTime() - date.getTime()) / 1000);
      
      if (diffInSeconds < 60) return 'just now';
      if (diffInSeconds < 3600) return `${Math.floor(diffInSeconds / 60)} minutes ago`;
      if (diffInSeconds < 86400) return `${Math.floor(diffInSeconds / 3600)} hours ago`;
      if (diffInSeconds < 2592000) return `${Math.floor(diffInSeconds / 86400)} days ago`;
      if (diffInSeconds < 31536000) return `${Math.floor(diffInSeconds / 2592000)} months ago`;
      return `${Math.floor(diffInSeconds / 31536000)} years ago`;
    } catch {
      return dateString;
    }
  };

  const getStatusChangedText = () => {
    if (goldenVersion.status_changed_at) {
      return formatRelativeTime(goldenVersion.status_changed_at);
    }
    return formatRelativeTime(goldenVersion.created_at);
  };

  const getFileTypeIcon = (fileName: string) => {
    const extension = fileName.split('.').pop()?.toLowerCase();
    switch (extension) {
      case 'json':
        return '🔧';
      case 'xml':
        return '📄';
      case 'yaml':
      case 'yml':
        return '📝';
      case 'txt':
        return '📄';
      case 'bin':
      case 'dat':
        return '💾';
      default:
        return '📄';
    }
  };

  return (
    <Badge.Ribbon
      text={
        <Space size={4}>
          <StarFilled />
          <Text style={{ color: 'white', fontWeight: 'bold' }}>GOLDEN IMAGE</Text>
        </Space>
      }
      color="gold"
      style={{ marginRight: -8 }}
    >
      <Card
        className={className}
        style={{
          background: 'linear-gradient(135deg, #fff9c4 0%, #fffbe0 100%)',
          border: '2px solid #fadb14',
          borderRadius: '12px',
          boxShadow: '0 4px 12px rgba(250, 219, 20, 0.15)',
          ...style
        }}
        styles={{ 
          body: { padding: '20px' },
          header: { 
            background: 'transparent',
            borderBottom: '1px solid #fadb14'
          }
        }}
        title={
          <Space size={8}>
            <TrophyOutlined style={{ color: '#fa8c16', fontSize: '18px' }} />
            <Title level={4} style={{ margin: 0, color: '#614700' }}>
              Golden Image Version
            </Title>
          </Space>
        }
      >
        <Row gutter={[16, 16]} align="middle">
          <Col flex="auto">
            <Space direction="vertical" size={12} style={{ width: '100%' }}>
              {/* Version Info */}
              <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
                <div style={{ fontSize: '24px' }}>
                  {getFileTypeIcon(goldenVersion.file_name)}
                </div>
                <div style={{ flex: 1 }}>
                  <Space size={8} wrap>
                    <Tag color="gold" style={{ fontWeight: 'bold' }}>
                      {formatVersion(goldenVersion.version_number)}
                    </Tag>
                    <Text strong style={{ fontSize: '16px', color: '#614700' }}>
                      {goldenVersion.file_name}
                    </Text>
                  </Space>
                </div>
              </div>

              {/* Metadata */}
              <div>
                <Space size={16} wrap>
                  <Tooltip title={`Author: ${goldenVersion.author_username}`}>
                    <Space size={4}>
                      <UserOutlined style={{ fontSize: '12px', color: '#8c6e00' }} />
                      <Text style={{ fontSize: '13px', color: '#8c6e00' }}>
                        {goldenVersion.author_username}
                      </Text>
                    </Space>
                  </Tooltip>
                  
                  <Tooltip title={`Golden since: ${formatDate(goldenVersion.status_changed_at || goldenVersion.created_at)}`}>
                    <Space size={4}>
                      <CalendarOutlined style={{ fontSize: '12px', color: '#8c6e00' }} />
                      <Text style={{ fontSize: '13px', color: '#8c6e00' }}>
                        Golden {getStatusChangedText()}
                      </Text>
                    </Space>
                  </Tooltip>
                  
                  <Tooltip title={`File size: ${formatFileSize(goldenVersion.file_size)}`}>
                    <Space size={4}>
                      <FileOutlined style={{ fontSize: '12px', color: '#8c6e00' }} />
                      <Text style={{ fontSize: '13px', color: '#8c6e00' }}>
                        {formatFileSize(goldenVersion.file_size)}
                      </Text>
                    </Space>
                  </Tooltip>
                </Space>
              </div>

              {/* Notes */}
              {goldenVersion.notes && (
                <div style={{ 
                  background: 'rgba(250, 219, 20, 0.1)', 
                  padding: '12px',
                  borderRadius: '6px',
                  border: '1px solid rgba(250, 219, 20, 0.3)'
                }}>
                  <Text 
                    style={{ 
                      fontSize: '13px', 
                      fontStyle: 'italic',
                      color: '#614700',
                      lineHeight: '1.5'
                    }}
                  >
                    {goldenVersion.notes}
                  </Text>
                </div>
              )}

              {/* Golden Badge Info */}
              <div style={{
                background: 'rgba(250, 140, 22, 0.1)',
                padding: '8px 12px',
                borderRadius: '6px',
                border: '1px solid rgba(250, 140, 22, 0.3)'
              }}>
                <Text style={{ fontSize: '12px', color: '#8c6e00' }}>
                  <TrophyOutlined style={{ marginRight: '4px' }} />
                  This is the official master version for disaster recovery operations
                </Text>
              </div>
            </Space>
          </Col>
          
          <Col flex="none">
            <Space direction="vertical" size={8}>
              {onViewDetails && (
                <Button
                  type="primary"
                  icon={<EyeOutlined />}
                  onClick={() => onViewDetails(goldenVersion)}
                  style={{
                    background: '#fa8c16',
                    borderColor: '#fa8c16',
                    minWidth: '120px'
                  }}
                >
                  View Details
                </Button>
              )}
              
              {onViewHistory && (
                <Button
                  icon={<HistoryOutlined />}
                  onClick={() => onViewHistory(goldenVersion)}
                  style={{
                    borderColor: '#fa8c16',
                    color: '#fa8c16',
                    minWidth: '120px'
                  }}
                >
                  View History
                </Button>
              )}
            </Space>
          </Col>
        </Row>
      </Card>
    </Badge.Ribbon>
  );
};

export default GoldenVersionIndicator;