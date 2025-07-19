import React from 'react';
import {
  Card,
  Typography,
  Space,
  Tag,
  Avatar,
  Button,
  Tooltip,
  Checkbox,
  Divider,
  Row,
  Col
} from 'antd';
import {
  FileOutlined,
  CalendarOutlined,
  UserOutlined,
  EyeOutlined,
  DownloadOutlined,
  CrownOutlined,
  TagOutlined,
  FileTextOutlined,
  ClockCircleOutlined
} from '@ant-design/icons';
import { BranchVersionInfo } from '../types/branches';

const { Text } = Typography;

interface BranchVersionCardProps {
  version: BranchVersionInfo;
  isFirst?: boolean;
  isLast?: boolean;
  isSelected?: boolean;
  onSelect?: (selected: boolean) => void;
  selectionMode?: boolean;
  onView?: () => void;
  onDownload?: () => void;
}

const BranchVersionCard: React.FC<BranchVersionCardProps> = ({
  version,
  isFirst = false,
  isLast = false,
  isSelected = false,
  onSelect,
  selectionMode = false,
  onView,
  onDownload
}) => {
  // Suppress unused variable warning for isFirst since it may be used in future
  void isFirst;
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

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 Bytes';
    
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };

  const getVersionColor = (versionNumber: string) => {
    if (versionNumber.includes('branch-v1')) return 'green';
    if (versionNumber.includes('branch-v')) {
      const num = parseInt(versionNumber.replace('branch-v', ''));
      if (num <= 3) return 'blue';
      if (num <= 6) return 'orange';
      return 'purple';
    }
    return 'default';
  };

  const getBorderColor = () => {
    if (version.is_branch_latest) return '#52c41a';
    if (isSelected) return '#1890ff';
    return '#d9d9d9';
  };

  const getBackgroundColor = () => {
    if (version.is_branch_latest) return '#f6ffed';
    if (isSelected) return '#e6f7ff';
    return '#ffffff';
  };

  return (
    <div style={{ position: 'relative', marginBottom: '16px' }}>
      {/* Timeline connector */}
      {!isLast && (
        <div
          style={{
            position: 'absolute',
            left: '20px',
            top: '60px',
            bottom: '-16px',
            width: '2px',
            backgroundColor: '#e8e8e8',
            zIndex: 0
          }}
        />
      )}

      <Card
        size="small"
        style={{
          border: `2px solid ${getBorderColor()}`,
          backgroundColor: getBackgroundColor(),
          position: 'relative',
          zIndex: 1
        }}
        bodyStyle={{ padding: '12px 16px' }}
      >
        <Row align="middle" gutter={16}>
          <Col flex="none">
            {/* Timeline dot */}
            <div
              style={{
                position: 'absolute',
                left: '12px',
                top: '24px',
                width: '16px',
                height: '16px',
                borderRadius: '50%',
                backgroundColor: version.is_branch_latest ? '#52c41a' : '#1890ff',
                border: '3px solid white',
                boxShadow: '0 0 0 2px #e8e8e8',
                zIndex: 2
              }}
            />
            
            {/* Selection checkbox */}
            {selectionMode && (
              <div style={{ marginLeft: '32px' }}>
                <Checkbox
                  checked={isSelected}
                  onChange={(e) => onSelect?.(e.target.checked)}
                />
              </div>
            )}
            
            {/* Version icon */}
            <div style={{ marginLeft: selectionMode ? '16px' : '32px' }}>
              <Avatar
                icon={<FileOutlined />}
                size={40}
                style={{
                  backgroundColor: version.is_branch_latest ? '#52c41a' : '#1890ff'
                }}
              />
            </div>
          </Col>

          <Col flex="auto">
            <div>
              {/* Version header */}
              <div style={{ marginBottom: '8px' }}>
                <Space size={8} wrap>
                  <Tag 
                    color={getVersionColor(version.branch_version_number)}
                    icon={<TagOutlined />}
                  >
                    {version.branch_version_number}
                  </Tag>
                  
                  {version.is_branch_latest && (
                    <Tag color="gold" icon={<CrownOutlined />}>
                      Latest
                    </Tag>
                  )}
                  
                  <Text strong style={{ fontSize: '14px' }}>
                    {version.file_name}
                  </Text>
                </Space>
              </div>

              {/* Version metadata */}
              <div style={{ marginBottom: '8px' }}>
                <Row gutter={[16, 4]}>
                  <Col span={12}>
                    <Space size={4}>
                      <UserOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                      <Text type="secondary" style={{ fontSize: '12px' }}>
                        {version.author_username}
                      </Text>
                    </Space>
                  </Col>
                  <Col span={12}>
                    <Space size={4}>
                      <CalendarOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                      <Text type="secondary" style={{ fontSize: '12px' }}>
                        {formatDate(version.created_at)}
                      </Text>
                    </Space>
                  </Col>
                  <Col span={12}>
                    <Space size={4}>
                      <FileOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                      <Text type="secondary" style={{ fontSize: '12px' }}>
                        {formatFileSize(version.file_size)}
                      </Text>
                    </Space>
                  </Col>
                  <Col span={12}>
                    <Space size={4}>
                      <ClockCircleOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                      <Text type="secondary" style={{ fontSize: '12px' }}>
                        Main: {version.version_number}
                      </Text>
                    </Space>
                  </Col>
                </Row>
              </div>

              {/* Version notes */}
              {version.notes && (
                <>
                  <Divider style={{ margin: '8px 0' }} />
                  <div style={{ marginBottom: '8px' }}>
                    <Space size={4}>
                      <FileTextOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                      <Text type="secondary" style={{ fontSize: '12px', fontStyle: 'italic' }}>
                        {version.notes.length > 100 
                          ? `${version.notes.substring(0, 100)}...` 
                          : version.notes
                        }
                      </Text>
                    </Space>
                  </div>
                </>
              )}
            </div>
          </Col>

          <Col flex="none">
            <Space>
              <Tooltip title="View Version Details">
                <Button
                  type="text"
                  icon={<EyeOutlined />}
                  size="small"
                  onClick={onView}
                />
              </Tooltip>
              
              <Tooltip title="Download Version">
                <Button
                  type="text"
                  icon={<DownloadOutlined />}
                  size="small"
                  onClick={onDownload}
                />
              </Tooltip>
            </Space>
          </Col>
        </Row>
      </Card>
    </div>
  );
};

export default BranchVersionCard;