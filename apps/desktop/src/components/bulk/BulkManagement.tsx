import React, { useState, useEffect } from 'react';
import {
  Layout,
  Card,
  Button,
  Space,
  Typography,
  Table,
  Tag,
  Popconfirm,
  notification,
  Row,
  Col,
  Statistic,
  Tabs,
  Alert,
  Empty,
  Badge,
} from 'antd';
import {
  PlusOutlined,
  PlayCircleOutlined,
  EyeOutlined,
  DeleteOutlined,
  DashboardOutlined,
  SettingOutlined,
  FileTextOutlined,
  BarChartOutlined,
  ImportOutlined,
} from '@ant-design/icons';
import { 
  BulkImportSession, 
  BulkImportStatus, 
  calculateSuccessRate,
} from '../../types/bulk';
import useBulkImportStore from '../../store/bulk';
import BulkImportWizard from './BulkImportWizard';
import ImportTemplateManager from './ImportTemplateManager';
import WorkflowDashboard from './WorkflowDashboard';
import BulkProgressTracker from './BulkProgressTracker';
import WorkflowIntegrationPanel from './WorkflowIntegrationPanel';

const { Title, Text } = Typography;

const BulkManagement: React.FC = () => {
  const [wizardVisible, setWizardVisible] = useState(false);
  const [templateManagerVisible, setTemplateManagerVisible] = useState(false);
  const [dashboardFullscreen, setDashboardFullscreen] = useState(false);
  const [selectedSessionId, setSelectedSessionId] = useState<number | null>(null);
  const [activeTab, setActiveTab] = useState('sessions');

  const {
    sessions,
    stats,
    isLoading,
    error,
    loadSessions,
    loadStats,
    deleteSession,
    clearError,
  } = useBulkImportStore();

  useEffect(() => {
    loadInitialData();
  }, []);

  useEffect(() => {
    if (error) {
      notification.error({
        message: 'Bulk Management Error',
        description: error,
      });
      clearError();
    }
  }, [error, clearError]);

  const loadInitialData = async () => {
    try {
      await Promise.all([
        loadSessions(),
        loadStats(),
      ]);
    } catch (error) {
      console.error('Failed to load initial data:', error);
    }
  };

  const handleDeleteSession = async (sessionId: number) => {
    try {
      await deleteSession(sessionId);
      notification.success({
        message: 'Session Deleted',
        description: 'Bulk import session has been deleted successfully',
      });
    } catch (error) {
      // Error handled by store
    }
  };

  const handleWizardComplete = (sessionId: number) => {
    setSelectedSessionId(sessionId);
    setActiveTab('progress');
    notification.success({
      message: 'Import Started',
      description: 'Your bulk import has been started successfully',
    });
  };

  const getStatusColor = (status: BulkImportStatus) => {
    switch (status) {
      case 'Created': return 'default';
      case 'Validating': return 'processing';
      case 'Processing': return 'processing';
      case 'Paused': return 'warning';
      case 'Completed': return 'success';
      case 'Failed': return 'error';
      case 'Cancelled': return 'default';
      default: return 'default';
    }
  };

  const sessionColumns = [
    {
      title: 'Session Name',
      dataIndex: 'session_name',
      key: 'session_name',
      render: (text: string, record: BulkImportSession) => (
        <Space direction="vertical" size={0}>
          <Text strong>{text}</Text>
          <Text type="secondary" style={{ fontSize: '12px' }}>
            {record.import_type} • Created {new Date(record.created_at).toLocaleDateString()}
          </Text>
        </Space>
      ),
    },
    {
      title: 'Status',
      dataIndex: 'status',
      key: 'status',
      render: (status: BulkImportStatus) => (
        <Tag color={getStatusColor(status)}>{status}</Tag>
      ),
    },
    {
      title: 'Progress',
      key: 'progress',
      render: (_: any, record: BulkImportSession) => {
        // Progress calculation not displayed in this view
        const successRate = calculateSuccessRate(record.processed_items, record.failed_items);
        
        return (
          <Space direction="vertical" size={0}>
            <Text>{record.processed_items} / {record.total_items} items</Text>
            <Text type="secondary" style={{ fontSize: '12px' }}>
              Success rate: {successRate}%
            </Text>
          </Space>
        );
      },
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_: any, record: BulkImportSession) => (
        <Space>
          <Button
            icon={<EyeOutlined />}
            size="small"
            onClick={() => {
              setSelectedSessionId(record.id);
              setActiveTab('progress');
            }}
          >
            View
          </Button>
          <Popconfirm
            title="Delete Session"
            description="Are you sure you want to delete this session?"
            onConfirm={() => handleDeleteSession(record.id)}
            okText="Delete"
            cancelText="Cancel"
            okType="danger"
          >
            <Button
              icon={<DeleteOutlined />}
              size="small"
              danger
            />
          </Popconfirm>
        </Space>
      ),
    },
  ];

  const renderOverview = () => (
    <Space direction="vertical" style={{ width: '100%' }} size="large">
      {/* Quick Stats */}
      {stats && (
        <Row gutter={[16, 16]}>
          <Col xs={12} sm={6}>
            <Card size="small">
              <Statistic
                title="Total Sessions"
                value={stats.total_sessions}
                prefix={<FileTextOutlined />}
              />
            </Card>
          </Col>
          <Col xs={12} sm={6}>
            <Card size="small">
              <Statistic
                title="Active Sessions"
                value={stats.active_sessions}
                prefix={<PlayCircleOutlined />}
                valueStyle={{ color: '#1890ff' }}
              />
            </Card>
          </Col>
          <Col xs={12} sm={6}>
            <Card size="small">
              <Statistic
                title="Items Processed"
                value={stats.total_items_processed}
                prefix={<BarChartOutlined />}
                valueStyle={{ color: '#52c41a' }}
              />
            </Card>
          </Col>
          <Col xs={12} sm={6}>
            <Card size="small">
              <Statistic
                title="Success Rate"
                value={stats.success_rate}
                suffix="%"
                precision={1}
                prefix={<BarChartOutlined />}
                valueStyle={{ 
                  color: stats.success_rate >= 80 ? '#52c41a' : '#faad14' 
                }}
              />
            </Card>
          </Col>
        </Row>
      )}

      {/* Sessions Table */}
      <Card 
        title="Recent Sessions"
        extra={
          <Space>
            <Button
              icon={<SettingOutlined />}
              onClick={() => setTemplateManagerVisible(true)}
            >
              Templates
            </Button>
            <Button
              type="primary"
              icon={<PlusOutlined />}
              onClick={() => setWizardVisible(true)}
            >
              New Import
            </Button>
          </Space>
        }
      >
        <Table
          columns={sessionColumns}
          dataSource={sessions}
          loading={isLoading}
          rowKey="id"
          pagination={{
            showSizeChanger: true,
            showQuickJumper: true,
            showTotal: (total, range) =>
              `${range[0]}-${range[1]} of ${total} sessions`,
          }}
          locale={{
            emptyText: (
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description="No import sessions yet"
                children={
                  <Button
                    type="primary"
                    icon={<PlusOutlined />}
                    onClick={() => setWizardVisible(true)}
                  >
                    Create Your First Import
                  </Button>
                }
              />
            ),
          }}
        />
      </Card>
    </Space>
  );

  const renderProgress = () => {
    if (!selectedSessionId) {
      return (
        <Card>
          <Empty
            description="No session selected"
            children={
              <Button onClick={() => setActiveTab('sessions')}>
                Go to Sessions
              </Button>
            }
          />
        </Card>
      );
    }

    return (
      <Space direction="vertical" style={{ width: '100%' }} size="large">
        <BulkProgressTracker
          sessionId={selectedSessionId}
          onStatusChange={(status) => {
            if (['Completed', 'Failed', 'Cancelled'].includes(status)) {
              // Refresh sessions when status changes to final state
              loadSessions();
            }
          }}
        />
        
        <WorkflowIntegrationPanel
          sessionId={selectedSessionId}
          showControls={true}
        />
      </Space>
    );
  };

  const renderDashboard = () => (
    <WorkflowDashboard
      fullscreen={dashboardFullscreen}
      onToggleFullscreen={() => setDashboardFullscreen(!dashboardFullscreen)}
    />
  );

  if (dashboardFullscreen) {
    return renderDashboard();
  }

  return (
    <Layout>
      <Layout.Content style={{ padding: '24px' }}>
        <div style={{ marginBottom: '24px' }}>
          <Title level={2}>
            <ImportOutlined /> Bulk Import Management
          </Title>
          <Text type="secondary">
            Manage bulk import operations for large-scale asset onboarding
          </Text>
        </div>

        {error && (
          <Alert
            message="Error"
            description={error}
            type="error"
            showIcon
            closable
            style={{ marginBottom: '16px' }}
            onClose={clearError}
          />
        )}

        <Tabs 
          activeKey={activeTab} 
          onChange={setActiveTab}
          items={[
            {
              key: 'sessions',
              label: (
                <span>
                  <FileTextOutlined />
                  Sessions
                  {stats && stats.active_sessions > 0 && (
                    <Badge count={stats.active_sessions} style={{ marginLeft: 8 }} />
                  )}
                </span>
              ),
              children: renderOverview(),
            },
            {
              key: 'progress',
              label: (
                <span>
                  <PlayCircleOutlined />
                  Progress
                </span>
              ),
              children: renderProgress(),
            },
            {
              key: 'dashboard',
              label: (
                <span>
                  <DashboardOutlined />
                  Dashboard
                </span>
              ),
              children: renderDashboard(),
            },
          ]}
        />

        {/* Modals and Drawers */}
        <BulkImportWizard
          visible={wizardVisible}
          onClose={() => setWizardVisible(false)}
          onComplete={handleWizardComplete}
        />

        <ImportTemplateManager
          visible={templateManagerVisible}
          onClose={() => setTemplateManagerVisible(false)}
        />
      </Layout.Content>
    </Layout>
  );
};

export default BulkManagement;