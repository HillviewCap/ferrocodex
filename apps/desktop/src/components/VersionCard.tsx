import React, { useState } from 'react';
import {
  Card,
  Typography,
  Space,
  Tag,
  Tooltip,
  Row,
  Col,
  Divider,
  Button,
  Dropdown,
  MenuProps
} from 'antd';
import {
  UserOutlined,
  CalendarOutlined,
  FileOutlined,
  NumberOutlined,
  CommentOutlined,
  BranchesOutlined,
  MoreOutlined,
  EditOutlined,
  HistoryOutlined,
  TrophyOutlined,
  DownloadOutlined,
  InboxOutlined,
  UndoOutlined,
  RocketOutlined
} from '@ant-design/icons';
import { ConfigurationVersionInfo, formatVersion, formatFileSize } from '../types/assets';
import ConfigurationStatusBadge from './ConfigurationStatusBadge';
import ChangeStatusModal from './ChangeStatusModal';
import StatusHistoryModal from './StatusHistoryModal';
import PromoteToGoldenWizard from './PromoteToGoldenWizard';
import ExportConfirmationModal from './ExportConfirmationModal';
import ArchiveConfirmationModal from './ArchiveConfirmationModal';
import RestoreConfirmationModal from './RestoreConfirmationModal';
import FirmwareSelector from './FirmwareSelector';
import ExportRecoveryPackageModal from './ExportRecoveryPackageModal';

const { Text } = Typography;

interface VersionCardProps {
  version: ConfigurationVersionInfo;
  onCreateBranch?: (version: ConfigurationVersionInfo) => void;
  showCreateBranch?: boolean;
  onStatusChange?: () => void;
  token?: string;
  canChangeStatus?: boolean;
  canPromoteToGolden?: boolean;
  onGoldenPromotion?: () => void;
  canExport?: boolean;
  onExport?: (exportPath: string) => void;
  canArchive?: boolean;
  canRestore?: boolean;
  canLinkFirmware?: boolean;
  onFirmwareLinked?: () => void;
  onFirmwareUnlinked?: () => void;
}

