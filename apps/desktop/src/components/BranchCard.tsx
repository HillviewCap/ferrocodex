import React from 'react';
import {
  Card,
  Typography,
  Space,
  Tag,
  Tooltip,
  Row,
  Col,
  Divider,
  Avatar,
  Button
} from 'antd';
import {
  BranchesOutlined,
  UserOutlined,
  CalendarOutlined,
  CommentOutlined,
  LinkOutlined,
  EyeOutlined,
  HistoryOutlined,
  TagOutlined,
  ImportOutlined,
  ExportOutlined,
  WarningOutlined,
  TrophyOutlined
} from '@ant-design/icons';
import { BranchInfo, getBranchStatusColor, getBranchStatusText } from '../types/branches';

const { Text } = Typography;

interface BranchCardProps {
  branch: BranchInfo;
  onViewDetails?: (branch: BranchInfo) => void;
  onSelectBranch?: (branch: BranchInfo) => void;
  onViewHistory?: (branch: BranchInfo) => void;
  onImportVersion?: (branch: BranchInfo) => void;
  onExportLatestVersion?: (branch: BranchInfo) => void;
  onPromoteToSilver?: (branch: BranchInfo) => void;
  showActions?: boolean;
  versionCount?: number;
  latestVersionNumber?: string;
}

