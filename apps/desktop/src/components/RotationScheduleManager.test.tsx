import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import RotationScheduleManager from './RotationScheduleManager';
import useAuthStore from '../store/auth';
import { useConsoleGuard } from '../test-utils/consoleGuard';

const admin = { id: 1, username: 'admin', role: 'Administrator' as const, created_at: '2024-01-01', is_active: true };
const engineer = { ...admin, id: 2, username: 'eng', role: 'Engineer' as const };

const schedule = {
  schedule_id: 4,
  vault_id: 1,
  rotation_interval: 90,
  alert_days_before: 14,
  is_active: true,
  created_at: '2024-01-01T00:00:00Z',
  created_by: 1,
  updated_at: '2024-02-01T00:00:00Z',
};

describe('RotationScheduleManager', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token', user: admin });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_rotation_schedule') return schedule;
      if (cmd === 'update_rotation_policy') return undefined;
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('loads the existing schedule into the form for an administrator', async () => {
    render(<RotationScheduleManager vaultId={1} vaultName="PLC Vault" />);

    // Space renders the title fragments as separate items
    expect(screen.getByText('Rotation Schedule -')).toBeInTheDocument();
    expect(screen.getByText('PLC Vault')).toBeInTheDocument();
    expect(screen.getByText('Password Rotation Policy')).toBeInTheDocument();
    expect(await screen.findByText('90 days')).toBeInTheDocument();
    expect(screen.getByText('14 days before')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_rotation_schedule', { token: 'test-token', vaultId: 1 });

    expect(screen.getByText('Rotation Interval')).toBeInTheDocument();
    expect(screen.getByText('Alert Before Rotation')).toBeInTheDocument();
    expect(screen.getByText('Schedule Status')).toBeInTheDocument();
    expect(screen.getByRole('switch')).toBeChecked();
    expect(screen.getByText('Next Rotation Due:')).toBeInTheDocument();
    expect(screen.getByText('Alert Will Be Sent On:')).toBeInTheDocument();
    expect(screen.getByText('Created On:')).toBeInTheDocument();
    expect(screen.getByText('Last Updated:')).toBeInTheDocument();

    expect(screen.getByRole('button', { name: /Save Schedule/ })).toBeDisabled();
    expect(screen.queryByText('View Only')).not.toBeInTheDocument();
  });

  it('enables saving after a change and updates the policy', async () => {
    const onUpdate = vi.fn();
    render(<RotationScheduleManager vaultId={1} vaultName="PLC Vault" onUpdate={onUpdate} />);
    await screen.findByText('90 days');

    fireEvent.click(screen.getByRole('switch'));
    const save = screen.getByRole('button', { name: /Save Schedule/ });
    await waitFor(() => expect(save).toBeEnabled());
    fireEvent.click(save);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('update_rotation_policy', {
        token: 'test-token',
        request: { schedule_id: 4, rotation_interval: 90, alert_days_before: 14, is_active: false },
      });
    });
    expect(await screen.findByText('Schedule Updated')).toBeInTheDocument();
    expect(onUpdate).toHaveBeenCalled();
  });

  it('is read-only for engineers', async () => {
    useAuthStore.setState({ user: engineer });
    render(<RotationScheduleManager vaultId={1} vaultName="PLC Vault" />);

    expect(await screen.findByText('View Only')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Save Schedule/ })).not.toBeInTheDocument();
    expect(screen.getByRole('switch')).toBeDisabled();
  });
});
