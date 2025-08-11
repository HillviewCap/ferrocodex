import React, { useState, useEffect } from 'react';
import {
  Layout,
  Card,
  Row,
  Col,
  Button,
  Space,
  Typography,
  Statistic,
  Progress,
  Table,
  Alert,
  Tag,
  Tooltip,
  notification,
  Drawer,
  Switch,
  InputNumber,
  Empty,
} from 'antd';
import {
  DashboardOutlined,
  SettingOutlined,
  ReloadOutlined,
  ExpandOutlined,
  ShrinkOutlined,
  BarChartOutlined,
  LineChartOutlined,
} from '@ant-design/icons';
import { 
  BulkImportSession, 
  BulkImportStatus,
  calculateSuccessRate,
} from '../../types/bulk';
import useBulkImportStore from '../../store/bulk';
import BulkProgressTracker from './BulkProgressTracker';
import WorkflowIntegrationPanel from './WorkflowIntegrationPanel';

const { Title, Text } = Typography;

interface DashboardWidget {
  id: string;
  type: 'stats' | 'progress' | 'sessions' | 'integration' | 'chart';
  title: string;
  size: 'small' | 'medium' | 'large';
  position: { x: number; y: number; w: number; h: number };
  config?: Record<string, any>;
  visible: boolean;
}

interface WorkflowDashboardProps {
  fullscreen?: boolean;
  onToggleFullscreen?: () => void;
}

const DEFAULT_WIDGETS: DashboardWidget[] = [
  {
    id: 'overall-stats',
    type: 'stats',
    title: 'Overall Statistics',
    size: 'large',
    position: { x: 0, y: 0, w: 6, h: 4 },
    visible: true,
  },
  {
    id: 'active-sessions',
    type: 'sessions',
    title: 'Active Sessions',
    size: 'medium',
    position: { x: 6, y: 0, w: 6, h: 4 },
    visible: true,
  },
  {
    id: 'workflow-integration',
    type: 'integration',
    title: 'Workflow Integration',
    size: 'medium',
    position: { x: 0, y: 4, w: 12, h: 3 },
    visible: true,
  },
  {
    id: 'recent-activity',
    type: 'chart',
    title: 'Recent Activity',
    size: 'large',
    position: { x: 0, y: 7, w: 12, h: 4 },
    config: { chartType: 'line' },
    visible: true,
  },
];