const BranchCard: React.FC<BranchCardProps> = React.memo(({ 
  branch, 
  onViewDetails, 
  onSelectBranch, 
  onViewHistory,
  onImportVersion,
  onExportLatestVersion,
  onPromoteToSilver,
  showActions = true,
  versionCount = 0,
  latestVersionNumber
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

  const getVersionColor = (versionNumber: string) => {
    const versionNum = parseInt(versionNumber.replace('v', ''));
    if (versionNum === 1) return 'green';
    if (versionNum <= 5) return 'blue';
    if (versionNum <= 10) return 'orange';
    return 'purple';
  };

  const getBranchIcon = () => {
    return branch.is_active ? '🌿' : '🥀';
  };

  const handleViewDetails = () => {
    if (onViewDetails) {
      onViewDetails(branch);
    }
  };

  const handleSelectBranch = () => {
    if (onSelectBranch) {
      onSelectBranch(branch);
    }
  };


  const handleViewHistory = () => {
    if (onViewHistory) {
      onViewHistory(branch);
    }
  };

  const handleImportVersion = () => {
    if (onImportVersion) {
      onImportVersion(branch);
    }
  };

  const handleExportLatestVersion = () => {
    if (onExportLatestVersion) {
      onExportLatestVersion(branch);
    }
  };

  const handlePromoteToSilver = () => {
    if (onPromoteToSilver) {
      onPromoteToSilver(branch);
    }
  };

  const isParentArchived = branch.parent_version_status === 'Archived';
  
  return (
    <Card 
      size="small" 
      style={{ 
        width: '100%',
        marginBottom: '12px',
        borderRadius: '8px',
        boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
        border: `1px solid ${isParentArchived ? '#faad14' : (branch.is_active ? '#d9f7be' : '#ffccc7')}`,
        backgroundColor: isParentArchived ? '#fffbe6' : undefined
      }}
      styles={{ body: { padding: '16px' } }}
    >
      <Row gutter={[16, 8]} align="middle">
        <Col flex="auto">
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px', marginBottom: '8px' }}>
            <Avatar 
              icon={<BranchesOutlined />} 
              size={40}
              style={{ 
                backgroundColor: branch.is_active ? '#52c41a' : '#ff7875',
                fontSize: '18px'
              }}
            />
            <div style={{ flex: 1 }}>
              <div style={{ marginBottom: '4px' }}>
                <Space size={8}>
                  <Text strong style={{ fontSize: '16px' }}>
                    {getBranchIcon()} {branch.name}
                  </Text>
                  <Tag color={getBranchStatusColor(branch.is_active)}>
                    {getBranchStatusText(branch.is_active)}
                  </Tag>
                </Space>
              </div>
              <div>
                <Space size={12} wrap>
                  <Space size={4}>
                    <UserOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                    <Text type="secondary" style={{ fontSize: '12px' }}>
                      {branch.created_by_username}
                    </Text>
                  </Space>
                  <Space size={4}>
                    <CalendarOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                    <Tooltip title={formatDate(branch.created_at)}>
                      <Text type="secondary" style={{ fontSize: '12px' }}>
                        {formatRelativeTime(branch.created_at)}
                      </Text>
                    </Tooltip>
                  </Space>
                  <Space size={4}>
                    <LinkOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                    <Text type="secondary" style={{ fontSize: '12px' }}>
                      Branched from: 
                    </Text>
                    <Tag 
                      color={branch.parent_version_status === 'Archived' ? 'orange' : getVersionColor(branch.parent_version_number)} 
                      style={{ cursor: 'pointer' }}
                      title={`Parent version ${branch.parent_version_number} is ${branch.parent_version_status}`}
                    >
                      {branch.parent_version_number}
                      {branch.parent_version_status === 'Archived' && (
                        <Tooltip title="This branch was created from a version that is now archived">
                          <WarningOutlined style={{ marginLeft: '4px' }} />
                        </Tooltip>
                      )}
                    </Tag>
                  </Space>
                  {versionCount > 0 && (
                    <Space size={4}>
                      <TagOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                      <Text type="secondary" style={{ fontSize: '12px' }}>
                        {versionCount} version{versionCount !== 1 ? 's' : ''}
                      </Text>
                      {latestVersionNumber && (
                        <Tag color="blue">
                          Latest: {latestVersionNumber}
                        </Tag>
                      )}
                    </Space>
                  )}
                </Space>
              </div>
            </div>
          </div>
          
          {branch.description && (
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
                  {branch.description}
                </Text>
              </div>
            </>
          )}
        </Col>
        
        {showActions && (
          <Col flex="none">
            <Space direction="vertical" size={8}>
              {onViewDetails && (
                <Button 
                  type="text" 
                  size="small" 
                  icon={<EyeOutlined />}
                  onClick={handleViewDetails}
                  style={{ width: '100%' }}
                >
                  View Details
                </Button>
              )}
              {onImportVersion && branch.is_active && (
                <Button 
                  type="primary" 
                  size="small" 
                  icon={<ImportOutlined />}
                  onClick={handleImportVersion}
                  style={{ width: '100%' }}
                >
                  Import Version
                </Button>
              )}
              {onExportLatestVersion && branch.is_active && (
                <Button 
                  type="default" 
                  size="small" 
                  icon={<ExportOutlined />}
                  onClick={handleExportLatestVersion}
                  style={{ width: '100%' }}
                >
                  Export Latest
                </Button>
              )}
              {onPromoteToSilver && branch.is_active && versionCount > 0 && (
                <Button 
                  type="default" 
                  size="small" 
                  icon={<TrophyOutlined />}
                  onClick={handlePromoteToSilver}
                  style={{ 
                    width: '100%',
                    color: '#00CED1',
                    borderColor: '#00CED1'
                  }}
                >
                  Promote to Silver
                </Button>
              )}
              {onViewHistory && versionCount > 0 && (
                <Button 
                  type="default" 
                  size="small" 
                  icon={<HistoryOutlined />}
                  onClick={handleViewHistory}
                  style={{ width: '100%' }}
                >
                  View History
                </Button>
              )}
              {onSelectBranch && branch.is_active && (
                <Button 
                  type="text" 
                  size="small" 
                  icon={<BranchesOutlined />}
                  onClick={handleSelectBranch}
                  style={{ width: '100%' }}
                >
                  Select Branch
                </Button>
              )}
            </Space>
          </Col>
        )}
      </Row>
    </Card>
  );
});

BranchCard.displayName = 'BranchCard';

export default BranchCard;