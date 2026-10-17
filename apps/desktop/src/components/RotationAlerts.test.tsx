import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import RotationAlerts from './RotationAlerts';
import { RotationAlert } from '../types/vault';
import useAuthStore from '../store/auth';
import { useConsoleGuard } from '../test-utils/consoleGuard';

const alerts: RotationAlert[] = [
  { secret_id: 1, vault_id: 1, secret_label: 'PLC admin', asset_name: 'PLC-001', days_until_rotation: -3, next_rotation_due: '2024-04-01T00:00:00Z', last_rotated: '2024-01-01T00:00:00Z' },
  { secret_id: 2, vault_id: 1, secret_label: 'HMI operator', asset_name: 'HMI-001', days_until_rotation: 2, next_rotation_due: '2024-04-06T00:00:00Z' },
  { secret_id: 3, vault_id: 2, secret_label: 'Switch console', asset_name: 'SW-001', days_until_rotation: 20, next_rotation_due: '2024-04-24T00:00:00Z' },
];

const metrics = {
  total_passwords: 12,
  overdue_passwords: 1,
  due_within_7_days: 1,
  avg_days_since_rotation: 40,
  compliance_percentage: 91.7,
};

describe('RotationAlerts', () => {
  useConsoleGuard({
    // List migration was explicitly deferred by d052492; Listy is not a drop-in.
    ignore: [/\[antd: List\]/],
  });

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token' });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_rotation_alerts') return alerts;
      if (cmd === 'get_rotation_compliance_metrics') return metrics;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('renders compliance statistics and alerts grouped by urgency', async () => {
    const onRotateClick = vi.fn();
    render(<RotationAlerts daysAhead={30} onRotateClick={onRotateClick} />);

    expect(await screen.findByText('Total Passwords')).toBeInTheDocument();
    expect(screen.getByText('Overdue')).toBeInTheDocument();
    expect(screen.getByText('Due in 7 Days')).toBeInTheDocument();
    expect(screen.getByText('Compliance')).toBeInTheDocument();
    expect(screen.getByText('12')).toBeInTheDocument();
    expect(screen.getByRole('progressbar')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_rotation_alerts', { token: 'test-token', days_ahead: 30 });

    expect(screen.getByText('Password Rotation Alerts')).toBeInTheDocument();
    expect(await screen.findByText('1 Overdue Passwords')).toBeInTheDocument();
    expect(screen.getByText('3 days overdue')).toBeInTheDocument();
    expect(screen.getByText('PLC admin')).toBeInTheDocument();
    expect(screen.getByText('Asset: PLC-001')).toBeInTheDocument();

    expect(screen.getByText(/Urgent \(Due within 7 days\)/)).toBeInTheDocument();
    expect(screen.getByText('HMI operator')).toBeInTheDocument();
    expect(screen.getByText('2 days')).toBeInTheDocument();

    expect(screen.getByText('Upcoming')).toBeInTheDocument();
    expect(screen.getByText('Switch console')).toBeInTheDocument();
    expect(screen.getByText('20 days')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Rotate Now/ }));
    expect(onRotateClick).toHaveBeenCalledWith(alerts[0]);
    fireEvent.click(screen.getByRole('button', { name: /Rotate Early/ }));
    expect(onRotateClick).toHaveBeenCalledWith(alerts[2]);
  });

  it('shows the empty state when nothing needs rotation', async () => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => (cmd === 'get_rotation_alerts' ? [] : metrics));
    render(<RotationAlerts />);

    expect(await screen.findByText('No passwords require rotation')).toBeInTheDocument();
  });
});
