import React, { useState, useEffect } from 'react';
import { 
  Card, 
  Table, 
  Space, 
  Typography, 
  Button, 
  Select, 
  DatePicker, 
  Input,
  Tag,
  Badge,
  Drawer,
  Descriptions,
  Alert,
  Pagination,
  Tooltip,
  Modal
} from 'antd';
import {
  SearchOutlined,
  FilterOutlined,
  DownloadOutlined,
  EyeOutlined,
  ExclamationCircleOutlined,
  CheckCircleOutlined,
  WarningOutlined,
  InfoCircleOutlined,
  ReloadOutlined
} from '@ant-design/icons';
import { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { 
  AuditEvent, 
  AuditFilter,
  AuditEventUI
} from '../../types/security';
import { invoke } from '@tauri-apps/api/core';
import useAuthStore from '../../store/auth';
import { useAuditEvents } from '../../store/security';
import useSecurityStore from '../../store/security';

const { Title, Text } = Typography;
const { RangePicker } = DatePicker;
const { Search } = Input;

interface SecurityAuditLogProps {
  pageSize?: number;
  showFilters?: boolean;
  compactView?: boolean;
  autoRefresh?: boolean;
  exportEnabled?: boolean;
}

const SecurityAuditLog: React.FC<SecurityAuditLogProps> = ({
  pageSize = 50,
  showFilters = true,
  compactView = false,
  autoRefresh = true,
  exportEnabled = true
}) => {
  const { token, user } = useAuthStore();
  const { events, totalCount } = useAuditEvents();
  const { auditFilters, setAuditFilters, addAuditEvent } = useSecurityStore();
  
  const [loading, setLoading] = useState(false);
  const [selectedEvent, setSelectedEvent] = useState<AuditEvent | null>(null);
  const [drawerVisible, setDrawerVisible] = useState(false);
  const [currentPage, setCurrentPage] = useState(1);
  const [searchText, setSearchText] = useState('');
  const [exportModalVisible, setExportModalVisible] = useState(false);

  // Fetch audit events
  const fetchAuditEvents = async (filters: AuditFilter = {}) => {
    try {
      setLoading(true);
      const result = await invoke<AuditEvent[]>('get_audit_events', {
        token,
        filter: filters
      });
      
      // Update store with fetched events
      result.forEach(event => addAuditEvent(event));
      
    } catch (error) {
      console.error('Failed to fetch audit events:', error);
    } finally {
      setLoading(false);
    }
  };

  // Auto-refresh data
  useEffect(() => {
    if (user?.role !== 'Administrator') return;
    
    fetchAuditEvents(auditFilters);
    
    if (autoRefresh) {
      const interval = setInterval(() => {
        fetchAuditEvents(auditFilters);
      }, 30000); // Refresh every 30 seconds
      
      return () => clearInterval(interval);
    }
  }, [token, user, auditFilters, autoRefresh]);

  // Handle filter changes
  const handleFilterChange = (newFilters: Partial<AuditFilter>) => {
    const updatedFilters = { ...auditFilters, ...newFilters };
    setAuditFilters(updatedFilters);
    setCurrentPage(1);
    fetchAuditEvents(updatedFilters);
  };

  // Handle search
  const handleSearch = (value: string) => {
    setSearchText(value);
    setCurrentPage(1);
  };

  // Handle export
  const handleExport = async (format: 'csv' | 'json' | 'pdf') => {
    try {
      const startDate = auditFilters.startDate || dayjs().subtract(30, 'days').toISOString();
      const endDate = auditFilters.endDate || dayjs().toISOString();
      
      await invoke('export_audit_log', {
        token,
        startDate,
        endDate,
        format,
        filters: auditFilters
      });
      
      setExportModalVisible(false);
    } catch (error) {
      console.error('Failed to export audit log:', error);
    }
  };

  // Get event type color and icon
  const getEventConfig = (eventType: string, result: string) => {
    const isSuccess = result === 'success';
    const isFailure = result === 'failure';
    
    const baseConfig = {
      'asset_creation': { color: '#1890ff', icon: <InfoCircleOutlined /> },
      'asset_modification': { color: '#52c41a', icon: <CheckCircleOutlined /> },
      'asset_deletion': { color: '#ff4d4f', icon: <WarningOutlined /> },
      'security_validation': { color: '#faad14', icon: <ExclamationCircleOutlined /> },
      'classification_change': { color: '#722ed1', icon: <InfoCircleOutlined /> },
      'login': { color: '#52c41a', icon: <CheckCircleOutlined /> },
      'logout': { color: '#d9d9d9', icon: <InfoCircleOutlined /> },
      'permission_change': { color: '#fa8c16', icon: <WarningOutlined /> }
    };
    
    const config = baseConfig[eventType as keyof typeof baseConfig] || { 
      color: '#d9d9d9', 
      icon: <InfoCircleOutlined /> 
    };
    
    if (isFailure) {
      return { ...config, color: '#ff4d4f', icon: <ExclamationCircleOutlined /> };
    }
    
    return config;
  };

  // Filter events based on search text
  const filteredEvents = events.filter(event => {
    if (!searchText) return true;
    
    const searchLower = searchText.toLowerCase();
    return (
      event.eventType.toLowerCase().includes(searchLower) ||
      event.details.toLowerCase().includes(searchLower) ||
      (event.userId && event.userId.toString().includes(searchLower))
    );
  });

  // Paginate events
  const paginatedEvents = filteredEvents.slice(
    (currentPage - 1) * pageSize,
    currentPage * pageSize
  );

  // Table columns
  const columns: ColumnsType<AuditEvent> = [
    {
      title: 'Timestamp',
      dataIndex: 'timestamp',
      key: 'timestamp',
      width: compactView ? 120 : 160,
      render: (timestamp: string) => (
        <Text style={{ fontSize: compactView ? '11px' : '12px' }}>
          {dayjs(timestamp).format(compactView ? 'MM/DD HH:mm' : 'MM/DD/YYYY HH:mm:ss')}
        </Text>
      ),
      sorter: (a, b) => dayjs(a.timestamp).unix() - dayjs(b.timestamp).unix(),
      defaultSortOrder: 'descend'
    },
    {
      title: 'Event Type',
      dataIndex: 'eventType',
      key: 'eventType',
      width: compactView ? 100 : 140,
      render: (eventType: string, record) => {
        const config = getEventConfig(eventType, record.result);
        return (
          <Space size="small">
            <Badge color={config.color} />
            <Text style={{ fontSize: compactView ? '11px' : '12px' }}>
              {eventType.replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase())}
            </Text>
          </Space>
        );
      },
      filters: [
        { text: 'Asset Creation', value: 'asset_creation' },
        { text: 'Asset Modification', value: 'asset_modification' },
        { text: 'Asset Deletion', value: 'asset_deletion' },
        { text: 'Security Validation', value: 'security_validation' },
        { text: 'Classification Change', value: 'classification_change' },
        { text: 'Login', value: 'login' },
        { text: 'Logout', value: 'logout' },
        { text: 'Permission Change', value: 'permission_change' }
      ],
      onFilter: (value, record) => record.eventType === value
    },
    {
      title: 'User',
      dataIndex: 'userId',
      key: 'userId',
      width: compactView ? 80 : 100,
      render: (userId: number) => (
        <Text style={{ fontSize: compactView ? '11px' : '12px' }}>
          {userId ? `User ${userId}` : 'System'}
        </Text>
      )
    },
    {
      title: 'Result',
      dataIndex: 'result',
      key: 'result',
      width: compactView ? 80 : 100,
      render: (result: string) => {
        const color = result === 'success' ? 'success' : 
                     result === 'failure' ? 'error' : 'warning';
        return (
          <Tag color={color} style={{ fontSize: compactView ? '10px' : '11px' }}>
            {result.toUpperCase()}
          </Tag>
        );
      },
      filters: [
        { text: 'Success', value: 'success' },
        { text: 'Failure', value: 'failure' },
        { text: 'Warning', value: 'warning' }
      ],
      onFilter: (value, record) => record.result === value
    },
    {
      title: 'Details',
      dataIndex: 'details',
      key: 'details',
      ellipsis: true,
      render: (details: string) => (
        <Tooltip title={details}>
          <Text style={{ fontSize: compactView ? '11px' : '12px' }}>
            {details.length > (compactView ? 30 : 50) 
              ? `${details.substring(0, compactView ? 30 : 50)}...` 
              : details}
          </Text>
        </Tooltip>
      )
    },
    {
      title: 'Actions',
      key: 'actions',
      width: 80,
      render: (_, record) => (
        <Button
          type="text"
          size="small"
          icon={<EyeOutlined />}
          onClick={() => {
            setSelectedEvent(record);
            setDrawerVisible(true);
          }}
        />
      )
    }
  ];

  if (user?.role !== 'Administrator') {
    return (
      <Alert
        message="Administrator Access Required"
        description="Security audit log is only available to administrators."
        type="warning"
        showIcon
      />
    );
  }

  return (
    <div>
      <Card
        title={
          <Space>
            <Title level={4} style={{ margin: 0 }}>Security Audit Log</Title>
            <Badge count={totalCount} showZero color="#1890ff" />
          </Space>
        }
        extra={
          <Space>
            <Button 
              icon={<ReloadOutlined />} 
              onClick={() => fetchAuditEvents(auditFilters)}
              loading={loading}
            >
              Refresh
            </Button>
            {exportEnabled && (
              <Button 
                icon={<DownloadOutlined />}
                onClick={() => setExportModalVisible(true)}
              >
                Export
              </Button>
            )}
          </Space>
        }
      >
        {/* Filters */}
        {showFilters && (
          <div style={{ marginBottom: 16 }}>
            <Space wrap>
              <Search
                placeholder="Search events..."
                style={{ width: 200 }}
                onSearch={handleSearch}
                allowClear
              />
              
              <RangePicker
                value={auditFilters.startDate && auditFilters.endDate ? [
                  dayjs(auditFilters.startDate),
                  dayjs(auditFilters.endDate)
                ] : undefined}
                onChange={(dates) => {
                  handleFilterChange({
                    startDate: dates?.[0]?.toISOString(),
                    endDate: dates?.[1]?.toISOString()
                  });
                }}
                style={{ width: 240 }}
              />
              
              <Select
                placeholder="Event Type"
                style={{ width: 150 }}
                allowClear
                value={auditFilters.eventType}
                onChange={(eventType) => handleFilterChange({ eventType })}
              >
                <Select.Option value="asset_creation">Asset Creation</Select.Option>
                <Select.Option value="asset_modification">Asset Modification</Select.Option>
                <Select.Option value="asset_deletion">Asset Deletion</Select.Option>
                <Select.Option value="security_validation">Security Validation</Select.Option>
                <Select.Option value="classification_change">Classification Change</Select.Option>
                <Select.Option value="login">Login</Select.Option>
                <Select.Option value="logout">Logout</Select.Option>
              </Select>
              
              <Select
                placeholder="User ID"
                style={{ width: 120 }}
                allowClear
                value={auditFilters.userId}
                onChange={(userId) => handleFilterChange({ userId })}
              >
                <Select.Option value={1}>User 1</Select.Option>
                <Select.Option value={2}>User 2</Select.Option>
                {/* Add more users dynamically */}
              </Select>
            </Space>
          </div>
        )}

        {/* Table */}
        <Table
          columns={columns}
          dataSource={paginatedEvents}
          loading={loading}
          rowKey="id"
          size={compactView ? 'small' : 'middle'}
          pagination={false}
          scroll={{ x: compactView ? 600 : 800 }}
        />

        {/* Pagination */}
        <div style={{ textAlign: 'right', marginTop: 16 }}>
          <Pagination
            current={currentPage}
            pageSize={pageSize}
            total={filteredEvents.length}
            onChange={(page) => setCurrentPage(page)}
            showSizeChanger
            showQuickJumper
            showTotal={(total, range) => 
              `${range[0]}-${range[1]} of ${total} events`
            }
          />
        </div>
      </Card>

      {/* Event Details Drawer */}
      <Drawer
        title="Audit Event Details"
        placement="right"
        width={500}
        open={drawerVisible}
        onClose={() => setDrawerVisible(false)}
      >
        {selectedEvent && (
          <Descriptions column={1} bordered size="small">
            <Descriptions.Item label="Event ID">
              {selectedEvent.id}
            </Descriptions.Item>
            <Descriptions.Item label="Timestamp">
              {dayjs(selectedEvent.timestamp).format('YYYY-MM-DD HH:mm:ss')}
            </Descriptions.Item>
            <Descriptions.Item label="Event Type">
              <Space>
                {getEventConfig(selectedEvent.eventType, selectedEvent.result).icon}
                {selectedEvent.eventType.replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase())}
              </Space>
            </Descriptions.Item>
            <Descriptions.Item label="User">
              {selectedEvent.userId ? `User ${selectedEvent.userId}` : 'System'}
            </Descriptions.Item>
            <Descriptions.Item label="Result">
              <Tag color={selectedEvent.result === 'success' ? 'success' : 
                         selectedEvent.result === 'failure' ? 'error' : 'warning'}>
                {selectedEvent.result.toUpperCase()}
              </Tag>
            </Descriptions.Item>
            <Descriptions.Item label="Details">
              <Text code style={{ whiteSpace: 'pre-wrap' }}>
                {selectedEvent.details}
              </Text>
            </Descriptions.Item>
          </Descriptions>
        )}
      </Drawer>

      {/* Export Modal */}
      <Modal
        title="Export Audit Log"
        open={exportModalVisible}
        onCancel={() => setExportModalVisible(false)}
        footer={null}
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          <Text>Select export format:</Text>
          <Space>
            <Button onClick={() => handleExport('csv')}>
              Export as CSV
            </Button>
            <Button onClick={() => handleExport('json')}>
              Export as JSON
            </Button>
            <Button onClick={() => handleExport('pdf')}>
              Export as PDF
            </Button>
          </Space>
        </Space>
      </Modal>
    </div>
  );
};

export default SecurityAuditLog;