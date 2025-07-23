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
  Modal,
  message,
  Empty
} from 'antd';
import {
  UserOutlined,
  CalendarOutlined,
  FileOutlined,
  CommentOutlined,
  MoreOutlined,
  DeleteOutlined,
  LockOutlined,
  CloudServerOutlined,
  BarcodeOutlined,
  FileSearchOutlined,
  LinkOutlined
} from '@ant-design/icons';
import { FirmwareVersionInfo, formatFirmwareFileSize, formatFirmwareHash, sortFirmwareVersions } from '../types/firmware';
import useAuthStore from '../store/auth';
import useFirmwareStore from '../store/firmware';
import FirmwareAnalysis from './firmware/FirmwareAnalysis';
import LinkedConfigurationsList from './LinkedConfigurationsList';

const { Text } = Typography;

interface FirmwareVersionListProps {
  versions: FirmwareVersionInfo[];
  onDelete?: () => void;
}

const FirmwareVersionList: React.FC<FirmwareVersionListProps> = ({ 
  versions, 
  onDelete 
}) => {
  const { user } = useAuthStore();
  const { deleteFirmware } = useFirmwareStore();
  const [deletingId, setDeletingId] = useState<number | null>(null);
  const [analysisModalVisible, setAnalysisModalVisible] = useState(false);
  const [selectedFirmwareId, setSelectedFirmwareId] = useState<number | null>(null);
  
  const isEngineer = user?.role === 'Engineer';
  const sortedVersions = sortFirmwareVersions(versions);

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

  const handleDelete = async (firmware: FirmwareVersionInfo) => {
    Modal.confirm({
      title: 'Delete Firmware?',
      content: (
        <div>
          <p>Are you sure you want to delete this firmware version?</p>
          <p><strong>Version:</strong> {firmware.version}</p>
          {firmware.vendor && <p><strong>Vendor:</strong> {firmware.vendor}</p>}
          {firmware.model && <p><strong>Model:</strong> {firmware.model}</p>}
          <p style={{ marginTop: '10px', color: '#ff4d4f' }}>
            This action cannot be undone. The firmware file will be permanently deleted.
          </p>
        </div>
      ),
      okText: 'Delete',
      okType: 'danger',
      cancelText: 'Cancel',
      onOk: async () => {
        try {
          setDeletingId(firmware.id);
          await deleteFirmware(firmware.id);
          message.success('Firmware deleted successfully');
          if (onDelete) {
            onDelete();
          }
        } catch (error) {
          console.error('Failed to delete firmware:', error);
          message.error(`Failed to delete firmware: ${error instanceof Error ? error.message : String(error)}`);
        } finally {
          setDeletingId(null);
        }
      }
    });
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'Golden':
        return 'gold';
      case 'Draft':
        return 'default';
      case 'Archived':
        return 'default';
      default:
        return 'default';
    }
  };

  const showAnalysis = (firmwareId: number) => {
    setSelectedFirmwareId(firmwareId);
    setAnalysisModalVisible(true);
  };

  const FirmwareCard: React.FC<{ firmware: FirmwareVersionInfo }> = ({ firmware }) => {
    const menuItems = [];

    // Analysis option available to all users
    menuItems.push({
      key: 'analysis',
      label: 'View Analysis',
      icon: <FileSearchOutlined />,
      onClick: () => showAnalysis(firmware.id)
    });

    if (isEngineer) {
      menuItems.push({
        key: 'delete',
        label: 'Delete',
        icon: <DeleteOutlined />,
        danger: true,
        onClick: () => handleDelete(firmware)
      });
    }

    return (
      <Card 
        style={{ marginBottom: '16px' }}
        loading={deletingId === firmware.id}
      >
        <Row gutter={[16, 8]}>
          <Col xs={24} sm={16}>
            <Space direction="vertical" size="small" style={{ width: '100%' }}>
              <Space align="center" size="middle">
                <CloudServerOutlined style={{ fontSize: '24px', color: '#1890ff' }} />
                <div>
                  <Text strong style={{ fontSize: '16px' }}>
                    {firmware.vendor || 'Generic'} {firmware.model ? `- ${firmware.model}` : ''}
                  </Text>
                  <div>
                    <Tag color="blue">{firmware.version}</Tag>
                    <Tag color={getStatusColor(firmware.status)}>{firmware.status}</Tag>
                  </div>
                </div>
              </Space>
              
              <Divider style={{ margin: '8px 0' }} />
              
              <Space wrap size="small">
                <Tooltip title="File Size">
                  <Space size={4}>
                    <FileOutlined style={{ color: '#8c8c8c' }} />
                    <Text type="secondary">{formatFirmwareFileSize(firmware.file_size)}</Text>
                  </Space>
                </Tooltip>
                
                <Tooltip title="File Hash (SHA-256)">
                  <Space size={4}>
                    <BarcodeOutlined style={{ color: '#8c8c8c' }} />
                    <Text type="secondary" copyable={{ text: firmware.file_hash }}>
                      {formatFirmwareHash(firmware.file_hash)}
                    </Text>
                  </Space>
                </Tooltip>

                <Tooltip title="Encrypted Storage">
                  <Space size={4}>
                    <LockOutlined style={{ color: '#52c41a' }} />
                    <Text type="secondary" style={{ color: '#52c41a' }}>Encrypted</Text>
                  </Space>
                </Tooltip>
              </Space>
              
              <Divider style={{ margin: '8px 0' }} />
              
              <LinkedConfigurationsList 
                firmwareId={firmware.id}
                assetId={firmware.asset_id}
              />

              {firmware.notes && (
                <>
                  <Divider style={{ margin: '8px 0' }} />
                  <Space size={4}>
                    <CommentOutlined style={{ color: '#8c8c8c' }} />
                    <Text type="secondary" style={{ fontSize: '13px' }}>
                      {firmware.notes}
                    </Text>
                  </Space>
                </>
              )}
            </Space>
          </Col>
          
          <Col xs={24} sm={8}>
            <div style={{ 
              display: 'flex', 
              flexDirection: 'column', 
              alignItems: 'flex-end',
              justifyContent: 'space-between',
              height: '100%'
            }}>
              <Space direction="vertical" align="end" size="small">
                <Tooltip title={formatDate(firmware.created_at)}>
                  <Space size={4}>
                    <CalendarOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                    <Text type="secondary" style={{ fontSize: '12px' }}>
                      {formatRelativeTime(firmware.created_at)}
                    </Text>
                  </Space>
                </Tooltip>
                
                <Space size={4}>
                  <UserOutlined style={{ fontSize: '12px', color: '#8c8c8c' }} />
                  <Text type="secondary" style={{ fontSize: '12px' }}>
                    {firmware.author_username}
                  </Text>
                </Space>
              </Space>

              {menuItems.length > 0 && (
                <Dropdown
                  menu={{ items: menuItems }}
                  placement="bottomRight"
                  trigger={['click']}
                >
                  <Button 
                    type="text" 
                    icon={<MoreOutlined />}
                    style={{ marginTop: '8px' }}
                  />
                </Dropdown>
              )}
            </div>
          </Col>
        </Row>
      </Card>
    );
  };

  if (sortedVersions.length === 0) {
    return (
      <Empty
        image={Empty.PRESENTED_IMAGE_SIMPLE}
        description="No firmware versions available"
      />
    );
  }

  return (
    <>
      <div>
        {sortedVersions.map(firmware => (
          <FirmwareCard key={firmware.id} firmware={firmware} />
        ))}
      </div>
      
      <Modal
        title="Firmware Analysis"
        open={analysisModalVisible}
        onCancel={() => {
          setAnalysisModalVisible(false);
          setSelectedFirmwareId(null);
        }}
        footer={null}
        width={800}
        destroyOnClose
      >
        {selectedFirmwareId && (
          <FirmwareAnalysis firmwareId={selectedFirmwareId} />
        )}
      </Modal>
    </>
  );
};

export default FirmwareVersionList;