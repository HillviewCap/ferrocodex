import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import SecurityAlert, { SecurityAlertPanel } from './SecurityAlert';
import useSecurityStore from '../../store/security';
import { SecurityAlert as SecurityAlertType } from '../../types/security';
import { useConsoleGuard } from '../../test-utils/consoleGuard';

const criticalAlert: SecurityAlertType = {
  id: 'alert-1',
  timestamp: new Date(Date.now() - 5 * 60_000).toISOString(),
  severity: 'critical',
  title: 'Asset Creation Failed',
  description: 'Failed to create asset "PLC-001": permission denied',
  assetName: 'PLC-001',
  userName: 'eng',
  acknowledged: false,
};

const warningAlert: SecurityAlertType = {
  id: 'alert-2',
  timestamp: new Date(Date.now() - 3 * 3600_000).toISOString(),
  severity: 'warning',
  title: 'High Classification Asset Created',
  description: 'Asset created with RESTRICTED classification',
  acknowledged: false,
};

describe('SecurityAlert', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useSecurityStore.setState({ alerts: [criticalAlert, warningAlert], unreadAlertCount: 2 });
  });

  it('renders the full alert with severity badge, metadata and actions', () => {
    const onAcknowledge = vi.fn();
    render(<SecurityAlert alert={criticalAlert} onAcknowledge={onAcknowledge} />);

    expect(screen.getByText('Critical')).toBeInTheDocument();
    expect(screen.getByText('Asset Creation Failed')).toBeInTheDocument();
    expect(screen.getByText('Asset: PLC-001')).toBeInTheDocument();
    expect(screen.getByText('Failed to create asset "PLC-001": permission denied')).toBeInTheDocument();
    expect(screen.getByText('5m ago')).toBeInTheDocument();
    expect(screen.getByText('by eng')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Acknowledge/ }));
    expect(onAcknowledge).toHaveBeenCalledWith('alert-1');
    expect(useSecurityStore.getState().alerts.find(a => a.id === 'alert-1')?.acknowledged).toBe(true);
  });

  it('offers Resolve once acknowledged and renders the compact variant', () => {
    const onResolve = vi.fn();
    const { rerender } = render(
      <SecurityAlert alert={{ ...warningAlert, acknowledged: true }} onResolve={onResolve} />,
    );
    expect(screen.getByText('Warning')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Resolve/ }));
    expect(onResolve).toHaveBeenCalledWith('alert-2');

    rerender(<SecurityAlert alert={warningAlert} compact />);
    expect(screen.getByText('High Classification Asset Created')).toBeInTheDocument();
    expect(screen.getByText('3h ago')).toBeInTheDocument();
    expect(screen.queryByText('Acknowledge')).not.toBeInTheDocument();
  });
});

describe('SecurityAlertPanel', () => {
  describe('without alerts', () => {
    useConsoleGuard();

    beforeEach(() => {
      useSecurityStore.setState({ alerts: [], unreadAlertCount: 0 });
    });

    it('opens an empty popover', async () => {
      render(<SecurityAlertPanel />);

      fireEvent.click(screen.getByRole('button'));
      expect(await screen.findByText('Security Alerts')).toBeInTheDocument();
      expect(screen.getByText('No security alerts')).toBeInTheDocument();
    });
  });

  describe('with alerts', () => {
    useConsoleGuard({
      // List migration was explicitly deferred by d052492; Listy is not a drop-in.
      ignore: [/\[antd: List\]/],
    });

    beforeEach(() => {
      useSecurityStore.setState({ alerts: [criticalAlert, warningAlert], unreadAlertCount: 2 });
    });

    it('groups critical and warning alerts in the popover', async () => {
      render(<SecurityAlertPanel />);

      fireEvent.click(screen.getByRole('button'));
      expect(await screen.findByText('CRITICAL (1)')).toBeInTheDocument();
      expect(screen.getByText('WARNINGS (1)')).toBeInTheDocument();
      expect(screen.getByText('Asset Creation Failed')).toBeInTheDocument();
      expect(screen.getByText('High Classification Asset Created')).toBeInTheDocument();
    });
  });
});