const VersionCard: React.FC<VersionCardProps> = React.memo(({ 
  version, 
  onCreateBranch, 
  showCreateBranch = false,
  onStatusChange,
  token,
  canChangeStatus = false,
  canPromoteToGolden = false,
  onGoldenPromotion,
  canExport = false,
  onExport,
  canArchive = false,
  canRestore = false,
  canLinkFirmware = false,
  onFirmwareLinked,
  onFirmwareUnlinked
}) => {
  const [showChangeStatusModal, setShowChangeStatusModal] = useState(false);
  const [showStatusHistoryModal, setShowStatusHistoryModal] = useState(false);
  const [showGoldenPromotionWizard, setShowGoldenPromotionWizard] = useState(false);
  const [showExportModal, setShowExportModal] = useState(false);
  const [showArchiveModal, setShowArchiveModal] = useState(false);
  const [showRestoreModal, setShowRestoreModal] = useState(false);
  const [showRecoveryPackageModal, setShowRecoveryPackageModal] = useState(false);
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

  const handleCreateBranch = () => {
    if (onCreateBranch) {
      onCreateBranch(version);
    }
  };

  const handleChangeStatus = () => {
    setShowChangeStatusModal(true);
  };

  const handleStatusChanged = () => {
    setShowChangeStatusModal(false);
    if (onStatusChange) {
      onStatusChange();
    }
  };

  const handleViewHistory = () => {
    setShowStatusHistoryModal(true);
  };

  const handlePromoteToGolden = () => {
    setShowGoldenPromotionWizard(true);
  };

  const handleGoldenPromotionSuccess = () => {
    setShowGoldenPromotionWizard(false);
    if (onGoldenPromotion) {
      onGoldenPromotion();
    }
    if (onStatusChange) {
      onStatusChange();
    }
  };

  const handleExport = () => {
    setShowExportModal(true);
  };

  const handleExportSuccess = (exportPath: string) => {
    setShowExportModal(false);
    if (onExport) {
      onExport(exportPath);
    }
  };

  const handleArchive = () => {
    setShowArchiveModal(true);
  };

  const handleArchiveSuccess = () => {
    setShowArchiveModal(false);
    if (onStatusChange) {
      onStatusChange();
    }
  };

  const handleRestore = () => {
    setShowRestoreModal(true);
  };

  const handleRestoreSuccess = () => {
    setShowRestoreModal(false);
    if (onStatusChange) {
      onStatusChange();
    }
  };

  const isArchived = version.status === 'Archived';
  
  const canShowPromoteToGolden = canPromoteToGolden && 
    token && 
    version.status === 'Approved';

  const statusMenuItems: MenuProps['items'] = [
    ...(canExport && token ? [
      {
        key: 'export',
        label: 'Export',
        icon: <DownloadOutlined />,
        onClick: handleExport
      }
    ] : []),
    ...(canExport && token && version.firmware_version_id ? [
      {
        key: 'export-recovery',
        label: 'Export Recovery Package',
        icon: <RocketOutlined />,
        onClick: () => setShowRecoveryPackageModal(true)
      }
    ] : []),
    ...(canShowPromoteToGolden ? [
      {
        key: 'promote-to-golden',
        label: 'Promote to Golden',
        icon: <TrophyOutlined />,
        onClick: handlePromoteToGolden
      }
    ] : []),
    ...(canChangeStatus && token && !isArchived ? [
      {
        key: 'change-status',
        label: 'Change Status',
        icon: <EditOutlined />,
        onClick: handleChangeStatus
      }
    ] : []),
    ...(canArchive && token && !isArchived ? [
      {
        key: 'archive',
        label: 'Archive Version',
        icon: <InboxOutlined />,
        onClick: handleArchive,
        danger: true
      }
    ] : []),
    ...(canRestore && token && isArchived ? [
      {
        key: 'restore',
        label: 'Restore Version',
        icon: <UndoOutlined />,
        onClick: handleRestore
      }
    ] : []),
    {
      key: 'view-history',
      label: 'View Status History',
      icon: <HistoryOutlined />,
      onClick: handleViewHistory
    }
  ];

  return (
    <Card 
      size="small" 
      style={{ 
        width: '100%',
        marginBottom: '0',
        borderRadius: '8px',
        boxShadow: '0 2px 4px rgba(0,0,0,0.1)',
        opacity: isArchived ? 0.7 : 1,
        backgroundColor: isArchived ? '#fafafa' : undefined
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
                <ConfigurationStatusBadge 
                  status={version.status}
                  onClick={canChangeStatus && token ? handleChangeStatus : undefined}
                />
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
          
          {/* Firmware Linking Section */}
          {canLinkFirmware && token && !isArchived && (
            <>
              <Divider style={{ margin: '8px 0' }} />
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <RocketOutlined style={{ fontSize: '14px', color: '#1890ff' }} />
                <Text style={{ fontSize: '13px', marginRight: '8px' }}>
                  Firmware:
                </Text>
                <FirmwareSelector
                  assetId={version.asset_id}
                  configId={version.id}
                  currentFirmwareId={version.firmware_version_id}
                  token={token}
                  onLink={(firmwareId) => {
                    if (onFirmwareLinked) onFirmwareLinked();
                  }}
                  onUnlink={() => {
                    if (onFirmwareUnlinked) onFirmwareUnlinked();
                  }}
                  disabled={isArchived}
                />
              </div>
            </>
          )}
        </Col>
        
        <Col flex="none">
          <Space>
            {statusMenuItems.length > 0 && (
              <Dropdown
                menu={{ items: statusMenuItems }}
                placement="bottomRight"
                trigger={['click']}
              >
                <Button 
                  size="small" 
                  icon={<MoreOutlined />}
                  style={{ minWidth: 'auto' }}
                  title="Status actions"
                />
              </Dropdown>
            )}
            
            {showCreateBranch && onCreateBranch && !isArchived && (
              <Button 
                type="primary" 
                size="small" 
                icon={<BranchesOutlined />}
                onClick={handleCreateBranch}
                style={{ minWidth: '120px' }}
              >
                Create Branch
              </Button>
            )}
          </Space>
        </Col>
      </Row>
      
      {/* Status Management Modals */}
      {token && (
        <>
          <ChangeStatusModal
            visible={showChangeStatusModal}
            onCancel={() => setShowChangeStatusModal(false)}
            onSuccess={handleStatusChanged}
            version={version}
            token={token}
          />
          
          <StatusHistoryModal
            visible={showStatusHistoryModal}
            onCancel={() => setShowStatusHistoryModal(false)}
            version={version}
            token={token}
          />
          
          <PromoteToGoldenWizard
            visible={showGoldenPromotionWizard}
            onCancel={() => setShowGoldenPromotionWizard(false)}
            onSuccess={handleGoldenPromotionSuccess}
            version={version}
            token={token}
          />
          
          <ExportConfirmationModal
            visible={showExportModal}
            onCancel={() => setShowExportModal(false)}
            onSuccess={handleExportSuccess}
            version={version}
            token={token}
          />
          
          <ArchiveConfirmationModal
            visible={showArchiveModal}
            onCancel={() => setShowArchiveModal(false)}
            onSuccess={handleArchiveSuccess}
            version={version}
            token={token}
          />
          
          <RestoreConfirmationModal
            visible={showRestoreModal}
            onCancel={() => setShowRestoreModal(false)}
            onSuccess={handleRestoreSuccess}
            version={version}
            token={token}
          />
          
          <ExportRecoveryPackageModal
            visible={showRecoveryPackageModal}
            onCancel={() => setShowRecoveryPackageModal(false)}
            onSuccess={(manifestPath) => {
              setShowRecoveryPackageModal(false);
              message.success(`Recovery package exported successfully!`);
              if (onExport) {
                onExport(manifestPath);
              }
            }}
            assetId={version.asset_id}
            assetName="" // Will be filled by the store if needed
            configuration={version}
            linkedFirmwareId={version.firmware_version_id}
          />
        </>
      )}
    </Card>
  );
});

VersionCard.displayName = 'VersionCard';

export default VersionCard;