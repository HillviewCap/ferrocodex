import React from 'react';
import {
  Card,
  Typography,
  Space,
  Tag,
  Tooltip,
  Row,
  Col,
  Divider
} from 'antd';
import {
  UserOutlined,
  CalendarOutlined,
  FileOutlined,
  NumberOutlined,
  CommentOutlined
} from '@ant-design/icons';
import { ConfigurationVersionInfo, formatVersion, formatFileSize } from '../types/assets';

const { Text } = Typography;

interface VersionCardProps {
  version: ConfigurationVersionInfo;
}

const VersionCard: React.FC<VersionCardProps> = ({ version }) => {
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

  const getVersionColor = (versionNumber: string) => {
    const versionNum = parseInt(versionNumber.replace('v', ''));
    if (versionNum === 1) return 'green';
    if (versionNum <= 5) return 'blue';
    if (versionNum <= 10) return 'orange';
    return 'purple';
  };

  return (
    <Card 
      size="small" 
      style={{ 
        width: '100%',
        marginBottom: '0',
        borderRadius: '8px',
        boxShadow: '0 2px 4px rgba(0,0,0,0.1)'
      }}
      styles={{ body: { padding: '16px' } }}
    >
      <Row gutter={[16, 8]} align="middle">
        <Col flex="auto">
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px', marginBottom: '8px' }}>
            <div style={{ fontSize: '20px' }}>
              {getFileTypeIcon(version.file_name)}
            </div>
            <div>
              <Space size={8}>
                <Tag color={getVersionColor(version.version_number)}>
                  {formatVersion(version.version_number)}
                </Tag>
                <Text strong style={{ fontSize: '16px' }}>
                  {version.file_name}
                </Text>
              </Space>
              <div style={{ marginTop: '4px' }}>
                <Space size={12} wrap>
                  <Space size={4}>
                    <UserOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                    <Text type="secondary" style={{ fontSize: '12px' }}>
                      {version.author_username}
                    </Text>
                  </Space>
                  <Space size={4}>
                    <CalendarOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                    <Tooltip title={formatDate(version.created_at)}>
                      <Text type="secondary" style={{ fontSize: '12px' }}>
                        {formatRelativeTime(version.created_at)}
                      </Text>
                    </Tooltip>
                  </Space>
                  <Space size={4}>
                    <FileOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                    <Text type="secondary" style={{ fontSize: '12px' }}>
                      {formatFileSize(version.file_size)}
                    </Text>
                  </Space>
                  <Space size={4}>
                    <NumberOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                    <Tooltip title={`Content Hash: ${version.content_hash}`}>
                      <Text type="secondary" style={{ fontSize: '12px' }}>
                        {version.content_hash.substring(0, 8)}...
                      </Text>
                    </Tooltip>
                  </Space>
                </Space>
              </div>
            </div>
          </div>
          
          {version.notes && (
            <>
              <Divider style={{ margin: '8px 0' }} />
              <div style={{ display: 'flex', alignItems: 'flex-start', gap: '8px' }}>
                <CommentOutlined style={{ fontSize: '12px', color: '#8c8c8c', marginTop: '2px' }} />
                <Text 
                  type="secondary" 
                  style={{ 
                    fontSize: '13px', 
                    lineHeight: '1.5',
                    fontStyle: 'italic',
                    flex: 1
                  }}
                >
                  {version.notes}
                </Text>
              </div>
            </>
          )}
        </Col>
      </Row>
    </Card>
  );
};

export default VersionCard;