import React, { useState, useEffect, useCallback, useRef } from 'react';
import {
  App,
  Card,
  Row,
  Col,
  Statistic,
  Progress,
  Badge,
  Space,
  Typography,
  Alert,
  Button,
  Select,
  Spin,
  Table,
  Tag,
  Empty
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import {
  SafetyCertificateOutlined,
  LoginOutlined,
  LockOutlined,
  TeamOutlined,
  KeyOutlined,
  ReloadOutlined,
  DownloadOutlined
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import dayjs from 'dayjs';
import {
  SecurityOverview,
  SecurityOverviewPeriod,
  SecurityEventSummary,
  SecuritySeverity,
  SecurityClassificationLevel,
  SECURITY_CLASSIFICATIONS
} from '../../types/security';
import useAuthStore from '../../store/auth';
import { formatUtcTimestamp, formatUtcDate } from '../../utils/utcTimestamp';

const { Title, Text } = Typography;

interface SecurityMonitoringDashboardProps {
  refreshInterval?: number; // ms
  onOpenAuditLog?: () => void;
}

const PERIOD_LABELS: Record<SecurityOverviewPeriod, string> = {
  '24h': 'last 24 hours',
  '7d': 'last 7 days',
  '30d': 'last 30 days'
};

const SEVERITY_TAG_COLORS: Record<SecuritySeverity, string> = {
  critical: 'red',
  warning: 'orange',
  info: 'blue'
};

const CLASSIFICATION_ORDER: SecurityClassificationLevel[] = [
  SecurityClassificationLevel.PUBLIC,
  SecurityClassificationLevel.INTERNAL,
  SecurityClassificationLevel.CONFIDENTIAL,
  SecurityClassificationLevel.RESTRICTED,
  SecurityClassificationLevel.SECRET
];

const SeverityTag: React.FC<{ severity: SecuritySeverity }> = ({ severity }) => (
  <Tag color={SEVERITY_TAG_COLORS[severity]}>{severity.toUpperCase()}</Tag>
);

const plural = (count: number, word: string) => `${count} ${word}${count === 1 ? '' : 's'}`;

const SecurityMonitoringDashboard: React.FC<SecurityMonitoringDashboardProps> = ({
  refreshInterval = 30000, // 30 seconds
  onOpenAuditLog
}) => {
  const { token, user } = useAuthStore();
  const { message } = App.useApp();
  const isAdmin = user?.role === 'Administrator';

  const [period, setPeriod] = useState<SecurityOverviewPeriod>('24h');
  const [overview, setOverview] = useState<SecurityOverview | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);

  // Only the newest request may update state; bumping this discards older responses,
  // including ones that resolve after unmount
  const latestRequest = useRef(0);
  const inFlight = useRef(false);

  const fetchOverview = useCallback(async (background = false) => {
    if (!isAdmin || !token) return;
    // Auto-refresh never stacks on top of a request that is still running
    if (background && inFlight.current) return;

    const requestId = ++latestRequest.current;
    inFlight.current = true;
    setLoading(true);
    try {
      const result = await invoke<SecurityOverview>('get_security_overview', { token, period });
      if (requestId !== latestRequest.current) return;
      setOverview(result);
      setError(null);
    } catch (err) {
      if (requestId !== latestRequest.current) return;
      setError(String(err));
    } finally {
      if (requestId === latestRequest.current) {
        inFlight.current = false;
        setLoading(false);
      }
    }
  }, [isAdmin, token, period]);

  useEffect(() => {
    fetchOverview();
    const interval = setInterval(() => fetchOverview(true), refreshInterval);
    return () => {
      clearInterval(interval);
      latestRequest.current++;
      inFlight.current = false;
    };
  }, [fetchOverview, refreshInterval]);

  const handleExportReport = async () => {
    if (!token) return;
    try {
      const filePath = await save({
        defaultPath: `security-report-${period}-${dayjs().format('YYYYMMDD')}.json`,
        filters: [{ name: 'JSON files', extensions: ['json'] }]
      });
      if (!filePath) return;

      setExporting(true);
      const writtenPath = await invoke<string>('export_security_report', { token, period, filePath });
      message.success(`Security report exported to ${writtenPath}`);
    } catch (err) {
      message.error(`Failed to export security report: ${err}`);
    } finally {
      setExporting(false);
    }
  };

  if (!isAdmin) {
    return (
      <Alert
        title="Administrator Access Required"
        description="Security monitoring dashboard is only available to administrators."
        type="warning"
        showIcon
      />
    );
  }

  const header = (
    <Row justify="space-between" align="middle" style={{ marginBottom: 24 }}>
      <Col>
        <Title level={3} style={{ margin: 0 }}>
          <Space>
            <SafetyCertificateOutlined />
            Security Monitoring Dashboard
          </Space>
        </Title>
      </Col>
      <Col>
        <Space>
          <Select
            value={period}
            onChange={setPeriod}
            style={{ width: 140 }}
            aria-label="Reporting period"
            options={[
              { value: '24h', label: 'Last 24 hours' },
              { value: '7d', label: 'Last 7 days' },
              { value: '30d', label: 'Last 30 days' }
            ]}
          />
          <Button icon={<ReloadOutlined />} onClick={() => fetchOverview()} loading={loading}>
            Refresh
          </Button>
          <Button icon={<DownloadOutlined />} onClick={handleExportReport} loading={exporting}>
            Export Report
          </Button>
        </Space>
      </Col>
    </Row>
  );

  const errorAlert = error && (
    <Alert
      type={overview ? 'warning' : 'error'}
      showIcon
      title={overview ? 'Could not refresh security data' : 'Failed to load security data'}
      description={overview
        ? `${error}. Showing data from ${formatUtcTimestamp(overview.generatedAt)}.`
        : error}
      action={<Button size="small" onClick={() => fetchOverview()}>Retry</Button>}
      style={{ marginBottom: 24 }}
    />
  );

  if (!overview) {
    return (
      <div style={{ padding: 24 }}>
        {header}
        {errorAlert}
        {!error && (
          <div style={{ textAlign: 'center', padding: 48 }}>
            <Spin size="large" />
          </div>
        )}
      </div>
    );
  }

  const { authentication, accessControl, users, encryption, assets, findings } = overview;
  // Label counts with the period the data was computed for, which lags the selector
  // until a refetch completes
  const periodLabel = PERIOD_LABELS[overview.period] ?? overview.period;

  const criticalCount = findings.filter(f => f.severity === 'critical').length;
  const warningCount = findings.filter(f => f.severity === 'warning').length;
  const infoCount = findings.filter(f => f.severity === 'info').length;
  const findingSummary = [
    plural(criticalCount, 'critical issue'),
    plural(warningCount, 'warning'),
    plural(infoCount, 'informational note')
  ].join(' · ');

  const statusBanner = {
    ok: { type: 'success' as const, title: 'No security issues detected' },
    warning: { type: 'warning' as const, title: 'Security warnings need attention' },
    critical: { type: 'error' as const, title: 'Critical security issues need immediate attention' }
  }[overview.status] ?? { type: 'info' as const, title: `Security status: ${overview.status}` };

  const classificationTotal = Math.max(
    assets.totalAssets,
    CLASSIFICATION_ORDER.reduce((sum, level) => sum + (assets.classificationBreakdown[level] ?? 0), 0)
  );

  const eventColumns: ColumnsType<SecurityEventSummary> = [
    {
      title: 'Time',
      dataIndex: 'timestamp',
      key: 'timestamp',
      width: 190,
      render: (timestamp: string) => <Text style={{ fontSize: 12 }}>{formatUtcTimestamp(timestamp)}</Text>
    },
    {
      title: 'Severity',
      dataIndex: 'severity',
      key: 'severity',
      width: 100,
      render: (severity: SecuritySeverity) => <SeverityTag severity={severity} />
    },
    {
      title: 'User',
      dataIndex: 'username',
      key: 'username',
      width: 140,
      render: (username: string | null) => username ?? <Text type="secondary">System</Text>
    },
    {
      title: 'Description',
      dataIndex: 'description',
      key: 'description'
    }
  ];

  return (
    <div style={{ padding: 24 }}>
      {header}
      {errorAlert}

      <Alert
        type={statusBanner.type}
        showIcon
        title={statusBanner.title}
        description={findingSummary}
        style={{ marginBottom: 24 }}
      />

      {/* Key metrics */}
      <Row gutter={[16, 16]} style={{ marginBottom: 24 }}>
        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title={`Failed logins (${periodLabel})`}
              value={authentication.failedLogins}
              styles={{ content: { color: authentication.failedLogins > 0 ? '#fa8c16' : undefined } }}
              prefix={<LoginOutlined />}
            />
            <Space orientation="vertical" size={0}>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {plural(authentication.lockouts, 'lockout')} · {authentication.successfulLogins} successful
              </Text>
              <Text type={authentication.currentlyLockedAccounts.length > 0 ? 'danger' : 'secondary'} style={{ fontSize: 12 }}>
                {authentication.currentlyLockedAccounts.length > 0
                  ? `Locked now: ${authentication.currentlyLockedAccounts.join(', ')}`
                  : 'No accounts locked'}
              </Text>
            </Space>
          </Card>
        </Col>

        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title={`Vault access denied (${periodLabel})`}
              value={accessControl.vaultAccessDenied}
              styles={{ content: { color: accessControl.vaultAccessDenied > 0 ? '#fa8c16' : undefined } }}
              prefix={<LockOutlined />}
            />
            <Space orientation="vertical" size={0}>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {plural(accessControl.pendingPermissionRequests, 'pending access request')}
              </Text>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {plural(accessControl.vaultExports, 'vault export')} · {plural(accessControl.secretReads, 'secret read')}
              </Text>
            </Space>
          </Card>
        </Col>

        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title="Active users"
              value={users.activeUsers}
              prefix={<TeamOutlined />}
            />
            <Space orientation="vertical" size={0}>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {plural(users.activeAdministrators, 'administrator')} · {users.inactiveUsers} inactive
              </Text>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {plural(users.roleChanges, 'role change')} ({periodLabel})
              </Text>
            </Space>
          </Card>
        </Col>

        <Col xs={24} sm={12} lg={6}>
          <Card>
            <Statistic
              title="Encryption key"
              value={encryption.activeKeyVersion != null ? `v${encryption.activeKeyVersion}` : 'None'}
              styles={{ content: { color: encryption.rotationInProgress ? '#ff4d4f' : undefined } }}
              prefix={<KeyOutlined />}
            />
            <Space orientation="vertical" size={0}>
              {encryption.rotationInProgress ? (
                <Text type="danger" style={{ fontSize: 12 }}>Rotation in progress</Text>
              ) : (
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {encryption.activeKeyAgeDays != null
                    ? `${plural(encryption.activeKeyAgeDays, 'day')} old`
                    : 'Key age unknown'}
                </Text>
              )}
              <Text type="secondary" style={{ fontSize: 12 }}>
                {encryption.lastRotationCompletedAt
                  ? `Last rotation: ${formatUtcDate(encryption.lastRotationCompletedAt)}`
                  : 'Never rotated'}
              </Text>
            </Space>
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]} style={{ marginBottom: 24 }}>
        <Col xs={24} lg={14}>
          <Card title="Findings" size="small" style={{ height: '100%' }}>
            {findings.length === 0 ? (
              <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="No issues found" />
            ) : (
              <Space orientation="vertical" size="middle" style={{ width: '100%' }}>
                {findings.map(finding => (
                  <div key={finding.id} data-testid={`finding-${finding.id}`} style={{ display: 'flex', gap: 8, alignItems: 'flex-start' }}>
                    <SeverityTag severity={finding.severity} />
                    <div>
                      <Text strong>{finding.title}</Text>
                      <br />
                      <Text type="secondary" style={{ fontSize: 12 }}>{finding.detail}</Text>
                    </div>
                  </div>
                ))}
              </Space>
            )}
          </Card>
        </Col>

        <Col xs={24} lg={10}>
          <Card
            title="Security Classification Distribution"
            size="small"
            extra={<Text type="secondary">{plural(assets.totalAssets, 'asset')}</Text>}
            style={{ height: '100%' }}
          >
            {CLASSIFICATION_ORDER.map(level => {
              const { displayName, color } = SECURITY_CLASSIFICATIONS[level];
              const count = assets.classificationBreakdown[level] ?? 0;
              return (
                <div key={level} data-testid={`classification-${level}`} style={{ marginBottom: 8 }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                    <Badge color={color} text={displayName} />
                    <Text strong>{count}</Text>
                  </div>
                  <Progress
                    percent={classificationTotal > 0 ? (count / classificationTotal) * 100 : 0}
                    strokeColor={color}
                    size="small"
                    showInfo={false}
                  />
                </div>
              );
            })}
          </Card>
        </Col>
      </Row>

      <Card
        title={`Recent Security Events (${periodLabel})`}
        size="small"
        extra={onOpenAuditLog && (
          <Button type="link" size="small" onClick={onOpenAuditLog}>
            Open audit log
          </Button>
        )}
      >
        <Table
          columns={eventColumns}
          dataSource={overview.recentEvents}
          rowKey="id"
          size="small"
          pagination={false}
          locale={{ emptyText: 'No security events in this period' }}
        />
      </Card>

      <div style={{ textAlign: 'right', marginTop: 16, fontSize: '12px', color: '#666' }}>
        Last updated: {formatUtcTimestamp(overview.generatedAt)}
      </div>
    </div>
  );
};

export default SecurityMonitoringDashboard;
