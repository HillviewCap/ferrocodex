import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { describe, test, expect, vi, beforeEach } from 'vitest';
import { App } from 'antd';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import SecurityMonitoringDashboard from './SecurityMonitoringDashboard';
import useAuthStore, { UserInfo } from '../../store/auth';
import { SecurityOverview } from '../../types/security';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

vi.mock('@tauri-apps/plugin-dialog', () => ({ save: vi.fn() }));

const adminUser: UserInfo = {
  id: 1,
  username: 'admin',
  role: 'Administrator',
  created_at: '2026-01-01 00:00:00',
  is_active: true,
};

const makeOverview = (overrides: Partial<SecurityOverview> = {}): SecurityOverview => ({
  period: '24h',
  generatedAt: '2026-10-19 14:30:00',
  status: 'warning',
  authentication: {
    successfulLogins: 12,
    failedLogins: 7,
    lockouts: 1,
    distinctFailedUsernames: 2,
    currentlyLockedAccounts: ['operator1'],
  },
  accessControl: {
    vaultAccessDenied: 3,
    vaultExports: 1,
    secretReads: 9,
    pendingPermissionRequests: 2,
    grantsExpiringSoon: 0,
  },
  users: {
    activeUsers: 5,
    activeAdministrators: 1,
    inactiveUsers: 2,
    usersPendingKeyUpgrade: 0,
    roleChanges: 0,
  },
  encryption: {
    activeKeyVersion: 2,
    activeKeyCreatedAt: '2026-06-01 00:00:00',
    activeKeyAgeDays: 140,
    rotationInProgress: false,
    lastRotationCompletedAt: '2026-06-01 00:05:00',
    recoveryKeyLastReplacedAt: null,
  },
  assets: {
    totalAssets: 17,
    classificationBreakdown: { PUBLIC: 4, INTERNAL: 6, CONFIDENTIAL: 3, RESTRICTED: 2, SECRET: 2 },
  },
  findings: [
    {
      id: 'accounts-locked',
      severity: 'warning',
      title: 'Accounts locked out',
      detail: 'Currently locked: operator1',
    },
    {
      id: 'single-administrator',
      severity: 'warning',
      title: 'Only one active administrator',
      detail: 'Losing that account locks out administration.',
    },
  ],
  recentEvents: [
    {
      id: 42,
      timestamp: '2026-10-19 14:00:00',
      eventCode: 'USER_006',
      category: 'user',
      severity: 'critical',
      username: 'operator1',
      description: 'Account locked after repeated failed logins',
    },
    {
      id: 41,
      timestamp: '2026-10-19 13:55:00',
      eventCode: 'AUTH_002',
      category: 'authentication',
      severity: 'warning',
      username: null,
      description: 'Failed login for unknown user',
    },
  ],
  ...overrides,
});

const overviewCalls = () =>
  vi.mocked(invoke).mock.calls.filter(([cmd]) => cmd === 'get_security_overview');

// The component reads message from App.useApp(), so render inside <App> as the real app does
const renderDashboard = (props: React.ComponentProps<typeof SecurityMonitoringDashboard> = {}) =>
  render(
    <App>
      <SecurityMonitoringDashboard {...props} />
    </App>,
  );

