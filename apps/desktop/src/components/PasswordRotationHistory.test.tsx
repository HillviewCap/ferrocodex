import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import PasswordRotationHistoryViewer from './PasswordRotationHistory';
import useAuthStore from '../store/auth';
import { useConsoleGuard } from '../test-utils/consoleGuard';

const history = [
  {
    rotation_id: 12,
    secret_id: 9,
    old_password_hash: 'hash-b',
    rotation_reason: 'Scheduled rotation',
    rotated_by: 2,
    rotated_at: '2024-04-01T10:00:00Z',
    batch_id: 7,
  },
  {
    rotation_id: 11,
    secret_id: 9,
    old_password_hash: 'hash-a',
    rotation_reason: 'Security incident',
    rotated_by: 1,
    rotated_at: '2024-01-01T10:00:00Z',
  },
];

const users: Record<number, { id: number; username: string }> = {
  1: { id: 1, username: 'admin' },
  2: { id: 2, username: 'alice' },
};

describe('PasswordRotationHistory', () => {
  useConsoleGuard();

  beforeEach(() => {
    vi.clearAllMocks();
    useAuthStore.setState({ token: 'test-token' });
    vi.mocked(invoke).mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'get_rotation_history') return history;
      if (cmd === 'get_user_info') return users[args?.user_id as number];
      throw new Error(`Unexpected command: ${cmd}`);
    });
  });

  it('renders the rotation timeline with Descriptions items per entry', async () => {
    render(<PasswordRotationHistoryViewer secretId={9} secretLabel="PLC admin" visible onClose={vi.fn()} />);

    // Space renders the title fragments as separate items
    expect(screen.getByText('Password Rotation History -')).toBeInTheDocument();
    expect(screen.getByText('PLC admin')).toBeInTheDocument();
    expect(await screen.findByText('Scheduled rotation')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('get_rotation_history', { token: 'test-token', secret_id: 9 });

    expect(screen.getByText('Security incident')).toBeInTheDocument();
    expect(screen.getByText('Latest')).toBeInTheDocument();
    expect(screen.getAllByText('Rotated By')).toHaveLength(2);
    expect(screen.getAllByText('Reason')).toHaveLength(2);
    expect(screen.getByText('Batch ID')).toBeInTheDocument();
    expect(screen.getByText('Batch #7')).toBeInTheDocument();
    expect(document.querySelectorAll('.ant-timeline-item')).toHaveLength(2);

    // User names resolve once get_user_info responds
    expect(await screen.findByText('alice')).toBeInTheDocument();
    expect(screen.getByText('admin')).toBeInTheDocument();
  });

  it('shows the empty state when no rotations exist', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    render(<PasswordRotationHistoryViewer secretId={9} secretLabel="PLC admin" visible onClose={vi.fn()} />);

    expect(await screen.findByText('No rotation history found')).toBeInTheDocument();
  });
});
