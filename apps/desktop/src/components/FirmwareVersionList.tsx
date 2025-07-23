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
  Empty,
  Input,
  Collapse
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
  LinkOutlined,
  EditOutlined,
  SwapOutlined,
  CrownOutlined,
  HistoryOutlined
} from '@ant-design/icons';
import { FirmwareVersionInfo, FirmwareStatus, formatFirmwareFileSize, formatFirmwareHash, sortFirmwareVersions } from '../types/firmware';
import useAuthStore from '../store/auth';
import useFirmwareStore from '../store/firmware';
import FirmwareAnalysis from './firmware/FirmwareAnalysis';
import LinkedConfigurationsList from './LinkedConfigurationsList';
import FirmwareHistoryTimeline from './firmware/FirmwareHistoryTimeline';
import FirmwareStatusDialog from './firmware/FirmwareStatusDialog';
import { canChangeFirmwareStatus, canPromoteFirmwareToGolden, canUpdateFirmwareNotes } from '../utils/roleUtils';

const { Text } = Typography;
const { Panel } = Collapse;
const { TextArea } = Input;

interface FirmwareVersionListProps {
  versions: FirmwareVersionInfo[];
  onDelete?: () => void;
}

const FirmwareVersionList: React.FC<FirmwareVersionListProps> = ({ 
  versions, 
  onDelete 
}) => {
  const { user } = useAuthStore();
  const { deleteFirmware, updateFirmwareStatus, getAvailableStatusTransitions, promoteFirmwareToGolden, updateFirmwareNotes } = useFirmwareStore();
  const [deletingId, setDeletingId] = useState<number | null>(null);
  const [analysisModalVisible, setAnalysisModalVisible] = useState(false);
  const [selectedFirmwareId, setSelectedFirmwareId] = useState<number | null>(null);
  const [statusDialogVisible, setStatusDialogVisible] = useState(false);
  const [selectedFirmwareForStatus, setSelectedFirmwareForStatus] = useState<FirmwareVersionInfo | null>(null);
  const [availableTransitions, setAvailableTransitions] = useState<FirmwareStatus[]>([]);
  const [editingNotesId, setEditingNotesId] = useState<number | null>(null);
  const [tempNotes, setTempNotes] = useState<string>('');
  const [expandedHistory, setExpandedHistory] = useState<string[]>([]);
  
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
      case 'Approved':
        return 'green';
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

  const handleStatusChange = async (firmware: FirmwareVersionInfo) => {
    try {
      const transitions = await getAvailableStatusTransitions(firmware.id);
      setAvailableTransitions(transitions);
      setSelectedFirmwareForStatus(firmware);
      setStatusDialogVisible(true);
    } catch (error) {
      message.error(`Failed to get available status transitions: ${error}`);
    }
  };

  const handleStatusUpdate = async (newStatus: FirmwareStatus, reason?: string) => {
    if (!selectedFirmwareForStatus) return;

    try {
      if (newStatus === 'Golden') {
        await promoteFirmwareToGolden(selectedFirmwareForStatus.id, reason || '');
      } else {
        await updateFirmwareStatus(selectedFirmwareForStatus.id, newStatus, reason);
      }
      
      message.success(`Firmware status updated to ${newStatus}`);
      setStatusDialogVisible(false);
      setSelectedFirmwareForStatus(null);
      
      if (onDelete) {
        onDelete(); // Refresh the list
      }
    } catch (error) {
      message.error(`Failed to update firmware status: ${error}`);
    }
  };

  const handleNotesEdit = (firmware: FirmwareVersionInfo) => {
    setEditingNotesId(firmware.id);
    setTempNotes(firmware.notes || '');
  };

  const handleNotesSave = async (firmware: FirmwareVersionInfo) => {
    try {
      await updateFirmwareNotes(firmware.id, tempNotes);
      message.success('Notes updated successfully');
      setEditingNotesId(null);
      setTempNotes('');
    } catch (error) {
      message.error(`Failed to update notes: ${error}`);
    }
  };

  const handleNotesCancel = () => {
    setEditingNotesId(null);
    setTempNotes('');
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

    // Status change option based on permissions
    if (canChangeFirmwareStatus(user)) {
      menuItems.push({
        key: 'status',
        label: 'Change Status',
        icon: <SwapOutlined />,
        onClick: () => handleStatusChange(firmware)
      });
    }

    // History option available to all users
    menuItems.push({
      key: 'history',
      label: 'View History',
      icon: <HistoryOutlined />,
      onClick: () => {
        setExpandedHistory(prev => 
          prev.includes(`history-${firmware.id}`)
            ? prev.filter(id => id !== `history-${firmware.id}`)
            : [...prev, `history-${firmware.id}`]
        );
      }
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
      <>
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
                    <Tag color={getStatusColor(firmware.status)}>
                      {firmware.status === 'Golden' && <CrownOutlined style={{ marginRight: '4px' }} />}
                      {firmware.status}
                    </Tag>
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

              {(firmware.notes || canUpdateFirmwareNotes(user)) && (
                <>
                  <Divider style={{ margin: '8px 0' }} />
                  {editingNotesId === firmware.id ? (
                    <div style={{ width: '100%' }}>
                      <TextArea
                        value={tempNotes}
                        onChange={(e) => setTempNotes(e.target.value)}
                        rows={3}
                        maxLength={1000}
                        showCount
                        style={{ marginBottom: '8px' }}
                      />
                      <Space>
                        <Button size="small" type="primary" onClick={() => handleNotesSave(firmware)}>
                          Save
                        </Button>
                        <Button size="small" onClick={handleNotesCancel}>
                          Cancel
                        </Button>
                      </Space>
                    </div>
                  ) : (
                    <Space size={4} align="start" style={{ width: '100%' }}>
                      <CommentOutlined style={{ color: '#8c8c8c', marginTop: '2px' }} />
                      <div style={{ flex: 1 }}>
                        <Text type="secondary" style={{ fontSize: '13px' }}>
                          {firmware.notes || 'No notes'}
                        </Text>
                        {canUpdateFirmwareNotes(user) && (
                          <Button
                            type="link"
                            size="small"
                            icon={<EditOutlined />}
                            onClick={() => handleNotesEdit(firmware)}
                            style={{ marginLeft: '8px', padding: '0 4px' }}
                          >
                            Edit
                          </Button>
                        )}
                      </div>
                    </Space>
                  )}
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
      
      {expandedHistory.includes(`history-${firmware.id}`) && (
        <Card 
          style={{ 
            marginTop: '-16px', 
            marginBottom: '16px',
            borderTop: 'none',
            borderTopLeftRadius: 0,
            borderTopRightRadius: 0
          }}
          size="small"
        >
          <Collapse 
            activeKey={[`history-${firmware.id}`]}
            ghost
            items={[{
              key: `history-${firmware.id}`,
              label: (
                <Space>
                  <HistoryOutlined />
                  <Text strong>Status History</Text>
                </Space>
              ),
              children: (
                <FirmwareHistoryTimeline 
                  firmwareVersionId={firmware.id}
                  onRefresh={onDelete}
                />
              )
            }]}
          />
        </Card>
      )}
      </>
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
      
      {selectedFirmwareForStatus && (
        <FirmwareStatusDialog
          visible={statusDialogVisible}
          currentStatus={selectedFirmwareForStatus.status}
          availableTransitions={availableTransitions}
          onConfirm={handleStatusUpdate}
          onCancel={() => {
            setStatusDialogVisible(false);
            setSelectedFirmwareForStatus(null);
          }}
          isPromotingToGolden={availableTransitions.includes('Golden')}
        />
      )}
    </>
  );
};

export default FirmwareVersionList;