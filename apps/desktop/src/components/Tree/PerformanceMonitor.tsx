import React, { useState, useEffect, useCallback } from 'react';
import { 
  Card, 
  Statistic, 
  Progress, 
  Tooltip, 
  Switch, 
  Row, 
  Col, 
  Alert,
  Tag,
  Space,
  Button,
  Collapse,
  Typography
} from 'antd';
import {
  TrophyOutlined,
  ThunderboltOutlined,
  DatabaseOutlined,
  ClockCircleOutlined,
  BarChartOutlined,
  SettingOutlined,
  InfoCircleOutlined,
  CheckCircleOutlined,
  WarningOutlined,
  ExclamationCircleOutlined
} from '@ant-design/icons';
import { TreePerformanceMonitor, PerformanceRecommendation } from '../../utils/treePerformance';

const { Text, Title } = Typography;
const { Panel } = Collapse;

export interface PerformanceMonitorProps {
  monitor: TreePerformanceMonitor;
  treeSize: number;
  maxDepth: number;
  visible?: boolean;
  onToggleVisibility?: (visible: boolean) => void;
  enableRealtimeUpdates?: boolean;
  compact?: boolean;
}

const PerformanceMonitor: React.FC<PerformanceMonitorProps> = ({
  monitor,
  treeSize,
  maxDepth,
  visible = false,
  onToggleVisibility,
  enableRealtimeUpdates = true,
  compact = false,
}) => {
  const [metrics, setMetrics] = useState(monitor.getPerformanceReport().metrics);
  const [recommendations, setRecommendations] = useState<PerformanceRecommendation[]>([]);
  const [isOptimal, setIsOptimal] = useState(true);

  // Update metrics periodically
  useEffect(() => {
    if (!enableRealtimeUpdates || !visible) return;

    const updateMetrics = () => {
      const report = monitor.getPerformanceReport();
      setMetrics(report.metrics);
      setIsOptimal(report.isOptimal);
      
      const newRecommendations = monitor.getRecommendations(treeSize, maxDepth);
      setRecommendations(newRecommendations);
    };

    updateMetrics();
    const interval = setInterval(updateMetrics, 1000);

    return () => clearInterval(interval);
  }, [monitor, treeSize, maxDepth, enableRealtimeUpdates, visible]);

  const getPerformanceStatus = useCallback(() => {
    if (metrics.renderTime < 8) return { status: 'excellent', color: '#52c41a' };
    if (metrics.renderTime < 16) return { status: 'good', color: '#1890ff' };
    if (metrics.renderTime < 32) return { status: 'fair', color: '#faad14' };
    return { status: 'poor', color: '#f5222d' };
  }, [metrics.renderTime]);

  const getFrameRateStatus = useCallback(() => {
    if (metrics.frameRate >= 55) return { status: 'excellent', color: '#52c41a' };
    if (metrics.frameRate >= 45) return { status: 'good', color: '#1890ff' };
    if (metrics.frameRate >= 30) return { status: 'fair', color: '#faad14' };
    return { status: 'poor', color: '#f5222d' };
  }, [metrics.frameRate]);

  const getMemoryStatus = useCallback(() => {
    const memoryMB = metrics.memoryUsage / (1024 * 1024);
    if (memoryMB < 25) return { status: 'excellent', color: '#52c41a' };
    if (memoryMB < 50) return { status: 'good', color: '#1890ff' };
    if (memoryMB < 100) return { status: 'fair', color: '#faad14' };
    return { status: 'poor', color: '#f5222d' };
  }, [metrics.memoryUsage]);

  const renderRecommendationIcon = (recommendation: PerformanceRecommendation) => {
    switch (recommendation.priority) {
      case 'critical':
        return <ExclamationCircleOutlined style={{ color: '#f5222d' }} />;
      case 'high':
        return <WarningOutlined style={{ color: '#faad14' }} />;
      case 'medium':
        return <InfoCircleOutlined style={{ color: '#1890ff' }} />;
      default:
        return <CheckCircleOutlined style={{ color: '#52c41a' }} />;
    }
  };

  const renderCompactView = () => (
    <div style={{ 
      padding: '8px 12px', 
      borderBottom: '1px solid #f0f0f0',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      background: isOptimal ? '#f6ffed' : '#fff7e6'
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <ThunderboltOutlined style={{ color: getPerformanceStatus().color }} />
        <Text style={{ fontSize: 12 }}>
          {metrics.renderTime.toFixed(1)}ms
        </Text>
        <Text type="secondary" style={{ fontSize: 10 }}>
          | {metrics.frameRate.toFixed(0)}fps
        </Text>
        {recommendations.length > 0 && (
          <Tag size="small" color={recommendations[0].priority === 'critical' ? 'error' : 'warning'}>
            {recommendations.length}
          </Tag>
        )}
      </div>
      <Switch
        size="small"
        checked={visible}
        onChange={onToggleVisibility}
        checkedChildren="📊"
        unCheckedChildren="📊"
      />
    </div>
  );

  const renderFullView = () => (
    <Card
      title={
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <span>
            <BarChartOutlined style={{ marginRight: 8 }} />
            Performance Monitor
          </span>
          <Space>
            <Tag color={isOptimal ? 'success' : 'warning'}>
              {isOptimal ? 'Optimal' : 'Needs Optimization'}
            </Tag>
            <Button
              size="small"
              icon={<SettingOutlined />}
              onClick={() => onToggleVisibility?.(false)}
            >
              Close
            </Button>
          </Space>
        </div>
      }
      size="small"
      style={{ width: '100%', fontSize: 12 }}
    >
      <Row gutter={[16, 16]}>
        {/* Core Performance Metrics */}
        <Col span={24}>
          <Title level={5}>Core Metrics</Title>
          <Row gutter={16}>
            <Col span={6}>
              <Statistic
                title="Render Time"
                value={metrics.renderTime}
                precision={1}
                suffix="ms"
                valueStyle={{ color: getPerformanceStatus().color, fontSize: 16 }}
                prefix={<ThunderboltOutlined />}
              />
              <Progress
                percent={Math.min(100, (metrics.renderTime / 16) * 100)}
                size="small"
                strokeColor={getPerformanceStatus().color}
                showInfo={false}
              />
            </Col>
            <Col span={6}>
              <Statistic
                title="Frame Rate"
                value={metrics.frameRate}
                precision={0}
                suffix="fps"
                valueStyle={{ color: getFrameRateStatus().color, fontSize: 16 }}
                prefix={<TrophyOutlined />}
              />
              <Progress
                percent={(metrics.frameRate / 60) * 100}
                size="small"
                strokeColor={getFrameRateStatus().color}
                showInfo={false}
              />
            </Col>
            <Col span={6}>
              <Statistic
                title="Memory Usage"
                value={metrics.memoryUsage / (1024 * 1024)}
                precision={1}
                suffix="MB"
                valueStyle={{ color: getMemoryStatus().color, fontSize: 16 }}
                prefix={<DatabaseOutlined />}
              />
              <Progress
                percent={Math.min(100, (metrics.memoryUsage / (100 * 1024 * 1024)) * 100)}
                size="small"
                strokeColor={getMemoryStatus().color}
                showInfo={false}
              />
            </Col>
            <Col span={6}>
              <Statistic
                title="Last Update"
                value={metrics.lastMeasurement.toLocaleTimeString()}
                valueStyle={{ fontSize: 14 }}
                prefix={<ClockCircleOutlined />}
              />
            </Col>
          </Row>
        </Col>

        {/* Tree Statistics */}
        <Col span={24}>
          <Collapse size="small" ghost>
            <Panel header="Tree Statistics" key="stats">
              <Row gutter={16}>
                <Col span={8}>
                  <Statistic
                    title="Tree Size"
                    value={treeSize}
                    suffix="nodes"
                    valueStyle={{ fontSize: 14 }}
                  />
                </Col>
                <Col span={8}>
                  <Statistic
                    title="Max Depth"
                    value={maxDepth}
                    suffix="levels"
                    valueStyle={{ fontSize: 14 }}
                  />
                </Col>
                <Col span={8}>
                  <Statistic
                    title="Cache Hit Rate"
                    value={metrics.cacheHitRate * 100}
                    precision={1}
                    suffix="%"
                    valueStyle={{ fontSize: 14 }}
                  />
                </Col>
              </Row>
            </Panel>
          </Collapse>
        </Col>

        {/* Recommendations */}
        {recommendations.length > 0 && (
          <Col span={24}>
            <Alert
              message="Performance Recommendations"
              description={
                <div style={{ marginTop: 8 }}>
                  {recommendations.slice(0, 3).map((rec, index) => (
                    <div key={index} style={{ marginBottom: 8 }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                        {renderRecommendationIcon(rec)}
                        <Text strong>{rec.description}</Text>
                        <Tag size="small" color={
                          rec.priority === 'critical' ? 'error' :
                          rec.priority === 'high' ? 'warning' :
                          rec.priority === 'medium' ? 'processing' : 'default'
                        }>
                          {rec.priority}
                        </Tag>
                      </div>
                      <Text type="secondary" style={{ fontSize: 11, marginLeft: 24 }}>
                        Impact: {rec.impact}
                      </Text>
                    </div>
                  ))}
                  {recommendations.length > 3 && (
                    <Text type="secondary" style={{ fontSize: 11 }}>
                      +{recommendations.length - 3} more recommendations
                    </Text>
                  )}
                </div>
              }
              type={recommendations.some(r => r.priority === 'critical') ? 'error' : 'warning'}
              showIcon
            />
          </Col>
        )}

        {/* Performance Tips */}
        <Col span={24}>
          <Collapse size="small" ghost>
            <Panel header="Performance Tips" key="tips">
              <div style={{ fontSize: 11 }}>
                <div>• Virtualization improves performance for trees with 100+ nodes</div>
                <div>• Lazy loading reduces initial render time for deep trees</div>
                <div>• Search debouncing prevents excessive re-renders</div>
                <div>• Cache warming can improve perceived performance</div>
                <div>• Target: &lt;16ms render time for 60fps experience</div>
              </div>
            </Panel>
          </Collapse>
        </Col>
      </Row>
    </Card>
  );

  if (!visible && compact) {
    return renderCompactView();
  }

  if (!visible) {
    return (
      <Tooltip title="Show Performance Monitor">
        <Button
          size="small"
          icon={<BarChartOutlined />}
          onClick={() => onToggleVisibility?.(true)}
          style={{
            position: 'fixed',
            bottom: 16,
            right: 16,
            zIndex: 1000,
            borderColor: isOptimal ? '#52c41a' : '#faad14',
            color: isOptimal ? '#52c41a' : '#faad14',
          }}
        />
      </Tooltip>
    );
  }

  return renderFullView();
};

export default PerformanceMonitor;