describe('SecurityMonitoringDashboard', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token', user: adminUser, isAuthenticated: true });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_security_overview') return makeOverview();
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  test('shows access denied and never fetches for non-administrators', () => {
    useAuthStore.setState({ user: { ...adminUser, id: 2, username: 'engineer', role: 'Engineer' } });

    renderDashboard();

    expect(screen.getByText('Administrator Access Required')).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();
  });

  test('fetches the overview for the default period on mount', async () => {
    renderDashboard();

    await screen.findByText('Accounts locked out');
    expect(invoke).toHaveBeenCalledWith('get_security_overview', { token: 'test-token', period: '24h' });
    expect(overviewCalls()).toHaveLength(1);
  });

  test('refetches with the new period when the period changes', async () => {
    renderDashboard();
    await screen.findByText('Accounts locked out');

    fireEvent.mouseDown(screen.getByLabelText('Reporting period'));
    const option = await waitFor(() => {
      const el = Array.from(document.querySelectorAll('.ant-select-item-option'))
        .find(node => node.textContent?.trim() === 'Last 7 days');
      expect(el).toBeTruthy();
      return el as HTMLElement;
    });
    fireEvent.click(option);

    await waitFor(() =>
      expect(invoke).toHaveBeenLastCalledWith('get_security_overview', { token: 'test-token', period: '7d' }),
    );
  });

  test('Refresh refetches the overview', async () => {
    renderDashboard();
    await screen.findByText('Accounts locked out');
    expect(overviewCalls()).toHaveLength(1);

    fireEvent.click(screen.getByRole('button', { name: /Refresh/ }));

    await waitFor(() => expect(overviewCalls()).toHaveLength(2));
  });

  test('renders status, metrics, findings, classification and events from the overview', async () => {
    const onOpenAuditLog = vi.fn();
    renderDashboard({ onOpenAuditLog });

    await screen.findByText('Accounts locked out');

    expect(screen.getByText('Security warnings need attention')).toBeInTheDocument();
    expect(screen.getByText('0 critical issues · 2 warnings · 0 informational notes')).toBeInTheDocument();
    expect(screen.getByText('Failed logins (last 24 hours)')).toBeInTheDocument();
    expect(screen.getByText('Locked now: operator1')).toBeInTheDocument();
    expect(screen.getByText('2 pending access requests')).toBeInTheDocument();
    expect(screen.getByText('1 administrator · 2 inactive')).toBeInTheDocument();
    expect(screen.getByText('140 days old')).toBeInTheDocument();

    expect(screen.getByText('Only one active administrator')).toBeInTheDocument();
    expect(screen.getByText('Losing that account locks out administration.')).toBeInTheDocument();

    expect(screen.getByText('17 assets')).toBeInTheDocument();
    const levels = ['PUBLIC', 'INTERNAL', 'CONFIDENTIAL', 'RESTRICTED', 'SECRET'];
    const rows = levels.map(level => screen.getByTestId(`classification-${level}`));
    // Fixed order, lowest to highest
    rows.slice(1).forEach((row, i) => {
      expect(rows[i].compareDocumentPosition(row) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    });
    expect(within(rows[0]).getByText('Public')).toBeInTheDocument();
    expect(within(rows[0]).getByText('4')).toBeInTheDocument();
    expect(within(rows[1]).getByText('6')).toBeInTheDocument();
    expect(within(rows[4]).getByText('Secret')).toBeInTheDocument();

    expect(screen.getByText('Account locked after repeated failed logins')).toBeInTheDocument();
    expect(screen.getByText('Failed login for unknown user')).toBeInTheDocument();
    expect(screen.getByText('System')).toBeInTheDocument();
    // Backend timestamps are UTC and shown in local time
    expect(screen.getByText(new Date(Date.UTC(2026, 9, 19, 14, 0, 0)).toLocaleString())).toBeInTheDocument();
    expect(screen.getByText(`Last updated: ${new Date(Date.UTC(2026, 9, 19, 14, 30, 0)).toLocaleString()}`)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Open audit log' }));
    expect(onOpenAuditLog).toHaveBeenCalled();
  });

  test('shows an empty findings state and a critical banner', async () => {
    vi.mocked(invoke).mockResolvedValue(makeOverview({ findings: [], status: 'ok', recentEvents: [] }));
    const { unmount } = renderDashboard();

    expect(await screen.findByText('No issues found')).toBeInTheDocument();
    expect(screen.getByText('No security issues detected')).toBeInTheDocument();
    expect(screen.getByText('No security events in this period')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Open audit log' })).not.toBeInTheDocument();
    unmount();

    vi.mocked(invoke).mockResolvedValue(makeOverview({
      status: 'critical',
      findings: [{
        id: 'rotation-in-progress',
        severity: 'critical',
        title: 'Key rotation in progress',
        detail: 'Encrypted data is unavailable until an administrator finishes the rotation.',
      }],
      encryption: { ...makeOverview().encryption, rotationInProgress: true },
    }));
    renderDashboard();

    expect(await screen.findByText('Critical security issues need immediate attention')).toBeInTheDocument();
    expect(screen.getByText('Rotation in progress')).toBeInTheDocument();
  });

  test('shows an error with Retry when the fetch fails, and recovers', async () => {
    vi.mocked(invoke).mockRejectedValueOnce('Database not initialized');
    renderDashboard();

    expect(await screen.findByText('Failed to load security data')).toBeInTheDocument();
    expect(screen.getByText('Database not initialized')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Retry' }));

    expect(await screen.findByText('Accounts locked out')).toBeInTheDocument();
    expect(screen.queryByText('Failed to load security data')).not.toBeInTheDocument();
  });

  test('keeps the last good data visible when a refresh fails', async () => {
    renderDashboard();
    await screen.findByText('Accounts locked out');

    vi.mocked(invoke).mockRejectedValueOnce('Session expired');
    fireEvent.click(screen.getByRole('button', { name: /Refresh/ }));

    expect(await screen.findByText('Could not refresh security data')).toBeInTheDocument();
    expect(screen.getByText('Accounts locked out')).toBeInTheDocument();
  });

  test('exports the report to the chosen file', async () => {
    vi.mocked(save).mockResolvedValue('/tmp/security-report.json');
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_security_overview') return makeOverview();
      if (cmd === 'export_security_report') return '/tmp/security-report.json';
      throw new Error(`Unexpected command: ${cmd}`);
    });
    renderDashboard();
    await screen.findByText('Accounts locked out');

    fireEvent.click(screen.getByRole('button', { name: /Export Report/ }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('export_security_report', {
        token: 'test-token',
        period: '24h',
        filePath: '/tmp/security-report.json',
      }),
    );
    expect(vi.mocked(save).mock.calls[0][0]?.defaultPath).toMatch(/^security-report-24h-\d{8}\.json$/);
    expect(await screen.findByText('Security report exported to /tmp/security-report.json')).toBeInTheDocument();
  });

  test('does nothing when the save dialog is cancelled', async () => {
    vi.mocked(save).mockResolvedValue(null);
    renderDashboard();
    await screen.findByText('Accounts locked out');

    fireEvent.click(screen.getByRole('button', { name: /Export Report/ }));

    await waitFor(() => expect(save).toHaveBeenCalled());
    expect(invoke).not.toHaveBeenCalledWith('export_security_report', expect.anything());
  });
});