const WorkflowDashboard: React.FC<WorkflowDashboardProps> = ({
  fullscreen = false,
  onToggleFullscreen,
}) => {
  const [widgets, setWidgets] = useState<DashboardWidget[]>(DEFAULT_WIDGETS);
  const [settingsVisible, setSettingsVisible] = useState(false);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [refreshInterval, setRefreshInterval] = useState(30); // seconds
  const [lastUpdateTime, setLastUpdateTime] = useState<Date | null>(null);
  const [selectedSessionId, setSelectedSessionId] = useState<number | null>(null);

  const {
    sessions,
    stats,
    isLoading,
    error,
    loadSessions,
    loadStats,
    clearError,
  } = useBulkImportStore();

  useEffect(() => {
    // Initial load
    loadDashboardData();
  }, []);

  useEffect(() => {
    let interval: ReturnType<typeof setInterval>;

    if (autoRefresh) {
      interval = setInterval(() => {
        loadDashboardData();
      }, refreshInterval * 1000);
    }

    return () => {
      if (interval) {
        clearInterval(interval);
      }
    };
  }, [autoRefresh, refreshInterval]);

  useEffect(() => {
    if (error) {
      notification.error({
        message: 'Dashboard Error',
        description: error,
      });
      clearError();
    }
  }, [error, clearError]);

  const loadDashboardData = async () => {
    try {
      await Promise.all([
        loadSessions(),
        loadStats(),
      ]);
      setLastUpdateTime(new Date());
    } catch (error) {
      console.error('Failed to load dashboard data:', error);
    }
  };

  const handleManualRefresh = async () => {
    await loadDashboardData();
    notification.success({
      message: 'Dashboard Refreshed',
      description: 'Data has been updated successfully',
    });
  };

  const toggleWidget = (widgetId: string) => {
    setWidgets(prev => prev.map(widget => 
      widget.id === widgetId 
        ? { ...widget, visible: !widget.visible }
        : widget
    ));
  };


  const renderStatsWidget = (_widget: DashboardWidget) => {
    if (!stats) {
      return <Empty description="No statistics available" />;
    }

    const activePercentage = stats.total_sessions > 0 
      ? Math.round((stats.active_sessions / stats.total_sessions) * 100)
      : 0;

    const completedPercentage = stats.total_sessions > 0 
      ? Math.round((stats.completed_sessions / stats.total_sessions) * 100)
      : 0;

    return (
      <Row gutter={[16, 16]}>
        <Col span={12}>
          <Statistic
            title="Total Sessions"
            value={stats.total_sessions}
            prefix={<DashboardOutlined />}
          />
        </Col>
        <Col span={12}>
          <Statistic
            title="Items Processed"
            value={stats.total_items_processed}
            prefix={<BarChartOutlined />}
          />
        </Col>
        <Col span={12}>
          <div>
            <Text type="secondary">Active Sessions</Text>
            <Progress
              percent={activePercentage}
              size="small"
              status="active"
              format={() => `${stats.active_sessions}`}
            />
          </div>
        </Col>
        <Col span={12}>
          <div>
            <Text type="secondary">Completed Sessions</Text>
            <Progress
              percent={completedPercentage}
              size="small"
              status="success"
              format={() => `${stats.completed_sessions}`}
            />
          </div>
        </Col>
        <Col span={12}>
          <Statistic
            title="Success Rate"
            value={stats.success_rate}
            suffix="%"
            precision={1}
            valueStyle={{ color: stats.success_rate >= 80 ? '#3f8600' : '#cf1322' }}
          />
        </Col>
        <Col span={12}>
          <Statistic
            title="Avg Processing Time"
            value={stats.average_processing_time}
            suffix="sec"
            precision={1}
          />
        </Col>
      </Row>
    );
  };

  const renderSessionsWidget = (_widget: DashboardWidget) => {
    const activeSessions = sessions.filter(s => 
      ['Created', 'Validating', 'Processing', 'Paused'].includes(s.status)
    );

    const columns = [
      {
        title: 'Session',
        dataIndex: 'session_name',
        key: 'session_name',
        ellipsis: true,
      },
      {
        title: 'Status',
        dataIndex: 'status',
        key: 'status',
        render: (status: BulkImportStatus) => (
          <Tag color={
            status === 'Processing' ? 'processing' :
            status === 'Completed' ? 'success' :
            status === 'Failed' ? 'error' :
            status === 'Paused' ? 'warning' : 'default'
          }>
            {status}
          </Tag>
        ),
      },
      {
        title: 'Progress',
        key: 'progress',
        render: (_: any, record: BulkImportSession) => {
          const percentage = record.total_items > 0 
            ? Math.round((record.processed_items / record.total_items) * 100)
            : 0;
          return (
            <Progress
              percent={percentage}
              size="small"
              showInfo={false}
            />
          );
        },
      },
      {
        title: 'Actions',
        key: 'actions',
        render: (_: any, record: BulkImportSession) => (
          <Button
            size="small"
            onClick={() => setSelectedSessionId(record.id)}
          >
            View
          </Button>
        ),
      },
    ];

    return (
      <Table
        dataSource={activeSessions}
        columns={columns}
        size="small"
        pagination={false}
        scroll={{ y: 200 }}
        locale={{
          emptyText: 'No active sessions',
        }}
      />
    );
  };

  const renderIntegrationWidget = (_widget: DashboardWidget) => (
    <WorkflowIntegrationPanel
      sessionId={selectedSessionId || undefined}
      compact={true}
    />
  );

  const renderChartWidget = (_widget: DashboardWidget) => {
    // Placeholder for chart implementation
    // In a real implementation, this would use a charting library like Chart.js or Recharts
    const recentSessions = sessions
      .slice(0, 10)
      .map(session => ({
        name: session.session_name,
        processed: session.processed_items,
        failed: session.failed_items,
        success_rate: calculateSuccessRate(session.processed_items, session.failed_items),
      }));

    return (
      <div style={{ textAlign: 'center', padding: '40px 0' }}>
        <LineChartOutlined style={{ fontSize: '48px', color: '#d9d9d9' }} />
        <div style={{ marginTop: '16px' }}>
          <Text type="secondary">Chart visualization will be implemented</Text>
          <br />
          <Text type="secondary">with your preferred charting library</Text>
        </div>
        {recentSessions.length > 0 && (
          <div style={{ marginTop: '16px' }}>
            <Text type="secondary">Data available for {recentSessions.length} recent sessions</Text>
          </div>
        )}
      </div>
    );
  };

  const renderWidget = (widget: DashboardWidget) => {
    if (!widget.visible) return null;

    let content;
    switch (widget.type) {
      case 'stats':
        content = renderStatsWidget(widget);
        break;
      case 'sessions':
        content = renderSessionsWidget(widget);
        break;
      case 'integration':
        content = renderIntegrationWidget(widget);
        break;
      case 'chart':
        content = renderChartWidget(widget);
        break;
      default:
        content = <Empty description="Widget type not implemented" />;
    }

    return (
      <Card
        key={widget.id}
        title={widget.title}
        size="small"
        loading={isLoading}
        style={{ height: '100%' }}
        extra={
          <Tooltip title="Widget Settings">
            <Button
              size="small"
              icon={<SettingOutlined />}
              onClick={() => {
                // Widget-specific settings could be implemented here
                notification.info({
                  message: 'Widget Settings',
                  description: `Settings for ${widget.title} will be implemented`,
                });
              }}
            />
          </Tooltip>
        }
      >
        {content}
      </Card>
    );
  };

  const renderSettings = () => (
    <Drawer
      title="Dashboard Settings"
      placement="right"
      open={settingsVisible}
      onClose={() => setSettingsVisible(false)}
      width={400}
    >
      <Space direction="vertical" style={{ width: '100%' }} size="large">
        <Card size="small" title="Auto Refresh">
          <Space direction="vertical" style={{ width: '100%' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <Text>Enable Auto Refresh</Text>
              <Switch
                checked={autoRefresh}
                onChange={setAutoRefresh}
              />
            </div>
            
            {autoRefresh && (
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <Text>Interval (seconds)</Text>
                <InputNumber
                  min={5}
                  max={300}
                  value={refreshInterval}
                  onChange={(value) => setRefreshInterval(value || 30)}
                />
              </div>
            )}
          </Space>
        </Card>

        <Card size="small" title="Widget Visibility">
          <Space direction="vertical" style={{ width: '100%' }}>
            {widgets.map(widget => (
              <div
                key={widget.id}
                style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}
              >
                <Text>{widget.title}</Text>
                <Switch
                  checked={widget.visible}
                  onChange={() => toggleWidget(widget.id)}
                />
              </div>
            ))}
          </Space>
        </Card>

        <Card size="small" title="Dashboard Layout">
          <Space direction="vertical" style={{ width: '100%' }}>
            <Button
              onClick={() => {
                setWidgets(DEFAULT_WIDGETS);
                notification.success({
                  message: 'Layout Reset',
                  description: 'Dashboard layout has been reset to default',
                });
              }}
              block
            >
              Reset to Default Layout
            </Button>
            
            <Button
              onClick={() => {
                notification.info({
                  message: 'Export Layout',
                  description: 'Layout export functionality will be implemented',
                });
              }}
              block
            >
              Export Layout
            </Button>
          </Space>
        </Card>
      </Space>
    </Drawer>
  );

  return (
    <Layout style={{ height: fullscreen ? '100vh' : 'auto' }}>
      <Layout.Header style={{ background: '#fff', padding: '0 24px', borderBottom: '1px solid #f0f0f0' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', height: '100%' }}>
          <Title level={3} style={{ margin: 0 }}>
            Workflow Dashboard
          </Title>
          
          <Space>
            {lastUpdateTime && (
              <Text type="secondary" style={{ fontSize: '12px' }}>
                Last updated: {lastUpdateTime.toLocaleTimeString()}
              </Text>
            )}
            
            <Button
              icon={<ReloadOutlined />}
              onClick={handleManualRefresh}
              loading={isLoading}
            >
              Refresh
            </Button>
            
            <Button
              icon={<SettingOutlined />}
              onClick={() => setSettingsVisible(true)}
            >
              Settings
            </Button>
            
            {onToggleFullscreen && (
              <Button
                icon={fullscreen ? <ShrinkOutlined /> : <ExpandOutlined />}
                onClick={onToggleFullscreen}
              >
                {fullscreen ? 'Exit Fullscreen' : 'Fullscreen'}
              </Button>
            )}
          </Space>
        </div>
      </Layout.Header>

      <Layout.Content style={{ padding: '24px' }}>
        {error && (
          <Alert
            message="Dashboard Error"
            description={error}
            type="error"
            showIcon
            closable
            style={{ marginBottom: '16px' }}
          />
        )}

        {/* Widget Grid */}
        <Row gutter={[16, 16]}>
          {widgets.filter(w => w.visible).map(widget => (
            <Col
              key={widget.id}
              xs={24}
              sm={widget.size === 'small' ? 8 : widget.size === 'medium' ? 12 : 24}
              lg={widget.size === 'small' ? 6 : widget.size === 'medium' ? 12 : 24}
              style={{ minHeight: '200px' }}
            >
              {renderWidget(widget)}
            </Col>
          ))}
        </Row>

        {/* Progress Tracker Modal for Selected Session */}
        {selectedSessionId && (
          <Drawer
            title="Session Progress"
            placement="right"
            open={!!selectedSessionId}
            onClose={() => setSelectedSessionId(null)}
            width={600}
          >
            <BulkProgressTracker
              sessionId={selectedSessionId}
              autoRefresh={true}
            />
          </Drawer>
        )}
      </Layout.Content>

      {renderSettings()}
    </Layout>
  );
};

export default WorkflowDashboard;